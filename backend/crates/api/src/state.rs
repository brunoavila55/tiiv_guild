use std::sync::Arc;

use db::PgPool;
use games::GameRegistry;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub games: Arc<GameRegistry>,
}

impl AppState {
    pub fn new(pool: PgPool, games: GameRegistry) -> Self {
        Self {
            pool,
            games: Arc::new(games),
        }
    }
}
