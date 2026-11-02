use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct Credentials {
    pub user: User,
    pub password_hash: Option<String>,
}

pub const EMAIL_KEY: &str = "users_email_key";

/// Falha com violação de [`EMAIL_KEY`] se o e-mail já existir.
pub async fn create(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    display_name: &str,
) -> sqlx::Result<User> {
    sqlx::query_as!(
        User,
        r#"INSERT INTO users (id, email, password_hash, display_name)
           VALUES ($1, $2, $3, $4)
           RETURNING id, email, display_name, avatar_url, created_at"#,
        Uuid::now_v7(),
        email,
        password_hash,
        display_name,
    )
    .fetch_one(pool)
    .await
}

pub async fn find_credentials(pool: &PgPool, email: &str) -> sqlx::Result<Option<Credentials>> {
    let row = sqlx::query!(
        r#"SELECT id, email, display_name, avatar_url, created_at, password_hash
             FROM users WHERE lower(email) = lower($1)"#,
        email
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| Credentials {
        user: User {
            id: r.id,
            email: r.email,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            created_at: r.created_at,
        },
        password_hash: r.password_hash,
    }))
}
