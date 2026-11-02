//! Gestão de membros: listagem com filtros, ficha, cadastro, edição, remoção (soft) e histórico.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use db::members::{MemberDetail, MemberFilter, MemberWrite, Rank};
use domain::{
    error::Validator,
    members::{
        check_owner_status, clamp_page, like_pattern, normalize_nickname, normalize_notes, Actor,
        MemberStatus,
    },
    permissions::Permission,
    users::{normalize_email, validate_email},
    Error,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::{error::ApiResult, state::AppState, tenant::TenantCtx};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/members", get(list_members).post(create_member))
        .route(
            "/members/{id}",
            get(show_member).patch(update_member).delete(remove_member),
        )
        .route("/members/{id}/history", get(member_history))
        .route("/ranks", get(list_ranks))
}

#[derive(Serialize)]
struct MemberDto {
    id: Uuid,
    nickname: String,
    has_account: bool,
    rank_id: Uuid,
    rank_name: String,
    status: MemberStatus,
    joined_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct MemberPageDto {
    items: Vec<MemberDto>,
    total: i64,
    page: u32,
    per_page: u32,
}

#[derive(Serialize)]
struct MemberDetailDto {
    id: Uuid,
    nickname: String,
    has_account: bool,
    /// Nome de exibição da conta vinculada (nunca o e-mail).
    account_name: Option<String>,
    is_owner: bool,
    rank_id: Uuid,
    rank_name: String,
    status: MemberStatus,
    /// Só para quem tem `members.manage`.
    notes: Option<String>,
    joined_at: DateTime<Utc>,
    left_at: Option<DateTime<Utc>>,
    /// O usuário logado pode editar/remover este membro (permissão + hierarquia).
    can_edit: bool,
    can_remove: bool,
}

#[derive(Serialize)]
struct HistoryDto {
    id: Uuid,
    action: String,
    payload: Value,
    actor_nickname: Option<String>,
    at: DateTime<Utc>,
}

#[derive(Serialize)]
struct RankDto {
    id: Uuid,
    name: String,
    position: i32,
    permissions: Vec<String>,
}

#[derive(Deserialize)]
struct ListQuery {
    status: Option<MemberStatus>,
    rank_id: Option<Uuid>,
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Deserialize)]
struct CreateReq {
    nickname: String,
    rank_id: Uuid,
    #[serde(default)]
    status: Option<MemberStatus>,
    notes: Option<String>,
    /// E-mail de uma conta existente para vincular; vazio = membro sem conta.
    email: Option<String>,
}

/// Campos ausentes ficam como estão; `notes: ""` limpa as notas.
#[derive(Deserialize)]
struct UpdateReq {
    nickname: Option<String>,
    rank_id: Option<Uuid>,
    status: Option<MemberStatus>,
    notes: Option<String>,
}

/// O usuário logado na hierarquia da guilda.
fn actor(ctx: &TenantCtx, ranks: &[Rank]) -> Actor {
    let rank_position = ranks
        .iter()
        .find(|r| r.id == ctx.membership.rank_id)
        .map_or(i32::MAX, |r| r.position);
    Actor {
        is_owner: ctx.is_owner,
        rank_position,
    }
}

fn find_rank(ranks: &[Rank], id: Uuid) -> Result<&Rank, Error> {
    ranks
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| Error::field("rank_id", "rank inválido"))
}

fn is_owner_member(ctx: &TenantCtx, m: &MemberDetail) -> bool {
    m.user_id == Some(ctx.guild.owner_user_id)
}

fn detail_dto(ctx: &TenantCtx, actor: Actor, m: MemberDetail) -> MemberDetailDto {
    let is_owner = is_owner_member(ctx, &m);
    let reachable = actor.check_target(m.rank_position, is_owner).is_ok();
    let manage = ctx.require(Permission::MembersManage).is_ok();
    let kick = ctx.require(Permission::MembersKick).is_ok();
    MemberDetailDto {
        has_account: m.user_id.is_some(),
        notes: if manage { m.notes } else { None },
        can_edit: manage && reachable,
        can_remove: kick && reachable && !is_owner && m.status != MemberStatus::Left,
        id: m.id,
        nickname: m.nickname,
        account_name: m.account_name,
        is_owner,
        rank_id: m.rank_id,
        rank_name: m.rank_name,
        status: m.status,
        joined_at: m.joined_at,
        left_at: m.left_at,
    }
}

