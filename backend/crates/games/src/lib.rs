//! Regras genéricas do catálogo de jogos. Sem IO e sem nada de jogo específico:
//! jogos, classes, servidores, campos de personagem e ranks padrão são dados
//! cadastrados pelo superadmin; aqui só se valida a forma desses dados.

pub mod schema;

use std::collections::HashSet;

use domain::DomainError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// Rank criado automaticamente em toda guilda nova do jogo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankTemplate {
    pub name: String,
    pub position: i32,
    pub permissions: Vec<String>,
}

/// Definição de um jogo como o superadmin cadastra.
#[derive(Debug, Clone)]
pub struct GameDefinition {
    pub level_min: i32,
    pub level_max: i32,
    pub character_schema: Value,
    pub default_ranks: Vec<RankTemplate>,
}

impl GameDefinition {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.level_min < 1 {
            return Err(invalid("level_min", "deve ser ao menos 1"));
        }
        if self.level_min > self.level_max {
            return Err(invalid("level_max", "deve ser maior ou igual a level_min"));
        }
        schema::check_schema(&self.character_schema)
            .map_err(|message| invalid("character_schema", &message))?;
        validate_ranks(&self.default_ranks)
    }

    /// Valida `characters.attributes` contra o `character_schema` do jogo.
    pub fn validate_character(&self, attrs: &Value) -> Result<(), DomainError> {
        schema::validate(&self.character_schema, attrs)
    }
}

fn validate_ranks(ranks: &[RankTemplate]) -> Result<(), DomainError> {
    if ranks.is_empty() {
        return Err(invalid("default_ranks", "precisa de ao menos um rank"));
    }
    let mut names = HashSet::new();
    for rank in ranks {
        let name = rank.name.trim();
        if name.is_empty() || name.chars().count() > 40 {
            return Err(invalid(
                "default_ranks",
                "nome de rank deve ter de 1 a 40 caracteres",
            ));
        }
        if !names.insert(name.to_lowercase()) {
            return Err(invalid("default_ranks", &format!("rank repetido: {name}")));
        }
        if let Some(p) = rank
            .permissions
            .iter()
            .find(|p| !domain::permissions::is_known(p))
        {
            return Err(invalid(
                "default_ranks",
                &format!("permissão desconhecida: {p}"),
            ));
        }
    }
    // O rank do topo (menor position) recebe o dono da guilda: precisa de todas as permissões.
    if let Some(top) = ranks.iter().min_by_key(|r| r.position) {
        if let Some(missing) = domain::permissions::ALL
            .iter()
            .find(|p| !top.permissions.iter().any(|t| t == *p))
        {
            return Err(invalid(
                "default_ranks",
                &format!(
                    "o rank do topo ({}) precisa da permissão {missing}",
                    top.name
                ),
            ));
        }
    }
    Ok(())
}

/// Slug de jogo (vira parte de URL): minúsculas, dígitos e hífen.
pub fn validate_game_slug(slug: &str) -> Result<(), DomainError> {
    validate_slug(slug, false)
}

/// Slug de classe/servidor: como o de jogo, aceitando também `_`.
pub fn validate_item_slug(slug: &str) -> Result<(), DomainError> {
    validate_slug(slug, true)
}

fn validate_slug(slug: &str, allow_underscore: bool) -> Result<(), DomainError> {
    let inner_ok = |c: char| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || (allow_underscore && c == '_')
    };
    let edge_ok = |c: Option<char>| c.is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let valid = (1..=40).contains(&slug.len())
        && slug.chars().all(inner_ok)
        && edge_ok(slug.chars().next())
        && edge_ok(slug.chars().last());
    if valid {
        Ok(())
    } else {
        Err(invalid(
            "slug",
            "use de 1 a 40 caracteres: letras minúsculas, números e hífen, sem hífen nas pontas",
        ))
    }
}

/// Nome exibido (jogo, classe, servidor), já sem espaços nas pontas.
pub fn validate_name(name: &str) -> Result<String, DomainError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(invalid("name", "deve ter de 1 a 80 caracteres"));
    }
    Ok(name.to_owned())
}

fn invalid(field: &str, message: &str) -> DomainError {
    DomainError::Validation {
        field: field.to_owned(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;

    fn all_permissions() -> Vec<String> {
        domain::permissions::ALL
            .iter()
            .map(|p| p.to_string())
            .collect()
    }

    fn definition() -> GameDefinition {
        GameDefinition {
            level_min: 1,
            level_max: 105,
            character_schema: json!({
                "type": "object",
                "additionalProperties": false,
                "properties": { "reino": { "type": "integer", "minimum": 0, "maximum": 10 } }
            }),
            default_ranks: vec![
                RankTemplate {
                    name: "Líder".into(),
                    position: 0,
                    permissions: all_permissions(),
                },
                RankTemplate {
                    name: "Membro".into(),
                    position: 1,
                    permissions: vec!["members.view".into()],
                },
            ],
        }
    }

    fn field_of(err: DomainError) -> String {
        match err {
            DomainError::Validation { field, .. } => field,
            other => panic!("esperava Validation, veio {other:?}"),
        }
    }

    #[test]
    fn valid_definition_passes_and_validates_characters() {
        let def = definition();
        def.validate().unwrap();
        def.validate_character(&json!({ "reino": 3 })).unwrap();
        assert!(def.validate_character(&json!({ "reino": 30 })).is_err());
    }

    #[test]
    fn invalid_definitions_point_to_the_field() {
        let mut def = definition();
        def.level_min = 10;
        def.level_max = 5;
        assert_eq!(field_of(def.validate().unwrap_err()), "level_max");

        let mut def = definition();
        def.character_schema = json!({ "type": "array" });
        assert_eq!(field_of(def.validate().unwrap_err()), "character_schema");

        let mut def = definition();
        def.default_ranks[0].permissions.pop();
        assert_eq!(field_of(def.validate().unwrap_err()), "default_ranks");

        let mut def = definition();
        def.default_ranks[1].permissions.push("raids.nuke".into());
        assert_eq!(field_of(def.validate().unwrap_err()), "default_ranks");

        let mut def = definition();
        def.default_ranks[1].name = "líder".into();
        assert_eq!(field_of(def.validate().unwrap_err()), "default_ranks");

        let mut def = definition();
        def.default_ranks.clear();
        assert_eq!(field_of(def.validate().unwrap_err()), "default_ranks");
    }

    #[test]
    fn slugs() {
        for ok in ["perfect-world", "pw2", "a"] {
            validate_game_slug(ok).unwrap();
        }
        let too_long = "a".repeat(41);
        for bad in [
            "",
            "-pw",
            "pw-",
            "Perfect",
            "pw world",
            "pw_1",
            too_long.as_str(),
        ] {
            assert!(validate_game_slug(bad).is_err(), "{bad}");
        }
        validate_item_slug("winged_elf").unwrap();
    }
}
