#![allow(clippy::expect_used)]

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

const PATH: &str = "/api/settings/member-fields";

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "discord": { "type": "string", "title": "Discord", "maxLength": 32 },
            "level": { "type": "integer", "title": "Nível", "minimum": 1, "maximum": 100 },
            "role": { "type": "string", "title": "Função", "enum": ["Tank", "Healer"] },
            "mic": { "type": "boolean", "title": "Microfone" }
        },
        "required": ["level"],
        "x-order": ["role", "level"]
    })
}

async fn rank_id(app: &TestApp, host: &str, cookie: &str, name: &str) -> String {
    let ranks = app.get(host, "/api/ranks", Some(cookie)).await;
    ranks
        .body
        .as_array()
        .into_iter()
        .flatten()
        .find(|r| r["name"] == name)
        .and_then(|r| r["id"].as_str())
        .expect("rank")
        .to_owned()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn guild_defines_and_validates_member_fields(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.create_guild(&owner, "acme").await;
    let host = guild_host("acme");

    // Guilda nova: schema vazio.
    let res = app.get(&host, PATH, Some(&owner)).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["properties"], json!({}));

    // Schema inválido → 422 por campo, nada salvo.
    let res = app
        .put(
            &host,
            PATH,
            Some(&owner),
            json!({ "properties": { "x": { "type": "string", "title": "X", "minimum": 1 } } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);
    assert!(res.body["fields"]["properties.x"].is_string());
    let res = app
        .put(&host, PATH, Some(&owner), json!({ "properties": [] }))
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);

    // Salva e devolve normalizado, com a ordem completa.
    let res = app.put(&host, PATH, Some(&owner), schema()).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    assert_eq!(
        res.body["x-order"],
        json!(["role", "level", "discord", "mic"])
    );
    assert_eq!(app.get(&host, PATH, Some(&owner)).await.body, res.body);

    // Cadastro sem campo obrigatório ou com valor inválido → 422 em `fields.<chave>`.
    let membro = rank_id(&app, &host, &owner, "Membro").await;
    let res = app
        .post(
            &host,
            "/api/members",
            Some(&owner),
            json!({ "nickname": "Novo", "rank_id": membro, "fields": { "role": "DPS", "ghost": 1 } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);
    for key in ["fields.level", "fields.role", "fields.ghost"] {
        assert!(res.body["fields"][key].is_string(), "{key}: {}", res.body);
    }

    let res = app
        .post(
            &host,
            "/api/members",
            Some(&owner),
            json!({
                "nickname": "Novo", "rank_id": membro,
                "fields": { "level": 10, "role": "Tank", "discord": "  novo#1 ", "mic": null }
            }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.body);
    assert_eq!(
        res.body["fields"],
        json!({ "level": 10, "role": "Tank", "discord": "novo#1" })
    );
    let path = format!("/api/members/{}", res.body["id"].as_str().expect("id"));

    // Edição sem `fields` não mexe neles; com `fields` substitui e entra no histórico.
    let res = app
        .patch(&host, &path, Some(&owner), json!({ "nickname": "Novo2" }))
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    assert_eq!(res.body["fields"]["level"], 10);

    let res = app
        .patch(
            &host,
            &path,
            Some(&owner),
            json!({ "fields": { "level": 101 } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);
    assert!(res.body["fields"]["fields.level"].is_string());

    let res = app
        .patch(
            &host,
            &path,
            Some(&owner),
            json!({ "fields": { "level": 20, "role": "Healer", "mic": true } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    assert_eq!(
        res.body["fields"],
        json!({ "level": 20, "role": "Healer", "mic": true })
    );
    let history = app
        .get(&host, &format!("{path}/history"), Some(&owner))
        .await;
    let last = &history.body[0];
    assert_eq!(last["action"], "member.updated");
    assert_eq!(
        last["payload"]["fields.level"],
        json!({ "from": 10, "to": 20 })
    );
    assert_eq!(
        last["payload"]["fields.discord"],
        json!({ "from": "novo#1", "to": null })
    );

    // Campo removido do schema some da ficha (o valor fica no banco).
    let mut smaller = schema();
    smaller["properties"]
        .as_object_mut()
        .expect("properties")
        .remove("mic");
    let res = app.put(&host, PATH, Some(&owner), smaller).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    let detail = app.get(&host, &path, Some(&owner)).await;
    assert_eq!(
        detail.body["fields"],
        json!({ "level": 20, "role": "Healer" })
    );

    // A página pública nunca mostra os campos.
    app.put(
        &host,
        "/api/branding",
        Some(&owner),
        json!({ "public_page_enabled": true }),
    )
    .await;
    let public = app.get(&host, "/api/public", None).await;
    assert_eq!(public.status, StatusCode::OK, "{}", public.body);
    assert!(!public.body.to_string().contains("Healer"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn member_fields_permissions_and_isolation(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    let membro_user = app.register("membro@example.com").await;
    let other = app.register("outro@example.com").await;
    app.create_guild(&owner, "acme").await;
    app.create_guild(&other, "beta").await;
    app.add_member("acme", "membro@example.com", "Membro").await;
    let acme = guild_host("acme");
    let beta = guild_host("beta");

    // Membro comum lê o schema (precisa dele para ver as fichas), mas não altera.
    assert_eq!(
        app.get(&acme, PATH, Some(&membro_user)).await.status,
        StatusCode::OK
    );
    let res = app.put(&acme, PATH, Some(&membro_user), schema()).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(
        app.get(&acme, PATH, None).await.status,
        StatusCode::UNAUTHORIZED
    );

    // O schema de uma guilda não vaza para a outra, nem pelo host da outra.
    let res = app.put(&acme, PATH, Some(&owner), schema()).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    let res = app.get(&beta, PATH, Some(&other)).await;
    assert_eq!(res.body["properties"], json!({}));
    assert_eq!(
        app.put(&beta, PATH, Some(&owner), schema()).await.status,
        StatusCode::FORBIDDEN
    );

    // Na guilda B os campos da A não existem.
    let membro = rank_id(&app, &beta, &other, "Membro").await;
    let res = app
        .post(
            &beta,
            "/api/members",
            Some(&other),
            json!({ "nickname": "Xis", "rank_id": membro, "fields": { "level": 5 } }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);
    assert!(res.body["fields"]["fields.level"].is_string());
}
