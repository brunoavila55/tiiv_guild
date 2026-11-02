//! Fluxo de autenticação (P0.3) com Postgres real.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{Request, StatusCode};
use axum::Router;
use db::PgPool;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

struct Res {
    status: StatusCode,
    set_cookie: Option<String>,
    body: Value,
}

impl Res {
    /// `nome=valor` do Set-Cookie, pronto para o header `Cookie`.
    fn cookie_pair(&self) -> String {
        let set_cookie = self.set_cookie.as_deref().expect("Set-Cookie ausente");
        set_cookie.split(';').next().unwrap().to_owned()
    }
}

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> Res {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        req = req.header(COOKIE, cookie);
    }
    let req = match body {
        Some(body) => req
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let set_cookie = res
        .headers()
        .get(SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_owned());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Res {
        status,
        set_cookie,
        body,
    }
}

fn app(pool: PgPool) -> Router {
    api::router(api::AppState::new(pool))
}

async fn register(app: &Router, email: &str) -> Res {
    send(
        app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({ "email": email, "password": "senha-forte-123", "display_name": "Líder" })),
    )
    .await
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn register_sets_secure_session_cookie_and_me_works(pool: PgPool) {
    let app = app(pool);

    let res = register(&app, "lider@guilda.com").await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.body["email"], "lider@guilda.com");
    assert!(res.body.get("password_hash").is_none());

    let set_cookie = res.set_cookie.clone().unwrap();
    for attr in ["HttpOnly", "Secure", "SameSite=Lax", "Path=/"] {
        assert!(set_cookie.contains(attr), "{attr} ausente em {set_cookie}");
    }

    let me = send(&app, "GET", "/api/me", Some(&res.cookie_pair()), None).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["id"], res.body["id"]);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn me_without_valid_session_is_401(pool: PgPool) {
    let app = app(pool);

    let res = send(&app, "GET", "/api/me", None, None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.body["code"], "unauthorized");

    let res = send(
        &app,
        "GET",
        "/api/me",
        Some("guildos_session=forjado"),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn password_is_stored_as_argon2id(pool: PgPool) {
    let app = app(pool.clone());
    register(&app, "lider@guilda.com").await;

    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(hash.starts_with("$argon2id$"), "{hash}");

    // Só o hash do token vai para o banco.
    let token_len: i32 = sqlx::query_scalar("SELECT length(token_hash) FROM sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(token_len, 32);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_with_right_and_wrong_password(pool: PgPool) {
    let app = app(pool);
    register(&app, "lider@guilda.com").await;

    let wrong = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "email": "lider@guilda.com", "password": "errada-123" })),
    )
    .await;
    assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong.body["code"], "invalid_credentials");
    assert!(wrong.set_cookie.is_none());

    // E-mail inexistente responde igual a senha errada.
    let unknown = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "email": "ninguem@guilda.com", "password": "senha-forte-123" })),
    )
    .await;
    assert_eq!(unknown.status, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown.body["code"], "invalid_credentials");

    // E-mail com outra caixa ainda entra.
    let ok = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "email": "LIDER@Guilda.com", "password": "senha-forte-123" })),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK);
    let me = send(&app, "GET", "/api/me", Some(&ok.cookie_pair()), None).await;
    assert_eq!(me.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn duplicate_email_is_409_case_insensitive(pool: PgPool) {
    let app = app(pool);
    register(&app, "lider@guilda.com").await;

    let res = register(&app, "Lider@Guilda.com").await;
    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.body["code"], "conflict");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn invalid_registration_is_422_with_field(pool: PgPool) {
    let app = app(pool);

    let res = send(
        &app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({ "email": "lider@guilda.com", "password": "curta", "display_name": "Líder" })),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.body["code"], "validation_failed");
    assert!(res.body["message"].as_str().unwrap().contains("password"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn logout_revokes_session(pool: PgPool) {
    let app = app(pool);
    let cookie = register(&app, "lider@guilda.com").await.cookie_pair();

    let res = send(&app, "POST", "/api/auth/logout", Some(&cookie), None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.set_cookie.unwrap().contains("Max-Age=0"));

    let me = send(&app, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn expired_session_is_401(pool: PgPool) {
    let app = app(pool.clone());
    let cookie = register(&app, "lider@guilda.com").await.cookie_pair();

    sqlx::query("UPDATE sessions SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();

    let me = send(&app, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);
}
