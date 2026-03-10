use shared::error::AppError;
use shared::models::user::User;
use sqlx::PgPool;
use uuid::Uuid;

#[allow(dead_code)]
pub async fn find_by_id(pool: &PgPool, user_id: Uuid) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    Ok(user)
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(r#"SELECT * FROM users WHERE lower(email) = lower($1)"#)
        .bind(email)
        .fetch_optional(pool)
        .await?;

    Ok(user)
}
