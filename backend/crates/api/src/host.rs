use axum::{
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts, Method},
    middleware::Next,
    response::{IntoResponse, Response},
};
use domain::host::{classify, is_within, HostKind};

use crate::{error::ApiError, state::AppState};

/// Contexto definido pelo host da requisição (landing, app, admin ou guilda).
pub struct RequestHost(pub HostKind);

impl FromRequestParts<AppState> for RequestHost {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        Ok(Self(host_kind(parts, &state.config.base_domain)))
    }
}

pub fn host_kind(parts: &Parts, base_domain: &str) -> HostKind {
    let host = parts
        .headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .or_else(|| parts.uri.host());
    host.map_or(HostKind::Unknown, |h| classify(h, base_domain))
}

/// CSRF: requisições que alteram estado vindas de navegador precisam ter `Origin`
/// dentro de `BASE_DOMAIN`. Sem `Origin` (clientes não-navegador) passa.
pub async fn reject_foreign_origin(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let safe = matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    if !safe {
        if let Some(origin) = req.headers().get(header::ORIGIN) {
            let allowed = origin
                .to_str()
                .ok()
                .and_then(origin_host)
                .is_some_and(|h| is_within(h, &state.config.base_domain));
            if !allowed {
                return ApiError::ForeignOrigin.into_response();
            }
        }
    }
    next.run(req).await
}

/// `https://acme.tiiv.com.br:443` → `acme.tiiv.com.br:443`.
fn origin_host(origin: &str) -> Option<&str> {
    let rest = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"))?;
    Some(rest.split('/').next().unwrap_or(rest))
}
