//! Acesso ao Postgres. Toda conexão do pool roda como `guildos_app` (sujeita ao RLS);
//! toda query de tenant roda dentro de [`tenant_tx`].

pub mod guilds;
pub mod members;
pub mod ranks;
pub mod sessions;
pub mod settings;
pub mod users;

use sqlx::{
    migrate::Migrator,
    postgres::{PgConnectOptions, PgPoolOptions},
    Connection, Executor, PgConnection, Postgres, Transaction,
};
use uuid::Uuid;

pub use sqlx::PgPool;

pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

/// Papel sem ownership nem BYPASSRLS usado por todas as queries da aplicação.
pub const APP_ROLE: &str = "guildos_app";

pub type Tx = Transaction<'static, Postgres>;

/// Aplica as migrations como dono do schema e devolve o pool da aplicação.
pub async fn connect(url: &str) -> sqlx::Result<PgPool> {
    let opts: PgConnectOptions = url.parse()?;
    let mut owner = PgConnection::connect_with(&opts).await?;
    MIGRATOR.run(&mut owner).await?;
    owner.close().await?;
    with_app_role(PgPoolOptions::new().max_connections(10))
        .connect_with(opts)
        .await
}

/// Faz toda conexão nova do pool assumir [`APP_ROLE`].
pub fn with_app_role(opts: PgPoolOptions) -> PgPoolOptions {
    opts.after_connect(|conn, _meta| {
        Box::pin(async move {
            conn.execute(format!("SET ROLE {APP_ROLE}").as_str())
                .await
                .map(|_| ())
        })
    })
}

/// Abre uma transação com o tenant definido (`app.guild_id`), escopo das policies de RLS.
pub async fn tenant_tx(pool: &PgPool, guild_id: Uuid) -> sqlx::Result<Tx> {
    let mut tx = pool.begin().await?;
    set_tenant(&mut tx, guild_id).await?;
    Ok(tx)
}

async fn set_tenant(conn: &mut PgConnection, guild_id: Uuid) -> sqlx::Result<()> {
    sqlx::query!(
        "SELECT set_config('app.guild_id', $1, true)",
        guild_id.to_string()
    )
    .fetch_one(conn)
    .await?;
    Ok(())
}

/// `true` se o erro é violação de unicidade da constraint/índice `name`.
pub fn is_unique_violation(err: &sqlx::Error, name: &str) -> bool {
    match err {
        sqlx::Error::Database(e) => e.is_unique_violation() && e.constraint() == Some(name),
        _ => false,
    }
}
