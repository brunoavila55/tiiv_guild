//! Criação de guilda (vira o subdomínio `{slug}.BASE_DOMAIN`).

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use db::guilds::NewGuild;
use domain::{error::collect, host::validate_slug, users::validate_text, Error};
use serde::{Deserialize, Serialize};

use crate::{
    auth::CurrentUser,
    error::{ApiError, ApiResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/guilds", post(create))
        .route("/guilds/slug-check", get(slug_check))
}

#[derive(Deserialize)]
struct CreateReq {
    slug: String,
    name: String,
    tag: Option<String>,
    /// Apelido do criador na guilda; padrão: nome de exibição da conta.
    nickname: Option<String>,
}

#[derive(Serialize)]
struct GuildCreatedDto {
    slug: String,
    name: String,
}

#[derive(Deserialize)]
struct SlugQuery {
    slug: String,
}

#[derive(Serialize)]
struct SlugCheckDto {
    available: bool,
    reason: Option<String>,
}

fn slug_taken() -> Error {
    Error::field("slug", "esse endereço já está em uso")
}

async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(req): Json<CreateReq>,
) -> ApiResult<(StatusCode, Json<GuildCreatedDto>)> {
    let slug = req.slug.trim().to_ascii_lowercase();
    let name = req.name.trim();
    let tag = req.tag.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let nickname = req
        .nickname
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(&user.display_name);
    collect([
        validate_slug(&slug),
        validate_text("name", name, 3, 48),
        tag.map_or(Ok(()), |t| validate_text("tag", t, 1, 8)),
        validate_text("nickname", nickname, 2, 32),
    ])?;
    if db::guilds::slug_exists(&state.pool, &slug).await? {
        return Err(slug_taken().into());
    }

    let guild = db::guilds::create(
        &state.pool,
        NewGuild {
            slug: &slug,
            name,
            tag,
            owner_user_id: user.id,
            owner_nickname: nickname,
        },
    )
    .await
    .map_err(|e| {
        if db::is_unique_violation(&e, db::guilds::SLUG_KEY) {
            slug_taken().into()
        } else {
            ApiError::from(e)
        }
    })?;
    Ok((
        StatusCode::CREATED,
        Json(GuildCreatedDto {
            slug: guild.slug,
            name: guild.name,
        }),
    ))
}

async fn slug_check(
    State(state): State<AppState>,
    Query(q): Query<SlugQuery>,
) -> ApiResult<Json<SlugCheckDto>> {
    let slug = q.slug.trim().to_ascii_lowercase();
    let result = match validate_slug(&slug) {
        Err(Error::Validation(mut f)) => Some(f.remove("slug").unwrap_or_default()),
        Err(e) => return Err(e.into()),
        Ok(()) if db::guilds::slug_exists(&state.pool, &slug).await? => {
            Some("esse endereço já está em uso".to_owned())
        }
        Ok(()) => None,
    };
    Ok(Json(SlugCheckDto {
        available: result.is_none(),
        reason: result,
    }))
}
