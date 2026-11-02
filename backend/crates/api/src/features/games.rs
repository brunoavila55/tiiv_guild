//! Catálogo de jogos (P0.4).
//!
//! Público: leitura dos jogos ativos (classes, servidores, `character_schema`, ranks padrão).
//! `/admin/...`: o superadmin cadastra e edita jogos, classes e servidores. Nada de jogo
//! específico existe no código; adicionar um jogo é só cadastro.

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use db::catalog::{ClassFields, ClassRow, GameFields, GameRow, ServerFields, ServerRow};
use db::Uuid;
use domain::DomainError;
use games::{GameDefinition, RankTemplate, RoleHint};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::SuperAdmin;
use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/games", get(list_public))
        .route("/games/{game}", get(detail_public))
        .route("/admin/games", get(list_admin).post(create_game))
        .route("/admin/games/{game}", get(detail_admin).put(update_game))
        .route("/admin/games/{game}/classes", post(create_class))
        .route("/admin/games/{game}/classes/{class}", put(update_class))
        .route("/admin/games/{game}/servers", post(create_server))
        .route("/admin/games/{game}/servers/{server}", put(update_server))
}

// ---------------------------------------------------------------------------
// Respostas
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct GameSummary {
    slug: String,
    name: String,
    active: bool,
}

#[derive(Serialize)]
struct GameDetail {
    id: Uuid,
    slug: String,
    name: String,
    active: bool,
    level_min: i32,
    level_max: i32,
    character_schema: Value,
    default_ranks: Vec<RankTemplate>,
    classes: Vec<ClassResponse>,
    servers: Vec<ServerResponse>,
}

#[derive(Serialize)]
struct ClassResponse {
    id: Uuid,
    slug: String,
    name: String,
    role_hint: String,
    metadata: Value,
    active: bool,
}

impl From<ClassRow> for ClassResponse {
    fn from(c: ClassRow) -> Self {
        Self {
            id: c.id,
            slug: c.slug,
            name: c.name,
            role_hint: c.role_hint,
            metadata: c.metadata,
            active: c.active,
        }
    }
}

#[derive(Serialize)]
struct ServerResponse {
    id: Uuid,
    slug: String,
    name: String,
    region: Option<String>,
    active: bool,
    /// Classes disponíveis no servidor; vazio = todas as classes do jogo.
    class_slugs: Vec<String>,
}

// ---------------------------------------------------------------------------
// Público
// ---------------------------------------------------------------------------

async fn list_public(State(state): State<AppState>) -> Result<Json<Vec<GameSummary>>, ApiError> {
    list(&state, false).await
}

async fn detail_public(
    State(state): State<AppState>,
    Path(game): Path<String>,
) -> Result<Json<GameDetail>, ApiError> {
    detail(&state, &game, false).await.map(Json)
}

// ---------------------------------------------------------------------------
// Superadmin
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateGameBody {
    slug: String,
    #[serde(flatten)]
    fields: GameBody,
}

#[derive(Deserialize)]
struct GameBody {
    name: String,
    #[serde(default = "default_true")]
    active: bool,
    level_min: i32,
    level_max: i32,
    #[serde(default = "empty_schema")]
    character_schema: Value,
    #[serde(default = "core_default_ranks")]
    default_ranks: Vec<RankTemplate>,
}

#[derive(Deserialize)]
struct CreateClassBody {
    slug: String,
    #[serde(flatten)]
    fields: ClassBody,
}

#[derive(Deserialize)]
struct ClassBody {
    name: String,
    role_hint: String,
    #[serde(default = "empty_object")]
    metadata: Value,
    #[serde(default = "default_true")]
    active: bool,
}

#[derive(Deserialize)]
struct CreateServerBody {
    slug: String,
    #[serde(flatten)]
    fields: ServerBody,
}

#[derive(Deserialize)]
struct ServerBody {
    name: String,
    #[serde(default)]
    region: Option<String>,
    #[serde(default = "default_true")]
    active: bool,
    /// Restringe as classes do servidor; vazio = todas.
    #[serde(default)]
    class_slugs: Vec<String>,
}

