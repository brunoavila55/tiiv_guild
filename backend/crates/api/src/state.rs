use std::sync::Arc;

use db::PgPool;

use crate::{config::Config, media::Media};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub media: Arc<Media>,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config) -> anyhow::Result<Self> {
        let media = Media::from_config(&config.storage)?;
        Ok(Self {
            pool,
            config: Arc::new(config),
            media: Arc::new(media),
        })
    }
}
