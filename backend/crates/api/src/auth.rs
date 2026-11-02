//! Senhas, tokens de sessão, cookie e extractors de usuário.

use std::sync::OnceLock;

use argon2::{
    password_hash::{rand_core::OsRng, rand_core::RngCore, PasswordHash, SaltString},
    Argon2, PasswordHasher, PasswordVerifier,
};
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, HeaderMap, HeaderValue},
};
use chrono::Duration;
use db::users::User;
use domain::host::HostKind;
use sha2::{Digest, Sha256};

use crate::{config::Config, error::ApiError, host::host_kind, state::AppState};

pub const SESSION_COOKIE: &str = "guildos_session";
pub const SESSION_TTL: Duration = Duration::days(30);

// ---------------------------------------------------------------------------
// Senhas
// ---------------------------------------------------------------------------

pub async fn hash_password(password: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| anyhow::anyhow!("argon2: {e}"))
    })
    .await
    .map_err(anyhow::Error::from)?
    .map_err(ApiError::from)
}

/// Verifica a senha; sem hash (usuário inexistente) compara com um hash fixo para
/// gastar o mesmo tempo e não revelar quais e-mails existem.
pub async fn verify_password(password: String, hash: Option<String>) -> Result<bool, ApiError> {
    static DUMMY: OnceLock<String> = OnceLock::new();
    tokio::task::spawn_blocking(move || {
        let exists = hash.is_some();
        let hash = hash.unwrap_or_else(|| {
            DUMMY
                .get_or_init(|| {
                    let salt = SaltString::generate(&mut OsRng);
                    Argon2::default()
                        .hash_password(b"dummy-password", &salt)
                        .map(|h| h.to_string())
                        .unwrap_or_default()
                })
                .clone()
        });
        let ok = PasswordHash::new(&hash)
            .map(|h| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &h)
                    .is_ok()
            })
            .unwrap_or(false);
        exists && ok
    })
    .await
    .map_err(|e| ApiError::Internal(e.into()))
}

// ---------------------------------------------------------------------------
// Tokens e cookie
// ---------------------------------------------------------------------------

/// Token aleatório (vai no cookie) e seu SHA-256 (vai no banco).
pub fn new_session_token() -> (String, Vec<u8>) {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let hash = hash_token(&token);
    (token, hash)
}

pub fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub fn session_cookie(config: &Config, token: &str) -> Result<HeaderValue, ApiError> {
    build_cookie(config, token, SESSION_TTL.num_seconds())
}

pub fn clear_cookie(config: &Config) -> Result<HeaderValue, ApiError> {
    build_cookie(config, "", 0)
}

fn build_cookie(config: &Config, value: &str, max_age: i64) -> Result<HeaderValue, ApiError> {
    let mut cookie =
        format!("{SESSION_COOKIE}={value}; Path=/; Max-Age={max_age}; HttpOnly; SameSite=Lax");
    if let Some(domain) = &config.cookie_domain {
        cookie.push_str("; Domain=");
        cookie.push_str(domain);
    }
    if config.cookie_secure {
        cookie.push_str("; Secure");
    }
    HeaderValue::from_str(&cookie).map_err(|e| ApiError::Internal(e.into()))
}

/// Valor do cookie de sessão na requisição, se houver.
pub fn session_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, value)| *name == SESSION_COOKIE && !value.is_empty())
        .map(|(_, value)| value)
}

// ---------------------------------------------------------------------------
// Extractors
// ---------------------------------------------------------------------------

/// Usuário logado, se houver.
pub struct MaybeUser(pub Option<User>);

impl FromRequestParts<AppState> for MaybeUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let Some(token) = session_token(&parts.headers) else {
            return Ok(Self(None));
        };
        let user = db::sessions::find_user(&state.pool, &hash_token(token)).await?;
        Ok(Self(user))
    }
}

/// Usuário logado (401 sem sessão válida).
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let MaybeUser(user) = MaybeUser::from_request_parts(parts, state).await?;
        user.map(Self).ok_or(domain::Error::Unauthenticated.into())
    }
}

/// Superadmin da plataforma; só existe no host `admin.` (404 nos demais).
pub struct SuperAdmin(pub User);

impl FromRequestParts<AppState> for SuperAdmin {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        if host_kind(parts, &state.config.base_domain) != HostKind::Admin {
            return Err(domain::Error::NotFound.into());
        }
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, state).await?;
        if !state.config.is_superadmin(&user.email) {
            return Err(domain::Error::Forbidden.into());
        }
        Ok(Self(user))
    }
}