async fn list_admin(
    _: SuperAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<GameSummary>>, ApiError> {
    list(&state, true).await
}

async fn detail_admin(
    _: SuperAdmin,
    State(state): State<AppState>,
    Path(game): Path<String>,
) -> Result<Json<GameDetail>, ApiError> {
    detail(&state, &game, true).await.map(Json)
}

async fn create_game(
    _: SuperAdmin,
    State(state): State<AppState>,
    Json(body): Json<CreateGameBody>,
) -> Result<(StatusCode, Json<GameDetail>), ApiError> {
    games::validate_game_slug(&body.slug)?;
    let (name, ranks) = validate_game(&body.fields)?;
    let row = db::catalog::insert_game(
        &state.pool,
        shared::ids::new_id(),
        &body.slug,
        &game_fields(&body.fields, &name, &ranks),
    )
    .await
    .map_err(|e| conflict_on_unique(e, "já existe um jogo com esse slug"))?;
    Ok((
        StatusCode::CREATED,
        Json(detail(&state, &row.slug, true).await?),
    ))
}

async fn update_game(
    _: SuperAdmin,
    State(state): State<AppState>,
    Path(game): Path<String>,
    Json(body): Json<GameBody>,
) -> Result<Json<GameDetail>, ApiError> {
    let (name, ranks) = validate_game(&body)?;
    db::catalog::update_game(&state.pool, &game, &game_fields(&body, &name, &ranks))
        .await?
        .ok_or(DomainError::NotFound)?;
    detail(&state, &game, true).await.map(Json)
}

async fn create_class(
    _: SuperAdmin,
    State(state): State<AppState>,
    Path(game): Path<String>,
    Json(body): Json<CreateClassBody>,
) -> Result<(StatusCode, Json<ClassResponse>), ApiError> {
    games::validate_item_slug(&body.slug)?;
    let game = find_game(&state, &game).await?;
    let (name, role_hint) = validate_class(&body.fields)?;
    let row = db::catalog::insert_class(
        &state.pool,
        shared::ids::new_id(),
        game.id,
        &body.slug,
        &class_fields(&body.fields, &name, role_hint),
    )
    .await
    .map_err(|e| conflict_on_unique(e, "já existe uma classe com esse slug neste jogo"))?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

async fn update_class(
    _: SuperAdmin,
    State(state): State<AppState>,
    Path((game, class)): Path<(String, String)>,
    Json(body): Json<ClassBody>,
) -> Result<Json<ClassResponse>, ApiError> {
    let game = find_game(&state, &game).await?;
    let (name, role_hint) = validate_class(&body)?;
    let row = db::catalog::update_class(
        &state.pool,
        game.id,
        &class,
        &class_fields(&body, &name, role_hint),
    )
    .await?
    .ok_or(DomainError::NotFound)?;
    Ok(Json(row.into()))
}

async fn create_server(
    _: SuperAdmin,
    State(state): State<AppState>,
    Path(game): Path<String>,
    Json(body): Json<CreateServerBody>,
) -> Result<(StatusCode, Json<GameDetail>), ApiError> {
    games::validate_item_slug(&body.slug)?;
    let game = find_game(&state, &game).await?;
    let name = games::validate_name(&body.fields.name)?;

    let mut tx = state.pool.begin().await?;
    let server_id = db::catalog::insert_server(
        &mut *tx,
        shared::ids::new_id(),
        game.id,
        &body.slug,
        &server_fields(&body.fields, &name),
    )
    .await
    .map_err(|e| conflict_on_unique(e, "já existe um servidor com esse slug neste jogo"))?;
    set_server_classes(&mut tx, game.id, server_id, &body.fields.class_slugs).await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(detail(&state, &game.slug, true).await?),
    ))
}

async fn update_server(
    _: SuperAdmin,
    State(state): State<AppState>,
    Path((game, server)): Path<(String, String)>,
    Json(body): Json<ServerBody>,
) -> Result<Json<GameDetail>, ApiError> {
    let game = find_game(&state, &game).await?;
    let name = games::validate_name(&body.name)?;

    let mut tx = state.pool.begin().await?;
    let server_id =
        db::catalog::update_server(&mut *tx, game.id, &server, &server_fields(&body, &name))
            .await?
            .ok_or(DomainError::NotFound)?;
    set_server_classes(&mut tx, game.id, server_id, &body.class_slugs).await?;
    tx.commit().await?;

    detail(&state, &game.slug, true).await.map(Json)
}

// ---------------------------------------------------------------------------
// Apoio
// ---------------------------------------------------------------------------

async fn list(
    state: &AppState,
    include_inactive: bool,
) -> Result<Json<Vec<GameSummary>>, ApiError> {
    let games = db::catalog::list_games(&state.pool, include_inactive)
        .await?
        .into_iter()
        .map(|g| GameSummary {
            slug: g.slug,
            name: g.name,
            active: g.active,
        })
        .collect();
    Ok(Json(games))
}

async fn find_game(state: &AppState, slug: &str) -> Result<GameRow, ApiError> {
    Ok(db::catalog::find_game(&state.pool, slug, true)
        .await?
        .ok_or(DomainError::NotFound)?)
}

