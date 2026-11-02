//! Sessão por cookie e hash de senha. Usado por todas as features que exigem login.

use std::sync::LazyLock;

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::header::COOKIE;
use axum::http::request::Parts;
use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use db::Uuid;
use domain::DomainError;
use sha2::{Digest, Sha256};
use sqlx::PgExecutor;

use crate::{ApiError, AppState};

pub const SESSION_COOKIE: &str = "guildos_session";
pub const SESSION_TTL_DAYS: i64 = 30;

/// Usuário autenticado pela sessão do cookie. Rejeita com 401 quando ausente/expirada.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let token = session_token(&parts.headers).ok_or(ApiError::Unauthorized)?;
        let session = db::sessions::find_valid(&state.pool, &hash_token(token))
            .await?
            .ok_or(ApiError::Unauthorized)?;
        Ok(Self {
            user_id: session.user_id,
            session_id: session.session_id,
        })
    }
}

/// Usuário logado cujo e-mail está em `SUPERADMIN_EMAILS`. 401 sem sessão, 403 sem o papel.
#[derive(Debug, Clone, Copy)]
pub struct SuperAdmin {
    pub user_id: Uuid,
}

impl FromRequestParts<AppState> for SuperAdmin {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        let row = db::users::find(&state.pool, user.user_id)
            .await?
            .ok_or(ApiError::Unauthorized)?;
        if state.is_superadmin(&row.email) {
            Ok(Self {
                user_id: user.user_id,
            })
        } else {
            Err(DomainError::Forbidden.into())
        }
    }
}

/// Cria a sessão e devolve o token (que só vai para o cookie).
pub async fn start_session(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    user_agent: Option<&str>,
) -> Result<String, ApiError> {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token = to_hex(&bytes);

    let expires_at = Utc::now() + Duration::days(SESSION_TTL_DAYS);
    let user_agent = user_agent.map(|ua| ua.chars().take(256).collect::<String>());
    db::sessions::create(
        db,
        shared::ids::new_id(),
        user_id,
        &hash_token(&token),
        expires_at,
        user_agent.as_deref(),
    )
    .await?;
    Ok(token)
}

pub fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Valor do cookie de sessão, se presente.
pub fn session_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value)
        .filter(|value| !value.is_empty())
}

/// `Set-Cookie` da sessão: HttpOnly + Secure + SameSite=Lax.
/// Navegadores aceitam `Secure` em http://localhost, então vale também em dev.
pub fn session_cookie(token: &str) -> String {
    let max_age = SESSION_TTL_DAYS * 24 * 60 * 60;
    format!("{SESSION_COOKIE}={token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age={max_age}")
}

pub fn clear_session_cookie() -> String {
    format!("{SESSION_COOKIE}=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0")
}

/// Argon2id com parâmetros padrão; roda fora do executor async (CPU-bound).
pub async fn hash_password(password: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|err| ApiError::Internal(anyhow::anyhow!("argon2: {err}")))
    })
    .await
    .map_err(|err| ApiError::Internal(err.into()))?
}

/// Hash usado quando o e-mail não existe, para o login levar o mesmo tempo
/// e não revelar quais e-mails têm conta.
static DUMMY_HASH: LazyLock<Option<String>> = LazyLock::new(|| {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(b"guildos-dummy-password", &salt)
        .map(|hash| hash.to_string())
        .ok()
});

pub async fn verify_password(password: String, hash: Option<String>) -> Result<bool, ApiError> {
    tokio::task::spawn_blocking(move || {
        let (hash, real) = match hash {
            Some(hash) => (hash, true),
            None => (DUMMY_HASH.clone().unwrap_or_default(), false),
        };
        let ok = PasswordHash::new(&hash).is_ok_and(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        });
        real && ok
    })
    .await
    .map_err(|err| ApiError::Internal(err.into()))
}

fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|b| [DIGITS[usize::from(b >> 4)], DIGITS[usize::from(b & 0x0f)]])
        .map(char::from)
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn reads_session_cookie_among_others() {
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_static("theme=dark; guildos_session=abc123; other=1"),
        );
        assert_eq!(session_token(&headers), Some("abc123"));
    }

    #[test]
    fn missing_or_empty_cookie_is_none() {
        let mut headers = HeaderMap::new();
        assert_eq!(session_token(&headers), None);
        headers.insert(COOKIE, HeaderValue::from_static("guildos_session="));
        assert_eq!(session_token(&headers), None);
    }

    #[test]
    fn hex_encoding() {
        assert_eq!(to_hex(&[0x00, 0xab, 0xff]), "00abff");
    }
}
