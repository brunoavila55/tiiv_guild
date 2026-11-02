//! Registro, login, logout e `/me` (P0.3).

use axum::extract::State;
use axum::http::header::{SET_COOKIE, USER_AGENT};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{AppendHeaders, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use db::users::UserRow;
use db::Uuid;
use domain::DomainError;
use serde::{Deserialize, Serialize};

use crate::auth::{self, AuthUser};
use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/me", get(me))
}

#[derive(Deserialize)]
struct RegisterBody {
    email: String,
    password: String,
    display_name: String,
}

#[derive(Deserialize)]
struct LoginBody {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct UserResponse {
    id: Uuid,
    email: String,
    display_name: String,
    avatar_url: Option<String>,
    created_at: DateTime<Utc>,
    /// Acesso ao painel da plataforma (catálogo de jogos).
    is_superadmin: bool,
}

impl UserResponse {
    fn new(row: UserRow, state: &AppState) -> Self {
        Self {
            is_superadmin: state.is_superadmin(&row.email),
            id: row.id,
            email: row.email,
            display_name: row.display_name,
            avatar_url: row.avatar_url,
            created_at: row.created_at,
        }
    }
}

async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegisterBody>,
) -> Result<impl IntoResponse, ApiError> {
    let new_user =
        domain::users::validate_registration(&body.email, &body.password, &body.display_name)?;
    let password_hash = auth::hash_password(body.password).await?;

    let mut tx = state.pool.begin().await?;
    let user = db::users::create(
        &mut *tx,
        shared::ids::new_id(),
        &new_user.email,
        &password_hash,
        &new_user.display_name,
    )
    .await
    .map_err(email_conflict)?;
    let token = auth::start_session(&mut *tx, user.id, user_agent(&headers)).await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        AppendHeaders([(SET_COOKIE, auth::session_cookie(&token))]),
        Json(UserResponse::new(user, &state)),
    ))
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LoginBody>,
) -> Result<impl IntoResponse, ApiError> {
    let credentials = match domain::users::normalize_email(&body.email) {
        Ok(email) => db::users::find_credentials_by_email(&state.pool, &email).await?,
        Err(_) => None,
    };
    let (user_id, hash) = match credentials {
        Some(c) => (Some(c.id), c.password_hash),
        None => (None, None),
    };

    // Verifica mesmo sem usuário (hash fictício) para não vazar quais e-mails existem.
    let valid = auth::verify_password(body.password, hash).await?;
    let user_id = user_id
        .filter(|_| valid)
        .ok_or(ApiError::InvalidCredentials)?;

    db::sessions::delete_expired_for_user(&state.pool, user_id).await?;
    let token = auth::start_session(&state.pool, user_id, user_agent(&headers)).await?;
    let user = db::users::find(&state.pool, user_id)
        .await?
        .ok_or(ApiError::InvalidCredentials)?;

    Ok((
        AppendHeaders([(SET_COOKIE, auth::session_cookie(&token))]),
        Json(UserResponse::new(user, &state)),
    ))
}

/// Idempotente: sem sessão válida, só limpa o cookie.
async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    if let Some(token) = auth::session_token(&headers) {
        db::sessions::delete_by_token_hash(&state.pool, &auth::hash_token(token)).await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        AppendHeaders([(SET_COOKIE, auth::clear_session_cookie())]),
    ))
}

async fn me(State(state): State<AppState>, user: AuthUser) -> Result<Json<UserResponse>, ApiError> {
    let row = db::users::find(&state.pool, user.user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(UserResponse::new(row, &state)))
}

fn user_agent(headers: &HeaderMap) -> Option<&str> {
    headers.get(USER_AGENT).and_then(|v| v.to_str().ok())
}

fn email_conflict(err: sqlx::Error) -> ApiError {
    let is_email_dup = err
        .as_database_error()
        .is_some_and(|e| e.constraint() == Some("users_email_key"));
    if is_email_dup {
        DomainError::Conflict("e-mail já cadastrado".to_owned()).into()
    } else {
        err.into()
    }
}
