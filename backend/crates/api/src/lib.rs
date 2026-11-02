pub mod auth;
pub mod config;
pub mod error;
pub mod features;
pub mod host;
pub mod media;
pub mod state;
pub mod tenant;

use axum::{middleware, Router};
use tower_http::trace::TraceLayer;

use state::AppState;

pub fn app(state: AppState) -> Router {
    Router::new()
        .nest("/api", features::router())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            host::reject_foreign_origin,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
