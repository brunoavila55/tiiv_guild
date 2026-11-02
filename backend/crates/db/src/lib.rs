//! Acesso ao Postgres: pool, repositórios SQLx e migrations.

use std::time::Duration;

use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, Executor, Postgres, Transaction};

pub mod sessions;
pub mod users;

pub use sqlx::types::Uuid;
pub use sqlx::PgPool;

/// Papel sem ownership nem BYPASSRLS com que a API executa as queries (ver migration base).
pub const APP_ROLE: &str = "guildos_app";

pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

/// Aplica as migrations pendentes com o usuário da URL (dono das tabelas).
pub async fn migrate(database_url: &str) -> Result<(), sqlx::migrate::MigrateError> {
    let mut conn = database_url.parse::<PgConnectOptions>()?.connect().await?;
    MIGRATOR.run(&mut conn).await
}

/// Cria o pool sem abrir conexão imediatamente; a primeira query conecta.
/// Assim a API sobe mesmo que o banco ainda esteja iniciando.
///
/// Toda conexão assume `APP_ROLE`, então RLS vale para qualquer query da API.
pub fn connect_lazy(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                conn.execute(format!("SET ROLE {APP_ROLE}").as_str())
                    .await?;
                Ok(())
            })
        })
        .connect_lazy(database_url)
}

/// Abre uma transação no escopo de uma guilda: `APP_ROLE` + `app.guild_id` locais à transação.
/// Única porta de entrada para queries em tabelas de tenant.
pub async fn begin_tenant(
    pool: &PgPool,
    guild_id: Uuid,
) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    tx.execute(format!("SET LOCAL ROLE {APP_ROLE}").as_str())
        .await?;
    sqlx::query("SELECT set_config('app.guild_id', $1, true)")
        .bind(guild_id.to_string())
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}
