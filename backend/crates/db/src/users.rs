//! Contas de usuário (tabela global).

use chrono::{DateTime, Utc};
use sqlx::types::Uuid;
use sqlx::PgExecutor;

#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Id e hash de senha, para o login. `password_hash` é `None` em contas sem senha (OAuth futuro).
#[derive(Debug, Clone)]
pub struct UserCredentials {
    pub id: Uuid,
    pub password_hash: Option<String>,
}

pub async fn create(
    db: impl PgExecutor<'_>,
    id: Uuid,
    email: &str,
    password_hash: &str,
    display_name: &str,
) -> Result<UserRow, sqlx::Error> {
    sqlx::query_as!(
        UserRow,
        r#"INSERT INTO users (id, email, password_hash, display_name)
           VALUES ($1, $2, $3, $4)
           RETURNING id, email, display_name, avatar_url, created_at"#,
        id,
        email,
        password_hash,
        display_name,
    )
    .fetch_one(db)
    .await
}

pub async fn find_credentials_by_email(
    db: impl PgExecutor<'_>,
    email: &str,
) -> Result<Option<UserCredentials>, sqlx::Error> {
    sqlx::query_as!(
        UserCredentials,
        "SELECT id, password_hash FROM users WHERE lower(email) = lower($1)",
        email,
    )
    .fetch_optional(db)
    .await
}

pub async fn find(db: impl PgExecutor<'_>, id: Uuid) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as!(
        UserRow,
        "SELECT id, email, display_name, avatar_url, created_at FROM users WHERE id = $1",
        id,
    )
    .fetch_optional(db)
    .await
}
