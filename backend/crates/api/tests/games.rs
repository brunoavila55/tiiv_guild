//! Catálogo de jogos (P0.4): cadastrado pelo superadmin, sem nada de jogo no código.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{Request, StatusCode};
use axum::Router;
use db::PgPool;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const ADMIN: &str = "admin@tiiv.com.br";

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value, Option<String>) {
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
    let cookie = res
        .headers()
        .get(SET_COOKIE)
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_owned());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        cookie,
    )
}

fn app(pool: PgPool) -> Router {
    api::router(api::AppState::new(pool).with_superadmins([ADMIN]))
}

/// Cadastra e devolve o cookie de sessão.
async fn login_as(app: &Router, email: &str) -> String {
    let (status, _, cookie) = send(
        app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({ "email": email, "password": "senha-forte-123", "display_name": "Alguém" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    cookie.unwrap()
}

fn perfect_world() -> Value {
    json!({
        "slug": "perfect-world",
        "name": "Perfect World",
        "level_min": 1,
        "level_max": 105,
        "character_schema": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "cultivo": { "type": "string", "enum": ["sabio", "demonio"], "x-enumLabels": ["Sábio", "Demônio"] },
                "reino": { "type": "integer", "minimum": 0, "maximum": 10 }
            }
        }
    })
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_routes_require_superadmin(pool: PgPool) {
    let app = app(pool);

    let (status, _, _) = send(&app, "GET", "/api/admin/games", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let member = login_as(&app, "membro@guilda.com").await;
    let (status, body, _) = send(
        &app,
        "POST",
        "/api/admin/games",
        Some(&member),
        Some(perfect_world()),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "forbidden");

    let (_, me, _) = send(&app, "GET", "/api/me", Some(&member), None).await;
    assert_eq!(me["is_superadmin"], false);

    // E-mail do superadmin é comparado sem diferenciar maiúsculas.
    let admin = login_as(&app, "Admin@Tiiv.com.br").await;
    let (_, me, _) = send(&app, "GET", "/api/me", Some(&admin), None).await;
    assert_eq!(me["is_superadmin"], true);
    let (status, _, _) = send(&app, "GET", "/api/admin/games", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn superadmin_builds_a_game_catalog(pool: PgPool) {
    let app = app(pool);
    let admin = login_as(&app, ADMIN).await;

    let (status, game, _) = send(
        &app,
        "POST",
        "/api/admin/games",
        Some(&admin),
        Some(perfect_world()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{game}");
    // Sem ranks informados, vale o padrão do core.
    assert_eq!(game["default_ranks"][0]["name"], "Líder");

    for (slug, name, role) in [
        ("cleric", "Sacerdote", "healer"),
        ("barbarian", "Bárbaro", "tank"),
        ("seeker", "Retalhador", "tank"),
    ] {
        let (status, class, _) = send(
            &app,
            "POST",
            "/api/admin/games/perfect-world/classes",
            Some(&admin),
            Some(json!({ "slug": slug, "name": name, "role_hint": role, "metadata": { "race": "x" } })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{class}");
    }

    // Servidor de versão antiga: sem Retalhador.
    let (status, _, _) = send(
        &app,
        "POST",
        "/api/admin/games/perfect-world/servers",
        Some(&admin),
        Some(json!({ "slug": "classico", "name": "Clássico", "region": "BR", "class_slugs": ["cleric", "barbarian"] })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _, _) = send(
        &app,
        "POST",
        "/api/admin/games/perfect-world/servers",
        Some(&admin),
        Some(json!({ "slug": "novo", "name": "Novo" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Leitura pública (sem login).
    let (status, pw, _) = send(&app, "GET", "/api/games/perfect-world", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(pw["level_max"], 105);
    assert_eq!(pw["classes"].as_array().unwrap().len(), 3);
    assert!(pw["character_schema"]["properties"]["cultivo"].is_object());
    let servers = pw["servers"].as_array().unwrap();
    let classico = servers.iter().find(|s| s["slug"] == "classico").unwrap();
    let mut allowed: Vec<&str> = classico["class_slugs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    allowed.sort_unstable();
    assert_eq!(allowed, ["barbarian", "cleric"]);
    let novo = servers.iter().find(|s| s["slug"] == "novo").unwrap();
    assert_eq!(novo["class_slugs"], json!([]));

    // Desativar classe: some do público, continua no admin.
    let (status, _, _) = send(
        &app,
        "PUT",
        "/api/admin/games/perfect-world/classes/seeker",
        Some(&admin),
        Some(json!({ "name": "Retalhador", "role_hint": "tank", "active": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, pw, _) = send(&app, "GET", "/api/games/perfect-world", None, None).await;
    assert_eq!(pw["classes"].as_array().unwrap().len(), 2);
    let (_, pw_admin, _) = send(
        &app,
        "GET",
        "/api/admin/games/perfect-world",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(pw_admin["classes"].as_array().unwrap().len(), 3);

    // Segundo jogo: só cadastro, nenhum código.
    let (status, _, _) = send(
        &app,
        "POST",
        "/api/admin/games",
        Some(&admin),
        Some(json!({ "slug": "lineage-2", "name": "Lineage II", "level_min": 1, "level_max": 85 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, list, _) = send(&app, "GET", "/api/games", None, None).await;
    let slugs: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["slug"].as_str().unwrap())
        .collect();
    assert_eq!(slugs, ["lineage-2", "perfect-world"]);

    // Jogo desativado: 404 no público.
    let mut body = perfect_world();
    body["active"] = json!(false);
    let (status, _, _) = send(
        &app,
        "PUT",
        "/api/admin/games/perfect-world",
        Some(&admin),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(&app, "GET", "/api/games/perfect-world", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn invalid_catalog_input_is_rejected_by_field(pool: PgPool) {
    let app = app(pool);
    let admin = login_as(&app, ADMIN).await;
    let (status, _, _) = send(
        &app,
        "POST",
        "/api/admin/games",
        Some(&admin),
        Some(perfect_world()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let mut bad_slug = perfect_world();
    bad_slug["slug"] = json!("Perfect World");
    let mut bad_schema = perfect_world();
    bad_schema["slug"] = json!("pw2");
    bad_schema["character_schema"] =
        json!({ "type": "object", "properties": { "x": { "type": "array" } } });
    let mut bad_ranks = perfect_world();
    bad_ranks["slug"] = json!("pw3");
    bad_ranks["default_ranks"] =
        json!([{ "name": "Chefe", "position": 0, "permissions": ["members.view"] }]);

    for (body, field) in [
        (bad_slug, "slug"),
        (bad_schema, "character_schema"),
        (bad_ranks, "default_ranks"),
    ] {
        let (status, res, _) =
            send(&app, "POST", "/api/admin/games", Some(&admin), Some(body)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{res}");
        assert!(res["message"].as_str().unwrap().contains(field), "{res}");
    }

    let (status, res, _) = send(
        &app,
        "POST",
        "/api/admin/games",
        Some(&admin),
        Some(perfect_world()),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{res}");

    let (status, res, _) = send(
        &app,
        "POST",
        "/api/admin/games/perfect-world/classes",
        Some(&admin),
        Some(json!({ "slug": "mago", "name": "Mago", "role_hint": "nuker" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(res["message"].as_str().unwrap().contains("role_hint"));

    let (status, res, _) = send(
        &app,
        "POST",
        "/api/admin/games/perfect-world/servers",
        Some(&admin),
        Some(json!({ "slug": "s1", "name": "S1", "class_slugs": ["nao-existe"] })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(res["message"].as_str().unwrap().contains("class_slugs"));

    let (status, _, _) = send(
        &app,
        "POST",
        "/api/admin/games/nao-existe/classes",
        Some(&admin),
        Some(json!({ "slug": "x", "name": "X", "role_hint": "dps" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
