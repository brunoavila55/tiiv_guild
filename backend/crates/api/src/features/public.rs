//! Página pública da guilda (`{slug}.BASE_DOMAIN/`): sem login, só o que a guilda publicou.

use std::collections::BTreeMap;

use axum::{extract::State, routing::get, Json, Router};
use domain::Error;
use serde::Serialize;
use uuid::Uuid;

use crate::{error::ApiResult, state::AppState, tenant::CurrentGuild};

pub fn router() -> Router<AppState> {
    Router::new().route("/public", get(public_page))
}

/// Nunca inclui e-mail, notas, status ou campos customizados dos membros.
#[derive(Serialize)]
struct PublicPageDto {
    slug: String,
    name: String,
    tag: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    logo_url: Option<String>,
    banner_url: Option<String>,
    primary_color: Option<String>,
    secondary_color: Option<String>,
    motto: Option<String>,
    description_md: Option<String>,
    socials: BTreeMap<String, String>,
    member_count: usize,
    ranks: Vec<PublicRankDto>,
}

/// Rank com os apelidos dos membros ativos; ranks vazios não aparecem.
#[derive(Serialize)]
struct PublicRankDto {
    name: String,
    members: Vec<String>,
}

async fn public_page(
    State(state): State<AppState>,
    CurrentGuild(guild): CurrentGuild,
) -> ApiResult<Json<PublicPageDto>> {
    let mut tx = db::tenant_tx(&state.pool, guild.id).await?;
    let branding = db::guilds::branding(&mut tx)
        .await?
        .filter(|b| b.public_page_enabled)
        .ok_or(Error::NotFound)?;
    let roster = db::members::public_roster(&mut tx).await?;
    tx.commit().await?;

    let member_count = roster.len();
    // O roster já vem ordenado por rank: agrupa os consecutivos.
    let mut ranks: Vec<(Uuid, PublicRankDto)> = Vec::new();
    for m in roster {
        match ranks.last_mut() {
            Some((id, r)) if *id == m.rank_id => r.members.push(m.nickname),
            _ => ranks.push((
                m.rank_id,
                PublicRankDto {
                    name: m.rank_name,
                    members: vec![m.nickname],
                },
            )),
        }
    }

    Ok(Json(PublicPageDto {
        socials: serde_json::from_value(branding.socials).unwrap_or_default(),
        logo_url: branding.logo_url,
        banner_url: branding.banner_url,
        primary_color: branding.primary_color,
        secondary_color: branding.secondary_color,
        motto: branding.motto,
        description_md: branding.description_md,
        member_count,
        ranks: ranks.into_iter().map(|(_, r)| r).collect(),
        slug: guild.slug,
        name: guild.name,
        tag: guild.tag,
        created_at: guild.created_at,
    }))
}
