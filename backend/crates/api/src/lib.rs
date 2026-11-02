//! API HTTP (Axum): rotas, extractors e middlewares.

pub mod config;
pub mod error;
pub mod features;
pub mod state;

use axum::Router;
use tower_http::trace::TraceLayer;

pub use config::Config;
pub use error::ApiError;
pub use state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/api", features::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
