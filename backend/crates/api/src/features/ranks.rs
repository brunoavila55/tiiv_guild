//! Ranks da guilda: listagem, criação, edição (nome e permissões), ordem e remoção.
//!
//! Hierarquia: o dono mexe em todos; quem tem `ranks.manage` só nos ranks abaixo do seu
//! e só concede/retira permissões que tem. O dono tem todas as permissões em qualquer
//! rank, então nunca perde a gestão.

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{get, patch, put},
    Json, Router,
};
use domain::{
    error::Validator,
    permissions::Permission,
    ranks::{
        apply_permissions, check_manage, check_reorder, normalize_name, parse_permissions,
        MAX_RANKS,
    },
    Error,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use super::members::actor;
use crate::{error::ApiResult, state::AppState, tenant::TenantCtx};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ranks", get(list_ranks).post(create_rank))
        .route("/ranks/order", put(reorder_ranks))
        .route("/ranks/{id}", patch(update_rank).delete(delete_rank))
}

#[derive(Serialize)]
struct RankDto {
    id: Uuid,
    name: String,
    position: i32,
    permissions: Vec<String>,
    /// Membros de qualquer status (inclusive quem saiu) neste rank.
    member_count: i64,
    /// O usuário logado pode editar, mover e apagar este rank.
    can_manage: bool,
}

#[derive(Deserialize)]
struct CreateReq {
    name: String,
    #[serde(default)]
    permissions: Vec<String>,
}

/// Campos ausentes ficam como estão.
#[derive(Deserialize)]
struct UpdateReq {
    name: Option<String>,
    permissions: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct OrderReq {
    /// Todos os ranks, do topo para baixo.
    ids: Vec<Uuid>,
}

#[derive(Deserialize)]
struct DeleteQuery {
    /// Rank que recebe os membros do apagado; obrigatório se ele tiver membros.
    reassign_to: Option<Uuid>,
}

const NAME_TAKEN: &str = "já existe um rank com esse nome";

async fn summaries(ctx: &TenantCtx, conn: &mut PgConnection) -> ApiResult<Vec<RankDto>> {
    let ranks = db::ranks::summaries(conn).await?;
    let actor = ctx.actor(ranks.iter().map(|r| (r.id, r.position)));
    let manage = ctx.require(Permission::RanksManage).is_ok();
    Ok(ranks
        .into_iter()
        .map(|r| RankDto {
            can_manage: manage && actor.outranks(r.position),
            id: r.id,
            name: r.name,
            position: r.position,
            permissions: r.permissions,
            member_count: r.member_count,
        })
        .collect())
}

async fn list_ranks(ctx: TenantCtx) -> ApiResult<Json<Vec<RankDto>>> {
    ctx.require(Permission::MembersView)?;
    let mut tx = ctx.tx().await?;
    let ranks = summaries(&ctx, &mut tx).await?;
    tx.commit().await?;
    Ok(Json(ranks))
}

fn one(ranks: Vec<RankDto>, id: Uuid) -> ApiResult<RankDto> {
    Ok(ranks
        .into_iter()
        .find(|r| r.id == id)
        .ok_or(Error::NotFound)?)
}

/// Rank novo entra no fim da hierarquia (abaixo de todos).
async fn create_rank(
    ctx: TenantCtx,
    Json(req): Json<CreateReq>,
) -> ApiResult<(StatusCode, Json<RankDto>)> {
    ctx.require(Permission::RanksManage)?;
    let mut v = Validator::default();
    let name = v.check(normalize_name(&req.name));
    let requested = v.check(parse_permissions(&req.permissions));
    v.finish()?;
    let permissions = apply_permissions(&ctx.permissions, &[], &requested)?;

    let mut tx = ctx.tx().await?;
    let ranks = db::ranks::list_for_update(&mut tx).await?;
    if ranks.len() >= MAX_RANKS {
        return Err(Error::Conflict("limite de ranks da guilda atingido").into());
    }
    let position = ranks.last().map_or(0, |r| r.position + 1);
    let id = match db::ranks::insert(&mut tx, ctx.guild.id, &name, position, &permissions).await {
        Ok(id) => id,
        Err(e) if db::is_unique_violation(&e, db::ranks::NAME_KEY) => {
            return Err(Error::field("name", NAME_TAKEN).into())
        }
        Err(e) => return Err(e.into()),
    };
    let dto = one(summaries(&ctx, &mut tx).await?, id)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(dto)))
}

