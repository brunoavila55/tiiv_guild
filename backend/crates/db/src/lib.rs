//! Acesso ao Postgres: pool, repositórios SQLx e migrations.

use std::time::Duration;

use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, Executor, PgConnection, Postgres, Transaction};

pub mod catalog;
pub mod sessions;
pub mod users;

pub use sqlx::types::Uuid;
pub use sqlx::PgPool;

/// Papel sem ownership nem BYPASSRLS com que a API executa as queries (ver migration base).
pub const APP_ROLE: &str = "guildos_app";

pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

/// Conexão avulsa com o usuário da URL (dono das tabelas), sem `APP_ROLE`.
/// Usada só no startup: migrations e sincronização do catálogo de jogos.
pub async fn connect_owner(database_url: &str) -> Result<PgConnection, sqlx::Error> {
    database_url.parse::<PgConnectOptions>()?.connect().await
}

/// Aplica as migrations pendentes.
pub async fn migrate(conn: &mut PgConnection) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(conn).await
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
