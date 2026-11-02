use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::users::User;

pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &[u8],
    expires_at: DateTime<Utc>,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
        Uuid::now_v7(),
        user_id,
        token_hash,
        expires_at,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Usuário dono de uma sessão ainda válida.
pub async fn find_user(pool: &PgPool, token_hash: &[u8]) -> sqlx::Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"SELECT u.id, u.email, u.display_name, u.avatar_url, u.created_at
             FROM sessions s JOIN users u ON u.id = s.user_id
            WHERE s.token_hash = $1 AND s.expires_at > now()"#,
        token_hash
    )
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &PgPool, token_hash: &[u8]) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM sessions WHERE token_hash = $1", token_hash)
        .execute(pool)
        .await?;
    Ok(())
}