async fn update_rank(
    ctx: TenantCtx,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateReq>,
) -> ApiResult<Json<RankDto>> {
    ctx.require(Permission::RanksManage)?;
    let mut v = Validator::default();
    let name = req.name.as_deref().map(|n| v.check(normalize_name(n)));
    let requested = req
        .permissions
        .as_deref()
        .map(|p| v.check(parse_permissions(p)));
    v.finish()?;

    let mut tx = ctx.tx().await?;
    let ranks = db::ranks::list_for_update(&mut tx).await?;
    let rank = ranks.iter().find(|r| r.id == id).ok_or(Error::NotFound)?;
    check_manage(actor(&ctx, &ranks), rank.position)?;

    let name = name.unwrap_or_else(|| rank.name.clone());
    let permissions = match requested {
        Some(p) => apply_permissions(&ctx.permissions, &rank.permissions, &p)?,
        None => rank.permissions.clone(),
    };
    if name != rank.name || permissions != rank.permissions {
        match db::ranks::update(&mut tx, id, &name, &permissions).await {
            Ok(()) => {}
            Err(e) if db::is_unique_violation(&e, db::ranks::NAME_KEY) => {
                return Err(Error::field("name", NAME_TAKEN).into())
            }
            Err(e) => return Err(e.into()),
        }
    }
    let dto = one(summaries(&ctx, &mut tx).await?, id)?;
    tx.commit().await?;
    Ok(Json(dto))
}

async fn reorder_ranks(ctx: TenantCtx, Json(req): Json<OrderReq>) -> ApiResult<Json<Vec<RankDto>>> {
    ctx.require(Permission::RanksManage)?;
    let mut tx = ctx.tx().await?;
    let ranks = db::ranks::list_for_update(&mut tx).await?;
    let current: Vec<_> = ranks.iter().map(|r| (r.id, r.position)).collect();
    check_reorder(actor(&ctx, &ranks), &current, &req.ids)?;
    db::ranks::set_order(&mut tx, &req.ids).await?;
    let dtos = summaries(&ctx, &mut tx).await?;
    tx.commit().await?;
    Ok(Json(dtos))
}

/// Rank com membros só sai realocando todos para `reassign_to` (cada um ganha um
/// registro no histórico). As posições são renumeradas sem buracos.
async fn delete_rank(
    ctx: TenantCtx,
    Path(id): Path<Uuid>,
    Query(q): Query<DeleteQuery>,
) -> ApiResult<StatusCode> {
    ctx.require(Permission::RanksManage)?;
    let mut tx = ctx.tx().await?;
    let ranks = db::ranks::list_for_update(&mut tx).await?;
    let actor = actor(&ctx, &ranks);
    let rank = ranks.iter().find(|r| r.id == id).ok_or(Error::NotFound)?;
    check_manage(actor, rank.position)?;
    if ranks.len() == 1 {
        return Err(Error::Conflict("a guilda precisa de pelo menos um rank").into());
    }

    let in_use = db::ranks::summaries(&mut tx)
        .await?
        .iter()
        .any(|r| r.id == id && r.member_count > 0);
    if in_use {
        let target_id = q.reassign_to.ok_or(Error::Conflict(
            "o rank tem membros: escolha para qual rank realocá-los",
        ))?;
        if target_id == id {
            return Err(Error::field("reassign_to", "escolha outro rank").into());
        }
        let target = ranks
            .iter()
            .find(|r| r.id == target_id)
            .ok_or_else(|| Error::field("reassign_to", "rank inválido"))?;
        actor.check_assign(target.position).map_err(|_| {
            Error::field("reassign_to", "você só pode atribuir ranks abaixo do seu")
        })?;
        let moved = db::ranks::move_members(&mut tx, id, target.id).await?;
        db::members::add_history_many(
            &mut tx,
            ctx.guild.id,
            &moved,
            ctx.membership.member_id,
            "member.updated",
            &json!({ "rank": { "from": rank.name, "to": target.name } }),
        )
        .await?;
    }

    db::ranks::delete(&mut tx, id).await?;
    let order: Vec<Uuid> = ranks.iter().map(|r| r.id).filter(|r| *r != id).collect();
    db::ranks::set_order(&mut tx, &order).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
