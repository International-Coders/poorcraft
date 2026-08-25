use serde::{Serialize, Deserialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub side: String,
    pub dependencies: Vec<String>,
    pub permissions: Vec<String>,
}

impl Default for ModManifest {
    fn default() -> Self {
        Self {
            id: "unknown".into(),
            name: "Unknown".into(),
            version: "0.1.0".into(),
            api_version: "1".into(),
            side: "both".into(),
            dependencies: vec![],
            permissions: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockDef {
    pub id: String,
    pub name: String,
    pub texture: String,
    pub hardness: f32,
    pub harvest_level: u8,
    pub light: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemDef {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SmeltingRecipe {
    pub input: String,
    pub output: String,
    pub xp: f32,
}

#[derive(Clone, Debug, Default)]
pub struct ModData {
    pub manifest: ModManifest,
    pub blocks: Vec<BlockDef>,
    pub items: Vec<ItemDef>,
    pub smelting_recipes: Vec<SmeltingRecipe>,
}

#[derive(Debug)]
pub enum ModError {
    InvalidManifest(String),
    MissingFile(String),
    ParseError(String),
}

impl std::fmt::Display for ModError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModError::InvalidManifest(e) => write!(f, "Invalid manifest: {}", e),
            ModError::MissingFile(p) => write!(f, "Missing required file: {}", p),
            ModError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for ModError {}

pub fn load_mod(mod_path: &Path) -> Result<ModData, ModError> {
    let manifest_path = mod_path.join("mod.toml");
    if !manifest_path.exists() {
        return Err(ModError::MissingFile(manifest_path.display().to_string()));
    }

    let manifest_content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| ModError::ParseError(e.to_string()))?;
    
    let manifest: ModManifest = toml::from_str(&manifest_content)
        .map_err(|e| ModError::ParseError(e.to_string()))?;

    let mut data = ModData {
        manifest,
        ..Default::default()
    };

    let data_dir = mod_path.join("data");
    let blocks_path = data_dir.join("blocks.toml");
    if blocks_path.exists() {
        let content = std::fs::read_to_string(&blocks_path).unwrap_or_default();
        #[derive(Deserialize)]
        struct BlockWrapper {
            blocks: Vec<BlockDef>,
        }
        if let Ok(w) = toml::from_str::<BlockWrapper>(&content) {
            data.blocks = w.blocks;
        }
    }

    let items_path = data_dir.join("items.toml");
    if items_path.exists() {
        let content = std::fs::read_to_string(&items_path).unwrap_or_default();
        #[derive(Deserialize)]
        struct ItemWrapper {
            items: Vec<ItemDef>,
        }
        if let Ok(w) = toml::from_str::<ItemWrapper>(&content) {
            data.items = w.items;
        }
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_mod() {
        let dir = tempdir().unwrap();
        let mod_path = dir.path().join("ember_ores");
        std::fs::create_dir_all(mod_path.join("data")).unwrap();

        std::fs::write(mod_path.join("mod.toml"), r#"
id = "ember_ores"
name = "Ember Ores"
version = "1.2.0"
api_version = "1"
side = "both"
dependencies = ["core"]
permissions = ["world.read"]
"#).unwrap();

        std::fs::write(mod_path.join("data/blocks.toml"), r#"
[[blocks]]
id = "ember_ores:ember_ore"
name = "Ember Ore"
texture = "ember_ore.png"
hardness = 4.5
harvest_level = 2
light = 7
"#).unwrap();

        std::fs::write(mod_path.join("data/items.toml"), r#"
[[items]]
id = "ember_ores:ember_ingot"
name = "Ember Ingot"
"#).unwrap();

        let result = load_mod(&mod_path);
        assert!(result.is_ok(), "{:?}", result.err());
        let data = result.unwrap();
        assert_eq!(data.manifest.id, "ember_ores");
        assert_eq!(data.blocks.len(), 1);
        assert_eq!(data.items.len(), 1);
    }
}
