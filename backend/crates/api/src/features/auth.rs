//! Registro, login, logout e `/me`. Valem em qualquer host (o cookie é do domínio todo).

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use db::users::User;
use domain::users::{normalize_email, validate_registration};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{
        clear_cookie, hash_password, hash_token, new_session_token, session_cookie, session_token,
        verify_password, CurrentUser, SESSION_TTL,
    },
    error::{ApiError, ApiResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/me", get(me))
}

#[derive(Deserialize)]
struct RegisterReq {
    email: String,
    password: String,
    display_name: String,
}

#[derive(Deserialize)]
struct LoginReq {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct UserDto {
    id: Uuid,
    email: String,
    display_name: String,
    avatar_url: Option<String>,
}

impl From<User> for UserDto {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            display_name: u.display_name,
            avatar_url: u.avatar_url,
        }
    }
}

#[derive(Serialize)]
struct MyGuildDto {
    slug: String,
    name: String,
    tag: Option<String>,
    is_owner: bool,
    rank_name: String,
}

#[derive(Serialize)]
struct MeDto {
    user: UserDto,
    is_superadmin: bool,
    guilds: Vec<MyGuildDto>,
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterReq>,
) -> ApiResult<Response> {
    let email = normalize_email(&req.email);
    let display_name = req.display_name.trim().to_owned();
    validate_registration(&email, &req.password, &display_name)?;

    let hash = hash_password(req.password).await?;
    let user = db::users::create(&state.pool, &email, &hash, &display_name)
        .await
        .map_err(|e| {
            if db::is_unique_violation(&e, db::users::EMAIL_KEY) {
                ApiError::from(domain::Error::field("email", "e-mail já cadastrado"))
            } else {
                e.into()
            }
        })?;
    start_session(&state, user, StatusCode::CREATED).await
}

async fn login(State(state): State<AppState>, Json(req): Json<LoginReq>) -> ApiResult<Response> {
    let creds = db::users::find_credentials(&state.pool, &normalize_email(&req.email)).await?;
    let hash = creds.as_ref().and_then(|c| c.password_hash.clone());
    if !verify_password(req.password, hash).await? {
        return Err(ApiError::InvalidCredentials);
    }
    let user = creds.ok_or(ApiError::InvalidCredentials)?.user;
    start_session(&state, user, StatusCode::OK).await
}

async fn start_session(state: &AppState, user: User, status: StatusCode) -> ApiResult<Response> {
    let (token, token_hash) = new_session_token();
    db::sessions::create(&state.pool, user.id, &token_hash, Utc::now() + SESSION_TTL).await?;
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, session_cookie(&state.config, &token)?);
    Ok((status, headers, Json(UserDto::from(user))).into_response())
}

async fn logout(State(state): State<AppState>, req_headers: HeaderMap) -> ApiResult<Response> {
    if let Some(token) = session_token(&req_headers) {
        db::sessions::delete(&state.pool, &hash_token(token)).await?;
    }
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, clear_cookie(&state.config)?);
    Ok((StatusCode::NO_CONTENT, headers).into_response())
}

async fn me(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> ApiResult<Json<MeDto>> {
    let guilds = db::guilds::user_guilds(&state.pool, user.id)
        .await?
        .into_iter()
        .map(|g| MyGuildDto {
            slug: g.slug,
            name: g.name,
            tag: g.tag,
            is_owner: g.is_owner,
            rank_name: g.rank_name,
        })
        .collect();
    Ok(Json(MeDto {
        is_superadmin: state.config.is_superadmin(&user.email),
        user: user.into(),
        guilds,
    }))
}
