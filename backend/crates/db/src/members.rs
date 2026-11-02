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

#[derive(Debug, Clone)]
pub struct Rank {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
    pub permissions: Vec<String>,
}

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

pub async fn list(conn: &mut PgConnection) -> sqlx::Result<Vec<Member>> {
    let rows = sqlx::query!(
        r#"SELECT m.id, m.user_id, m.nickname, m.rank_id, r.name AS rank_name,
                  m.status::text AS "status!", m.joined_at
             FROM guild_members m JOIN guild_ranks r ON r.id = m.rank_id
            WHERE m.status <> 'left'
            ORDER BY r.position, lower(m.nickname)"#
    )
    .fetch_all(conn)
    .await?;
    rows.into_iter()
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
        .collect()
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
    Ok(match s {
        "active" => MemberStatus::Active,
        "trial" => MemberStatus::Trial,
        "inactive" => MemberStatus::Inactive,
        "left" => MemberStatus::Left,
        "banned" => MemberStatus::Banned,
        other => {
            return Err(sqlx::Error::Decode(
                format!("member_status desconhecido: {other}").into(),
            ))
        }
    })
}
