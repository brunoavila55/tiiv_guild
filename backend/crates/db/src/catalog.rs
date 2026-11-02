//! Catálogo global de jogos (games, game_classes, game_servers).
//! Escrita só no startup, com o dono das tabelas; a API apenas lê.

use serde_json::Value;
use sqlx::types::Uuid;
use sqlx::PgExecutor;

#[derive(Debug, Clone)]
pub struct GameRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ClassRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub role_hint: String,
    pub metadata: Value,
}

#[derive(Debug, Clone)]
pub struct ServerRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub region: Option<String>,
}

/// Cria ou atualiza o jogo pelo slug; devolve o id existente quando já há registro.
pub async fn upsert_game(
    db: impl PgExecutor<'_>,
    new_id: Uuid,
    slug: &str,
    name: &str,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"INSERT INTO games (id, slug, name) VALUES ($1, $2, $3)
           ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name
           RETURNING id"#,
        new_id,
        slug,
        name,
    )
    .fetch_one(db)
    .await
}

pub async fn upsert_class(
    db: impl PgExecutor<'_>,
    new_id: Uuid,
    game_id: Uuid,
    slug: &str,
    name: &str,
    role_hint: &str,
    metadata: &Value,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO game_classes (id, game_id, slug, name, role_hint, metadata)
           VALUES ($1, $2, $3, $4, $5, $6)
           ON CONFLICT (game_id, slug) DO UPDATE
           SET name = EXCLUDED.name, role_hint = EXCLUDED.role_hint, metadata = EXCLUDED.metadata"#,
        new_id,
        game_id,
        slug,
        name,
        role_hint,
        metadata,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn upsert_server(
    db: impl PgExecutor<'_>,
    new_id: Uuid,
    game_id: Uuid,
    slug: &str,
    name: &str,
    region: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO game_servers (id, game_id, slug, name, region)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (game_id, slug) DO UPDATE
           SET name = EXCLUDED.name, region = EXCLUDED.region"#,
        new_id,
        game_id,
        slug,
        name,
        region,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn list_games(db: impl PgExecutor<'_>) -> Result<Vec<GameRow>, sqlx::Error> {
    sqlx::query_as!(
        GameRow,
        "SELECT id, slug, name FROM games WHERE active ORDER BY name"
    )
    .fetch_all(db)
    .await
}

pub async fn find_game(
    db: impl PgExecutor<'_>,
    slug: &str,
) -> Result<Option<GameRow>, sqlx::Error> {
    sqlx::query_as!(
        GameRow,
        "SELECT id, slug, name FROM games WHERE slug = $1 AND active",
        slug,
    )
    .fetch_optional(db)
    .await
}

pub async fn list_classes(
    db: impl PgExecutor<'_>,
    game_id: Uuid,
) -> Result<Vec<ClassRow>, sqlx::Error> {
    sqlx::query_as!(
        ClassRow,
        "SELECT id, slug, name, role_hint, metadata FROM game_classes WHERE game_id = $1 ORDER BY name",
        game_id,
    )
    .fetch_all(db)
    .await
}

pub async fn list_servers(
    db: impl PgExecutor<'_>,
    game_id: Uuid,
) -> Result<Vec<ServerRow>, sqlx::Error> {
    sqlx::query_as!(
        ServerRow,
        "SELECT id, slug, name, region FROM game_servers WHERE game_id = $1 ORDER BY name",
        game_id,
    )
    .fetch_all(db)
    .await
}
