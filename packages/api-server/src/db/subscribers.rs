use serde::Serialize;
use shared::error::AppError;
use shared::models::subscriber::Subscriber;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SubscriberListItem {
    pub id: Uuid,
    pub email: String,
    pub is_verified: bool,
    pub verification_sent_at: Option<chrono::DateTime<chrono::Utc>>,
    pub verified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_or_refresh_pending(
    pool: &PgPool,
    org_id: Uuid,
    email: &str,
    verification_token: &str,
    unsubscribe_token: &str,
) -> Result<(Subscriber, bool), AppError> {
    let existing = sqlx::query_as::<_, Subscriber>(
        "SELECT * FROM subscribers WHERE org_id = $1 AND lower(email) = lower($2)",
    )
    .bind(org_id)
    .bind(email)
    .fetch_optional(pool)
    .await?;

    if let Some(subscriber) = existing {
        if subscriber.is_verified {
            return Ok((subscriber, false));
        }

        let updated = sqlx::query_as::<_, Subscriber>(
            r#"
            UPDATE subscribers
            SET
                verification_token = $3,
                verification_sent_at = NOW(),
                unsubscribe_token = COALESCE(unsubscribe_token, $4),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(subscriber.id)
        .bind(org_id)
        .bind(verification_token)
        .bind(unsubscribe_token)
        .fetch_one(pool)
        .await?;

        return Ok((updated, true));
    }

    let subscriber = sqlx::query_as::<_, Subscriber>(
        r#"
        INSERT INTO subscribers (
            org_id,
            email,
            verification_token,
            verification_sent_at,
            unsubscribe_token
        )
        VALUES ($1, $2, $3, NOW(), $4)
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(email)
    .bind(verification_token)
    .bind(unsubscribe_token)
    .fetch_one(pool)
    .await?;

    Ok((subscriber, true))
}

pub async fn verify(
    pool: &PgPool,
    org_id: Uuid,
    token: &str,
) -> Result<Option<Subscriber>, AppError> {
    let subscriber = sqlx::query_as::<_, Subscriber>(
        r#"
        UPDATE subscribers
        SET
            is_verified = TRUE,
            verification_token = NULL,
            verified_at = NOW(),
            updated_at = NOW()
        WHERE org_id = $1 AND verification_token = $2
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(subscriber)
}

pub async fn unsubscribe(
    pool: &PgPool,
    org_id: Uuid,
    token: &str,
) -> Result<Option<Subscriber>, AppError> {
    let subscriber = sqlx::query_as::<_, Subscriber>(
        r#"
        DELETE FROM subscribers
        WHERE org_id = $1 AND unsubscribe_token = $2
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(subscriber)
}

pub async fn find_verified_by_org(
    pool: &PgPool,
    org_id: Uuid,
) -> Result<Vec<Subscriber>, AppError> {
    let subscribers = sqlx::query_as::<_, Subscriber>(
        r#"
        SELECT *
        FROM subscribers
        WHERE org_id = $1 AND is_verified = TRUE
        ORDER BY created_at DESC
        "#,
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?;

    Ok(subscribers)
}

pub async fn list_by_org(pool: &PgPool, org_id: Uuid) -> Result<Vec<SubscriberListItem>, AppError> {
    let subscribers = sqlx::query_as::<_, SubscriberListItem>(
        r#"
        SELECT
            id,
            email,
            is_verified,
            verification_sent_at,
            verified_at,
            created_at,
            updated_at
        FROM subscribers
        WHERE org_id = $1
        ORDER BY
            is_verified DESC,
            COALESCE(verified_at, verification_sent_at, created_at) DESC,
            created_at DESC
        "#,
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?;

    Ok(subscribers)
}

pub async fn refresh_pending_verification_by_id(
    pool: &PgPool,
    org_id: Uuid,
    id: Uuid,
    verification_token: &str,
) -> Result<Option<Subscriber>, AppError> {
    let subscriber = sqlx::query_as::<_, Subscriber>(
        r#"
        UPDATE subscribers
        SET
            verification_token = $3,
            verification_sent_at = NOW(),
            updated_at = NOW()
        WHERE org_id = $1
          AND id = $2
          AND is_verified = FALSE
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(id)
    .bind(verification_token)
    .fetch_optional(pool)
    .await?;

    Ok(subscriber)
}

pub async fn delete_by_id(pool: &PgPool, org_id: Uuid, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM subscribers WHERE org_id = $1 AND id = $2")
        .bind(org_id)
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Subscriber not found".to_string()));
    }

    Ok(())
}
