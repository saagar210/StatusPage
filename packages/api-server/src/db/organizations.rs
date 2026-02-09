use shared::error::AppError;
use shared::models::organization::{Organization, CreateOrganizationRequest, UpdateOrganizationRequest};
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

pub async fn find_by_slug(pool: &PgPool, slug: &str) -> Result<Option<Organization>, AppError> {
    let org = sqlx::query_as::<_, Organization>(
        "SELECT * FROM organizations WHERE slug = $1",
    )
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
            logo_url = COALESCE($6, logo_url),
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
    .fetch_one(pool)
    .await?;

    Ok(org)
}

pub async fn slug_exists(pool: &PgPool, slug: &str) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM organizations WHERE slug = $1)",
    )
    .bind(slug)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}
