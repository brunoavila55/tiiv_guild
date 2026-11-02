//! Uma feature = um módulo com o próprio `router()`.

mod admin;
mod auth;
mod branding;
mod guilds;
mod health;
mod media;
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
        .merge(branding::router())
        .merge(media::router())
        .merge(admin::router())
}
