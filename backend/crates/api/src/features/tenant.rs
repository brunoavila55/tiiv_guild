//! A guilda do host atual: identidade pública e o contexto do membro logado.

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::ApiResult,
    state::AppState,
    tenant::{CurrentGuild, TenantCtx},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tenant", get(tenant))
        .route("/guild", get(guild))
}

/// Identidade visual mínima, pública: usada pelo frontend para montar o layout
/// do tenant (inclusive na tela de "entre para continuar").
#[derive(Serialize)]
struct TenantDto {
    slug: String,
    name: String,
    tag: Option<String>,
    logo_url: Option<String>,
    primary_color: Option<String>,
    secondary_color: Option<String>,
    public_page_enabled: bool,
}

#[derive(Serialize)]
struct MemberCtxDto {
    member_id: Uuid,
    nickname: String,
    rank_name: String,
    is_owner: bool,
    permissions: Vec<&'static str>,
}

#[derive(Serialize)]
struct GuildDto {
    slug: String,
    name: String,
    tag: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    me: MemberCtxDto,
}

async fn tenant(
    State(state): State<AppState>,
    CurrentGuild(guild): CurrentGuild,
) -> ApiResult<Json<TenantDto>> {
    let mut tx = db::tenant_tx(&state.pool, guild.id).await?;
    let branding = db::guilds::branding(&mut tx).await?;
    tx.commit().await?;
    let b = branding.as_ref();
    Ok(Json(TenantDto {
        logo_url: b.and_then(|b| b.logo_url.clone()),
        primary_color: b.and_then(|b| b.primary_color.clone()),
        secondary_color: b.and_then(|b| b.secondary_color.clone()),
        public_page_enabled: b.is_some_and(|b| b.public_page_enabled),
        slug: guild.slug,
        name: guild.name,
        tag: guild.tag,
    }))
}

async fn guild(ctx: TenantCtx) -> ApiResult<Json<GuildDto>> {
    Ok(Json(GuildDto {
        me: MemberCtxDto {
            member_id: ctx.membership.member_id,
            nickname: ctx.membership.nickname.clone(),
            rank_name: ctx.membership.rank_name.clone(),
            is_owner: ctx.is_owner,
            permissions: ctx.permissions.iter().map(|p| p.as_str()).collect(),
        },
        slug: ctx.guild.slug,
        name: ctx.guild.name,
        tag: ctx.guild.tag,
        created_at: ctx.guild.created_at,
    }))
}
