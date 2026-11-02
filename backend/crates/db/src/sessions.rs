//! Sessões de login. Só o hash do token é persistido.

use chrono::{DateTime, Utc};
use sqlx::types::Uuid;
use sqlx::PgExecutor;

#[derive(Debug, Clone)]
pub struct SessionUser {
    pub session_id: Uuid,
    pub user_id: Uuid,
}

pub async fn create(
    db: impl PgExecutor<'_>,
    id: Uuid,
    user_id: Uuid,
    token_hash: &[u8],
    expires_at: DateTime<Utc>,
    user_agent: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at, user_agent) VALUES ($1, $2, $3, $4, $5)",
        id,
        user_id,
        token_hash,
        expires_at,
        user_agent,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Sessão válida (não expirada) para o hash do token.
pub async fn find_valid(
    db: impl PgExecutor<'_>,
    token_hash: &[u8],
) -> Result<Option<SessionUser>, sqlx::Error> {
    sqlx::query_as!(
        SessionUser,
        "SELECT id AS session_id, user_id FROM sessions WHERE token_hash = $1 AND expires_at > now()",
        token_hash,
    )
    .fetch_optional(db)
    .await
}

pub async fn delete_by_token_hash(
    db: impl PgExecutor<'_>,
    token_hash: &[u8],
) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM sessions WHERE token_hash = $1", token_hash)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete_expired_for_user(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM sessions WHERE user_id = $1 AND expires_at <= now()",
        user_id,
    )
    .execute(db)
    .await?;
    Ok(())
}
