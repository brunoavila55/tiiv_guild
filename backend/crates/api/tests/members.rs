#![allow(clippy::expect_used)]

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

/// Id do rank `name` na guilda do host.
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

/// Cadastra um membro pela API e devolve a ficha.
async fn add(app: &TestApp, host: &str, cookie: &str, body: Value) -> Value {
    let res = app.post(host, "/api/members", Some(cookie), body).await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.body);
    res.body
}

fn id(v: &Value) -> &str {
    v["id"].as_str().expect("id")
}

fn nicknames(page: &Value) -> Vec<&str> {
    page["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["nickname"].as_str())
        .collect()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn owner_manages_members_with_history(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.register("conta@example.com").await;
    app.create_guild(&owner, "acme").await;
    let host = guild_host("acme");
    let membro = rank_id(&app, &host, &owner, "Membro").await;
    let oficial = rank_id(&app, &host, &owner, "Oficial").await;

    // Sem conta.
    let offline = add(
        &app,
        &host,
        &owner,
        json!({ "nickname": "  Offline  ", "rank_id": membro, "notes": "veio do fórum" }),
    )
    .await;
    assert_eq!(offline["nickname"], "Offline");
    assert_eq!(offline["has_account"], false);
    assert_eq!(offline["status"], "active");
    assert_eq!(offline["notes"], "veio do fórum");
    assert_eq!(offline["can_edit"], true);

    // Com conta existente, pelo e-mail (sem diferenciar maiúsculas); nunca devolve o e-mail.
    let linked = add(
        &app,
        &host,
        &owner,
        json!({ "nickname": "Conta", "rank_id": membro, "email": "CONTA@example.com", "status": "trial" }),
    )
    .await;
    assert_eq!(linked["has_account"], true);
    assert_eq!(linked["account_name"], "Jogador");
    assert!(!linked.to_string().contains("conta@example.com"));

    // Editar rank, status e notas gera um único registro com o diff.
    let path = format!("/api/members/{}", id(&offline));
    let res = app
        .patch(
            &host,
            &path,
            Some(&owner),
            json!({ "rank_id": oficial, "status": "inactive", "notes": "" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    assert_eq!(res.body["rank_name"], "Oficial");
    assert_eq!(res.body["status"], "inactive");
    assert_eq!(res.body["notes"], Value::Null);

    // Remoção é soft: sai da lista padrão, aparece no filtro `left`, ficha continua.
    let res = app.delete(&host, &path, Some(&owner)).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let detail = app.get(&host, &path, Some(&owner)).await;
    assert_eq!(detail.body["status"], "left");
    assert!(detail.body["left_at"].is_string());
    assert_eq!(detail.body["can_remove"], false);

    let list = app.get(&host, "/api/members", Some(&owner)).await;
    assert_eq!(list.body["total"], 2);
    assert!(!nicknames(&list.body).contains(&"Offline"));
    let left = app
        .get(&host, "/api/members?status=left", Some(&owner))
        .await;
    assert_eq!(nicknames(&left.body), vec!["Offline"]);

    // Volta a ficar ativo: `left_at` some.
    let res = app
        .patch(&host, &path, Some(&owner), json!({ "status": "active" }))
        .await;
    assert_eq!(res.body["left_at"], Value::Null);

    let history = app
        .get(&host, &format!("{path}/history"), Some(&owner))
        .await;
    let actions: Vec<_> = history
        .body
        .as_array()
        .into_iter()
        .flatten()
        .map(|h| h["action"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        actions,
        vec![
            "member.updated",
            "member.removed",
            "member.updated",
            "member.added"
        ]
    );
    let update = &history.body[2];
    assert_eq!(update["actor_nickname"], "Jogador");
    assert_eq!(
        update["payload"]["rank"],
        json!({ "from": "Membro", "to": "Oficial" })
    );
    assert_eq!(update["payload"]["notes"], true);
    assert!(!history.body.to_string().contains("veio do fórum"));

    // PATCH sem mudança não gera histórico.
    app.patch(&host, &path, Some(&owner), json!({ "status": "active" }))
        .await;
    let again = app
        .get(&host, &format!("{path}/history"), Some(&owner))
        .await;
    assert_eq!(again.body.as_array().map(Vec::len), Some(4));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn filters_search_and_paginates(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.create_guild(&owner, "acme").await;
    let host = guild_host("acme");
    let membro = rank_id(&app, &host, &owner, "Membro").await;
    let recruta = rank_id(&app, &host, &owner, "Recruta").await;
    for (nick, rank, status) in [
        ("alfa", &membro, "active"),
        ("bravo", &membro, "trial"),
        ("charlie", &recruta, "active"),
        ("100%_real", &recruta, "inactive"),
    ] {
        add(
            &app,
            &host,
            &owner,
            json!({ "nickname": nick, "rank_id": rank, "status": status }),
        )
        .await;
    }

    let get = |q: String| {
        let (app, host, owner) = (&app, &host, &owner);
        async move {
            app.get(host, &format!("/api/members{q}"), Some(owner))
                .await
        }
    };

    let page1 = get("?per_page=2".into()).await;
    assert_eq!(page1.body["total"], 5);
    assert_eq!(nicknames(&page1.body), vec!["Jogador", "alfa"]);
    let page3 = get("?per_page=2&page=3".into()).await;
    assert_eq!(nicknames(&page3.body), vec!["charlie"]);

    let by_rank = get(format!("?rank_id={recruta}")).await;
    assert_eq!(nicknames(&by_rank.body), vec!["100%_real", "charlie"]);
    let by_status = get("?status=trial".into()).await;
    assert_eq!(nicknames(&by_status.body), vec!["bravo"]);
    let search = get("?q=AR".into()).await;
    assert_eq!(nicknames(&search.body), vec!["charlie"]);
    // `%` e `_` são literais na busca.
    let literal = get("?q=%25_".into()).await;
    assert_eq!(nicknames(&literal.body), vec!["100%_real"]);
    let none = get("?q=_".into()).await;
    assert_eq!(nicknames(&none.body), vec!["100%_real"]);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn validates_new_members(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.create_guild(&owner, "acme").await;
    let host = guild_host("acme");
    let membro = rank_id(&app, &host, &owner, "Membro").await;

    let res = app
        .post(
            &host,
            "/api/members",
            Some(&owner),
            json!({ "nickname": "x", "rank_id": membro, "status": "banned", "email": "nao-e-email", "notes": "n".repeat(2001) }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    for f in ["nickname", "status", "email", "notes"] {
        assert!(res.body["fields"][f].is_string(), "{f}: {}", res.body);
    }

    let res = app
        .post(
            &host,
            "/api/members",
            Some(&owner),
            json!({ "nickname": "Fantasma", "rank_id": membro, "email": "ninguem@example.com" }),
        )
        .await;
    assert!(res.body["fields"]["email"].is_string(), "{}", res.body);

    // A conta do dono já é membro.
    let res = app
        .post(
            &host,
            "/api/members",
            Some(&owner),
            json!({ "nickname": "De novo", "rank_id": membro, "email": "dono@example.com" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(res.body["fields"]["email"].is_string(), "{}", res.body);

    let res = app
        .post(
            &host,
            "/api/members",
            Some(&owner),
            json!({ "nickname": "Sem rank", "rank_id": uuid::Uuid::now_v7() }),
        )
        .await;
    assert!(res.body["fields"]["rank_id"].is_string(), "{}", res.body);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn permissions_and_hierarchy(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    let officer = app.register("oficial@example.com").await;
    let member = app.register("membro@example.com").await;
    app.create_guild(&owner, "acme").await;
    app.add_member("acme", "oficial@example.com", "Oficial")
        .await;
    app.add_member("acme", "membro@example.com", "Membro").await;
    let host = guild_host("acme");
    let lider = rank_id(&app, &host, &owner, "Líder").await;
    let oficial = rank_id(&app, &host, &owner, "Oficial").await;
    let membro = rank_id(&app, &host, &owner, "Membro").await;

    let list = app.get(&host, "/api/members", Some(&owner)).await;
    let find = |nick: &str| {
        list.body["items"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|m| m["nickname"] == nick)
            .and_then(|m| m["id"].as_str())
            .map(|id| format!("/api/members/{id}"))
            .expect("membro")
    };
    let owner_path = find("Jogador");
    let all = app.get(&host, "/api/members", Some(&owner)).await;
    let officer_path = all.body["items"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|m| m["rank_name"] == "Oficial")
        .and_then(|m| m["id"].as_str())
        .map(|id| format!("/api/members/{id}"))
        .expect("oficial");

    // Membro comum só vê; não cadastra, não edita, não vê notas.
    let res = app
        .post(
            &host,
            "/api/members",
            Some(&member),
            json!({ "nickname": "Novo", "rank_id": membro }),
        )
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    let target = add(
        &app,
        &host,
        &owner,
        json!({ "nickname": "Alvo", "rank_id": membro, "notes": "secreto" }),
    )
    .await;
    let target_path = format!("/api/members/{}", id(&target));
    let seen = app.get(&host, &target_path, Some(&member)).await;
    assert_eq!(seen.status, StatusCode::OK);
    assert_eq!(seen.body["notes"], Value::Null);
    assert_eq!(seen.body["can_edit"], false);
    assert_eq!(
        app.delete(&host, &target_path, Some(&member)).await.status,
        StatusCode::FORBIDDEN
    );

    // Oficial gerencia quem está abaixo, mas não atribui o próprio rank nem acima.
    let res = app
        .patch(
            &host,
            &target_path,
            Some(&officer),
            json!({ "nickname": "Alvo 2" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    for rank in [&oficial, &lider] {
        let res = app
            .patch(
                &host,
                &target_path,
                Some(&officer),
                json!({ "rank_id": rank }),
            )
            .await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);
    }
    let res = app
        .post(
            &host,
            "/api/members",
            Some(&officer),
            json!({ "nickname": "Novo oficial", "rank_id": oficial }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    // Nem a si mesmo, nem ao dono.
    for path in [&officer_path, &owner_path] {
        let res = app
            .patch(&host, path, Some(&officer), json!({ "nickname": "Hack" }))
            .await;
        assert_eq!(res.status, StatusCode::FORBIDDEN, "{path}");
        assert_eq!(
            app.delete(&host, path, Some(&officer)).await.status,
            StatusCode::FORBIDDEN
        );
    }

    // Banir exige `members.kick` (o Oficial tem).
    let res = app
        .patch(
            &host,
            &target_path,
            Some(&officer),
            json!({ "status": "banned" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);

    // O dono nunca perde o acesso, nem por ele mesmo.
    assert_eq!(
        app.delete(&host, &owner_path, Some(&owner)).await.status,
        StatusCode::CONFLICT
    );
    let res = app
        .patch(
            &host,
            &owner_path,
            Some(&owner),
            json!({ "status": "inactive" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CONFLICT);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn members_are_isolated_between_guilds(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let ana = app.register("ana@example.com").await;
    let bia = app.register("bia@example.com").await;
    app.create_guild(&ana, "guilda-a").await;
    app.create_guild(&bia, "guilda-b").await;
    let host_a = guild_host("guilda-a");
    let host_b = guild_host("guilda-b");
    let rank_b = rank_id(&app, &host_b, &bia, "Membro").await;
    let member_b = add(
        &app,
        &host_b,
        &bia,
        json!({ "nickname": "Só de B", "rank_id": rank_b }),
    )
    .await;
    let path_b = format!("/api/members/{}", id(&member_b));

    // Dona de A, no host de A, não alcança o membro de B por id.
    assert_eq!(
        app.get(&host_a, &path_b, Some(&ana)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.patch(
            &host_a,
            &path_b,
            Some(&ana),
            json!({ "nickname": "Invadido" })
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.delete(&host_a, &path_b, Some(&ana)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.get(&host_a, &format!("{path_b}/history"), Some(&ana))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    // Nem cria membro em A com rank de B.
    let res = app
        .post(
            &host_a,
            "/api/members",
            Some(&ana),
            json!({ "nickname": "Cruzado", "rank_id": rank_b }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    // E no host de B ela não é membro.
    assert_eq!(
        app.get(&host_b, &path_b, Some(&ana)).await.status,
        StatusCode::FORBIDDEN
    );

    let b = app.get(&host_b, &path_b, Some(&bia)).await;
    assert_eq!(b.body["nickname"], "Só de B");
    let a = app.get(&host_a, "/api/members", Some(&ana)).await;
    assert_eq!(a.body["total"], 1);
}
