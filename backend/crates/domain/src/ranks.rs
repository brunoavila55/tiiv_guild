use crate::permissions::Permission;

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
