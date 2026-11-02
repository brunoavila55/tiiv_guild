//! Visual da guilda: cores, textos, redes sociais, logo e banner.

use std::collections::BTreeMap;

use axum::{
    extract::{multipart::MultipartError, DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use bytes::Bytes;
use db::guilds::{Branding, BrandingUpdate, ImageSlot};
use domain::{
    branding::{
        normalize_color, normalize_socials, optional_text, validate_image, DESCRIPTION_MAX,
        IMAGE_MAX_BYTES, MOTTO_MAX,
    },
    error::Validator,
    permissions::Permission,
    Error,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
    tenant::TenantCtx,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/branding", get(show).put(update))
        .route(
            "/branding/{slot}",
            put(upload)
                .delete(remove)
                // folga para os cabeçalhos do multipart; o limite real é checado no arquivo
                .layer(DefaultBodyLimit::max(IMAGE_MAX_BYTES + 64 * 1024)),
        )
}

#[derive(Serialize)]
pub struct BrandingDto {
    logo_url: Option<String>,
    banner_url: Option<String>,
    primary_color: Option<String>,
    secondary_color: Option<String>,
    motto: Option<String>,
    description_md: Option<String>,
    public_page_enabled: bool,
    socials: BTreeMap<String, String>,
}

impl From<Branding> for BrandingDto {
    fn from(b: Branding) -> Self {
        Self {
            socials: serde_json::from_value(b.socials).unwrap_or_default(),
            logo_url: b.logo_url,
            banner_url: b.banner_url,
            primary_color: b.primary_color,
            secondary_color: b.secondary_color,
            motto: b.motto,
            description_md: b.description_md,
            public_page_enabled: b.public_page_enabled,
        }
    }
}

#[derive(Deserialize)]
struct UpdateReq {
    primary_color: Option<String>,
    secondary_color: Option<String>,
    motto: Option<String>,
    description_md: Option<String>,
    public_page_enabled: bool,
    #[serde(default)]
    socials: BTreeMap<String, String>,
}

fn parse_slot(slot: &str) -> Result<(ImageSlot, &'static str), ApiError> {
    match slot {
        "logo" => Ok((ImageSlot::Logo, "logo")),
        "banner" => Ok((ImageSlot::Banner, "banner")),
        _ => Err(Error::NotFound.into()),
    }
}

async fn load(ctx: &TenantCtx) -> ApiResult<BrandingDto> {
    let mut tx = ctx.tx().await?;
    let branding = db::guilds::branding(&mut tx)
        .await?
        .ok_or(Error::NotFound)?;
    tx.commit().await?;
    Ok(branding.into())
}

async fn show(ctx: TenantCtx) -> ApiResult<Json<BrandingDto>> {
    Ok(Json(load(&ctx).await?))
}

async fn update(ctx: TenantCtx, Json(req): Json<UpdateReq>) -> ApiResult<Json<BrandingDto>> {
    ctx.require(Permission::GuildBrandingEdit)?;

    let color = |field: &str, v: &Option<String>| match v.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(c) => normalize_color(field, c).map(Some),
    };
    let mut v = Validator::default();
    let primary = v.check(color("primary_color", &req.primary_color));
    let secondary = v.check(color("secondary_color", &req.secondary_color));
    let motto = v.check(optional_text("motto", req.motto.as_deref(), MOTTO_MAX));
    let description = v.check(optional_text(
        "description_md",
        req.description_md.as_deref(),
        DESCRIPTION_MAX,
    ));
    let socials = v.check(normalize_socials(&req.socials));
    v.finish()?;
    let socials = serde_json::to_value(socials).map_err(anyhow::Error::from)?;

    let mut tx = ctx.tx().await?;
    db::guilds::update_branding(
        &mut tx,
        &BrandingUpdate {
            primary_color: primary.as_deref(),
            secondary_color: secondary.as_deref(),
            motto: motto.as_deref(),
            description_md: description.as_deref(),
            public_page_enabled: req.public_page_enabled,
            socials: &socials,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Json(load(&ctx).await?))
}

async fn upload(
    State(state): State<AppState>,
    Path(slot): Path<String>,
    ctx: TenantCtx,
    mut multipart: Multipart,
) -> ApiResult<Json<BrandingDto>> {
    let (slot, name) = parse_slot(&slot)?;
    ctx.require(Permission::GuildBrandingEdit)?;

    let bytes = read_file_field(&mut multipart).await?;
    let kind = validate_image("file", &bytes)?;
    let key = format!(
        "guilds/{}/{name}-{}.{}",
        ctx.guild.id,
        Uuid::now_v7(),
        kind.extension()
    );
    state.media.put(&key, bytes, kind.mime()).await?;

    let replaced = async {
        let mut tx = ctx.tx().await?;
        let old = db::guilds::set_image(&mut tx, slot, Some(&state.media.url(&key))).await?;
        tx.commit().await?;
        Ok::<_, ApiError>(old)
    }
    .await;
    match replaced {
        Ok(old) => delete_previous(&state, old).await,
        Err(e) => {
            state.media.delete_quietly(&key).await;
            return Err(e);
        }
    }
    Ok(Json(load(&ctx).await?))
}

async fn remove(
    State(state): State<AppState>,
    Path(slot): Path<String>,
    ctx: TenantCtx,
) -> ApiResult<Json<BrandingDto>> {
    let (slot, _) = parse_slot(&slot)?;
    ctx.require(Permission::GuildBrandingEdit)?;
    let mut tx = ctx.tx().await?;
    let old = db::guilds::set_image(&mut tx, slot, None).await?;
    tx.commit().await?;
    delete_previous(&state, old).await;
    Ok(Json(load(&ctx).await?))
}

async fn delete_previous(state: &AppState, old_url: Option<String>) {
    if let Some(key) = old_url.as_deref().and_then(|u| state.media.key_from_url(u)) {
        state.media.delete_quietly(key).await;
    }
}

/// Lê o campo `file` do multipart; estouro do limite vira erro de validação.
async fn read_file_field(multipart: &mut Multipart) -> ApiResult<Bytes> {
    let too_big = || Error::field("file", "a imagem deve ter no máximo 2 MB");
    let map = |e: MultipartError| -> ApiError {
        if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
            too_big().into()
        } else {
            Error::field("file", "envio inválido").into()
        }
    };
    while let Some(field) = multipart.next_field().await.map_err(map)? {
        if field.name() == Some("file") {
            return field.bytes().await.map_err(map);
        }
    }
    Err(Error::field("file", "envie a imagem no campo file").into())
}
