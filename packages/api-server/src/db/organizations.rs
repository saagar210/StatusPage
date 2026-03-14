use chrono::{DateTime, Utc};
use shared::enums::{CustomDomainStatus, DowngradeState, OrganizationPlan, SubscriptionStatus};
use shared::error::AppError;
use shared::models::organization::{
    CreateOrganizationRequest, Organization, UpdateOrganizationRequest,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct BillingSyncUpdate<'a> {
    pub stripe_customer_id: Option<&'a str>,
    pub stripe_subscription_id: Option<&'a str>,
    pub subscription_status: SubscriptionStatus,
    pub stripe_price_id: Option<&'a str>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub billing_email: Option<&'a str>,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub plan: OrganizationPlan,
}

#[derive(Debug, Clone)]
pub struct DowngradeLifecycle {
    pub target_plan: Option<OrganizationPlan>,
    pub started_at: Option<DateTime<Utc>>,
    pub grace_ends_at: Option<DateTime<Utc>>,
    pub state: DowngradeState,
    pub warning_stage: i32,
}

pub async fn create(
    pool: &PgPool,
    req: &CreateOrganizationRequest,
    slug: &str,
) -> Result<Organization, AppError> {
    let org = sqlx::query_as::<_, Organization>(
        r#"
        INSERT INTO organizations (name, slug)
        VALUES ($1, $2)
        RETURNING *
        "#,
    )
    .bind(&req.name)
    .bind(slug)
    .fetch_one(pool)
    .await?;

    Ok(org)
}

#[allow(dead_code)]
pub async fn find_by_slug(pool: &PgPool, slug: &str) -> Result<Option<Organization>, AppError> {
    let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE slug = $1")
        .bind(slug)
        .fetch_optional(pool)
        .await?;

    Ok(org)
}

pub async fn find_by_id(pool: &PgPool, org_id: Uuid) -> Result<Option<Organization>, AppError> {
    let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
        .bind(org_id)
        .fetch_optional(pool)
        .await?;

    Ok(org)
}

pub async fn find_by_stripe_customer_id(
    pool: &PgPool,
    stripe_customer_id: &str,
) -> Result<Option<Organization>, AppError> {
    let org = sqlx::query_as::<_, Organization>(
        "SELECT * FROM organizations WHERE stripe_customer_id = $1",
    )
    .bind(stripe_customer_id)
    .fetch_optional(pool)
    .await?;

    Ok(org)
}

