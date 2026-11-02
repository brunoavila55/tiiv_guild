//! Painel da plataforma (`admin.BASE_DOMAIN`). Não dá acesso a dados de tenant.

use axum::{extract::State, routing::get, Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{auth::SuperAdmin, error::ApiResult, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/guilds", get(list_guilds))
}

#[derive(Serialize)]
struct AdminGuildDto {
    slug: String,
    name: String,
    tag: Option<String>,
    owner_email: String,
    created_at: DateTime<Utc>,
}

async fn list_guilds(
    State(state): State<AppState>,
    _admin: SuperAdmin,
) -> ApiResult<Json<Vec<AdminGuildDto>>> {
    let guilds = db::guilds::list_all(&state.pool).await?;
    Ok(Json(
        guilds
            .into_iter()
            .map(|(g, owner_email)| AdminGuildDto {
                slug: g.slug,
                name: g.name,
                tag: g.tag,
                owner_email,
                created_at: g.created_at,
            })
            .collect(),
    ))
}
