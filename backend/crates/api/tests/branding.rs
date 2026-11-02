mod common;

use axum::http::{header, StatusCode};
use common::*;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn owner_edits_branding(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.create_guild(&owner, "acme").await;
    let host = guild_host("acme");

    let empty = app.get(&host, "/api/branding", Some(&owner)).await;
    assert_eq!(empty.status, StatusCode::OK);
    assert_eq!(empty.body["public_page_enabled"], false);
    assert_eq!(empty.body["socials"], json!({}));

    let res = app
        .put(
            &host,
            "/api/branding",
            Some(&owner),
            json!({
                "primary_color": "#FF8800",
                "secondary_color": "",
                "motto": "  Juntos até o fim  ",
                "description_md": "# Sobre\nGuilda de amigos.",
                "public_page_enabled": true,
                "socials": { "discord": "https://discord.gg/acme", "youtube": "" }
            }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    assert_eq!(res.body["primary_color"], "#ff8800");
    assert_eq!(res.body["secondary_color"], json!(null));
    assert_eq!(res.body["motto"], "Juntos até o fim");
    assert_eq!(
        res.body["socials"],
        json!({ "discord": "https://discord.gg/acme" })
    );

    // As cores valem no layout público do tenant.
    let tenant = app.get(&host, "/api/tenant", None).await;
    assert_eq!(tenant.body["primary_color"], "#ff8800");
    assert_eq!(tenant.body["public_page_enabled"], true);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn validates_branding(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.create_guild(&owner, "acme").await;

    let res = app
        .put(
            &guild_host("acme"),
            "/api/branding",
            Some(&owner),
            json!({
                "primary_color": "laranja",
                "motto": "x".repeat(121),
                "public_page_enabled": false,
                "socials": { "x": "http://x.com/acme", "myspace": "https://myspace.com" }
            }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    for f in ["primary_color", "motto", "socials.x", "socials.myspace"] {
        assert!(res.body["fields"][f].is_string(), "{f}: {}", res.body);
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn requires_branding_permission(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    let member = app.register("membro@example.com").await;
    app.create_guild(&owner, "acme").await;
    app.add_member("acme", "membro@example.com", "Membro").await;
    let host = guild_host("acme");

    // Membro vê o branding, mas não edita.
    assert_eq!(
        app.get(&host, "/api/branding", Some(&member)).await.status,
        StatusCode::OK
    );
    let body = json!({ "public_page_enabled": true });
    assert_eq!(
        app.put(&host, "/api/branding", Some(&member), body)
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.upload(&host, "/api/branding/logo", Some(&member), &png(10))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.delete(&host, "/api/branding/logo", Some(&member))
            .await
            .status,
        StatusCode::FORBIDDEN
    );

    // Dono de outra guilda não toca nesta.
    let other = app.register("outro@example.com").await;
    app.create_guild(&other, "outra").await;
    let body = json!({ "public_page_enabled": true });
    assert_eq!(
        app.put(&host, "/api/branding", Some(&other), body)
            .await
            .status,
        StatusCode::FORBIDDEN
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn uploads_replaces_and_removes_images(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.create_guild(&owner, "acme").await;
    let host = guild_host("acme");

    let first = png(100);
    let res = app
        .upload(&host, "/api/branding/logo", Some(&owner), &first)
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    let logo = res.body["logo_url"].as_str().unwrap_or_default().to_owned();
    assert!(
        logo.starts_with("/api/media/guilds/") && logo.ends_with(".png"),
        "{logo}"
    );
    assert_eq!(
        app.get(&host, "/api/tenant", None).await.body["logo_url"],
        logo.as_str()
    );

    // O arquivo é servido em qualquer host, como imagem e sem sniffing.
    let file = app.get(APP, &logo, None).await;
    assert_eq!(file.status, StatusCode::OK);
    assert_eq!(file.bytes, first);
    assert_eq!(file.header(header::CONTENT_TYPE), "image/png");
    assert_eq!(file.header(header::X_CONTENT_TYPE_OPTIONS), "nosniff");

    // Trocar apaga o arquivo anterior.
    let res = app
        .upload(&host, "/api/branding/logo", Some(&owner), &png(200))
        .await;
    let new_logo = res.body["logo_url"].as_str().unwrap_or_default().to_owned();
    assert_ne!(new_logo, logo);
    assert_eq!(
        app.get(APP, &logo, None).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(app.get(APP, &new_logo, None).await.status, StatusCode::OK);

    // Banner é independente do logo.
    let res = app
        .upload(
            &host,
            "/api/branding/banner",
            Some(&owner),
            b"RIFF\0\0\0\0WEBPVP8 ",
        )
        .await;
    assert!(res.body["banner_url"]
        .as_str()
        .unwrap_or_default()
        .ends_with(".webp"));
    assert_eq!(res.body["logo_url"], new_logo.as_str());

    let res = app.delete(&host, "/api/branding/logo", Some(&owner)).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["logo_url"], json!(null));
    assert_eq!(
        app.get(APP, &new_logo, None).await.status,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rejects_invalid_images(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.create_guild(&owner, "acme").await;
    let host = guild_host("acme");

    for file in [
        b"<svg onload=alert(1)>".to_vec(),
        Vec::new(),
        png(2 * 1024 * 1024),
        png(3 * 1024 * 1024),
    ] {
        let res = app
            .upload(&host, "/api/branding/logo", Some(&owner), &file)
            .await;
        assert_eq!(
            res.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{} bytes: {}",
            file.len(),
            res.body
        );
        assert!(res.body["fields"]["file"].is_string());
    }
    assert_eq!(
        app.upload(&host, "/api/branding/avatar", Some(&owner), &png(10))
            .await
            .status,
        StatusCode::NOT_FOUND
    );

    // Só chaves geradas pelo upload são servidas.
    assert_eq!(
        app.get(APP, "/api/media/../etc/passwd.png", None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.get(APP, "/api/media/guilds/x/logo.svg", None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}
