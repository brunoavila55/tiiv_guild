//! Catálogo global de jogos (games, game_classes, game_servers, game_server_classes),
//! mantido pelo superadmin.

use serde_json::Value;
use sqlx::types::Uuid;
use sqlx::PgExecutor;

#[derive(Debug, Clone)]
pub struct GameRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub active: bool,
    pub level_min: i32,
    pub level_max: i32,
    pub character_schema: Value,
    pub default_ranks: Value,
}

#[derive(Debug, Clone)]
pub struct ClassRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub role_hint: String,
    pub metadata: Value,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct ServerRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub region: Option<String>,
    pub active: bool,
    /// Classes liberadas no servidor; vazio = todas as classes ativas do jogo.
    pub class_ids: Vec<Uuid>,
}

/// Campos editáveis de um jogo.
pub struct GameFields<'a> {
    pub name: &'a str,
    pub active: bool,
    pub level_min: i32,
    pub level_max: i32,
    pub character_schema: &'a Value,
    pub default_ranks: &'a Value,
}

pub struct ClassFields<'a> {
    pub name: &'a str,
    pub role_hint: &'a str,
    pub metadata: &'a Value,
    pub active: bool,
}

pub struct ServerFields<'a> {
    pub name: &'a str,
    pub region: Option<&'a str>,
    pub active: bool,
}

pub async fn list_games(
    db: impl PgExecutor<'_>,
    include_inactive: bool,
) -> Result<Vec<GameRow>, sqlx::Error> {
    sqlx::query_as!(
        GameRow,
        r#"SELECT id, slug, name, active, level_min, level_max, character_schema, default_ranks
           FROM games WHERE active OR $1 ORDER BY name"#,
        include_inactive,
    )
    .fetch_all(db)
    .await
}

pub async fn find_game(
    db: impl PgExecutor<'_>,
    slug: &str,
    include_inactive: bool,
) -> Result<Option<GameRow>, sqlx::Error> {
    sqlx::query_as!(
        GameRow,
        r#"SELECT id, slug, name, active, level_min, level_max, character_schema, default_ranks
           FROM games WHERE slug = $1 AND (active OR $2)"#,
        slug,
        include_inactive,
    )
    .fetch_optional(db)
    .await
}

pub async fn insert_game(
    db: impl PgExecutor<'_>,
    id: Uuid,
    slug: &str,
    f: &GameFields<'_>,
) -> Result<GameRow, sqlx::Error> {
    sqlx::query_as!(
        GameRow,
        r#"INSERT INTO games (id, slug, name, active, level_min, level_max, character_schema, default_ranks)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, slug, name, active, level_min, level_max, character_schema, default_ranks"#,
        id,
        slug,
        f.name,
        f.active,
        f.level_min,
        f.level_max,
        f.character_schema,
        f.default_ranks,
    )
    .fetch_one(db)
    .await
}

pub async fn update_game(
    db: impl PgExecutor<'_>,
    slug: &str,
    f: &GameFields<'_>,
) -> Result<Option<GameRow>, sqlx::Error> {
    sqlx::query_as!(
        GameRow,
        r#"UPDATE games
           SET name = $2, active = $3, level_min = $4, level_max = $5,
               character_schema = $6, default_ranks = $7
           WHERE slug = $1
           RETURNING id, slug, name, active, level_min, level_max, character_schema, default_ranks"#,
        slug,
        f.name,
        f.active,
        f.level_min,
        f.level_max,
        f.character_schema,
        f.default_ranks,
    )
    .fetch_optional(db)
    .await
}

pub async fn list_classes(
    db: impl PgExecutor<'_>,
    game_id: Uuid,
    include_inactive: bool,
) -> Result<Vec<ClassRow>, sqlx::Error> {
    sqlx::query_as!(
        ClassRow,
        r#"SELECT id, slug, name, role_hint, metadata, active
           FROM game_classes WHERE game_id = $1 AND (active OR $2) ORDER BY name"#,
        game_id,
        include_inactive,
    )
    .fetch_all(db)
    .await
}

