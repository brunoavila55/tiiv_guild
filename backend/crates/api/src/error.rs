use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use domain::FieldErrors;
use serde::Serialize;

/// Erro de API: vira `{ "code", "message", "fields"? }` com o status HTTP correspondente.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    Domain(#[from] domain::Error),
    #[error("e-mail ou senha incorretos")]
    InvalidCredentials,
    #[error("origem da requisição não permitida")]
    ForeignOrigin,
    #[error("erro interno")]
    Internal(#[from] anyhow::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        Self::Internal(e.into())
    }
}

#[derive(Serialize)]
struct Body<'a> {
    code: &'a str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<&'a FieldErrors>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        use domain::Error as D;
        let (status, code, fields) = match &self {
            Self::Domain(D::Validation(f)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                Some(f),
            ),
            Self::Domain(D::Conflict(_)) => (StatusCode::CONFLICT, "conflict", None),
            Self::Domain(D::NotFound) => (StatusCode::NOT_FOUND, "not_found", None),
            Self::Domain(D::Unauthenticated) => (StatusCode::UNAUTHORIZED, "unauthenticated", None),
            Self::Domain(D::Forbidden) => (StatusCode::FORBIDDEN, "forbidden", None),
            Self::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid_credentials", None),
            Self::ForeignOrigin => (StatusCode::FORBIDDEN, "foreign_origin", None),
            Self::Internal(e) => {
                tracing::error!(error = ?e, "erro interno");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal", None)
            }
        };
        let body = Body {
            code,
            message: self.to_string(),
            fields,
        };
        (status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
