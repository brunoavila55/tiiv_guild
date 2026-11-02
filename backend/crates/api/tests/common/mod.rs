#![allow(dead_code, clippy::expect_used)]

use api::{config::Config, state::AppState};
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tower::ServiceExt;

pub const BASE: &str = "tiiv.test";
pub const APP: &str = "app.tiiv.test";
pub const ADMIN: &str = "admin.tiiv.test";
pub const SUPERADMIN: &str = "root@example.com";

pub struct TestApp {
    pub router: Router,
}

pub struct Res {
    pub status: StatusCode,
    pub body: Value,
    pub set_cookie: Option<String>,
}

impl Res {
    /// `nome=valor` do Set-Cookie, pronto para o header `Cookie`.
    pub fn cookie(&self) -> String {
        self.set_cookie
            .as_deref()
            .and_then(|c| c.split(';').next())
            .unwrap_or_default()
            .to_owned()
    }
}

pub async fn app(opts: PgPoolOptions, conn: PgConnectOptions) -> TestApp {
    let pool = db::with_app_role(opts)
        .connect_with(conn)
        .await
        .expect("pool de teste");
    let config = Config {
        database_url: String::new(),
        bind_addr: String::new(),
        base_domain: BASE.into(),
        cookie_domain: Some(BASE.into()),
        cookie_secure: true,
        superadmin_emails: vec![SUPERADMIN.into()],
    };
    TestApp {
        router: api::app(AppState::new(pool, config)),
    }
}

pub fn guild_host(slug: &str) -> String {
    format!("{slug}.{BASE}")
}

impl TestApp {
    pub async fn send(
        &self,
        method: Method,
        host: &str,
        path: &str,
        cookie: Option<&str>,
        body: Option<Value>,
        origin: Option<&str>,
    ) -> Res {
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, host);
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        if let Some(o) = origin {
            req = req.header(header::ORIGIN, o);
        }
        let req = match body {
            Some(b) => req
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(b.to_string())),
            None => req.body(Body::empty()),
        }
        .expect("request");
        let res = self.router.clone().oneshot(req).await.expect("resposta");
        let status = res.status();
        let set_cookie = res
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let bytes = res.into_body().collect().await.expect("body").to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Res {
            status,
            body,
            set_cookie,
        }
    }

    pub async fn get(&self, host: &str, path: &str, cookie: Option<&str>) -> Res {
        self.send(Method::GET, host, path, cookie, None, None).await
    }

    pub async fn post(&self, host: &str, path: &str, cookie: Option<&str>, body: Value) -> Res {
        self.send(Method::POST, host, path, cookie, Some(body), None)
            .await
    }

    /// Registra um usuário e devolve o cookie de sessão.
    pub async fn register(&self, email: &str) -> String {
        let res = self
            .post(
                APP,
                "/api/auth/register",
                None,
                json!({ "email": email, "password": "senha-forte-123", "display_name": "Jogador" }),
            )
            .await;
        assert_eq!(res.status, StatusCode::CREATED, "{}", res.body);
        res.cookie()
    }

    /// Cria uma guilda com o usuário do cookie como dono.
    pub async fn create_guild(&self, cookie: &str, slug: &str) {
        let res = self
            .post(
                APP,
                "/api/guilds",
                Some(cookie),
                json!({ "slug": slug, "name": format!("Guilda {slug}") }),
            )
            .await;
        assert_eq!(res.status, StatusCode::CREATED, "{}", res.body);
    }
}