pub async fn find_by_user_id(pool: &PgPool, user_id: Uuid) -> Result<Vec<Organization>, AppError> {
    let orgs = sqlx::query_as::<_, Organization>(
        r#"
        SELECT o.* FROM organizations o
        JOIN members m ON m.org_id = o.id
        WHERE m.user_id = $1
        ORDER BY o.name
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(orgs)
}

pub async fn update(
    pool: &PgPool,
    org_id: Uuid,
    req: &UpdateOrganizationRequest,
) -> Result<Organization, AppError> {
    let org = sqlx::query_as::<_, Organization>(
        r#"
        UPDATE organizations SET
            name = COALESCE($2, name),
            slug = COALESCE($3, slug),
            brand_color = COALESCE($4, brand_color),
            timezone = COALESCE($5, timezone),
            logo_url = CASE WHEN $6 IS NOT NULL THEN NULLIF($6, '') ELSE logo_url END,
            custom_domain = CASE WHEN $7 IS NOT NULL THEN NULLIF($7, '') ELSE custom_domain END,
            custom_domain_verified_at = CASE
                WHEN $7 IS NOT NULL AND NULLIF($7, '') IS DISTINCT FROM custom_domain THEN NULL
                ELSE custom_domain_verified_at
            END,
            custom_domain_status = CASE
                WHEN $7 IS NULL THEN custom_domain_status
                WHEN NULLIF($7, '') IS NULL THEN 'not_configured'
                WHEN NULLIF($7, '') IS DISTINCT FROM custom_domain THEN 'pending_verification'
                WHEN custom_domain_verified_at IS NOT NULL THEN 'verified'
                ELSE custom_domain_status
            END,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(&req.name)
    .bind(&req.slug)
    .bind(&req.brand_color)
    .bind(&req.timezone)
    .bind(&req.logo_url)
    .bind(&req.custom_domain)
    .fetch_one(pool)
    .await?;

    Ok(org)
}

pub async fn mark_checkout_pending(
    pool: &PgPool,
    org_id: Uuid,
    stripe_price_id: &str,
    billing_email: &str,
) -> Result<Organization, AppError> {
    let org = sqlx::query_as::<_, Organization>(
        r#"
        UPDATE organizations SET
            subscription_status = 'checkout_pending',
            stripe_price_id = $2,
            billing_email = NULLIF($3, ''),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(stripe_price_id)
    .bind(billing_email)
    .fetch_one(pool)
    .await?;

    Ok(org)
}

pub async fn sync_checkout_session(
    pool: &PgPool,
    org_id: Uuid,
    stripe_customer_id: Option<&str>,
    stripe_subscription_id: Option<&str>,
    billing_email: Option<&str>,
) -> Result<Organization, AppError> {
    let org = sqlx::query_as::<_, Organization>(
        r#"
        UPDATE organizations SET
            stripe_customer_id = COALESCE($2, stripe_customer_id),
            stripe_subscription_id = COALESCE($3, stripe_subscription_id),
            billing_email = COALESCE($4, billing_email),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(stripe_customer_id)
    .bind(stripe_subscription_id)
    .bind(billing_email)
    .fetch_one(pool)
    .await?;

    Ok(org)
}

pub async fn sync_billing_state(
    pool: &PgPool,
    org_id: Uuid,
    update: &BillingSyncUpdate<'_>,
    downgrade: &DowngradeLifecycle,
) -> Result<Organization, AppError> {
    let org = sqlx::query_as::<_, Organization>(
        r#"
        UPDATE organizations SET
            stripe_customer_id = COALESCE($2, stripe_customer_id),
            stripe_subscription_id = $3,
            subscription_status = $4,
            stripe_price_id = $5,
            current_period_end = $6,
            cancel_at_period_end = $7,
            billing_email = COALESCE($8, billing_email),
            trial_ends_at = $9,
            plan = $10,
            downgrade_target_plan = $11,
            downgrade_started_at = $12,
            downgrade_grace_ends_at = $13,
            downgrade_state = $14,
            downgrade_warning_stage = $15,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(update.stripe_customer_id)
    .bind(update.stripe_subscription_id)
    .bind(update.subscription_status)
    .bind(update.stripe_price_id)
    .bind(update.current_period_end)
    .bind(update.cancel_at_period_end)
    .bind(update.billing_email)
    .bind(update.trial_ends_at)
    .bind(update.plan)
    .bind(downgrade.target_plan)
    .bind(downgrade.started_at)
    .bind(downgrade.grace_ends_at)
    .bind(downgrade.state)
    .bind(downgrade.warning_stage)
    .fetch_one(pool)
    .await?;

    Ok(org)
}

#[allow(dead_code)]
pub async fn clear_custom_domain(pool: &PgPool, org_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE organizations
        SET
            custom_domain = NULL,
            custom_domain_verified_at = NULL,
            custom_domain_status = 'not_configured',
            updated_at = NOW()
        WHERE id = $1 AND custom_domain IS NOT NULL
        "#,
    )
    .bind(org_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_custom_domain_verified(
    pool: &PgPool,
    org_id: Uuid,
) -> Result<Organization, AppError> {
    let org = sqlx::query_as::<_, Organization>(
        r#"
        UPDATE organizations
        SET
            custom_domain_verified_at = NOW(),
            custom_domain_status = 'verified',
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await?;

    Ok(org)
}

pub async fn slug_exists(pool: &PgPool, slug: &str) -> Result<bool, AppError> {
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM organizations WHERE slug = $1)")
            .bind(slug)
            .fetch_one(pool)
            .await?;

    Ok(exists)
}

pub async fn set_custom_domain_status(
    pool: &PgPool,
    org_id: Uuid,
    status: CustomDomainStatus,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE organizations
        SET
            custom_domain_status = $2,
            custom_domain_verified_at = CASE
                WHEN $2 = 'verified' THEN COALESCE(custom_domain_verified_at, NOW())
                WHEN $2 = 'blocked_by_plan' THEN NULL
                WHEN $2 = 'pending_verification' THEN NULL
                ELSE custom_domain_verified_at
            END,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(org_id)
    .bind(status)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn find_due_for_downgrade_enforcement(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<Organization>, AppError> {
    let orgs = sqlx::query_as::<_, Organization>(
        r#"
        SELECT *
        FROM organizations
        WHERE downgrade_state IN ('pending_customer_action', 'ready_to_enforce')
          AND downgrade_grace_ends_at IS NOT NULL
          AND downgrade_grace_ends_at <= NOW()
        ORDER BY downgrade_grace_ends_at ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(orgs)
}

pub async fn find_orgs_with_active_downgrades(
    pool: &PgPool,
) -> Result<Vec<Organization>, AppError> {
    let orgs = sqlx::query_as::<_, Organization>(
        r#"
        SELECT *
        FROM organizations
        WHERE downgrade_state = 'pending_customer_action'
        ORDER BY downgrade_started_at ASC NULLS LAST
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(orgs)
}

pub async fn mark_downgrade_warning_stage(
    pool: &PgPool,
    org_id: Uuid,
    stage: i32,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE organizations
        SET downgrade_warning_stage = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(org_id)
    .bind(stage)
    .execute(pool)
    .await?;

    Ok(())
}

#[allow(dead_code)]
pub async fn mark_downgrade_state(
    pool: &PgPool,
    org_id: Uuid,
    state: DowngradeState,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE organizations
        SET downgrade_state = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(org_id)
    .bind(state)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn search_for_support(
    pool: &PgPool,
    query: &str,
    limit: i64,
) -> Result<Vec<Organization>, AppError> {
    let needle = format!("%{}%", query.trim().to_lowercase());
    let orgs = sqlx::query_as::<_, Organization>(
        r#"
        SELECT *
        FROM organizations
        WHERE lower(slug) LIKE $1
           OR lower(name) LIKE $1
           OR lower(COALESCE(billing_email, '')) LIKE $1
           OR lower(COALESCE(stripe_customer_id, '')) LIKE $1
           OR lower(COALESCE(stripe_subscription_id, '')) LIKE $1
        ORDER BY updated_at DESC
        LIMIT $2
        "#,
    )
    .bind(needle)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(orgs)
}

#[allow(dead_code)]
pub async fn update_downgrade_lifecycle(
    pool: &PgPool,
    org_id: Uuid,
    lifecycle: &DowngradeLifecycle,
) -> Result<Organization, AppError> {
    let org = sqlx::query_as::<_, Organization>(
        r#"
        UPDATE organizations
        SET
            downgrade_target_plan = $2,
            downgrade_started_at = $3,
            downgrade_grace_ends_at = $4,
            downgrade_state = $5,
            downgrade_warning_stage = $6,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(lifecycle.target_plan)
    .bind(lifecycle.started_at)
    .bind(lifecycle.grace_ends_at)
    .bind(lifecycle.state)
    .bind(lifecycle.warning_stage)
    .fetch_one(pool)
    .await?;

    Ok(org)
}

pub async fn complete_downgrade_enforcement(
    pool: &PgPool,
    org_id: Uuid,
    plan: OrganizationPlan,
) -> Result<Organization, AppError> {
    let org = sqlx::query_as::<_, Organization>(
        r#"
        UPDATE organizations
        SET
            plan = $2,
            downgrade_state = 'enforced',
            downgrade_warning_stage = 4,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(plan)
    .fetch_one(pool)
    .await?;

    Ok(org)
}

pub async fn cancel_downgrade(
    pool: &PgPool,
    org_id: Uuid,
    plan: OrganizationPlan,
) -> Result<Organization, AppError> {
    let org = sqlx::query_as::<_, Organization>(
        r#"
        UPDATE organizations
        SET
            plan = $2,
            downgrade_target_plan = NULL,
            downgrade_started_at = NULL,
            downgrade_grace_ends_at = NULL,
            downgrade_state = 'canceled',
            downgrade_warning_stage = 0,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(plan)
    .fetch_one(pool)
    .await?;

    Ok(org)
}

pub async fn custom_domain_exists(
    pool: &PgPool,
    custom_domain: &str,
    exclude_org_id: Option<Uuid>,
) -> Result<bool, AppError> {
    let normalized = custom_domain
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM organizations
            WHERE lower(custom_domain) = $1
              AND ($2::uuid IS NULL OR id != $2)
        )
        "#,
    )
    .bind(normalized)
    .bind(exclude_org_id)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}
