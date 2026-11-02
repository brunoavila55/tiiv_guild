//! Configurações da guilda: por enquanto, o schema dos campos customizados de membro.

use axum::{routing::get, Json, Router};
use domain::{fields::ObjectSchema, permissions::Permission};
use serde_json::Value;
use sqlx::PgConnection;

use crate::{error::ApiResult, state::AppState, tenant::TenantCtx};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/settings/member-fields",
        get(show_member_fields).put(update_member_fields),
    )
}

/// Schema dos campos de membro da guilda do tenant (vazio se nunca foi salvo).
pub async fn member_schema(conn: &mut PgConnection) -> ApiResult<ObjectSchema> {
    Ok(db::settings::member_fields(conn)
        .await?
        .map(ObjectSchema::from_stored)
        .unwrap_or_default())
}

/// Quem vê membros precisa do schema para mostrar e preencher a ficha.
async fn show_member_fields(ctx: TenantCtx) -> ApiResult<Json<Value>> {
    ctx.require(Permission::MembersView)?;
    let mut tx = ctx.tx().await?;
    let schema = member_schema(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(schema.to_value()))
}

/// Troca o schema inteiro. Valores já salvos de campos removidos ficam no banco, mas
/// deixam de aparecer; campo que vira obrigatório só é exigido na próxima edição dos campos.
async fn update_member_fields(ctx: TenantCtx, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    ctx.require(Permission::GuildSettingsEdit)?;
    let schema = ObjectSchema::parse(body)?.normalize()?.to_value();
    let mut tx = ctx.tx().await?;
    db::settings::set_member_fields(&mut tx, ctx.guild.id, &schema).await?;
    tx.commit().await?;
    Ok(Json(schema))
}
