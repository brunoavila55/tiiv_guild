use chrono::{DateTime, Utc};
use domain::ranks::default_ranks;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::set_tenant;

#[derive(Debug, Clone)]
pub struct Guild {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub tag: Option<String>,
    pub owner_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Branding {
    pub logo_url: Option<String>,
    pub banner_url: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub motto: Option<String>,
    pub description_md: Option<String>,
    pub public_page_enabled: bool,
    pub socials: serde_json::Value,
}

/// Uma guilda da qual o usuário é membro ativo.
#[derive(Debug, Clone)]
pub struct UserGuild {
    pub slug: String,
    pub name: String,
    pub tag: Option<String>,
    pub is_owner: bool,
    pub rank_name: String,
}

pub struct NewGuild<'a> {
    pub slug: &'a str,
    pub name: &'a str,
    pub tag: Option<&'a str>,
    pub owner_user_id: Uuid,
    pub owner_nickname: &'a str,
}

pub const SLUG_KEY: &str = "guilds_slug_key";

pub async fn find_by_slug(pool: &PgPool, slug: &str) -> sqlx::Result<Option<Guild>> {
    sqlx::query_as!(
        Guild,
        "SELECT id, slug, name, tag, owner_user_id, created_at FROM guilds WHERE slug = $1",
        slug
    )
    .fetch_optional(pool)
    .await
}

pub async fn slug_exists(pool: &PgPool, slug: &str) -> sqlx::Result<bool> {
    let row = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM guilds WHERE slug = $1) AS "exists!""#,
        slug
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Cria a guilda com branding vazio, ranks padrão e o criador como membro no rank do topo.
/// Falha com violação de [`SLUG_KEY`] se o slug já existir.
pub async fn create(pool: &PgPool, new: NewGuild<'_>) -> sqlx::Result<Guild> {
    let mut tx = pool.begin().await?;
    let guild = sqlx::query_as!(
        Guild,
        r#"INSERT INTO guilds (id, slug, name, tag, owner_user_id)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, slug, name, tag, owner_user_id, created_at"#,
        Uuid::now_v7(),
        new.slug,
        new.name,
        new.tag,
        new.owner_user_id,
    )
    .fetch_one(&mut *tx)
    .await?;

    set_tenant(&mut tx, guild.id).await?;
    sqlx::query!(
        "INSERT INTO guild_branding (guild_id) VALUES ($1)",
        guild.id
    )
    .execute(&mut *tx)
    .await?;

    let mut top_rank = None;
    for (position, rank) in (0_i32..).zip(default_ranks()) {
        let id = Uuid::now_v7();
        let perms: Vec<String> = rank
            .permissions
            .iter()
            .map(|p| p.as_str().to_owned())
            .collect();
        sqlx::query!(
            "INSERT INTO guild_ranks (id, guild_id, name, position, permissions) VALUES ($1, $2, $3, $4, $5)",
            id,
            guild.id,
            rank.name,
            position,
            &perms,
        )
        .execute(&mut *tx)
        .await?;
        top_rank.get_or_insert(id);
    }
    let top_rank = top_rank.ok_or_else(|| sqlx::Error::Protocol("sem ranks padrão".into()))?;

    let member_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO guild_members (id, guild_id, user_id, nickname, rank_id) VALUES ($1, $2, $3, $4, $5)",
        member_id,
        guild.id,
        new.owner_user_id,
        new.owner_nickname,
        top_rank,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO member_history (id, guild_id, member_id, actor_member_id, action) VALUES ($1, $2, $3, $3, 'guild.created')",
        Uuid::now_v7(),
        guild.id,
        member_id,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(guild)
}

pub async fn user_guilds(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<UserGuild>> {
    sqlx::query_as!(
        UserGuild,
        r#"SELECT slug AS "slug!", name AS "name!", tag, is_owner AS "is_owner!", rank_name AS "rank_name!"
             FROM user_guilds($1)"#,
        user_id
    )
    .fetch_all(pool)
    .await
}

/// Todas as guildas, para o painel da plataforma.
pub async fn list_all(pool: &PgPool) -> sqlx::Result<Vec<(Guild, String)>> {
    let rows = sqlx::query!(
        r#"SELECT g.id, g.slug, g.name, g.tag, g.owner_user_id, g.created_at, u.email AS owner_email
             FROM guilds g JOIN users u ON u.id = g.owner_user_id
            ORDER BY g.created_at DESC"#
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            (
                Guild {
                    id: r.id,
                    slug: r.slug,
                    name: r.name,
                    tag: r.tag,
                    owner_user_id: r.owner_user_id,
                    created_at: r.created_at,
                },
                r.owner_email,
            )
        })
        .collect())
}

/// Branding da guilda do tenant atual.
pub async fn branding(conn: &mut PgConnection) -> sqlx::Result<Option<Branding>> {
    sqlx::query_as!(
        Branding,
        r#"SELECT logo_url, banner_url, primary_color, secondary_color, motto, description_md,
                  public_page_enabled, socials
             FROM guild_branding"#
    )
    .fetch_optional(conn)
    .await
}
