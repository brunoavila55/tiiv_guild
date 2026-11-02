//! Catálogo de jogos: classes, servidores, ranks padrão e `character_schema` (P0.4).

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use db::Uuid;
use domain::DomainError;
use games::{GameRegistry, RankSeed};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgConnection;

use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/games", get(list))
        .route("/games/{slug}", get(detail))
}

/// Grava os seeds de cada pack em games / game_classes / game_servers (idempotente).
/// Roda no startup com o dono das tabelas; o papel da aplicação só tem SELECT nelas.
pub async fn sync_catalog(
    conn: &mut PgConnection,
    registry: &GameRegistry,
) -> Result<(), sqlx::Error> {
    for pack in registry.iter() {
        let game_id =
            db::catalog::upsert_game(&mut *conn, shared::ids::new_id(), pack.slug(), pack.name())
                .await?;
        for class in pack.classes() {
            db::catalog::upsert_class(
                &mut *conn,
                shared::ids::new_id(),
                game_id,
                &class.slug,
                &class.name,
                class.role_hint.as_str(),
                &class.metadata,
            )
            .await?;
        }
        for server in pack.servers() {
            db::catalog::upsert_server(
                &mut *conn,
                shared::ids::new_id(),
                game_id,
                &server.slug,
                &server.name,
                server.region.as_deref(),
            )
            .await?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct GameSummary {
    slug: String,
    name: String,
}

#[derive(Serialize)]
struct GameDetail {
    id: Uuid,
    slug: String,
    name: String,
    level_range: (i32, i32),
    character_schema: Value,
    classes: Vec<ClassResponse>,
    servers: Vec<ServerResponse>,
    default_ranks: Vec<RankSeed>,
}

#[derive(Serialize)]
struct ClassResponse {
    id: Uuid,
    slug: String,
    name: String,
    role_hint: String,
    metadata: Value,
}

#[derive(Serialize)]
struct ServerResponse {
    id: Uuid,
    slug: String,
    name: String,
    region: Option<String>,
}

/// Jogos ativos no banco que têm pack registrado.
async fn list(State(state): State<AppState>) -> Result<Json<Vec<GameSummary>>, ApiError> {
    let games = db::catalog::list_games(&state.pool)
        .await?
        .into_iter()
        .filter(|g| state.games.get(&g.slug).is_some())
        .map(|g| GameSummary {
            slug: g.slug,
            name: g.name,
        })
        .collect();
    Ok(Json(games))
}

async fn detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<GameDetail>, ApiError> {
    let pack = state.games.get(&slug).ok_or(DomainError::NotFound)?;
    let game = db::catalog::find_game(&state.pool, &slug)
        .await?
        .ok_or(DomainError::NotFound)?;

    let classes = db::catalog::list_classes(&state.pool, game.id)
        .await?
        .into_iter()
        .map(|c| ClassResponse {
            id: c.id,
            slug: c.slug,
            name: c.name,
            role_hint: c.role_hint,
            metadata: c.metadata,
        })
        .collect();
    let servers = db::catalog::list_servers(&state.pool, game.id)
        .await?
        .into_iter()
        .map(|s| ServerResponse {
            id: s.id,
            slug: s.slug,
            name: s.name,
            region: s.region,
        })
        .collect();

    Ok(Json(GameDetail {
        id: game.id,
        slug: game.slug,
        name: game.name,
        level_range: pack.level_range(),
        character_schema: pack.character_schema().clone(),
        classes,
        servers,
        default_ranks: pack.default_ranks(),
    }))
}