async fn list_members(
    ctx: TenantCtx,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<MemberPageDto>> {
    ctx.require(Permission::MembersView)?;
    let (page, per_page) = clamp_page(q.page, q.per_page);
    let like = q.q.as_deref().and_then(like_pattern);
    let filter = MemberFilter {
        status: q.status,
        rank_id: q.rank_id,
        nickname_like: like.as_deref(),
        limit: per_page.into(),
        offset: i64::from(page - 1) * i64::from(per_page),
    };
    let mut tx = ctx.tx().await?;
    let (members, total) = db::members::page(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(Json(MemberPageDto {
        items: members
            .into_iter()
            .map(|m| MemberDto {
                id: m.id,
                nickname: m.nickname,
                has_account: m.user_id.is_some(),
                rank_id: m.rank_id,
                rank_name: m.rank_name,
                status: m.status,
                joined_at: m.joined_at,
            })
            .collect(),
        total,
        page,
        per_page,
    }))
}

async fn load_detail(ctx: &TenantCtx, id: Uuid) -> ApiResult<MemberDetailDto> {
    let mut tx = ctx.tx().await?;
    let ranks = db::members::ranks(&mut tx).await?;
    let member = db::members::find(&mut tx, id)
        .await?
        .ok_or(Error::NotFound)?;
    tx.commit().await?;
    Ok(detail_dto(ctx, actor(ctx, &ranks), member))
}

async fn show_member(ctx: TenantCtx, Path(id): Path<Uuid>) -> ApiResult<Json<MemberDetailDto>> {
    ctx.require(Permission::MembersView)?;
    Ok(Json(load_detail(&ctx, id).await?))
}

async fn create_member(
    State(state): State<AppState>,
    ctx: TenantCtx,
    Json(req): Json<CreateReq>,
) -> ApiResult<(StatusCode, Json<MemberDetailDto>)> {
    ctx.require(Permission::MembersManage)?;

    let mut v = Validator::default();
    let nickname = v.check(normalize_nickname(&req.nickname));
    let notes = v.check(normalize_notes(req.notes.as_deref()));
    let status = req.status.unwrap_or(MemberStatus::Active);
    if status.is_removal() {
        v.check::<()>(Err(Error::field(
            "status",
            "um membro novo deve ser ativo, em teste ou inativo",
        )));
    }
    let email = req
        .email
        .as_deref()
        .map(normalize_email)
        .filter(|e| !e.is_empty());
    if let Some(e) = &email {
        v.check(validate_email(e));
    }
    v.finish()?;

    let user_id = match &email {
        Some(e) => Some(
            db::users::find_id_by_email(&state.pool, e)
                .await?
                .ok_or_else(|| Error::field("email", "nenhuma conta com esse e-mail"))?,
        ),
        None => None,
    };

    let mut tx = ctx.tx().await?;
    let ranks = db::members::ranks(&mut tx).await?;
    let rank = find_rank(&ranks, req.rank_id)?;
    actor(&ctx, &ranks).check_assign(rank.position)?;

    let write = MemberWrite {
        nickname: &nickname,
        rank_id: rank.id,
        status,
        notes: notes.as_deref(),
    };
    let id = match db::members::insert(&mut tx, ctx.guild.id, user_id, &write).await {
        Ok(id) => id,
        Err(e) if db::is_unique_violation(&e, db::members::USER_KEY) => {
            return Err(Error::field("email", "essa conta já é membro desta guilda").into())
        }
        Err(e) => return Err(e.into()),
    };
    let payload = json!({
        "nickname": nickname,
        "rank": rank.name,
        "status": status,
        "has_account": user_id.is_some(),
    });
    db::members::add_history(
        &mut tx,
        ctx.guild.id,
        id,
        ctx.membership.member_id,
        "member.added",
        &payload,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(load_detail(&ctx, id).await?)))
}

async fn update_member(
    ctx: TenantCtx,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateReq>,
) -> ApiResult<Json<MemberDetailDto>> {
    ctx.require(Permission::MembersManage)?;

    let mut v = Validator::default();
    let nickname = req
        .nickname
        .as_deref()
        .map(|n| v.check(normalize_nickname(n)));
    let notes = req
        .notes
        .as_deref()
        .map(|n| v.check(normalize_notes(Some(n))));
    v.finish()?;

    let mut tx = ctx.tx().await?;
    let ranks = db::members::ranks(&mut tx).await?;
    let actor = actor(&ctx, &ranks);
    let current = db::members::find(&mut tx, id)
        .await?
        .ok_or(Error::NotFound)?;
    let target_is_owner = is_owner_member(&ctx, &current);
    actor.check_target(current.rank_position, target_is_owner)?;

    let rank = match req.rank_id {
        Some(rank_id) if rank_id != current.rank_id => {
            let rank = find_rank(&ranks, rank_id)?;
            actor.check_assign(rank.position)?;
            rank
        }
        _ => find_rank(&ranks, current.rank_id)?,
    };
    let status = req.status.unwrap_or(current.status);
    if status != current.status && (status.is_removal() || current.status.is_removal()) {
        ctx.require(Permission::MembersKick)?;
    }
    check_owner_status(target_is_owner, status)?;

    let nickname = nickname.unwrap_or_else(|| current.nickname.clone());
    let notes = notes.unwrap_or_else(|| current.notes.clone());

    let mut changes = Map::new();
    let mut diff = |key: &str, from: Value, to: Value| {
        if from != to {
            changes.insert(key.into(), json!({ "from": from, "to": to }));
        }
    };
    diff("nickname", json!(current.nickname), json!(nickname));
    diff("rank", json!(current.rank_name), json!(rank.name));
    diff("status", json!(current.status), json!(status));
    // O conteúdo das notas não vai para o histórico: quem só tem `members.view` não deve lê-lo.
    if notes != current.notes {
        changes.insert("notes".into(), json!(true));
    }

    if !changes.is_empty() {
        db::members::update(
            &mut tx,
            id,
            &MemberWrite {
                nickname: &nickname,
                rank_id: rank.id,
                status,
                notes: notes.as_deref(),
            },
        )
        .await?;
        db::members::add_history(
            &mut tx,
            ctx.guild.id,
            id,
            ctx.membership.member_id,
            "member.updated",
            &Value::Object(changes),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Json(load_detail(&ctx, id).await?))
}

/// Remoção é soft: status `left` com `left_at`; a ficha e o histórico continuam.
async fn remove_member(ctx: TenantCtx, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    ctx.require(Permission::MembersKick)?;

    let mut tx = ctx.tx().await?;
    let ranks = db::members::ranks(&mut tx).await?;
    let current = db::members::find(&mut tx, id)
        .await?
        .ok_or(Error::NotFound)?;
    let target_is_owner = is_owner_member(&ctx, &current);
    actor(&ctx, &ranks).check_target(current.rank_position, target_is_owner)?;
    check_owner_status(target_is_owner, MemberStatus::Left)?;

    if current.status != MemberStatus::Left {
        db::members::update(
            &mut tx,
            id,
            &MemberWrite {
                nickname: &current.nickname,
                rank_id: current.rank_id,
                status: MemberStatus::Left,
                notes: current.notes.as_deref(),
            },
        )
        .await?;
        db::members::add_history(
            &mut tx,
            ctx.guild.id,
            id,
            ctx.membership.member_id,
            "member.removed",
            &json!({ "status": { "from": current.status, "to": MemberStatus::Left } }),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn member_history(ctx: TenantCtx, Path(id): Path<Uuid>) -> ApiResult<Json<Vec<HistoryDto>>> {
    ctx.require(Permission::MembersView)?;
    let mut tx = ctx.tx().await?;
    if db::members::find(&mut tx, id).await?.is_none() {
        return Err(Error::NotFound.into());
    }
    let entries = db::members::history(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(
        entries
            .into_iter()
            .map(|h| HistoryDto {
                id: h.id,
                action: h.action,
                payload: h.payload,
                actor_nickname: h.actor_nickname,
                at: h.at,
            })
            .collect(),
    ))
}

async fn list_ranks(ctx: TenantCtx) -> ApiResult<Json<Vec<RankDto>>> {
    ctx.require(Permission::MembersView)?;
    let mut tx = ctx.tx().await?;
    let ranks = db::members::ranks(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(
        ranks
            .into_iter()
            .map(|r| RankDto {
                id: r.id,
                name: r.name,
                position: r.position,
                permissions: r.permissions,
            })
            .collect(),
    ))
}
