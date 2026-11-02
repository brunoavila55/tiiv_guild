#![allow(dead_code, clippy::expect_used)]

use api::{config::Config, media::StorageConfig, state::AppState};
use axum::{
    body::Body,
    http::{header, request::Builder, HeaderMap, Method, Request, StatusCode},
    Router,
};
use db::PgPool;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

pub const BASE: &str = "tiiv.test";
pub const APP: &str = "app.tiiv.test";
pub const ADMIN: &str = "admin.tiiv.test";
pub const SUPERADMIN: &str = "root@example.com";

pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
}

pub struct Res {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub bytes: Vec<u8>,
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

    pub fn header(&self, name: header::HeaderName) -> &str {
        self.headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
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
        storage: StorageConfig::Memory,
    };
    TestApp {
        router: api::app(AppState::new(pool.clone(), config).expect("estado")),
        pool,
    }
}

pub fn guild_host(slug: &str) -> String {
    format!("{slug}.{BASE}")
}

/// PNG mínimo válido (só a assinatura importa para a validação).
pub fn png(extra: usize) -> Vec<u8> {
    let mut b = b"\x89PNG\r\n\x1a\n".to_vec();
    b.resize(8 + extra, 7);
    b
}

fn builder(method: Method, host: &str, path: &str, cookie: Option<&str>) -> Builder {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header(header::HOST, host);
    match cookie {
        Some(c) => req.header(header::COOKIE, c),
        None => req,
    }
}

impl TestApp {
    pub async fn request(&self, req: Request<Body>) -> Res {
        let res = self.router.clone().oneshot(req).await.expect("resposta");
        let status = res.status();
        let headers = res.headers().clone();
        let set_cookie = headers
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let bytes = res
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes()
            .to_vec();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Res {
            status,
            headers,
            bytes,
            body,
            set_cookie,
        }
    }

    pub async fn send(
        &self,
        method: Method,
        host: &str,
        path: &str,
        cookie: Option<&str>,
        body: Option<Value>,
        origin: Option<&str>,
    ) -> Res {
        let mut req = builder(method, host, path, cookie);
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
        self.request(req).await
    }

    pub async fn get(&self, host: &str, path: &str, cookie: Option<&str>) -> Res {
        self.send(Method::GET, host, path, cookie, None, None).await
    }

    pub async fn post(&self, host: &str, path: &str, cookie: Option<&str>, body: Value) -> Res {
        self.send(Method::POST, host, path, cookie, Some(body), None)
            .await
    }

    pub async fn put(&self, host: &str, path: &str, cookie: Option<&str>, body: Value) -> Res {
        self.send(Method::PUT, host, path, cookie, Some(body), None)
            .await
    }

    pub async fn delete(&self, host: &str, path: &str, cookie: Option<&str>) -> Res {
        self.send(Method::DELETE, host, path, cookie, None, None)
            .await
    }

    /// `PUT` multipart com o arquivo no campo `file`.
    pub async fn upload(&self, host: &str, path: &str, cookie: Option<&str>, file: &[u8]) -> Res {
        let boundary = "XTESTBOUNDARYX";
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"img\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(file);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let req = builder(Method::PUT, host, path, cookie)
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .expect("request");
        self.request(req).await
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

    /// Vincula a conta `email` à guilda no rank `rank` (direto no banco, até existir o convite).
    pub async fn add_member(&self, slug: &str, email: &str, rank: &str) {
        let guild = db::guilds::find_by_slug(&self.pool, slug)
            .await
            .expect("query")
            .expect("guilda");
        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(email)
            .fetch_one(&self.pool)
            .await
            .expect("usuário");
        let mut tx = db::tenant_tx(&self.pool, guild.id).await.expect("tx");
        sqlx::query(
            "INSERT INTO guild_members (id, guild_id, user_id, nickname, rank_id)
             SELECT $1, $2, $3, 'membro', id FROM guild_ranks WHERE name = $4",
        )
        .bind(Uuid::now_v7())
        .bind(guild.id)
        .bind(user_id)
        .bind(rank)
        .execute(&mut *tx)
        .await
        .expect("insert");
        tx.commit().await.expect("commit");
    }
}
