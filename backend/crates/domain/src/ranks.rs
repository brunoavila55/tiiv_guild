//! Ranks da guilda: padrões de guilda nova, nome, permissões e reordenação.

use crate::{
    members::Actor,
    permissions::{Permission, PermissionSet},
    users::validate_text,
    Error,
};

pub const NAME_MIN: usize = 1;
pub const NAME_MAX: usize = 32;
pub const MAX_RANKS: usize = 30;

pub struct DefaultRank {
    pub name: &'static str,
    pub permissions: &'static [Permission],
}

/// Ranks criados em toda guilda nova. Genéricos da plataforma, sem relação com jogo;
/// a guilda renomeia e reorganiza depois. O primeiro (topo) tem todas as permissões.
pub fn default_ranks() -> &'static [DefaultRank] {
    use Permission::*;
    &[
        DefaultRank {
            name: "Líder",
            permissions: Permission::ALL,
        },
        DefaultRank {
            name: "Oficial",
            permissions: &[MembersView, MembersManage, MembersKick],
        },
        DefaultRank {
            name: "Membro",
            permissions: &[MembersView],
        },
        DefaultRank {
            name: "Recruta",
            permissions: &[],
        },
    ]
}

/// Nome obrigatório, já sem espaços nas pontas.
pub fn normalize_name(value: &str) -> Result<String, Error> {
    validate_text("name", value, NAME_MIN, NAME_MAX)?;
    Ok(value.trim().to_owned())
}

/// Permissões pedidas pelo cliente; string desconhecida → 422 em `permissions`.
pub fn parse_permissions<S: AsRef<str>>(values: &[S]) -> Result<Vec<Permission>, Error> {
    values
        .iter()
        .map(|v| {
            Permission::parse(v.as_ref()).ok_or_else(|| {
                Error::field(
                    "permissions",
                    format!("permissão desconhecida: {}", v.as_ref()),
                )
            })
        })
        .collect()
}

/// Permissões que o rank passa a ter. Quem não é dono só concede ou retira as permissões
/// que ele mesmo tem (sem escalar privilégio). Strings que o core não conhece (de um módulo
/// desativado) ficam como estavam. Sai na ordem de [`Permission::ALL`], sem repetição.
pub fn apply_permissions(
    actor: &PermissionSet,
    current: &[String],
    requested: &[Permission],
) -> Result<Vec<String>, Error> {
    let had = PermissionSet::from_strings(current);
    let mut out = Vec::new();
    for &p in Permission::ALL {
        let wanted = requested.contains(&p);
        if wanted != had.has(p) && !actor.has(p) {
            return Err(Error::field(
                "permissions",
                "você só pode conceder ou retirar permissões que tem",
            ));
        }
        if wanted {
            out.push(p.as_str().to_owned());
        }
    }
    out.extend(
        current
            .iter()
            .filter(|s| Permission::parse(s).is_none())
            .cloned(),
    );
    Ok(out)
}

/// Só o dono ou quem está acima do rank (`position`) mexe nele.
pub fn check_manage(actor: Actor, position: i32) -> Result<(), Error> {
    if actor.outranks(position) {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}

/// Nova ordem dos ranks (`current` = ids em ordem de posição). Precisa listar cada rank
/// uma única vez; quem não é dono não move o próprio rank nem os de cima.
pub fn check_reorder<T: PartialEq>(
    actor: Actor,
    current: &[(T, i32)],
    requested: &[T],
) -> Result<(), Error> {
    let permutation = requested.len() == current.len()
        && current
            .iter()
            .all(|(id, _)| requested.iter().filter(|r| *r == id).count() == 1);
    if !permutation {
        return Err(Error::field("ids", "a ordem deve listar cada rank uma vez"));
    }
    let fixed = current
        .iter()
        .zip(requested)
        .filter(|((_, position), _)| !actor.outranks(*position))
        .all(|((id, _), r)| id == r);
    if fixed {
        Ok(())
    } else {
        Err(Error::field(
            "ids",
            "você só pode reordenar ranks abaixo do seu",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Permission::*;

    fn strings(perms: &[Permission]) -> Vec<String> {
        perms.iter().map(|p| p.as_str().to_owned()).collect()
    }

    #[test]
    fn names_and_parsing() {
        assert_eq!(
            normalize_name("  Veterano ").ok().as_deref(),
            Some("Veterano")
        );
        assert!(normalize_name("   ").is_err());
        assert!(normalize_name(&"x".repeat(NAME_MAX + 1)).is_err());
        assert_eq!(
            parse_permissions(&["members.view"]).ok(),
            Some(vec![MembersView])
        );
        assert!(parse_permissions(&["games.manage"]).is_err());
    }

    #[test]
    fn permissions_without_escalation() {
        let officer = PermissionSet::from_strings(&strings(&[MembersView, MembersManage]));
        let current = vec!["members.view".to_owned(), "games.manage".to_owned()];

        // Concede o que tem; ordem canônica; string de módulo preservada.
        assert_eq!(
            apply_permissions(&officer, &current, &[MembersManage, MembersView]).ok(),
            Some(vec![
                "members.view".to_owned(),
                "members.manage".to_owned(),
                "games.manage".to_owned()
            ])
        );
        // Não concede nem retira o que não tem.
        assert!(apply_permissions(&officer, &current, &[MembersView, RanksManage]).is_err());
        let with_kick = strings(&[MembersKick]);
        assert!(apply_permissions(&officer, &with_kick, &[]).is_err());
        // Manter uma permissão que não tem é permitido.
        assert!(apply_permissions(&officer, &with_kick, &[MembersKick]).is_ok());
        // O dono (todas as permissões) faz qualquer mudança.
        assert!(apply_permissions(&PermissionSet::all(), &[], Permission::ALL).is_ok());
    }

    #[test]
    fn reorder_respects_hierarchy() {
        let current = [("a", 0), ("b", 1), ("c", 2), ("d", 3)];
        let officer = Actor {
            is_owner: false,
            rank_position: 1,
        };
        assert!(check_reorder(officer, &current, &["a", "b", "d", "c"]).is_ok());
        assert!(check_reorder(officer, &current, &["a", "c", "b", "d"]).is_err());
        assert!(check_reorder(officer, &current, &["a", "b", "c"]).is_err());
        assert!(check_reorder(officer, &current, &["a", "b", "c", "c"]).is_err());

        let owner = Actor {
            is_owner: true,
            rank_position: 3,
        };
        assert!(check_reorder(owner, &current, &["d", "c", "b", "a"]).is_ok());
        assert!(check_manage(owner, 0).is_ok());
        assert!(check_manage(officer, 1).is_err());
        assert!(check_manage(officer, 2).is_ok());
    }
}
