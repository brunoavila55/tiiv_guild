//! Uma feature = um módulo com o próprio `router()`.

mod admin;
mod auth;
mod guilds;
mod health;
mod members;
mod tenant;

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(guilds::router())
        .merge(tenant::router())
        .merge(members::router())
        .merge(admin::router())
}
