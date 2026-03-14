use shared::enums::{IncidentImpact, IncidentStatus, ServiceStatus};
use shared::error::AppError;

use crate::db;

struct PublicUrls {
    base: String,
    history: String,
    verify_prefix: String,
    unsubscribe_prefix: String,
}

pub async fn queue_subscription_verification(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    app_base_url: &str,
    org_slug: &str,
    org_name: &str,
    email: &str,
    verification_token: &str,
) -> Result<(), AppError> {
    let public_urls = public_urls(pool, org_id, app_base_url, org_slug).await?;
    let verify_link = format!("{}{}", public_urls.verify_prefix, verification_token);

    let subject = format!("Confirm your subscription to {org_name}");
    let body = format!(
        "You requested updates for {org_name}.\n\nConfirm your subscription:\n{verify_link}\n\nIf you did not request this, you can ignore this email."
    );

    db::notification_logs::enqueue(
        pool,
        org_id,
        "subscriber_verification",
        "subscriber",
        email,
        &subject,
        &body,
    )
    .await
}

pub async fn queue_incident_created(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    app_base_url: &str,
    org_slug: &str,
    title: &str,
    impact: IncidentImpact,
    affected_services: &[uuid::Uuid],
) -> Result<(), AppError> {
    if !email_event_enabled(pool, org_id, "incident.created").await? {
        return Ok(());
    }

    let public_urls = public_urls(pool, org_id, app_base_url, org_slug).await?;
    let subscribers = db::subscribers::find_verified_by_org(pool, org_id).await?;
    let services = service_names(pool, affected_services).await?;

    for subscriber in subscribers {
        let unsubscribe_link = format!(
            "{}{}",
            public_urls.unsubscribe_prefix, subscriber.unsubscribe_token
        );
        let subject = format!("New incident: {title}");
        let affected = if services.is_empty() {
            "n/a".to_string()
        } else {
            services.join(", ")
        };
        let body = format!(
            "A new incident has been posted.\n\nTitle: {title}\nImpact: {}\nAffected services: {affected}\n\nFollow updates at:\n{}\n\nUnsubscribe:\n{unsubscribe_link}",
            impact.as_str(),
            public_urls.history,
        );
        db::notification_logs::enqueue(
            pool,
            org_id,
            "incident_created",
            "subscriber",
            &subscriber.email,
            &subject,
            &body,
        )
        .await?;
    }

    Ok(())
}

pub async fn queue_incident_updated(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    app_base_url: &str,
    org_slug: &str,
    status: IncidentStatus,
    message: &str,
) -> Result<(), AppError> {
    let event_key = if status == IncidentStatus::Resolved {
        "incident.resolved"
    } else {
        "incident.updated"
    };
    if !email_event_enabled(pool, org_id, event_key).await? {
        return Ok(());
    }

    let public_urls = public_urls(pool, org_id, app_base_url, org_slug).await?;
    let subscribers = db::subscribers::find_verified_by_org(pool, org_id).await?;
    for subscriber in subscribers {
        let unsubscribe_link = format!(
            "{}{}",
            public_urls.unsubscribe_prefix, subscriber.unsubscribe_token
        );
        let subject = if status == IncidentStatus::Resolved {
            "Incident resolved".to_string()
        } else {
            format!("Incident update: {}", status.as_str())
        };
        let notification_type = if status == IncidentStatus::Resolved {
            "incident_resolved"
        } else {
            "incident_updated"
        };
        let body = format!(
            "An incident update has been posted.\n\nStatus: {}\nMessage: {}\n\nFollow updates at:\n{}\n\nUnsubscribe:\n{unsubscribe_link}",
            status.as_str(),
            message,
            public_urls.history,
        );
        db::notification_logs::enqueue(
            pool,
            org_id,
            notification_type,
            "subscriber",
            &subscriber.email,
            &subject,
            &body,
        )
        .await?;
    }

    Ok(())
}

pub async fn queue_service_status_changed(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    app_base_url: &str,
    org_slug: &str,
    service_name: &str,
    old_status: ServiceStatus,
    new_status: ServiceStatus,
) -> Result<(), AppError> {
    if !email_event_enabled(pool, org_id, "service.status_changed").await? {
        return Ok(());
    }

    let public_urls = public_urls(pool, org_id, app_base_url, org_slug).await?;
    let subscribers = db::subscribers::find_verified_by_org(pool, org_id).await?;
    for subscriber in subscribers {
        let unsubscribe_link = format!(
            "{}{}",
            public_urls.unsubscribe_prefix, subscriber.unsubscribe_token
        );
        let subject = format!("Service status changed: {service_name}");
        let body = format!(
            "{service_name} changed from {} to {}.\n\nSee the latest status at:\n{}\n\nUnsubscribe:\n{unsubscribe_link}",
            old_status.as_str(),
            new_status.as_str(),
            public_urls.base,
        );
        db::notification_logs::enqueue(
            pool,
            org_id,
            "service_status_changed",
            "subscriber",
            &subscriber.email,
            &subject,
            &body,
        )
        .await?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn queue_invitation_email(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    app_base_url: &str,
    org_name: &str,
    invitation_id: uuid::Uuid,
    email: &str,
    role: shared::enums::MemberRole,
    token: &str,
) -> Result<(), AppError> {
    let invite_link = format!("{}/invite/{}", app_base_url.trim_end_matches('/'), token);
    let subject = format!("Join {org_name} on StatusPage");
    let body = format!(
        "You were invited to join {org_name} as a {role}.\n\nAccept the invitation:\n{invite_link}\n\nSign in with the GitHub account that matches this email address."
    );

    db::notification_logs::enqueue(
        pool,
        org_id,
        "invitation_email",
        &format!("invitation:{invitation_id}"),
        email,
        &subject,
        &body,
    )
    .await
}

async fn email_event_enabled(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    event: &str,
) -> Result<bool, AppError> {
    let preferences = db::notification_preferences::get_or_create(pool, org_id).await?;

    Ok(match event {
        "incident.created" => preferences.email_on_incident_created,
        "incident.updated" => preferences.email_on_incident_updated,
        "incident.resolved" => preferences.email_on_incident_resolved,
        "service.status_changed" => preferences.email_on_service_status_changed,
        _ => false,
    })
}

async fn service_names(
    pool: &sqlx::PgPool,
    service_ids: &[uuid::Uuid],
) -> Result<Vec<String>, AppError> {
    if service_ids.is_empty() {
        return Ok(Vec::new());
    }

    let services = sqlx::query_scalar::<_, String>(
        "SELECT name FROM services WHERE id = ANY($1) ORDER BY name",
    )
    .bind(service_ids)
    .fetch_all(pool)
    .await?;

    Ok(services)
}

async fn public_urls(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    app_base_url: &str,
    org_slug: &str,
) -> Result<PublicUrls, AppError> {
    let custom_domain = sqlx::query_scalar::<_, Option<String>>(
        "SELECT custom_domain FROM organizations WHERE id = $1 AND custom_domain_status = 'verified'",
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await?
    .flatten()
    .map(|domain| domain.trim().trim_end_matches('/').to_lowercase())
    .filter(|domain| !domain.is_empty());

    let base = if let Some(domain) = custom_domain {
        format!("https://{domain}")
    } else {
        format!("{}/s/{org_slug}", app_base_url.trim_end_matches('/'))
    };

    Ok(PublicUrls {
        history: format!("{base}/history"),
        verify_prefix: format!("{base}/verify?token="),
        unsubscribe_prefix: format!("{base}/unsubscribe?token="),
        base,
    })
}
