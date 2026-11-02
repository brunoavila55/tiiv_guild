//! Uma feature = um módulo. Cada uma expõe `routes()` e é montada aqui.

use axum::Router;

use crate::AppState;

pub mod auth;
pub mod health;

pub fn routes() -> Router<AppState> {
    Router::new().merge(health::routes()).merge(auth::routes())
}
