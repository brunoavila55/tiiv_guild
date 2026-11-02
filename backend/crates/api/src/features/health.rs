use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(live))
        .route("/health/ready", get(ready))
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

/// Liveness: o processo está de pé.
async fn live() -> Json<Health> {
    Json(Health { status: "ok" })
}

/// Readiness: o banco responde.
async fn ready(State(state): State<AppState>) -> Result<Json<Health>, ApiError> {
    sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "banco indisponível");
            ApiError::Unavailable
        })?;
    Ok(Json(Health { status: "ok" }))
}
