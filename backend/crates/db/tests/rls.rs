//! Isolamento entre guildas (P0.2): RLS no Postgres com o papel da aplicação.
//!
//! O pool do `sqlx::test` conecta como dono das tabelas (sem RLS) e é usado só para
//! semear dados; as asserções rodam como `guildos_app`, igual à API.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use db::{begin_tenant, PgPool, Uuid, APP_ROLE};
use shared::ids::new_id;
use sqlx::Executor;

const TENANT_TABLES: [&str; 5] = [
    "guild_branding",
    "guild_ranks",
    "guild_members",
    "characters",
    "member_history",
];

struct Guild {
    id: Uuid,
    rank_id: Uuid,
    member_id: Uuid,
}

struct Seed {
    a: Guild,
    b: Guild,
}

async fn seed(pool: &PgPool) -> Seed {
    let user_id = new_id();
    let game_id = new_id();
    let class_id = new_id();
    sqlx::query(
        "INSERT INTO users (id, email, display_name) VALUES ($1, 'lider@example.com', 'Líder')",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO games (id, slug, name) VALUES ($1, 'test-game', 'Test Game')")
        .bind(game_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO game_classes (id, game_id, slug, name, role_hint) VALUES ($1, $2, 'warrior', 'Warrior', 'tank')",
    )
    .bind(class_id)
    .bind(game_id)
    .execute(pool)
    .await
    .unwrap();

    let mut guilds = Vec::new();
    for slug in ["guilda-a", "guilda-b"] {
        let guild = Guild {
            id: new_id(),
            rank_id: new_id(),
            member_id: new_id(),
        };
        sqlx::query(
            "INSERT INTO guilds (id, game_id, slug, name, owner_user_id) VALUES ($1, $2, $3, $3, $4)",
        )
        .bind(guild.id)
        .bind(game_id)
        .bind(slug)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO guild_branding (guild_id) VALUES ($1)")
            .bind(guild.id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO guild_ranks (id, guild_id, name, position, permissions) VALUES ($1, $2, 'Líder', 0, '{members.manage}')",
        )
        .bind(guild.rank_id)
        .bind(guild.id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO guild_members (id, guild_id, user_id, nickname, rank_id) VALUES ($1, $2, $3, 'lider', $4)",
        )
        .bind(guild.member_id)
        .bind(guild.id)
        .bind(user_id)
        .bind(guild.rank_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO characters (id, guild_id, member_id, game_class_id, name, level, is_main) VALUES ($1, $2, $3, $4, 'Main', 100, true)",
        )
        .bind(new_id())
        .bind(guild.id)
        .bind(guild.member_id)
        .bind(class_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO member_history (id, guild_id, member_id, action) VALUES ($1, $2, $3, 'joined')",
        )
        .bind(new_id())
        .bind(guild.id)
        .bind(guild.member_id)
        .execute(pool)
        .await
        .unwrap();
        guilds.push(guild);
    }

    let b = guilds.pop().unwrap();
    let a = guilds.pop().unwrap();
    Seed { a, b }
}

async fn count(conn: &mut sqlx::PgConnection, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(conn).await.unwrap()
}

fn sqlstate(err: &sqlx::Error) -> String {
    err.as_database_error()
        .and_then(|e| e.code())
        .map(|c| c.into_owned())
        .unwrap_or_default()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn app_role_is_not_owner_and_cannot_bypass_rls(pool: PgPool) {
    let (superuser, bypass): (bool, bool) =
        sqlx::query_as("SELECT rolsuper, rolbypassrls FROM pg_roles WHERE rolname = $1")
            .bind(APP_ROLE)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!superuser && !bypass);

    let owned: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_tables WHERE schemaname = 'public' AND tableowner = $1",
    )
    .bind(APP_ROLE)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        owned, 0,
        "o papel da aplicação não pode ser dono de tabelas"
    );

    for table in TENANT_TABLES.iter().chain(&["guilds"]) {
        let rls: bool = sqlx::query_scalar(
            "SELECT relrowsecurity FROM pg_class WHERE oid = ('public.' || $1)::regclass",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(rls, "RLS desligado em {table}");
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn without_tenant_no_tenant_rows_are_visible(pool: PgPool) {
    seed(&pool).await;

    let mut tx = pool.begin().await.unwrap();
    tx.execute(format!("SET LOCAL ROLE {APP_ROLE}").as_str())
        .await
        .unwrap();

    assert_eq!(count(&mut tx, "SELECT count(*) FROM guilds").await, 0);
    for table in TENANT_TABLES {
        let sql = format!("SELECT count(*) FROM {table}");
        assert_eq!(count(&mut tx, &sql).await, 0, "{table} vazou sem tenant");
    }

    // Tabelas globais continuam legíveis.
    assert_eq!(count(&mut tx, "SELECT count(*) FROM games").await, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_reads_only_its_own_rows(pool: PgPool) {
    let seed = seed(&pool).await;

    for guild in [&seed.a, &seed.b] {
        let mut tx = begin_tenant(&pool, guild.id).await.unwrap();

        let current: String = sqlx::query_scalar("SELECT current_user::text")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(current, APP_ROLE);

        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM guilds")
            .fetch_all(&mut *tx)
            .await
            .unwrap();
        assert_eq!(ids, vec![guild.id]);

        for table in TENANT_TABLES {
            let sql = format!("SELECT count(*) FROM {table} WHERE guild_id <> $1");
            let foreign: i64 = sqlx::query_scalar(&sql)
                .bind(guild.id)
                .fetch_one(&mut *tx)
                .await
                .unwrap();
            assert_eq!(foreign, 0, "{table} expôs linhas de outra guilda");

            let sql = format!("SELECT count(*) FROM {table}");
            assert_eq!(count(&mut tx, &sql).await, 1, "{table} sem a linha própria");
        }
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_cannot_write_rows_of_another_guild(pool: PgPool) {
    let seed = seed(&pool).await;

    // INSERT com guild_id de B a partir de A: barrado pelo WITH CHECK.
    let mut tx = begin_tenant(&pool, seed.a.id).await.unwrap();
    let err = sqlx::query(
        "INSERT INTO guild_ranks (id, guild_id, name, position) VALUES ($1, $2, 'Intruso', 9)",
    )
    .bind(new_id())
    .bind(seed.b.id)
    .execute(&mut *tx)
    .await
    .unwrap_err();
    assert_eq!(sqlstate(&err), "42501");
    drop(tx);

    // UPDATE/DELETE em linhas de B não enxergam nada.
    let mut tx = begin_tenant(&pool, seed.a.id).await.unwrap();
    let updated = sqlx::query("UPDATE guild_members SET nickname = 'hack' WHERE id = $1")
        .bind(seed.b.member_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert_eq!(updated.rows_affected(), 0);
    let deleted = sqlx::query("DELETE FROM guilds WHERE id = $1")
        .bind(seed.b.id)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert_eq!(deleted.rows_affected(), 0);

    // Mover uma linha própria para B também é barrado.
    let err = sqlx::query("UPDATE guild_ranks SET guild_id = $1 WHERE id = $2")
        .bind(seed.b.id)
        .bind(seed.a.rank_id)
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(sqlstate(&err), "42501");
    drop(tx);

    let nickname: String = sqlx::query_scalar("SELECT nickname FROM guild_members WHERE id = $1")
        .bind(seed.b.member_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(nickname, "lider");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn member_cannot_use_rank_of_another_guild(pool: PgPool) {
    let seed = seed(&pool).await;

    // Mesmo como dono (sem RLS), a FK composta impede misturar guildas.
    let err = sqlx::query(
        "INSERT INTO guild_members (id, guild_id, nickname, rank_id) VALUES ($1, $2, 'x', $3)",
    )
    .bind(new_id())
    .bind(seed.a.id)
    .bind(seed.b.rank_id)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(sqlstate(&err), "23503");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn member_history_is_append_only_for_the_app(pool: PgPool) {
    let seed = seed(&pool).await;

    let mut tx = begin_tenant(&pool, seed.a.id).await.unwrap();
    let err = sqlx::query("UPDATE member_history SET action = 'edited'")
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(sqlstate(&err), "42501");
    drop(tx);

    let mut tx = begin_tenant(&pool, seed.a.id).await.unwrap();
    let err = sqlx::query("DELETE FROM member_history")
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(sqlstate(&err), "42501");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn only_one_main_character_per_member(pool: PgPool) {
    let seed = seed(&pool).await;
    let class_id: Uuid = sqlx::query_scalar("SELECT id FROM game_classes")
        .fetch_one(&pool)
        .await
        .unwrap();

    let mut tx = begin_tenant(&pool, seed.a.id).await.unwrap();
    let err = sqlx::query(
        "INSERT INTO characters (id, guild_id, member_id, game_class_id, name, level, is_main) VALUES ($1, $2, $3, $4, 'Outro', 90, true)",
    )
    .bind(new_id())
    .bind(seed.a.id)
    .bind(seed.a.member_id)
    .bind(class_id)
    .execute(&mut *tx)
    .await
    .unwrap_err();
    assert_eq!(sqlstate(&err), "23505");
}
