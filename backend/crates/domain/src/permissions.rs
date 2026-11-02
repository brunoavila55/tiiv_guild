use serde::{Deserialize, Serialize};

/// Permissões do core. Módulos acrescentam variantes próprias sem mudar as existentes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    #[serde(rename = "guild.settings.edit")]
    GuildSettingsEdit,
    #[serde(rename = "guild.branding.edit")]
    GuildBrandingEdit,
    #[serde(rename = "members.view")]
    MembersView,
    #[serde(rename = "members.manage")]
    MembersManage,
    #[serde(rename = "members.kick")]
    MembersKick,
    #[serde(rename = "ranks.manage")]
    RanksManage,
    #[serde(rename = "modules.manage")]
    ModulesManage,
}

impl Permission {
    pub const ALL: &[Permission] = &[
        Self::GuildSettingsEdit,
        Self::GuildBrandingEdit,
        Self::MembersView,
        Self::MembersManage,
        Self::MembersKick,
        Self::RanksManage,
        Self::ModulesManage,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::GuildSettingsEdit => "guild.settings.edit",
            Self::GuildBrandingEdit => "guild.branding.edit",
            Self::MembersView => "members.view",
            Self::MembersManage => "members.manage",
            Self::MembersKick => "members.kick",
            Self::RanksManage => "ranks.manage",
            Self::ModulesManage => "modules.manage",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|p| p.as_str() == s)
    }
}

/// Permissões efetivas de um membro.
#[derive(Debug, Clone, Default)]
pub struct PermissionSet(Vec<Permission>);

impl PermissionSet {
    pub fn all() -> Self {
        Self(Permission::ALL.to_vec())
    }

    /// Ignora strings desconhecidas (ex.: de um módulo desativado).
    pub fn from_strings<S: AsRef<str>>(perms: &[S]) -> Self {
        Self(
            perms
                .iter()
                .filter_map(|p| Permission::parse(p.as_ref()))
                .collect(),
        )
    }

    pub fn has(&self, perm: Permission) -> bool {
        self.0.contains(&perm)
    }

    pub fn iter(&self) -> impl Iterator<Item = Permission> + '_ {
        self.0.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_strings() {
        for p in Permission::ALL {
            assert_eq!(Permission::parse(p.as_str()), Some(*p));
        }
        let set = PermissionSet::from_strings(&["members.view", "games.manage"]);
        assert!(set.has(Permission::MembersView));
        assert!(!set.has(Permission::MembersManage));
    }
}
