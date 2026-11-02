//! Perfect World: seeds em `game-packs/perfect_world/`, embutidos no binário.

use crate::{JsonGamePack, PackError, PackSources};

pub const SLUG: &str = "perfect-world";

macro_rules! seed {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../game-packs/perfect_world/",
            $file
        ))
    };
}

pub fn pack() -> Result<JsonGamePack, PackError> {
    JsonGamePack::from_sources(&PackSources {
        game: seed!("game.json"),
        classes: seed!("classes.json"),
        servers: seed!("servers.json"),
        ranks: seed!("ranks.json"),
        character_schema: seed!("character_schema.json"),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{GamePack, GameRegistry};

    #[test]
    fn builtin_registry_has_perfect_world() {
        let registry = GameRegistry::builtin().unwrap();
        let pw = registry.get(SLUG).unwrap();
        assert_eq!(pw.name(), "Perfect World");
        assert_eq!(pw.level_range(), (1, 105));
        assert_eq!(pw.classes().len(), 12);
        assert_eq!(pw.default_ranks()[0].name, "Líder");
    }

    #[test]
    fn validates_character_attributes() {
        let pw = pack().unwrap();
        pw.validate_character(&json!({ "cultivo": "sabio", "reino": 5, "ataque": 12000 }))
            .unwrap();
        assert!(pw.validate_character(&json!({ "reino": 99 })).is_err());
        assert!(pw
            .validate_character(&json!({ "nivel_de_vip": 1 }))
            .is_err());
    }
}
