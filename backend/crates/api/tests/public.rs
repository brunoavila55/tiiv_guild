#![allow(clippy::expect_used)]

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

/// Publica (ou despublica) a página da guilda com um lema.
async fn set_public(app: &TestApp, cookie: &str, slug: &str, enabled: bool) {
    let res = app
        .put(
            &guild_host(slug),
            "/api/branding",
            Some(cookie),
            json!({
                "primary_color": "#ff8800",
                "motto": format!("Lema {slug}"),
                "description_md": "## Sobre\nGuilda de amigos.",
                "public_page_enabled": enabled,
                "socials": { "discord": "https://discord.gg/acme" }
            }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
}

/// Membro sem conta, direto no banco (até existir a gestão de membros).
async fn add_offline_member(app: &TestApp, slug: &str, nickname: &str, rank: &str, status: &str) {
    let guild = db::guilds::find_by_slug(&app.pool, slug)
        .await
        .expect("query")
        .expect("guilda");
    let mut tx = db::tenant_tx(&app.pool, guild.id).await.expect("tx");
    sqlx::query(
        "INSERT INTO guild_members (id, guild_id, nickname, rank_id, status, notes, fields)
         SELECT $1, $2, $3, id, $5::member_status, 'nota-secreta', '{\"campo\":\"valor-secreto\"}'
           FROM guild_ranks WHERE name = $4",
    )
    .bind(Uuid::now_v7())
    .bind(guild.id)
    .bind(nickname)
    .bind(rank)
    .bind(status)
    .execute(&mut *tx)
    .await
    .expect("insert");
    tx.commit().await.expect("commit");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn public_page_shows_branding_and_active_members(
    opts: PgPoolOptions,
    conn: PgConnectOptions,
) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.create_guild(&owner, "acme").await;
    set_public(&app, &owner, "acme", true).await;
    add_offline_member(&app, "acme", "bravo", "Membro", "active").await;
    add_offline_member(&app, "acme", "alfa", "Membro", "active").await;
    add_offline_member(&app, "acme", "recruta", "Recruta", "active").await;
    for status in ["trial", "inactive", "left", "banned"] {
        add_offline_member(&app, "acme", &format!("oculto-{status}"), "Membro", status).await;
    }

    // Sem login.
    let res = app.get(&guild_host("acme"), "/api/public", None).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    assert_eq!(res.body["name"], "Guilda acme");
    assert_eq!(res.body["motto"], "Lema acme");
    assert_eq!(res.body["primary_color"], "#ff8800");
    assert_eq!(res.body["description_md"], "## Sobre\nGuilda de amigos.");
    assert_eq!(
        res.body["socials"],
        json!({ "discord": "https://discord.gg/acme" })
    );
    assert_eq!(res.body["member_count"], 4);
    // Ordem da hierarquia, apelidos em ordem alfabética, ranks vazios omitidos.
    assert_eq!(
        res.body["ranks"],
        json!([
            { "name": "Líder", "members": ["Jogador"] },
            { "name": "Membro", "members": ["alfa", "bravo"] },
            { "name": "Recruta", "members": ["recruta"] },
        ])
    );

    // Nada privado vaza.
    let raw = String::from_utf8_lossy(&res.bytes);
    for secret in [
        "dono@example.com",
        "nota-secreta",
        "valor-secreto",
        "oculto-",
    ] {
        assert!(!raw.contains(secret), "{secret} vazou: {raw}");
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disabled_or_unknown_public_page_is_404(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.create_guild(&owner, "acme").await;
    let host = guild_host("acme");

    // Guilda nova nasce com a página desligada.
    let res = app.get(&host, "/api/public", None).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    // Nem o dono vê enquanto estiver desligada.
    let res = app.get(&host, "/api/public", Some(&owner)).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    set_public(&app, &owner, "acme", true).await;
    assert_eq!(
        app.get(&host, "/api/public", None).await.status,
        StatusCode::OK
    );
    set_public(&app, &owner, "acme", false).await;
    assert_eq!(
        app.get(&host, "/api/public", None).await.status,
        StatusCode::NOT_FOUND
    );

    for host in [
        guild_host("nao-existe"),
        APP.into(),
        ADMIN.into(),
        BASE.into(),
    ] {
        assert_eq!(
            app.get(&host, "/api/public", None).await.status,
            StatusCode::NOT_FOUND,
            "{host}"
        );
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn public_page_is_isolated_per_guild(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let a = app.register("a@example.com").await;
    let b = app.register("b@example.com").await;
    app.create_guild(&a, "guilda-a").await;
    app.create_guild(&b, "guilda-b").await;
    set_public(&app, &a, "guilda-a", true).await;
    set_public(&app, &b, "guilda-b", true).await;
    add_offline_member(&app, "guilda-a", "so-da-a", "Membro", "active").await;
    add_offline_member(&app, "guilda-b", "so-da-b", "Membro", "active").await;

    let res_a = app.get(&guild_host("guilda-a"), "/api/public", None).await;
    let res_b = app.get(&guild_host("guilda-b"), "/api/public", None).await;
    let raw_a = String::from_utf8_lossy(&res_a.bytes);
    let raw_b = String::from_utf8_lossy(&res_b.bytes);
    assert!(
        raw_a.contains("so-da-a") && !raw_a.contains("so-da-b"),
        "{raw_a}"
    );
    assert!(
        raw_b.contains("so-da-b") && !raw_b.contains("so-da-a"),
        "{raw_b}"
    );
    assert_eq!(res_a.body["motto"], "Lema guilda-a");
    assert_eq!(res_b.body["motto"], "Lema guilda-b");
    assert_eq!(res_a.body["member_count"], 2);
}
