//! Catálogo de jogos (P0.4): Perfect World e um game pack fake registrado só no teste,
//! sem nenhuma mudança no core.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use db::PgPool;
use games::{ClassSeed, GamePack, GameRegistry, RankSeed, RoleHint, ServerSeed};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

struct FakePack {
    schema: Value,
    classes: Vec<ClassSeed>,
    servers: Vec<ServerSeed>,
}

impl FakePack {
    fn new() -> Self {
        Self {
            schema: json!({
                "type": "object",
                "additionalProperties": false,
                "properties": { "facção": { "type": "string", "enum": ["luz", "trevas"] } },
                "required": ["facção"]
            }),
            classes: vec![
                ClassSeed {
                    slug: "paladino".into(),
                    name: "Paladino".into(),
                    role_hint: RoleHint::Tank,
                    metadata: Value::Null,
                },
                ClassSeed {
                    slug: "bardo".into(),
                    name: "Bardo".into(),
                    role_hint: RoleHint::Support,
                    metadata: json!({ "instrumento": "alaúde" }),
                },
            ],
            servers: vec![ServerSeed {
                slug: "aurora".into(),
                name: "Aurora".into(),
                region: Some("BR".into()),
            }],
        }
    }
}

impl GamePack for FakePack {
    fn slug(&self) -> &str {
        "fake-game"
    }
    fn name(&self) -> &str {
        "Fake Game"
    }
    fn character_schema(&self) -> &Value {
        &self.schema
    }
    fn default_ranks(&self) -> Vec<RankSeed> {
        vec![RankSeed {
            name: "Mestre".into(),
            position: 0,
            permissions: domain::permissions::ALL
                .iter()
                .map(|p| p.to_string())
                .collect(),
        }]
    }
    fn level_range(&self) -> (i32, i32) {
        (1, 60)
    }
    fn classes(&self) -> &[ClassSeed] {
        &self.classes
    }
    fn servers(&self) -> &[ServerSeed] {
        &self.servers
    }
}

async fn setup(pool: &PgPool, registry: GameRegistry) -> Router {
    let mut conn = pool.acquire().await.unwrap();
    api::features::games::sync_catalog(&mut conn, &registry)
        .await
        .unwrap();
    api::router(api::AppState::new(pool.clone(), registry))
}

async fn get(app: &Router, uri: &str) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn perfect_world_returns_classes_and_character_schema(pool: PgPool) {
    let app = setup(&pool, GameRegistry::builtin().unwrap()).await;

    let (status, body) = get(&app, "/api/games/perfect-world").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Perfect World");
    assert_eq!(body["level_range"], json!([1, 105]));

    let classes = body["classes"].as_array().unwrap();
    assert_eq!(classes.len(), 12);
    let cleric = classes.iter().find(|c| c["slug"] == "cleric").unwrap();
    assert_eq!(cleric["role_hint"], "healer");
    assert!(cleric["id"].is_string());

    assert_eq!(body["character_schema"]["type"], "object");
    assert!(body["character_schema"]["properties"]["cultivo"].is_object());
    assert_eq!(body["default_ranks"][0]["name"], "Líder");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn fake_pack_registers_without_core_changes(pool: PgPool) {
    let mut registry = GameRegistry::builtin().unwrap();
    registry.register(FakePack::new()).unwrap();
    let app = setup(&pool, registry).await;

    let (status, body) = get(&app, "/api/games/fake-game").await;
    assert_eq!(status, StatusCode::OK);
    let slugs: Vec<&str> = body["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["slug"].as_str().unwrap())
        .collect();
    assert_eq!(slugs, ["bardo", "paladino"]);
    assert_eq!(body["servers"][0]["region"], "BR");
    assert_eq!(body["character_schema"]["required"], json!(["facção"]));

    let (_, list) = get(&app, "/api/games").await;
    let names: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["slug"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["fake-game", "perfect-world"]);

    // A validação de atributos vem do pack, não do core.
    let registry_pack = FakePack::new();
    assert!(registry_pack
        .validate_character(&json!({ "facção": "luz" }))
        .is_ok());
    assert!(registry_pack
        .validate_character(&json!({ "facção": "cinza" }))
        .is_err());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn invalid_pack_is_rejected_at_registration(pool: PgPool) {
    let _ = pool;
    let mut pack = FakePack::new();
    pack.schema = json!({ "type": "object", "properties": { "x": { "type": "object" } } });

    let mut registry = GameRegistry::new();
    assert!(registry.register(pack).is_err());
    assert!(registry.register(FakePack::new()).is_ok());
    assert!(
        registry.register(FakePack::new()).is_err(),
        "slug duplicado"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sync_is_idempotent_and_keeps_ids(pool: PgPool) {
    let registry = GameRegistry::builtin().unwrap();
    let app = setup(&pool, registry.clone()).await;
    let (_, first) = get(&app, "/api/games/perfect-world").await;

    let app = setup(&pool, registry).await;
    let (_, second) = get(&app, "/api/games/perfect-world").await;

    assert_eq!(first["id"], second["id"]);
    assert_eq!(first["classes"], second["classes"]);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unknown_game_is_404(pool: PgPool) {
    let app = setup(&pool, GameRegistry::builtin().unwrap()).await;
    let (status, body) = get(&app, "/api/games/nao-existe").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");
}
