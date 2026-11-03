#![allow(clippy::expect_used)]

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

async fn ranks(app: &TestApp, host: &str, cookie: &str) -> Vec<Value> {
    let res = app.get(host, "/api/ranks", Some(cookie)).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    res.body.as_array().cloned().unwrap_or_default()
}

fn names(ranks: &[Value]) -> Vec<&str> {
    ranks.iter().filter_map(|r| r["name"].as_str()).collect()
}

fn rank<'a>(ranks: &'a [Value], name: &str) -> &'a Value {
    ranks.iter().find(|r| r["name"] == name).expect("rank")
}

fn id<'a>(ranks: &'a [Value], name: &str) -> &'a str {
    rank(ranks, name)["id"].as_str().expect("id")
}

fn ids(ranks: &[Value], order: &[&str]) -> Value {
    json!(order.iter().map(|n| id(ranks, n)).collect::<Vec<_>>())
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn owner_manages_ranks(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.create_guild(&owner, "acme").await;
    let host = guild_host("acme");

    let list = ranks(&app, &host, &owner).await;
    assert_eq!(names(&list), ["Líder", "Oficial", "Membro", "Recruta"]);
    assert_eq!(rank(&list, "Líder")["member_count"], 1);
    assert!(list.iter().all(|r| r["can_manage"] == true));

    // Criar: entra no fim; permissões em ordem canônica, sem repetição.
    let res = app
        .post(
            &host,
            "/api/ranks",
            Some(&owner),
            json!({ "name": " Veterano ", "permissions": ["members.manage", "members.view", "members.view"] }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.body);
    assert_eq!(res.body["name"], "Veterano");
    assert_eq!(res.body["position"], 4);
    assert_eq!(
        res.body["permissions"],
        json!(["members.view", "members.manage"])
    );
    let veterano = res.body["id"].as_str().expect("id").to_owned();

    // Nome repetido (sem diferenciar maiúsculas), vazio ou permissão desconhecida → 422.
    for body in [
        json!({ "name": "membro" }),
        json!({ "name": "  " }),
        json!({ "name": "X", "permissions": ["games.manage"] }),
    ] {
        let res = app.post(&host, "/api/ranks", Some(&owner), body).await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);
    }
    let res = app
        .patch(
            &host,
            &format!("/api/ranks/{veterano}"),
            Some(&owner),
            json!({ "name": "OFICIAL" }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);
    assert!(res.body["fields"]["name"].is_string());

    // Editar nome e permissões.
    let res = app
        .patch(
            &host,
            &format!("/api/ranks/{veterano}"),
            Some(&owner),
            json!({ "name": "Veterana", "permissions": ["members.view"] }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    assert_eq!(res.body["name"], "Veterana");
    assert_eq!(res.body["permissions"], json!(["members.view"]));

    // Reordenar: lista completa; posições 0..n.
    let list = ranks(&app, &host, &owner).await;
    let order = ["Líder", "Oficial", "Veterana", "Membro", "Recruta"];
    let res = app
        .put(
            &host,
            "/api/ranks/order",
            Some(&owner),
            json!({ "ids": ids(&list, &order) }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    let list = res.body.as_array().cloned().unwrap_or_default();
    assert_eq!(names(&list), order);
    let positions: Vec<_> = list.iter().map(|r| r["position"].clone()).collect();
    assert_eq!(positions, [0, 1, 2, 3, 4]);

    let res = app
        .put(
            &host,
            "/api/ranks/order",
            Some(&owner),
            json!({ "ids": ids(&list, &["Líder", "Oficial"]) }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rank_in_use_needs_reassignment(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.create_guild(&owner, "acme").await;
    let host = guild_host("acme");
    let list = ranks(&app, &host, &owner).await;
    let membro = id(&list, "Membro").to_owned();
    let recruta = id(&list, "Recruta").to_owned();

    let res = app
        .post(
            &host,
            "/api/members",
            Some(&owner),
            json!({ "nickname": "Fulano", "rank_id": membro }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.body);
    let member = res.body["id"].as_str().expect("id").to_owned();
    assert_eq!(
        rank(&ranks(&app, &host, &owner).await, "Membro")["member_count"],
        1
    );

    // Sem destino → 409; destino igual ao apagado ou inexistente → 422.
    let path = format!("/api/ranks/{membro}");
    let res = app.delete(&host, &path, Some(&owner)).await;
    assert_eq!(res.status, StatusCode::CONFLICT, "{}", res.body);
    for target in [membro.as_str(), "019a0000-0000-7000-8000-000000000000"] {
        let res = app
            .delete(&host, &format!("{path}?reassign_to={target}"), Some(&owner))
            .await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);
    }

    // Realocando: membro muda de rank, com histórico; posições sem buracos.
    let res = app
        .delete(
            &host,
            &format!("{path}?reassign_to={recruta}"),
            Some(&owner),
        )
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT, "{}", res.body);
    let detail = app
        .get(&host, &format!("/api/members/{member}"), Some(&owner))
        .await;
    assert_eq!(detail.body["rank_name"], "Recruta");
    let history = app
        .get(
            &host,
            &format!("/api/members/{member}/history"),
            Some(&owner),
        )
        .await;
    assert_eq!(history.body[0]["action"], "member.updated");
    assert_eq!(
        history.body[0]["payload"],
        json!({ "rank": { "from": "Membro", "to": "Recruta" } })
    );
    let list = ranks(&app, &host, &owner).await;
    assert_eq!(names(&list), ["Líder", "Oficial", "Recruta"]);
    assert_eq!(rank(&list, "Recruta")["position"], 2);

    // Rank vazio sai direto.
    let res = app
        .delete(
            &host,
            &format!("/api/ranks/{}", id(&list, "Oficial")),
            Some(&owner),
        )
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT, "{}", res.body);
    let res = app
        .delete(
            &host,
            &format!("/api/ranks/{}", id(&list, "Oficial")),
            Some(&owner),
        )
        .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn officers_only_manage_below_without_escalation(
    opts: PgPoolOptions,
    conn: PgConnectOptions,
) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    let officer = app.register("oficial@example.com").await;
    let member = app.register("membro@example.com").await;
    app.create_guild(&owner, "acme").await;
    app.add_member("acme", "oficial@example.com", "Oficial")
        .await;
    app.add_member("acme", "membro@example.com", "Membro").await;
    let host = guild_host("acme");
    let list = ranks(&app, &host, &owner).await;

    // Quem não tem `ranks.manage` só lê.
    let res = app
        .post(&host, "/api/ranks", Some(&member), json!({ "name": "X" }))
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert!(ranks(&app, &host, &member)
        .await
        .iter()
        .all(|r| r["can_manage"] == false));

    let res = app
        .patch(
            &host,
            &format!("/api/ranks/{}", id(&list, "Oficial")),
            Some(&owner),
            json!({ "permissions": ["members.view", "members.manage", "ranks.manage"] }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);

    let seen = ranks(&app, &host, &officer).await;
    let manageable: Vec<_> = seen
        .iter()
        .filter(|r| r["can_manage"] == true)
        .filter_map(|r| r["name"].as_str())
        .collect();
    assert_eq!(manageable, ["Membro", "Recruta"]);

    // O próprio rank e os de cima: 403.
    for name in ["Líder", "Oficial"] {
        let res = app
            .patch(
                &host,
                &format!("/api/ranks/{}", id(&list, name)),
                Some(&officer),
                json!({ "permissions": [] }),
            )
            .await;
        assert_eq!(res.status, StatusCode::FORBIDDEN, "{name}");
        let res = app
            .delete(
                &host,
                &format!("/api/ranks/{}", id(&list, name)),
                Some(&officer),
            )
            .await;
        assert_eq!(res.status, StatusCode::FORBIDDEN, "{name}");
    }

    // Só concede o que tem.
    let res = app
        .post(
            &host,
            "/api/ranks",
            Some(&officer),
            json!({ "name": "Escudeiro", "permissions": ["guild.settings.edit"] }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);
    assert!(res.body["fields"]["permissions"].is_string());
    let res = app
        .post(
            &host,
            "/api/ranks",
            Some(&officer),
            json!({ "name": "Escudeiro", "permissions": ["members.view"] }),
        )
        .await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.body);

    // Reordena só abaixo do próprio rank.
    let list = ranks(&app, &host, &owner).await;
    let res = app
        .put(
            &host,
            "/api/ranks/order",
            Some(&officer),
            json!({ "ids": ids(&list, &["Líder", "Oficial", "Escudeiro", "Membro", "Recruta"]) }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    let res = app
        .put(
            &host,
            "/api/ranks/order",
            Some(&officer),
            json!({ "ids": ids(&list, &["Oficial", "Líder", "Escudeiro", "Membro", "Recruta"]) }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);

    // Não realoca membros para o próprio rank.
    let res = app
        .delete(
            &host,
            &format!(
                "/api/ranks/{}?reassign_to={}",
                id(&list, "Membro"),
                id(&list, "Oficial")
            ),
            Some(&officer),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);
    assert!(res.body["fields"]["reassign_to"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn owner_never_loses_management(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let owner = app.register("dono@example.com").await;
    app.create_guild(&owner, "acme").await;
    let host = guild_host("acme");
    let list = ranks(&app, &host, &owner).await;

    // O rank do dono perde todas as permissões e vai para o fim da hierarquia...
    let res = app
        .patch(
            &host,
            &format!("/api/ranks/{}", id(&list, "Líder")),
            Some(&owner),
            json!({ "permissions": [] }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);
    let res = app
        .put(
            &host,
            "/api/ranks/order",
            Some(&owner),
            json!({ "ids": ids(&list, &["Oficial", "Membro", "Recruta", "Líder"]) }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.body);

    // ...e o dono continua gerindo tudo.
    let guild = app.get(&host, "/api/guild", Some(&owner)).await;
    assert!(guild.body["me"]["permissions"]
        .as_array()
        .expect("permissions")
        .contains(&json!("ranks.manage")));
    assert!(ranks(&app, &host, &owner)
        .await
        .iter()
        .all(|r| r["can_manage"] == true));
    let res = app
        .post(&host, "/api/ranks", Some(&owner), json!({ "name": "Novo" }))
        .await;
    assert_eq!(res.status, StatusCode::CREATED, "{}", res.body);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ranks_are_isolated_between_guilds(opts: PgPoolOptions, conn: PgConnectOptions) {
    let app = app(opts, conn).await;
    let a = app.register("a@example.com").await;
    let b = app.register("b@example.com").await;
    app.create_guild(&a, "guilda-a").await;
    app.create_guild(&b, "guilda-b").await;
    let host_a = guild_host("guilda-a");
    let host_b = guild_host("guilda-b");
    let list_a = ranks(&app, &host_a, &a).await;
    let recruta_a = id(&list_a, "Recruta");

    // Pelo host de B, o rank de A não existe.
    let path = format!("/api/ranks/{recruta_a}");
    let res = app
        .patch(&host_b, &path, Some(&b), json!({ "name": "Invadido" }))
        .await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let res = app.delete(&host_b, &path, Some(&b)).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let list_b = ranks(&app, &host_b, &b).await;
    let mut order: Vec<Value> = list_b.iter().map(|r| r["id"].clone()).collect();
    order[3] = json!(recruta_a);
    let res = app
        .put(
            &host_b,
            "/api/ranks/order",
            Some(&b),
            json!({ "ids": order }),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    // Pelo host de A, B não é membro.
    let res = app
        .patch(&host_a, &path, Some(&b), json!({ "name": "Invadido" }))
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    assert_eq!(
        names(&ranks(&app, &host_a, &a).await),
        ["Líder", "Oficial", "Membro", "Recruta"]
    );
}
