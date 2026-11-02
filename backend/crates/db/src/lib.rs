//! Acesso ao Postgres: pool, repositórios SQLx e migrations.

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

pub use sqlx::PgPool;

/// Cria o pool sem abrir conexão imediatamente; a primeira query conecta.
/// Assim a API sobe mesmo que o banco ainda esteja iniciando.
pub fn connect_lazy(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect_lazy(database_url)
}
