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

pub async fn find_by_id(pool: &PgPool, member_id: Uuid) -> Result<Option<Member>, AppError> {
    let member = sqlx::query_as::<_, Member>("SELECT * FROM members WHERE id = $1")
        .bind(member_id)
        .fetch_optional(pool)
        .await?;

    Ok(member)
}

pub async fn update_role(
    pool: &PgPool,
    org_id: Uuid,
    member_id: Uuid,
    role: MemberRole,
) -> Result<Member, AppError> {
    let member = sqlx::query_as::<_, Member>(
        r#"
        UPDATE members
        SET role = $3
        WHERE org_id = $1 AND id = $2
        RETURNING *
        "#,
    )
    .bind(org_id)
    .bind(member_id)
    .bind(role)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;

    Ok(member)
}

pub async fn count_by_role(pool: &PgPool, org_id: Uuid, role: MemberRole) -> Result<i64, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM members WHERE org_id = $1 AND role = $2",
    )
    .bind(org_id)
    .bind(role)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

pub async fn delete_scoped(pool: &PgPool, org_id: Uuid, member_id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM members WHERE org_id = $1 AND id = $2")
        .bind(org_id)
        .bind(member_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Member not found".to_string()));
    }

    Ok(())
}
