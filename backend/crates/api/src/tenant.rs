//! Resolução do tenant pelo subdomínio.

use axum::{extract::FromRequestParts, http::request::Parts};
use db::{guilds::Guild, members::Membership, users::User, PgPool, Tx};
use domain::{
    host::HostKind,
    members::Actor,
    permissions::{Permission, PermissionSet},
    Error,
};
use uuid::Uuid;

use crate::{auth::CurrentUser, error::ApiError, host::host_kind, state::AppState};

/// Guilda do host atual, sem exigir login (404 se o host não é de uma guilda existente).
pub struct CurrentGuild(pub Guild);

impl FromRequestParts<AppState> for CurrentGuild {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let HostKind::Tenant(slug) = host_kind(parts, &state.config.base_domain) else {
            return Err(Error::NotFound.into());
        };
        db::guilds::find_by_slug(&state.pool, &slug)
            .await?
            .map(Self)
            .ok_or(Error::NotFound.into())
    }
}

/// Contexto de tenant: guilda do host + usuário logado que é membro ativo dela.
/// 404 sem guilda, 401 sem sessão, 403 se não é membro.
pub struct TenantCtx {
    pub guild: Guild,
    pub user: User,
    pub membership: Membership,
    pub is_owner: bool,
    pub permissions: PermissionSet,
    pool: PgPool,
}

impl TenantCtx {
    pub fn require(&self, perm: Permission) -> Result<(), ApiError> {
        if self.permissions.has(perm) {
            Ok(())
        } else {
            Err(Error::Forbidden.into())
        }
    }

    /// Transação com o RLS restrito a esta guilda. Única porta para dados de tenant.
    pub async fn tx(&self) -> Result<Tx, ApiError> {
        Ok(db::tenant_tx(&self.pool, self.guild.id).await?)
    }

    /// O usuário logado na hierarquia da guilda, dado `(id, position)` de cada rank.
    pub fn actor(&self, ranks: impl IntoIterator<Item = (Uuid, i32)>) -> Actor {
        let rank_position = ranks
            .into_iter()
            .find(|(id, _)| *id == self.membership.rank_id)
            .map_or(i32::MAX, |(_, position)| position);
        Actor {
            is_owner: self.is_owner,
            rank_position,
        }
    }
}

impl FromRequestParts<AppState> for TenantCtx {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let CurrentGuild(guild) = CurrentGuild::from_request_parts(parts, state).await?;
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, state).await?;

        let mut tx = db::tenant_tx(&state.pool, guild.id).await?;
        let membership = db::members::membership(&mut tx, user.id)
            .await?
            .filter(|m| m.status.can_access())
            .ok_or(Error::Forbidden)?;
        tx.commit().await?;

        let is_owner = guild.owner_user_id == user.id;
        let permissions = if is_owner {
            PermissionSet::all()
        } else {
            PermissionSet::from_strings(&membership.permissions)
        };
        Ok(Self {
            guild,
            user,
            membership,
            is_owner,
            permissions,
            pool: state.pool.clone(),
        })
    }
}
