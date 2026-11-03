//! Ranks do tenant atual. Toda função recebe a conexão de um [`crate::tenant_tx`].

use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Rank {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
    pub permissions: Vec<String>,
}

/// Rank com quantos membros (de qualquer status) estão nele.
#[derive(Debug, Clone)]
pub struct RankSummary {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
    pub permissions: Vec<String>,
    pub member_count: i64,
}

/// Índice do nome único (sem diferenciar maiúsculas) na guilda.
pub const NAME_KEY: &str = "guild_ranks_name_key";

/// Ranks do topo para baixo.
pub async fn list(conn: &mut PgConnection) -> sqlx::Result<Vec<Rank>> {
    sqlx::query_as!(
        Rank,
        "SELECT id, name, position, permissions FROM guild_ranks ORDER BY position"
    )
    .fetch_all(conn)
    .await
}

/// Como [`list`], mas trava os ranks até o fim da transação: toda escrita em ranks
/// começa por aqui, então edições concorrentes da hierarquia se enfileiram.
pub async fn list_for_update(conn: &mut PgConnection) -> sqlx::Result<Vec<Rank>> {
    sqlx::query_as!(
        Rank,
        "SELECT id, name, position, permissions FROM guild_ranks ORDER BY position FOR UPDATE"
    )
    .fetch_all(conn)
    .await
}

pub async fn summaries(conn: &mut PgConnection) -> sqlx::Result<Vec<RankSummary>> {
    sqlx::query_as!(
        RankSummary,
        r#"SELECT r.id, r.name, r.position, r.permissions, count(m.id) AS "member_count!"
             FROM guild_ranks r LEFT JOIN guild_members m ON m.rank_id = r.id
            GROUP BY r.id
            ORDER BY r.position"#
    )
    .fetch_all(conn)
    .await
}

/// Falha com violação de [`NAME_KEY`] se o nome já existir.
pub async fn insert(
    conn: &mut PgConnection,
    guild_id: Uuid,
    name: &str,
    position: i32,
    permissions: &[String],
) -> sqlx::Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO guild_ranks (id, guild_id, name, position, permissions) VALUES ($1, $2, $3, $4, $5)",
        id,
        guild_id,
        name,
        position,
        permissions,
    )
    .execute(conn)
    .await?;
    Ok(id)
}

/// Falha com violação de [`NAME_KEY`] se o nome já existir.
pub async fn update(
    conn: &mut PgConnection,
    id: Uuid,
    name: &str,
    permissions: &[String],
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE guild_ranks SET name = $2, permissions = $3 WHERE id = $1",
        id,
        name,
        permissions,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Posições 0..n na ordem de `ids` (todos os ranks da guilda).
pub async fn set_order(conn: &mut PgConnection, ids: &[Uuid]) -> sqlx::Result<()> {
    sqlx::query!(
        r#"UPDATE guild_ranks r SET position = o.ord - 1
             FROM unnest($1::uuid[]) WITH ORDINALITY AS o(id, ord)
            WHERE r.id = o.id AND r.position <> o.ord - 1"#,
        ids,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Passa todos os membros de `from` para `to` e devolve quem mudou.
pub async fn move_members(
    conn: &mut PgConnection,
    from: Uuid,
    to: Uuid,
) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar!(
        "UPDATE guild_members SET rank_id = $2 WHERE rank_id = $1 RETURNING id",
        from,
        to,
    )
    .fetch_all(conn)
    .await
}

/// Só apaga rank sem membros (a FK de `guild_members` garante).
pub async fn delete(conn: &mut PgConnection, id: Uuid) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM guild_ranks WHERE id = $1", id)
        .execute(conn)
        .await?;
    Ok(())
}
