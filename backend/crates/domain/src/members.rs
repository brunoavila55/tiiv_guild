//! Membros da guilda: status, textos e quem pode mexer em quem.

use serde::{Deserialize, Serialize};

use crate::{branding::optional_text, users::validate_text, Error};

pub const NICKNAME_MIN: usize = 2;
pub const NICKNAME_MAX: usize = 32;
pub const NOTES_MAX: usize = 2_000;
pub const PER_PAGE_DEFAULT: u32 = 25;
pub const PER_PAGE_MAX: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberStatus {
    Active,
    Trial,
    Inactive,
    Left,
    Banned,
}

impl MemberStatus {
    /// Pode acessar o painel da guilda.
    pub fn can_access(self) -> bool {
        matches!(self, Self::Active | Self::Trial)
    }

    /// Saída e banimento (nos dois sentidos) exigem `members.kick`, não só `members.manage`.
    pub fn is_removal(self) -> bool {
        matches!(self, Self::Left | Self::Banned)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Trial => "trial",
            Self::Inactive => "inactive",
            Self::Left => "left",
            Self::Banned => "banned",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [
            Self::Active,
            Self::Trial,
            Self::Inactive,
            Self::Left,
            Self::Banned,
        ]
        .into_iter()
        .find(|v| v.as_str() == s)
    }
}

/// Apelido obrigatório, já sem espaços nas pontas.
pub fn normalize_nickname(value: &str) -> Result<String, Error> {
    validate_text("nickname", value, NICKNAME_MIN, NICKNAME_MAX)?;
    Ok(value.trim().to_owned())
}

/// Notas internas (só quem gerencia membros vê): vazio vira `None`.
pub fn normalize_notes(value: Option<&str>) -> Result<Option<String>, Error> {
    optional_text("notes", value, NOTES_MAX)
}

/// Quem está agindo: o dono passa por cima da hierarquia; os demais só alcançam
/// ranks estritamente abaixo do seu (menor `position` = topo).
#[derive(Debug, Clone, Copy)]
pub struct Actor {
    pub is_owner: bool,
    pub rank_position: i32,
}

impl Actor {
    /// Pode editar/remover um membro do rank `position` (e atribuir esse rank a alguém).
    pub fn outranks(self, position: i32) -> bool {
        self.is_owner || position > self.rank_position
    }

    /// Checa se pode mexer no membro alvo. O dono da guilda só é editado por ele mesmo.
    pub fn check_target(self, target_position: i32, target_is_owner: bool) -> Result<(), Error> {
        if self.is_owner || (!target_is_owner && self.outranks(target_position)) {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }

    /// Checa se pode atribuir o rank `position`.
    pub fn check_assign(self, position: i32) -> Result<(), Error> {
        if self.outranks(position) {
            Ok(())
        } else {
            Err(Error::field(
                "rank_id",
                "você só pode atribuir ranks abaixo do seu",
            ))
        }
    }
}

/// O dono nunca perde o acesso: não sai, não é banido nem fica inativo.
pub fn check_owner_status(target_is_owner: bool, status: MemberStatus) -> Result<(), Error> {
    if target_is_owner && !status.can_access() {
        Err(Error::Conflict(
            "o dono da guilda não pode ser removido, banido nem ficar inativo",
        ))
    } else {
        Ok(())
    }
}

/// Termo de busca para `ILIKE`, com `%`, `_` e `\` escapados; vazio vira `None`.
pub fn like_pattern(q: &str) -> Option<String> {
    let q = q.trim();
    if q.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(q.len() + 2);
    out.push('%');
    for c in q.chars().take(64) {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    Some(out)
}

/// Página (a partir de 1) e itens por página dentro dos limites.
pub fn clamp_page(page: Option<u32>, per_page: Option<u32>) -> (u32, u32) {
    let per_page = per_page.unwrap_or(PER_PAGE_DEFAULT).clamp(1, PER_PAGE_MAX);
    (page.unwrap_or(1).max(1), per_page)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_strings() {
        for s in ["active", "trial", "inactive", "left", "banned"] {
            assert_eq!(MemberStatus::parse(s).map(MemberStatus::as_str), Some(s));
        }
        assert_eq!(MemberStatus::parse("x"), None);
    }

    #[test]
    fn hierarchy() {
        let officer = Actor {
            is_owner: false,
            rank_position: 1,
        };
        assert!(officer.check_target(2, false).is_ok());
        assert!(officer.check_target(1, false).is_err(), "mesmo rank");
        assert!(officer.check_target(0, false).is_err(), "acima");
        assert!(officer.check_target(3, true).is_err(), "dono");
        assert!(officer.check_assign(2).is_ok());
        assert!(officer.check_assign(1).is_err());

        let owner = Actor {
            is_owner: true,
            rank_position: 3,
        };
        assert!(owner.check_target(0, true).is_ok());
        assert!(owner.check_assign(0).is_ok());
    }

    #[test]
    fn owner_keeps_access() {
        assert!(check_owner_status(true, MemberStatus::Trial).is_ok());
        assert!(check_owner_status(true, MemberStatus::Left).is_err());
        assert!(check_owner_status(false, MemberStatus::Banned).is_ok());
    }

    #[test]
    fn search_and_pages() {
        assert_eq!(like_pattern("  "), None);
        assert_eq!(like_pattern("a_b%").as_deref(), Some("%a\\_b\\%%"));
        assert_eq!(clamp_page(None, None), (1, PER_PAGE_DEFAULT));
        assert_eq!(clamp_page(Some(0), Some(1000)), (1, PER_PAGE_MAX));
    }
}
