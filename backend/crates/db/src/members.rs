//! Membros e ranks do tenant atual. Toda função recebe a conexão de um [`crate::tenant_tx`].

use chrono::{DateTime, Utc};
use domain::members::MemberStatus;
use sqlx::PgConnection;
use uuid::Uuid;

/// Vínculo do usuário logado com a guilda do tenant.
#[derive(Debug, Clone)]
pub struct Membership {
    pub member_id: Uuid,
    pub nickname: String,
    pub status: MemberStatus,
    pub rank_id: Uuid,
    pub rank_name: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub nickname: String,
    pub rank_id: Uuid,
    pub rank_name: String,
    pub status: MemberStatus,
    pub joined_at: DateTime<Utc>,
}

/// Membro completo, para a ficha no painel.
#[derive(Debug, Clone)]
pub struct MemberDetail {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub account_name: Option<String>,
    pub nickname: String,
    pub rank_id: Uuid,
    pub rank_name: String,
    pub rank_position: i32,
    pub status: MemberStatus,
    pub notes: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct Rank {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub id: Uuid,
    pub action: String,
    pub payload: serde_json::Value,
    pub actor_nickname: Option<String>,
    pub at: DateTime<Utc>,
}

/// Filtros da listagem. Sem `status`, lista todos menos quem saiu.
#[derive(Debug, Clone, Default)]
pub struct MemberFilter<'a> {
    pub status: Option<MemberStatus>,
    pub rank_id: Option<Uuid>,
    /// Padrão `ILIKE` já escapado (ver `domain::members::like_pattern`).
    pub nickname_like: Option<&'a str>,
    pub limit: i64,
    pub offset: i64,
}

/// Dados gravados ao criar ou editar um membro.
pub struct MemberWrite<'a> {
    pub nickname: &'a str,
    pub rank_id: Uuid,
    pub status: MemberStatus,
    pub notes: Option<&'a str>,
}

/// Índice que garante uma conta por guilda.
pub const USER_KEY: &str = "guild_members_user_key";

pub async fn membership(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> sqlx::Result<Option<Membership>> {
    let row = sqlx::query!(
        r#"SELECT m.id, m.nickname, m.status::text AS "status!", m.rank_id, r.name AS rank_name, r.permissions
             FROM guild_members m JOIN guild_ranks r ON r.id = m.rank_id
            WHERE m.user_id = $1"#,
        user_id
    )
    .fetch_optional(conn)
    .await?;
    row.map(|r| {
        Ok(Membership {
            member_id: r.id,
            nickname: r.nickname,
            status: parse_status(&r.status)?,
            rank_id: r.rank_id,
            rank_name: r.rank_name,
            permissions: r.permissions,
        })
    })
    .transpose()
}

/// Uma página de membros (hierarquia, depois apelido) e o total que bate com o filtro.
pub async fn page(
    conn: &mut PgConnection,
    f: &MemberFilter<'_>,
) -> sqlx::Result<(Vec<Member>, i64)> {
    let status = f.status.map(MemberStatus::as_str);
    let total = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!"
             FROM guild_members m
            WHERE (($1::text IS NULL AND m.status <> 'left') OR m.status::text = $1)
              AND ($2::uuid IS NULL OR m.rank_id = $2)
              AND ($3::text IS NULL OR m.nickname ILIKE $3)"#,
        status,
        f.rank_id,
        f.nickname_like,
    )
    .fetch_one(&mut *conn)
    .await?;
    let rows = sqlx::query!(
        r#"SELECT m.id, m.user_id, m.nickname, m.rank_id, r.name AS rank_name,
                  m.status::text AS "status!", m.joined_at
             FROM guild_members m JOIN guild_ranks r ON r.id = m.rank_id
            WHERE (($1::text IS NULL AND m.status <> 'left') OR m.status::text = $1)
              AND ($2::uuid IS NULL OR m.rank_id = $2)
              AND ($3::text IS NULL OR m.nickname ILIKE $3)
            ORDER BY r.position, lower(m.nickname), m.id
            LIMIT $4 OFFSET $5"#,
        status,
        f.rank_id,
        f.nickname_like,
        f.limit,
        f.offset,
    )
    .fetch_all(&mut *conn)
    .await?;
    let members = rows
        .into_iter()
        .map(|r| {
            Ok(Member {
                id: r.id,
                user_id: r.user_id,
                nickname: r.nickname,
                rank_id: r.rank_id,
                rank_name: r.rank_name,
                status: parse_status(&r.status)?,
                joined_at: r.joined_at,
            })
        })
        .collect::<sqlx::Result<_>>()?;
    Ok((members, total))
}

