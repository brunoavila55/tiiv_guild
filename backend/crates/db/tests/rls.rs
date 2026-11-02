//! O RLS isola os tenants mesmo com SQL direto, como `guildos_app`.

use db::guilds::{self, NewGuild};
use db::{members, settings, tenant_tx, users, with_app_role};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;
use uuid::Uuid;

const TENANT_TABLES: &[&str] = &[
    "guild_branding",
    "guild_ranks",
    "guild_members",
    "member_history",
    "guild_settings",
];

async fn new_guild(pool: &PgPool, slug: &str, owner: Uuid) -> sqlx::Result<guilds::Guild> {
    guilds::create(
        pool,
        NewGuild {
            slug,
            name: slug,
            tag: None,
            owner_user_id: owner,
            owner_nickname: "dono",
        },
    )
    .await
}

async fn count(conn: &mut sqlx::PgConnection, table: &str) -> sqlx::Result<i64> {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(conn)
        .await
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenants_are_isolated(opts: PgPoolOptions, conn: PgConnectOptions) -> sqlx::Result<()> {
    let pool = with_app_role(opts).connect_with(conn).await?;
    let owner = users::create(&pool, "dono@example.com", "x", "Dono").await?;
    let a = new_guild(&pool, "guilda-a", owner.id).await?;
    let b = new_guild(&pool, "guilda-b", owner.id).await?;
    for g in [&a, &b] {
        let mut tx = tenant_tx(&pool, g.id).await?;
        settings::set_member_fields(&mut tx, g.id, &serde_json::json!({ "properties": {} }))
            .await?;
        tx.commit().await?;
    }

    // Sem tenant definido nenhuma linha de tenant é visível.
    let mut no_tenant = pool.acquire().await?;
    for table in TENANT_TABLES {
        assert_eq!(count(&mut no_tenant, table).await?, 0, "{table}");
    }
    drop(no_tenant);

    // Com o tenant A, só as linhas de A.
    let mut tx = tenant_tx(&pool, a.id).await?;
    for table in TENANT_TABLES {
        let foreign: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM {table} WHERE guild_id <> $1"
        ))
        .bind(a.id)
        .fetch_one(&mut *tx)
        .await?;
        assert_eq!(foreign, 0, "{table}");
    }
    let all = members::MemberFilter {
        limit: 100,
        ..Default::default()
    };
    assert_eq!(members::page(&mut tx, &all).await?.1, 1);
    assert_eq!(members::ranks(&mut tx).await?.len(), 4);

    // Escrever na guilda B com o tenant A falha (WITH CHECK).
    let write_b = sqlx::query(
        "INSERT INTO guild_ranks (id, guild_id, name, position) VALUES ($1, $2, 'intruso', 99)",
    )
    .bind(Uuid::now_v7())
    .bind(b.id)
    .execute(&mut *tx)
    .await;
    assert!(write_b.is_err());
    drop(tx);

    // Atualizar linhas de B a partir de A não afeta nada.
    let mut tx = tenant_tx(&pool, a.id).await?;
    let updated = sqlx::query("UPDATE guild_members SET nickname = 'hack' WHERE guild_id = $1")
        .bind(b.id)
        .execute(&mut *tx)
        .await?;
    assert_eq!(updated.rows_affected(), 0);
    tx.commit().await?;

    // A única leitura entre tenants é a função `user_guilds`.
    let mine = guilds::user_guilds(&pool, owner.id).await?;
    assert_eq!(mine.len(), 2);
    assert!(mine.iter().all(|g| g.is_owner && g.rank_name == "Líder"));
    Ok(())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn app_role_cannot_bypass_rls(
    opts: PgPoolOptions,
    conn: PgConnectOptions,
) -> sqlx::Result<()> {
    let pool = with_app_role(opts).connect_with(conn).await?;
    let (role, bypass): (String, bool) = sqlx::query_as(
        "SELECT current_user::text, rolbypassrls FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(role, db::APP_ROLE);
    assert!(!bypass);
    Ok(())
}
