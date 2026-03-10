use std::time::Duration;

use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use sqlx::PgPool;

use crate::config::Config;
use crate::db::notification_logs::{self, NotificationFailureUpdate};

pub fn spawn(pool: PgPool, config: Config) {
    let Some(smtp_host) = config.smtp_host.clone() else {
        tracing::info!("SMTP is not configured; email dispatcher disabled");
        return;
    };

    tokio::spawn(async move {
        let mailer = match build_mailer(&config, &smtp_host) {
            Ok(mailer) => mailer,
            Err(error) => {
                tracing::error!(error = %error, "Failed to configure SMTP mailer");
                return;
            }
        };

        let from = match config.email_from.parse::<Mailbox>() {
            Ok(from) => from,
            Err(error) => {
                tracing::error!(error = %error, "Invalid EMAIL_FROM value");
                return;
            }
        };

        let mut ticker =
            tokio::time::interval(Duration::from_secs(config.email_dispatch_interval_secs));
        ticker.tick().await;

        loop {
            ticker.tick().await;

            match notification_logs::claim_pending(&pool, config.email_dispatch_batch_size).await {
                Ok(notifications) => {
                    for notification in notifications {
                        let subject = notification
                            .subject
                            .clone()
                            .unwrap_or_else(|| "StatusPage notification".to_string());

                        let recipient = match notification.recipient_email.parse::<Mailbox>() {
                            Ok(recipient) => recipient,
                            Err(error) => {
                                let _ = notification_logs::mark_failed(
                                    &pool,
                                    notification.id,
                                    NotificationFailureUpdate {
                                        attempt_count: notification.attempt_count,
                                        max_attempts: notification.max_attempts,
                                        error_message: Some(&format!(
                                            "Invalid recipient email: {error}"
                                        )),
                                        next_retry_at: None,
                                    },
                                )
                                .await;
                                continue;
                            }
                        };

                        let message = match Message::builder()
                            .from(from.clone())
                            .to(recipient)
                            .subject(subject)
                            .body(notification.body_text.clone())
                        {
                            Ok(message) => message,
                            Err(error) => {
                                let _ = notification_logs::mark_failed(
                                    &pool,
                                    notification.id,
                                    NotificationFailureUpdate {
                                        attempt_count: notification.attempt_count,
                                        max_attempts: notification.max_attempts,
                                        error_message: Some(&format!(
                                            "Failed to build email message: {error}"
                                        )),
                                        next_retry_at: None,
                                    },
                                )
                                .await;
                                continue;
                            }
                        };

                        match mailer.send(message).await {
                            Ok(_) => {
                                if let Err(error) =
                                    notification_logs::mark_sent(&pool, notification.id).await
                                {
                                    tracing::warn!(
                                        error = %error,
                                        notification_id = %notification.id,
                                        "Failed to mark notification email as sent"
                                    );
                                }
                            }
                            Err(error) => {
                                if let Err(mark_error) = notification_logs::mark_failed(
                                    &pool,
                                    notification.id,
                                    NotificationFailureUpdate {
                                        attempt_count: notification.attempt_count,
                                        max_attempts: notification.max_attempts,
                                        error_message: Some(&error.to_string()),
                                        next_retry_at: next_retry_at(
                                            notification.attempt_count,
                                            notification.max_attempts,
                                        ),
                                    },
                                )
                                .await
                                {
                                    tracing::warn!(
                                        error = %mark_error,
                                        notification_id = %notification.id,
                                        "Failed to record email delivery failure"
                                    );
                                }
                            }
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, "Failed to claim pending email notifications");
                }
            }
        }
    });
}

fn build_mailer(
    config: &Config,
    smtp_host: &str,
) -> anyhow::Result<AsyncSmtpTransport<Tokio1Executor>> {
    let builder = if config.smtp_secure {
        AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_host)?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(smtp_host)
    };

    let builder = builder.port(config.smtp_port);

    let builder = match (&config.smtp_username, &config.smtp_password) {
        (Some(username), Some(password)) => {
            builder.credentials(Credentials::new(username.clone(), password.clone()))
        }
        _ => builder,
    };

    Ok(builder.build())
}

fn next_retry_at(attempt_count: i32, max_attempts: i32) -> Option<chrono::DateTime<chrono::Utc>> {
    notification_logs::next_retry_at(attempt_count, max_attempts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_schedule_matches_notification_log_policy() {
        let first = next_retry_at(1, 5).expect("first retry should exist");
        let second = next_retry_at(2, 5).expect("second retry should exist");
        let final_attempt = next_retry_at(5, 5);

        assert!(second > first);
        assert!(final_attempt.is_none());
    }
}
