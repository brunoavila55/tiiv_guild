//! Trait `GamePack`, `GameRegistry` e implementações por jogo (`perfect_world`, ...).
//!
//! Tudo que é específico de jogo vive aqui (ou nos seeds de `game-packs/`), nunca no core.
//! Adicionar um jogo = novo pack registrado em [`GameRegistry::builtin`]; nada mais muda.

mod json_pack;
pub mod perfect_world;
pub mod schema;

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use domain::DomainError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use json_pack::{JsonGamePack, PackSources};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoleHint {
    Tank,
    Healer,
    Dps,
    Support,
}

impl RoleHint {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tank => "tank",
            Self::Healer => "healer",
            Self::Dps => "dps",
            Self::Support => "support",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassSeed {
    pub slug: String,
    pub name: String,
    pub role_hint: RoleHint,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSeed {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankSeed {
    pub name: String,
    pub position: i32,
    pub permissions: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("game pack `{slug}`: {message}")]
pub struct PackError {
    pub slug: String,
    pub message: String,
}

impl PackError {
    pub fn new(slug: &str, message: impl Into<String>) -> Self {
        Self {
            slug: slug.to_owned(),
            message: message.into(),
        }
    }
}

pub trait GamePack: Send + Sync {
    /// Identificador estável, ex. `"perfect-world"`.
    fn slug(&self) -> &str;
    fn name(&self) -> &str;
    /// JSON Schema (subconjunto de [`schema`]) de `characters.attributes`.
    fn character_schema(&self) -> &Value;
    fn validate_character(&self, attrs: &Value) -> Result<(), DomainError> {
        schema::validate(self.character_schema(), attrs)
    }
    fn default_ranks(&self) -> Vec<RankSeed>;
    fn level_range(&self) -> (i32, i32);
    fn classes(&self) -> &[ClassSeed];
    fn servers(&self) -> &[ServerSeed];
    // futuro: activity_kinds(), default_party_composition() ...
}

/// Packs disponíveis, montado no startup.
#[derive(Clone, Default)]
pub struct GameRegistry {
    packs: BTreeMap<String, Arc<dyn GamePack>>,
}

impl GameRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Packs embutidos no binário.
    pub fn builtin() -> Result<Self, PackError> {
        let mut registry = Self::new();
        registry.register(perfect_world::pack()?)?;
        Ok(registry)
    }

    /// Registra um pack depois de checar a consistência dos seeds.
    pub fn register(&mut self, pack: impl GamePack + 'static) -> Result<(), PackError> {
        check_pack(&pack)?;
        let slug = pack.slug().to_owned();
        if self.packs.contains_key(&slug) {
            return Err(PackError::new(&slug, "registrado duas vezes"));
        }
        self.packs.insert(slug, Arc::new(pack));
        Ok(())
    }

    pub fn get(&self, slug: &str) -> Option<&dyn GamePack> {
        self.packs.get(slug).map(|p| p.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn GamePack> {
        self.packs.values().map(|p| p.as_ref())
    }
}

fn check_pack(pack: &dyn GamePack) -> Result<(), PackError> {
    let slug = pack.slug();
    let err = |message: String| PackError::new(slug, message);

    let valid_slug = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    };
    if !valid_slug(slug) {
        return Err(err("slug deve usar só a-z, 0-9, - e _".to_owned()));
    }

    let (min, max) = pack.level_range();
    if min < 1 || min > max {
        return Err(err(format!("level_range inválido: ({min}, {max})")));
    }

    schema::check_schema(pack.character_schema())
        .map_err(|m| err(format!("character_schema: {m}")))?;

    let mut seen = HashSet::new();
    for class in pack.classes() {
        if !valid_slug(&class.slug) || !seen.insert(class.slug.as_str()) {
            return Err(err(format!(
                "classe com slug inválido ou repetido: {}",
                class.slug
            )));
        }
    }
    let mut seen = HashSet::new();
    for server in pack.servers() {
        if !valid_slug(&server.slug) || !seen.insert(server.slug.as_str()) {
            return Err(err(format!(
                "servidor com slug inválido ou repetido: {}",
                server.slug
            )));
        }
    }

    let ranks = pack.default_ranks();
    if ranks.is_empty() {
        return Err(err("precisa de ao menos um rank padrão".to_owned()));
    }
    let mut names = HashSet::new();
    for rank in &ranks {
        if !names.insert(rank.name.as_str()) {
            return Err(err(format!("rank repetido: {}", rank.name)));
        }
        if let Some(p) = rank
            .permissions
            .iter()
            .find(|p| !domain::permissions::is_known(p))
        {
            return Err(err(format!(
                "rank {}: permissão desconhecida {p}",
                rank.name
            )));
        }
    }
    // O rank do topo (menor position) recebe o dono da guilda: precisa de todas as permissões.
    let top = ranks
        .iter()
        .min_by_key(|r| r.position)
        .map(|r| &r.permissions);
    if let Some(top) = top {
        if let Some(missing) = domain::permissions::ALL
            .iter()
            .find(|p| !top.iter().any(|t| t == *p))
        {
            return Err(err(format!("rank do topo sem a permissão {missing}")));
        }
    }
    Ok(())
}
