//! Configurações do tenant atual (`guild_settings`). Recebe a conexão de um [`crate::tenant_tx`].

use sqlx::PgConnection;
use uuid::Uuid;

/// Schema dos campos customizados de membro; `None` se a guilda nunca salvou.
pub async fn member_fields(conn: &mut PgConnection) -> sqlx::Result<Option<serde_json::Value>> {
    sqlx::query_scalar!("SELECT member_fields FROM guild_settings")
        .fetch_optional(conn)
        .await
}

pub async fn set_member_fields(
    conn: &mut PgConnection,
    guild_id: Uuid,
    schema: &serde_json::Value,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"INSERT INTO guild_settings (guild_id, member_fields) VALUES ($1, $2)
           ON CONFLICT (guild_id) DO UPDATE SET member_fields = EXCLUDED.member_fields"#,
        guild_id,
        schema,
    )
    .execute(conn)
    .await?;
    Ok(())
}
