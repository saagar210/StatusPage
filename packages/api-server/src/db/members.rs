use shared::enums::MemberRole;
use shared::error::AppError;
use shared::models::member::{Member, MemberWithUser};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create(
    pool: &PgPool,
    org_id: Uuid,
    user_id: Uuid,
    role: MemberRole,
) -> Result<Member, AppError> {
    let member = sqlx::query_as::<_, Member>(
        r#"
        INSERT INTO members (org_id, user_id, role)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .bind(role)
    .fetch_one(pool)
    .await?;

    Ok(member)
}

#[allow(dead_code)]
pub async fn find_by_org(pool: &PgPool, org_id: Uuid) -> Result<Vec<MemberWithUser>, AppError> {
    let members = sqlx::query_as::<_, MemberWithUser>(
        r#"
        SELECT m.id, m.org_id, m.user_id, m.role, m.created_at,
               u.name as user_name, u.email as user_email, u.image as user_image
        FROM members m
        JOIN users u ON u.id = m.user_id
        WHERE m.org_id = $1
        ORDER BY m.created_at
        "#,
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?;

    Ok(members)
}

#[allow(dead_code)]
pub async fn find_by_user_and_org(
    pool: &PgPool,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<Option<Member>, AppError> {
    let member =
        sqlx::query_as::<_, Member>("SELECT * FROM members WHERE user_id = $1 AND org_id = $2")
            .bind(user_id)
            .bind(org_id)
            .fetch_optional(pool)
            .await?;

    Ok(member)
}

#[allow(dead_code)]
pub async fn delete(pool: &PgPool, member_id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM members WHERE id = $1")
        .bind(member_id)
        .execute(pool)
        .await?;

    Ok(())
}
