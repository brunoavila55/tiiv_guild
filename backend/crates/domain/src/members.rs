use serde::{Deserialize, Serialize};

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
}