async fn detail(
    state: &AppState,
    slug: &str,
    include_inactive: bool,
) -> Result<GameDetail, ApiError> {
    let game = db::catalog::find_game(&state.pool, slug, include_inactive)
        .await?
        .ok_or(DomainError::NotFound)?;
    let classes = db::catalog::list_classes(&state.pool, game.id, include_inactive).await?;
    let servers = db::catalog::list_servers(&state.pool, game.id, include_inactive).await?;

    let slug_by_id: HashMap<Uuid, &str> = classes.iter().map(|c| (c.id, c.slug.as_str())).collect();
    let servers = servers
        .into_iter()
        .map(|s: ServerRow| ServerResponse {
            class_slugs: s
                .class_ids
                .iter()
                .filter_map(|id| slug_by_id.get(id).map(|s| (*s).to_owned()))
                .collect(),
            id: s.id,
            slug: s.slug,
            name: s.name,
            region: s.region,
            active: s.active,
        })
        .collect();

    let default_ranks = serde_json::from_value(game.default_ranks)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("default_ranks inválido no banco: {e}")))?;

    Ok(GameDetail {
        id: game.id,
        slug: game.slug,
        name: game.name,
        active: game.active,
        level_min: game.level_min,
        level_max: game.level_max,
        character_schema: game.character_schema,
        default_ranks,
        classes: classes.into_iter().map(Into::into).collect(),
        servers,
    })
}

fn validate_game(body: &GameBody) -> Result<(String, Value), ApiError> {
    let name = games::validate_name(&body.name)?;
    GameDefinition {
        level_min: body.level_min,
        level_max: body.level_max,
        character_schema: body.character_schema.clone(),
        default_ranks: body.default_ranks.clone(),
    }
    .validate()?;
    let ranks =
        serde_json::to_value(&body.default_ranks).map_err(|e| ApiError::Internal(e.into()))?;
    Ok((name, ranks))
}

fn game_fields<'a>(body: &'a GameBody, name: &'a str, ranks: &'a Value) -> GameFields<'a> {
    GameFields {
        name,
        active: body.active,
        level_min: body.level_min,
        level_max: body.level_max,
        character_schema: &body.character_schema,
        default_ranks: ranks,
    }
}

fn validate_class(body: &ClassBody) -> Result<(String, RoleHint), ApiError> {
    let name = games::validate_name(&body.name)?;
    let role_hint = serde_json::from_value::<RoleHint>(Value::String(body.role_hint.clone()))
        .map_err(|_| invalid("role_hint", "use tank, healer, dps ou support"))?;
    if !body.metadata.is_object() {
        return Err(invalid("metadata", "deve ser um objeto").into());
    }
    Ok((name, role_hint))
}

fn class_fields<'a>(body: &'a ClassBody, name: &'a str, role_hint: RoleHint) -> ClassFields<'a> {
    ClassFields {
        name,
        role_hint: role_hint.as_str(),
        metadata: &body.metadata,
        active: body.active,
    }
}

fn server_fields<'a>(body: &'a ServerBody, name: &'a str) -> ServerFields<'a> {
    ServerFields {
        name,
        region: body
            .region
            .as_deref()
            .map(str::trim)
            .filter(|r| !r.is_empty()),
        active: body.active,
    }
}

async fn set_server_classes(
    tx: &mut sqlx::PgConnection,
    game_id: Uuid,
    server_id: Uuid,
    class_slugs: &[String],
) -> Result<(), ApiError> {
    let found = db::catalog::class_ids_by_slugs(&mut *tx, game_id, class_slugs).await?;
    if let Some(missing) = class_slugs
        .iter()
        .find(|slug| !found.iter().any(|(_, s)| s == *slug))
    {
        return Err(invalid("class_slugs", &format!("classe inexistente: {missing}")).into());
    }
    let ids: Vec<Uuid> = found.into_iter().map(|(id, _)| id).collect();
    db::catalog::clear_server_classes(&mut *tx, server_id).await?;
    db::catalog::add_server_classes(&mut *tx, game_id, server_id, &ids).await?;
    Ok(())
}

fn conflict_on_unique(err: sqlx::Error, message: &str) -> ApiError {
    let unique = err
        .as_database_error()
        .and_then(|e| e.code())
        .is_some_and(|code| code == "23505");
    if unique {
        DomainError::Conflict(message.to_owned()).into()
    } else {
        err.into()
    }
}

fn invalid(field: &str, message: &str) -> DomainError {
    DomainError::Validation {
        field: field.to_owned(),
        message: message.to_owned(),
    }
}

fn default_true() -> bool {
    true
}

fn empty_object() -> Value {
    Value::Object(Default::default())
}

fn empty_schema() -> Value {
    serde_json::json!({ "type": "object", "additionalProperties": false, "properties": {} })
}

/// Ranks padrão quando o superadmin não informa: líder com tudo + membro.
fn core_default_ranks() -> Vec<RankTemplate> {
    use domain::permissions::{ALL, CHARACTERS_MANAGE_OWN, MEMBERS_VIEW};
    vec![
        RankTemplate {
            name: "Líder".to_owned(),
            position: 0,
            permissions: ALL.iter().map(|p| (*p).to_owned()).collect(),
        },
        RankTemplate {
            name: "Membro".to_owned(),
            position: 1,
            permissions: vec![MEMBERS_VIEW.to_owned(), CHARACTERS_MANAGE_OWN.to_owned()],
        },
    ]
}
