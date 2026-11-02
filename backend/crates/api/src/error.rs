use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use domain::DomainError;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error("não autenticado")]
    Unauthorized,

    #[error("serviço indisponível")]
    Unavailable,

    #[error("erro interno")]
    Internal(#[source] anyhow::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        Self::Internal(err.into())
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: String,
}

impl ApiError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::Domain(DomainError::NotFound) => (StatusCode::NOT_FOUND, "not_found"),
            Self::Domain(DomainError::Forbidden) => (StatusCode::FORBIDDEN, "forbidden"),
            Self::Domain(DomainError::Conflict(_)) => (StatusCode::CONFLICT, "conflict"),
            Self::Domain(DomainError::Validation { .. }) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "validation_failed")
            }
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        if let Self::Internal(err) = &self {
            tracing::error!(error = ?err, "erro interno");
        }
        let body = ErrorBody {
            code,
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}
