//! Game pack definido inteiramente por arquivos JSON (ver `game-packs/<jogo>/`).

use serde::Deserialize;
use serde_json::Value;

use crate::{ClassSeed, GamePack, PackError, RankSeed, ServerSeed};

/// Conteúdo bruto dos arquivos de um pack.
pub struct PackSources<'a> {
    pub game: &'a str,
    pub classes: &'a str,
    pub servers: &'a str,
    pub ranks: &'a str,
    pub character_schema: &'a str,
}

#[derive(Deserialize)]
struct GameFile {
    slug: String,
    name: String,
    level_range: (i32, i32),
}

pub struct JsonGamePack {
    slug: String,
    name: String,
    level_range: (i32, i32),
    classes: Vec<ClassSeed>,
    servers: Vec<ServerSeed>,
    ranks: Vec<RankSeed>,
    character_schema: Value,
}

impl JsonGamePack {
    pub fn from_sources(sources: &PackSources<'_>) -> Result<Self, PackError> {
        let game: GameFile = parse("<desconhecido>", "game.json", sources.game)?;
        let slug = game.slug.as_str();
        Ok(Self {
            classes: parse(slug, "classes.json", sources.classes)?,
            servers: parse(slug, "servers.json", sources.servers)?,
            ranks: parse(slug, "ranks.json", sources.ranks)?,
            character_schema: parse(slug, "character_schema.json", sources.character_schema)?,
            level_range: game.level_range,
            name: game.name,
            slug: game.slug,
        })
    }
}

fn parse<T: for<'de> Deserialize<'de>>(slug: &str, file: &str, raw: &str) -> Result<T, PackError> {
    serde_json::from_str(raw).map_err(|e| PackError::new(slug, format!("{file}: {e}")))
}

impl GamePack for JsonGamePack {
    fn slug(&self) -> &str {
        &self.slug
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn character_schema(&self) -> &Value {
        &self.character_schema
    }

    fn default_ranks(&self) -> Vec<RankSeed> {
        self.ranks.clone()
    }

    fn level_range(&self) -> (i32, i32) {
        self.level_range
    }

    fn classes(&self) -> &[ClassSeed] {
        &self.classes
    }

    fn servers(&self) -> &[ServerSeed] {
        &self.servers
    }
}