/// Ficha do membro. `FOR UPDATE` trava a linha até o fim da transação (edições concorrentes).
pub async fn find(conn: &mut PgConnection, id: Uuid) -> sqlx::Result<Option<MemberDetail>> {
    let row = sqlx::query!(
        r#"SELECT m.id, m.user_id, u.display_name AS "account_name?", m.nickname, m.rank_id,
                  r.name AS rank_name, r.position AS rank_position, m.status::text AS "status!",
                  m.notes, m.joined_at, m.left_at
             FROM guild_members m
             JOIN guild_ranks r ON r.id = m.rank_id
             LEFT JOIN users u ON u.id = m.user_id
            WHERE m.id = $1
              FOR UPDATE OF m"#,
        id
    )
    .fetch_optional(conn)
    .await?;
    row.map(|r| {
        Ok(MemberDetail {
            id: r.id,
            user_id: r.user_id,
            account_name: r.account_name,
            nickname: r.nickname,
            rank_id: r.rank_id,
            rank_name: r.rank_name,
            rank_position: r.rank_position,
            status: parse_status(&r.status)?,
            notes: r.notes,
            joined_at: r.joined_at,
            left_at: r.left_at,
        })
    })
    .transpose()
}

/// Falha com violação de [`USER_KEY`] se a conta já for membro.
pub async fn insert(
    conn: &mut PgConnection,
    guild_id: Uuid,
    user_id: Option<Uuid>,
    m: &MemberWrite<'_>,
) -> sqlx::Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"INSERT INTO guild_members (id, guild_id, user_id, nickname, rank_id, status, notes)
           VALUES ($1, $2, $3, $4, $5, ($6::text)::member_status, $7)"#,
        id,
        guild_id,
        user_id,
        m.nickname,
        m.rank_id,
        m.status.as_str(),
        m.notes,
    )
    .execute(conn)
    .await?;
    Ok(id)
}

/// Grava todos os campos editáveis. `left_at` acompanha o status `left`.
pub async fn update(conn: &mut PgConnection, id: Uuid, m: &MemberWrite<'_>) -> sqlx::Result<()> {
    sqlx::query!(
        r#"UPDATE guild_members
              SET nickname = $2, rank_id = $3, status = ($4::text)::member_status, notes = $5,
                  left_at = CASE WHEN $4 = 'left' THEN coalesce(left_at, now()) END
            WHERE id = $1"#,
        id,
        m.nickname,
        m.rank_id,
        m.status.as_str(),
        m.notes,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn add_history(
    conn: &mut PgConnection,
    guild_id: Uuid,
    member_id: Uuid,
    actor_member_id: Uuid,
    action: &str,
    payload: &serde_json::Value,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"INSERT INTO member_history (id, guild_id, member_id, actor_member_id, action, payload)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
        Uuid::now_v7(),
        guild_id,
        member_id,
        actor_member_id,
        action,
        payload,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Histórico do membro, do mais recente para o mais antigo.
pub async fn history(conn: &mut PgConnection, member_id: Uuid) -> sqlx::Result<Vec<HistoryEntry>> {
    sqlx::query_as!(
        HistoryEntry,
        r#"SELECT h.id, h.action, h.payload, a.nickname AS "actor_nickname?", h.at
             FROM member_history h
             LEFT JOIN guild_members a ON a.id = h.actor_member_id
            WHERE h.member_id = $1
            ORDER BY h.at DESC, h.id DESC
            LIMIT 200"#,
        member_id
    )
    .fetch_all(conn)
    .await
}

/// Membro como aparece na página pública: só apelido e rank.
#[derive(Debug, Clone)]
pub struct PublicMember {
    pub nickname: String,
    pub rank_id: Uuid,
    pub rank_name: String,
}

/// Membros ativos, do topo da hierarquia para baixo.
pub async fn public_roster(conn: &mut PgConnection) -> sqlx::Result<Vec<PublicMember>> {
    sqlx::query_as!(
        PublicMember,
        r#"SELECT m.nickname, m.rank_id, r.name AS rank_name
             FROM guild_members m JOIN guild_ranks r ON r.id = m.rank_id
            WHERE m.status = 'active'
            ORDER BY r.position, lower(m.nickname)"#
    )
    .fetch_all(conn)
    .await
}

pub async fn ranks(conn: &mut PgConnection) -> sqlx::Result<Vec<Rank>> {
    sqlx::query_as!(
        Rank,
        "SELECT id, name, position, permissions FROM guild_ranks ORDER BY position"
    )
    .fetch_all(conn)
    .await
}

fn parse_status(s: &str) -> sqlx::Result<MemberStatus> {
    MemberStatus::parse(s)
        .ok_or_else(|| sqlx::Error::Decode(format!("member_status desconhecido: {s}").into()))
}