pub async fn insert_class(
    db: impl PgExecutor<'_>,
    id: Uuid,
    game_id: Uuid,
    slug: &str,
    f: &ClassFields<'_>,
) -> Result<ClassRow, sqlx::Error> {
    sqlx::query_as!(
        ClassRow,
        r#"INSERT INTO game_classes (id, game_id, slug, name, role_hint, metadata, active)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, slug, name, role_hint, metadata, active"#,
        id,
        game_id,
        slug,
        f.name,
        f.role_hint,
        f.metadata,
        f.active,
    )
    .fetch_one(db)
    .await
}

pub async fn update_class(
    db: impl PgExecutor<'_>,
    game_id: Uuid,
    slug: &str,
    f: &ClassFields<'_>,
) -> Result<Option<ClassRow>, sqlx::Error> {
    sqlx::query_as!(
        ClassRow,
        r#"UPDATE game_classes SET name = $3, role_hint = $4, metadata = $5, active = $6
           WHERE game_id = $1 AND slug = $2
           RETURNING id, slug, name, role_hint, metadata, active"#,
        game_id,
        slug,
        f.name,
        f.role_hint,
        f.metadata,
        f.active,
    )
    .fetch_optional(db)
    .await
}

pub async fn list_servers(
    db: impl PgExecutor<'_>,
    game_id: Uuid,
    include_inactive: bool,
) -> Result<Vec<ServerRow>, sqlx::Error> {
    sqlx::query_as!(
        ServerRow,
        r#"SELECT s.id, s.slug, s.name, s.region, s.active,
                  COALESCE(array_agg(sc.class_id) FILTER (WHERE sc.class_id IS NOT NULL), '{}') AS "class_ids!"
           FROM game_servers s
           LEFT JOIN game_server_classes sc ON sc.server_id = s.id
           WHERE s.game_id = $1 AND (s.active OR $2)
           GROUP BY s.id
           ORDER BY s.name"#,
        game_id,
        include_inactive,
    )
    .fetch_all(db)
    .await
}

pub async fn insert_server(
    db: impl PgExecutor<'_>,
    id: Uuid,
    game_id: Uuid,
    slug: &str,
    f: &ServerFields<'_>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"INSERT INTO game_servers (id, game_id, slug, name, region, active)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id"#,
        id,
        game_id,
        slug,
        f.name,
        f.region,
        f.active,
    )
    .fetch_one(db)
    .await
}

pub async fn update_server(
    db: impl PgExecutor<'_>,
    game_id: Uuid,
    slug: &str,
    f: &ServerFields<'_>,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"UPDATE game_servers SET name = $3, region = $4, active = $5
           WHERE game_id = $1 AND slug = $2
           RETURNING id"#,
        game_id,
        slug,
        f.name,
        f.region,
        f.active,
    )
    .fetch_optional(db)
    .await
}

/// Ids das classes do jogo com esses slugs (ignora os que não existem).
pub async fn class_ids_by_slugs(
    db: impl PgExecutor<'_>,
    game_id: Uuid,
    slugs: &[String],
) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT id, slug FROM game_classes WHERE game_id = $1 AND slug = ANY($2)",
        game_id,
        slugs,
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|r| (r.id, r.slug)).collect())
}

pub async fn clear_server_classes(
    db: impl PgExecutor<'_>,
    server_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM game_server_classes WHERE server_id = $1",
        server_id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn add_server_classes(
    db: impl PgExecutor<'_>,
    game_id: Uuid,
    server_id: Uuid,
    class_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO game_server_classes (game_id, server_id, class_id)
           SELECT $1, $2, unnest($3::uuid[])"#,
        game_id,
        server_id,
        class_ids,
    )
    .execute(db)
    .await?;
    Ok(())
}
