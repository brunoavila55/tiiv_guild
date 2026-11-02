mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn creates_guild_with_owner_and_default_ranks(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;

    let res = app
        .post(
            APP,
            "/api/guilds",
            Some(&owner),
            json!({ "slug": " Os-Lobos ", "name": "Os Lobos", "tag": "LOB", "nickname": "Alfa" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.body);
    assert_eq!(res.body["slug"], "os-lobos");

    let host = guild_host("os-lobos");
    let guild = app.get(&host, "/api/guild", Some(&owner)).await;
    assert_eq!(guild.status, StatusCode::OK, "{}", guild.body);
    assert_eq!(guild.body["name"], "Os Lobos");
    assert_eq!(guild.body["me"]["nickname"], "Alfa");
    assert_eq!(guild.body["me"]["rank_name"], "Líder");
    assert_eq!(guild.body["me"]["is_owner"], true);
    assert_eq!(
        guild.body["me"]["permissions"].as_array().map(Vec::len),
        Some(7)
    );

    let ranks = app.get(&host, "/api/ranks", Some(&owner)).await;
    let names: Vec<_> = ranks
        .body
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| r["name"].clone())
        .collect();
    assert_eq!(names, vec!["Líder", "Oficial", "Membro", "Recruta"]);

    let members = app.get(&host, "/api/members", Some(&owner)).await;
    assert_eq!(members.body["total"], 1);
    assert_eq!(members.body["items"][0]["has_account"], true);

    let me = app.get(APP, "/api/me", Some(&owner)).await;
    assert_eq!(me.body["guilds"][0]["slug"], "os-lobos");
    assert_eq!(me.body["guilds"][0]["is_owner"], true);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn validates_slug(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let user = app.register("u@example.com").await;
    app.create_guild(&user, "acme").await;

    for slug in ["ab", "app", "admin", "www", "com espaço", "-x-", "acme"] {
        let res = app
            .post(
                APP,
                "/api/guilds",
                Some(&user),
                json!({ "slug": slug, "name": "Qualquer" }),
            )
            .await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{slug}");
        assert!(res.body["fields"]["slug"].is_string(), "{slug}");
    }

    let taken = app.get(APP, "/api/guilds/slug-check?slug=acme", None).await;
    assert_eq!(taken.body["available"], false);
    let reserved = app
        .get(APP, "/api/guilds/slug-check?slug=admin", None)
        .await;
    assert_eq!(reserved.body["available"], false);
    let free = app
        .get(APP, "/api/guilds/slug-check?slug=nova-guilda", None)
        .await;
    assert_eq!(free.body, json!({ "available": true, "reason": null }));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_guild_requires_session(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let res = app
        .post(
            APP,
            "/api/guilds",
            None,
            json!({ "slug": "acme", "name": "Acme" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_resolution_by_host(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    let stranger = app.register("estranho@example.com").await;
    app.create_guild(&owner, "acme").await;
    let host = guild_host("acme");

    // Identidade pública da guilda existe sem login; host sem guilda → 404.
    let public = app.get(&host, "/api/tenant", None).await;
    assert_eq!(public.status, StatusCode::OK);
    assert_eq!(public.body["name"], "Guilda acme");
    assert_eq!(public.body["public_page_enabled"], false);
    assert_eq!(
        app.get(&guild_host("nao-existe"), "/api/tenant", None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );

    // Rotas de tenant só existem em host de guilda.
    for h in [APP, ADMIN, BASE, "outro-dominio.com"] {
        assert_eq!(
            app.get(h, "/api/guild", Some(&owner)).await.status,
            StatusCode::NOT_FOUND,
            "{h}"
        );
    }

    assert_eq!(
        app.get(&host, "/api/guild", None).await.status,
        StatusCode::UNAUTHORIZED
    );
    let res = app.get(&host, "/api/guild", Some(&stranger)).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(
        app.get(&host, "/api/members", Some(&stranger)).await.status,
        StatusCode::FORBIDDEN
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn guilds_are_isolated_by_host(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let ana = app.register("ana@example.com").await;
    let bia = app.register("bia@example.com").await;
    app.create_guild(&ana, "guilda-a").await;
    app.create_guild(&bia, "guilda-b").await;

    // Cada dona vê só a própria guilda, e o cookie dela não abre a outra.
    let a = app
        .get(&guild_host("guilda-a"), "/api/members", Some(&ana))
        .await;
    assert_eq!(a.status, StatusCode::OK);
    assert_eq!(a.body["total"], 1);
    assert_eq!(a.body["items"][0]["nickname"], "Jogador");
    assert_eq!(
        app.get(&guild_host("guilda-b"), "/api/members", Some(&ana))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.get(&guild_host("guilda-a"), "/api/ranks", Some(&bia))
            .await
            .status,
        StatusCode::FORBIDDEN
    );

    let me = app.get(APP, "/api/me", Some(&ana)).await;
    assert_eq!(me.body["guilds"].as_array().map(Vec::len), Some(1));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_only_on_admin_host_for_superadmin(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let root = app.register(SUPERADMIN).await;
    let user = app.register("u@example.com").await;
    app.create_guild(&user, "acme").await;

    let res = app.get(ADMIN, "/api/admin/guilds", Some(&root)).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body[0]["slug"], "acme");
    assert_eq!(res.body[0]["owner_email"], "u@example.com");

    assert_eq!(
        app.get(APP, "/api/admin/guilds", Some(&root)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.get(ADMIN, "/api/admin/guilds", Some(&user))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.get(ADMIN, "/api/admin/guilds", None).await.status,
        StatusCode::UNAUTHORIZED
    );

    // Superadmin não é membro de tenant.
    assert_eq!(
        app.get(&guild_host("acme"), "/api/guild", Some(&root))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.get(APP, "/api/me", Some(&root)).await.body["is_superadmin"],
        true
    );
}
