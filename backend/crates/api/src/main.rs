use anyhow::Context;
use api::{config::Config, state::AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("api=info")),
        )
        .init();

    let config = Config::from_env()?;
    let pool = db::connect(&config.database_url)
        .await
        .context("conectando/migrando o banco")?;
    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("bind em {}", config.bind_addr))?;
    tracing::info!(addr = %config.bind_addr, base_domain = %config.base_domain, "API no ar");

    axum::serve(listener, api::app(AppState::new(pool, config)))
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;
    Ok(())
}
