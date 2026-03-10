use std::time::Duration;

use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::db::webhook_deliveries::{self, DeliveryFailureUpdate, PendingWebhookDelivery};

type HmacSha256 = Hmac<Sha256>;

pub fn spawn(pool: PgPool, config: Config) {
    tokio::spawn(async move {
        let client = match Client::builder()
            .timeout(Duration::from_secs(config.webhook_timeout_secs))
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                tracing::error!(error = %error, "Failed to build webhook HTTP client");
                return;
            }
        };

        let mut ticker =
            tokio::time::interval(Duration::from_secs(config.webhook_dispatch_interval_secs));
        ticker.tick().await;

        loop {
            ticker.tick().await;

            match webhook_deliveries::claim_pending(&pool, config.webhook_dispatch_batch_size).await
            {
                Ok(deliveries) => {
                    for delivery in deliveries {
                        if let Err(error) = deliver_once(&pool, &client, delivery).await {
                            tracing::warn!(error = %error, "Webhook delivery attempt failed");
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, "Failed to claim pending webhook deliveries");
                }
            }
        }
    });
}

async fn deliver_once(
    pool: &PgPool,
    client: &Client,
    delivery: PendingWebhookDelivery,
) -> anyhow::Result<()> {
    let payload_text = serde_json::to_string(&delivery.payload.0)?;
    let timestamp = Utc::now().timestamp().to_string();
    let signature = sign_payload(&delivery.secret, &timestamp, &payload_text);

    let response = client
        .post(&delivery.url)
        .header("Content-Type", "application/json")
        .header("User-Agent", "StatusPage-Webhooks/1.0")
        .header("X-StatusPage-Delivery", delivery.delivery_id.to_string())
        .header("X-StatusPage-Event", &delivery.event_type)
        .header("X-StatusPage-Timestamp", &timestamp)
        .header("X-StatusPage-Signature-256", format!("sha256={signature}"))
        .body(payload_text)
        .send()
        .await;

    match response {
        Ok(response) => {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            let body_text = truncate(&body_text, 4000);

            if status.is_success() {
                webhook_deliveries::mark_success(
                    pool,
                    delivery.delivery_id,
                    Some(status.as_u16() as i32),
                    body_text.as_deref(),
                )
                .await?;
            } else {
                webhook_deliveries::mark_failure(
                    pool,
                    delivery.delivery_id,
                    DeliveryFailureUpdate {
                        attempt_count: delivery.attempt_count,
                        max_attempts: delivery.max_attempts,
                        response_status_code: Some(status.as_u16() as i32),
                        response_body: body_text.as_deref(),
                        error_message: Some(&format!(
                            "Webhook endpoint returned HTTP {}",
                            status.as_u16()
                        )),
                        next_retry_at: next_retry_at(
                            delivery.delivery_id,
                            delivery.attempt_count,
                            delivery.max_attempts,
                        ),
                    },
                )
                .await?;
            }
        }
        Err(error) => {
            webhook_deliveries::mark_failure(
                pool,
                delivery.delivery_id,
                DeliveryFailureUpdate {
                    attempt_count: delivery.attempt_count,
                    max_attempts: delivery.max_attempts,
                    response_status_code: None,
                    response_body: None,
                    error_message: Some(&error.to_string()),
                    next_retry_at: next_retry_at(
                        delivery.delivery_id,
                        delivery.attempt_count,
                        delivery.max_attempts,
                    ),
                },
            )
            .await?;
        }
    }

    Ok(())
}

fn next_retry_at(
    _delivery_id: Uuid,
    attempt_count: i32,
    max_attempts: i32,
) -> Option<chrono::DateTime<chrono::Utc>> {
    if attempt_count >= max_attempts {
        return None;
    }

    let delay_secs = retry_delay_secs(attempt_count);
    Some(Utc::now() + chrono::Duration::seconds(delay_secs))
}

fn retry_delay_secs(attempt_count: i32) -> i64 {
    match attempt_count {
        0 | 1 => 15,
        2 => 60,
        3 => 300,
        _ => 900,
    }
}

fn sign_payload(secret: &str, timestamp: &str, payload: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any size");
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(payload.as_bytes());
    let signature = mac.finalize().into_bytes();
    hex::encode(signature)
}

fn truncate(value: &str, max_len: usize) -> Option<String> {
    if value.is_empty() {
        return None;
    }

    Some(value.chars().take(max_len).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_schedule_grows_by_attempt() {
        assert_eq!(retry_delay_secs(1), 15);
        assert_eq!(retry_delay_secs(2), 60);
        assert_eq!(retry_delay_secs(3), 300);
        assert_eq!(retry_delay_secs(4), 900);
    }

    #[test]
    fn signature_is_stable_for_same_input() {
        let first = sign_payload("secret", "123", "{\"ok\":true}");
        let second = sign_payload("secret", "123", "{\"ok\":true}");
        assert_eq!(first, second);
        assert!(!first.is_empty());
    }
}
