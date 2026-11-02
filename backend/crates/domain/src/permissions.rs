//! Permissões granulares por rank. Novas features adicionam strings aqui, sem refatorar.

pub const GUILD_BRANDING_EDIT: &str = "guild.branding.edit";
pub const MEMBERS_VIEW: &str = "members.view";
pub const MEMBERS_MANAGE: &str = "members.manage";
pub const MEMBERS_KICK: &str = "members.kick";
pub const RANKS_MANAGE: &str = "ranks.manage";
pub const CHARACTERS_MANAGE_OWN: &str = "characters.manage_own";
pub const CHARACTERS_MANAGE_ANY: &str = "characters.manage_any";

pub const ALL: &[&str] = &[
    GUILD_BRANDING_EDIT,
    MEMBERS_VIEW,
    MEMBERS_MANAGE,
    MEMBERS_KICK,
    RANKS_MANAGE,
    CHARACTERS_MANAGE_OWN,
    CHARACTERS_MANAGE_ANY,
];

pub fn is_known(permission: &str) -> bool {
    ALL.contains(&permission)
}
