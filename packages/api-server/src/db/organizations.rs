use shared::error::AppError;
use shared::models::organization::{
    CreateOrganizationRequest, Organization, UpdateOrganizationRequest,
};
use sqlx::PgPool;
use uuid::Uuid;

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

pub async fn slug_exists(pool: &PgPool, slug: &str) -> Result<bool, AppError> {
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM organizations WHERE slug = $1)")
            .bind(slug)
            .fetch_one(pool)
            .await?;

    Ok(exists)
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
