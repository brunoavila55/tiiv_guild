use axum::{routing::get, Json, Router};
use chrono::{DateTime, Utc};
use domain::{members::MemberStatus, permissions::Permission};
use serde::Serialize;
use uuid::Uuid;

use crate::{error::ApiResult, state::AppState, tenant::TenantCtx};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/members", get(list_members))
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
struct RankDto {
    id: Uuid,
    name: String,
    position: i32,
    permissions: Vec<String>,
}

async fn list_members(ctx: TenantCtx) -> ApiResult<Json<Vec<MemberDto>>> {
    ctx.require(Permission::MembersView)?;
    let mut tx = ctx.tx().await?;
    let members = db::members::list(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(
        members
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
