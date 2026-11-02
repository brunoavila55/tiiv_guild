use anyhow::Context;
use api::{AppState, Config};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // .env é opcional (dev local); em produção as variáveis vêm do ambiente.
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let games = games::GameRegistry::builtin().context("game packs inválidos")?;
    {
        let mut owner = db::connect_owner(&config.database_url)
            .await
            .context("falha ao conectar no banco")?;
        db::migrate(&mut owner)
            .await
            .context("falha ao aplicar migrations")?;
        api::features::games::sync_catalog(&mut owner, &games)
            .await
            .context("falha ao sincronizar o catálogo de jogos")?;
    }
    tracing::info!("migrations aplicadas e catálogo de jogos sincronizado");

    let pool = db::connect_lazy(&config.database_url).context("DATABASE_URL inválida")?;
    let app = api::router(AppState::new(pool, games));

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("não foi possível escutar em {}", config.bind_addr))?;
    tracing::info!(addr = %config.bind_addr, "API ouvindo");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("servidor HTTP falhou")?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %err, "falha ao escutar Ctrl+C");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(err) => {
                tracing::error!(error = %err, "falha ao escutar SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("encerrando");
}
