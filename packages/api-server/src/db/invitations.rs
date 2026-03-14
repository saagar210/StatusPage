use chrono::{Duration, Utc};
use shared::enums::MemberRole;
use shared::error::AppError;
use shared::models::invitation::{Invitation, InvitationWithInviter};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create(
    pool: &PgPool,
    org_id: Uuid,
    email: &str,
    role: MemberRole,
    invited_by: Uuid,
    token: &str,
) -> Result<InvitationWithInviter, AppError> {
    let invitation = sqlx::query_as::<_, InvitationWithInviter>(
        r#"
        INSERT INTO invitations (org_id, email, role, invited_by, token, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
            id,
            org_id,
            email,
            role,
            invited_by,
            token,
            expires_at,
            accepted_at,
            canceled_at,
            last_sent_at,
            created_at,
            updated_at,
            NULL::text AS inviter_name,
            ''::text AS inviter_email,
            'pending'::varchar AS delivery_status
        "#,
    )
    .bind(org_id)
    .bind(email)
    .bind(role)
    .bind(invited_by)
    .bind(token)
    .bind(Utc::now() + Duration::days(7))
    .fetch_one(pool)
    .await?;

    get_by_id(pool, invitation.id)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("invitation lookup failed")))
}

pub async fn list_by_org(
    pool: &PgPool,
    org_id: Uuid,
) -> Result<Vec<InvitationWithInviter>, AppError> {
    let invitations = sqlx::query_as::<_, InvitationWithInviter>(
        r#"
        SELECT
            i.id,
            i.org_id,
            i.email,
            i.role,
            i.invited_by,
            i.token,
            i.expires_at,
            i.accepted_at,
            i.canceled_at,
            i.last_sent_at,
            i.created_at,
            i.updated_at,
            u.name AS inviter_name,
            u.email AS inviter_email,
            (
            CASE
                WHEN i.accepted_at IS NOT NULL THEN 'accepted'
                WHEN i.canceled_at IS NOT NULL THEN 'canceled'
                WHEN i.expires_at <= NOW() THEN 'expired'
                WHEN EXISTS (
                    SELECT 1
                    FROM notification_logs nl
                    WHERE nl.org_id = i.org_id
                      AND nl.recipient_type = ('invitation:' || i.id::text)
                      AND nl.status = 'failed'
                ) THEN 'delivery_failed'
                ELSE 'pending'
            END
            )::varchar AS delivery_status
        FROM invitations i
        JOIN users u ON u.id = i.invited_by
        WHERE i.org_id = $1
        ORDER BY i.created_at DESC
        "#,
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?;

    Ok(invitations)
}

pub async fn find_active_by_email(
    pool: &PgPool,
    org_id: Uuid,
    email: &str,
) -> Result<Option<Invitation>, AppError> {
    let invitation = sqlx::query_as::<_, Invitation>(
        r#"
        SELECT *
        FROM invitations
        WHERE org_id = $1
          AND lower(email) = lower($2)
          AND accepted_at IS NULL
          AND canceled_at IS NULL
          AND expires_at > NOW()
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(org_id)
    .bind(email)
    .fetch_optional(pool)
    .await?;

    Ok(invitation)
}

pub async fn find_by_token(pool: &PgPool, token: &str) -> Result<Option<Invitation>, AppError> {
    let invitation = sqlx::query_as::<_, Invitation>("SELECT * FROM invitations WHERE token = $1")
        .bind(token)
        .fetch_optional(pool)
        .await?;

    Ok(invitation)
}

pub async fn mark_accepted(pool: &PgPool, id: Uuid) -> Result<Invitation, AppError> {
    let invitation = sqlx::query_as::<_, Invitation>(
        r#"
        UPDATE invitations
        SET accepted_at = NOW(), updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(invitation)
}

pub async fn cancel_scoped(pool: &PgPool, org_id: Uuid, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query(
        r#"
        UPDATE invitations
        SET canceled_at = NOW(), updated_at = NOW()
        WHERE org_id = $1 AND id = $2 AND accepted_at IS NULL AND canceled_at IS NULL
        "#,
    )
    .bind(org_id)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Invitation not found".to_string()));
    }

    Ok(())
}

pub async fn touch_last_sent_at(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE invitations
        SET last_sent_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn find_by_id(
    pool: &PgPool,
    org_id: Uuid,
    invitation_id: Uuid,
) -> Result<Option<InvitationWithInviter>, AppError> {
    let invitation = sqlx::query_as::<_, InvitationWithInviter>(
        r#"
        SELECT
            i.id,
            i.org_id,
            i.email,
            i.role,
            i.invited_by,
            i.token,
            i.expires_at,
            i.accepted_at,
            i.canceled_at,
            i.last_sent_at,
            i.created_at,
            i.updated_at,
            u.name AS inviter_name,
            u.email AS inviter_email,
            (
            CASE
                WHEN i.accepted_at IS NOT NULL THEN 'accepted'
                WHEN i.canceled_at IS NOT NULL THEN 'canceled'
                WHEN i.expires_at <= NOW() THEN 'expired'
                WHEN EXISTS (
                    SELECT 1
                    FROM notification_logs nl
                    WHERE nl.org_id = i.org_id
                      AND nl.recipient_type = ('invitation:' || i.id::text)
                      AND nl.status = 'failed'
                ) THEN 'delivery_failed'
                ELSE 'pending'
            END
            )::varchar AS delivery_status
        FROM invitations i
        JOIN users u ON u.id = i.invited_by
        WHERE i.org_id = $1 AND i.id = $2
        "#,
    )
    .bind(org_id)
    .bind(invitation_id)
    .fetch_optional(pool)
    .await?;

    Ok(invitation)
}

async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<InvitationWithInviter>, AppError> {
    let invitation = sqlx::query_as::<_, InvitationWithInviter>(
        r#"
        SELECT
            i.id,
            i.org_id,
            i.email,
            i.role,
            i.invited_by,
            i.token,
            i.expires_at,
            i.accepted_at,
            i.canceled_at,
            i.last_sent_at,
            i.created_at,
            i.updated_at,
            u.name AS inviter_name,
            u.email AS inviter_email,
            (
            CASE
                WHEN i.accepted_at IS NOT NULL THEN 'accepted'
                WHEN i.canceled_at IS NOT NULL THEN 'canceled'
                WHEN i.expires_at <= NOW() THEN 'expired'
                WHEN EXISTS (
                    SELECT 1
                    FROM notification_logs nl
                    WHERE nl.org_id = i.org_id
                      AND nl.recipient_type = ('invitation:' || i.id::text)
                      AND nl.status = 'failed'
                ) THEN 'delivery_failed'
                ELSE 'pending'
            END
            )::varchar AS delivery_status
        FROM invitations i
        JOIN users u ON u.id = i.invited_by
        WHERE i.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(invitation)
}
