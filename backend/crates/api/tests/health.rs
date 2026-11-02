#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn app() -> axum::Router {
    // Pool lazy: /api/health não toca no banco.
    let pool = db::connect_lazy("postgres://localhost/unused").expect("url válida");
    api::router(api::AppState::new(
        pool,
        games::GameRegistry::builtin().unwrap(),
    ))
}

#[tokio::test]
async fn health_returns_ok() {
    let res = app()
        .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn unknown_route_is_404() {
    let res = app()
        .oneshot(Request::get("/api/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
