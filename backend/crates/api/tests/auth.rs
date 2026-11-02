mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn health(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let res = app.get(APP, "/api/health", None).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["status"], "ok");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn register_login_me_logout(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;

    let res = app
        .post(
            APP,
            "/api/auth/register",
            None,
            json!({ "email": " Ana@Example.com ", "password": "senha-forte-123", "display_name": "Ana" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.body["email"], "ana@example.com");
    let set_cookie = res.set_cookie.clone().unwrap_or_default();
    for attr in [
        "HttpOnly",
        "SameSite=Lax",
        "Secure",
        "Domain=tiiv.test",
        "Path=/",
    ] {
        assert!(set_cookie.contains(attr), "{attr} em {set_cookie}");
    }

    // O mesmo cookie vale em qualquer subdomínio.
    let me = app
        .get("qualquer.tiiv.test", "/api/me", Some(&res.cookie()))
        .await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["user"]["display_name"], "Ana");
    assert_eq!(me.body["is_superadmin"], false);
    assert_eq!(me.body["guilds"], json!([]));

    let bad = app
        .post(
            APP,
            "/api/auth/login",
            None,
            json!({ "email": "ana@example.com", "password": "errada-123" }),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNAUTHORIZED);
    assert_eq!(bad.body["code"], "invalid_credentials");

    let login = app
        .post(
            APP,
            "/api/auth/login",
            None,
            json!({ "email": "ANA@example.com", "password": "senha-forte-123" }),
        )
        .await;
    assert_eq!(login.status, StatusCode::OK);
    let cookie = login.cookie();

    let out = app
        .post(APP, "/api/auth/logout", Some(&cookie), json!({}))
        .await;
    assert_eq!(out.status, StatusCode::NO_CONTENT);
    assert!(out.set_cookie.unwrap_or_default().contains("Max-Age=0"));
    assert_eq!(
        app.get(APP, "/api/me", Some(&cookie)).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn me_requires_session(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let res = app.get(APP, "/api/me", None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.body["code"], "unauthenticated");
    let forged = app
        .get(APP, "/api/me", Some("guildos_session=deadbeef"))
        .await;
    assert_eq!(forged.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn register_validates(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let res = app
        .post(
            APP,
            "/api/auth/register",
            None,
            json!({ "email": "x", "password": "1", "display_name": "" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    for f in ["email", "password", "display_name"] {
        assert!(res.body["fields"][f].is_string(), "{f}");
    }

    app.register("dup@example.com").await;
    let dup = app
        .post(
            APP,
            "/api/auth/register",
            None,
            json!({ "email": "DUP@example.com", "password": "senha-forte-123", "display_name": "Outro" }),
        )
        .await;
    assert_eq!(dup.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(dup.body["fields"]["email"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rejects_foreign_origin(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let body =
        json!({ "email": "a@example.com", "password": "senha-forte-123", "display_name": "Ana" });
    let evil = app
        .send(
            Method::POST,
            APP,
            "/api/auth/register",
            None,
            Some(body.clone()),
            Some("https://evil.com"),
        )
        .await;
    assert_eq!(evil.status, StatusCode::FORBIDDEN);
    assert_eq!(evil.body["code"], "foreign_origin");

    let lookalike = app
        .send(
            Method::POST,
            APP,
            "/api/auth/register",
            None,
            Some(body.clone()),
            Some("https://eviltiiv.test"),
        )
        .await;
    assert_eq!(lookalike.status, StatusCode::FORBIDDEN);

    let ok = app
        .send(
            Method::POST,
            APP,
            "/api/auth/register",
            None,
            Some(body),
            Some("https://app.tiiv.test"),
        )
        .await;
    assert_eq!(ok.status, StatusCode::CREATED);
}
