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

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SmeltingFile {
    #[serde(default)]
    pub smelting: Vec<SmeltingRecipe>,
}

#[derive(Clone, Debug, Default)]
pub struct ModData {
    pub manifest: ModManifest,
    pub blocks: Vec<BlockDef>,
    pub items: Vec<ItemDef>,
    pub smelting_recipes: Vec<SmeltingRecipe>,
}

/// Stable block id assigned to a mod block (vanilla ids stay untouched).
pub fn mod_block_id(namespace: &str, block_id: &str) -> u32 {
    let full = format!("{}:{}", namespace, block_id);
    let mut h: u32 = 2166136261;
    for b in full.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    lf_voxel::registry::MOD_BLOCK_BASE + (h % 1_000_000)
}

/// Register everything in ModData with the live game registries:
/// blocks (solid/opaque/drops), items, smelting entries, ore hooks.
/// Call once at boot, after loading mod directories.
pub fn apply_mod(data: &ModData) {
    let ns = &data.manifest.id;
    // blocks
    for block in &data.blocks {
        // strip the "namespace:" prefix from ids if present
        let short = block.id.split(':').last().unwrap_or(&block.id).to_string();
        let id = mod_block_id(ns, &short);
        lf_voxel::registry::register_mod_block(id, lf_voxel::registry::ModBlockDef {
            name: format!("{}:{}", ns, short),
            solid: true,
            opaque: true,
            drop: Some(format!("{}:{}", ns, short)),
        });
        // item form so it can be held/dropped/smelted
        lf_game::items::register_mod_item(
            format!("{}:{}", ns, short),
            block.name.clone(),
            lf_game::items::ItemKind::Block(id),
            64,
        );
        // *_ore blocks become worldgen veins (docs/README contract: y 8..50).
        // The noise offset derives from the stable block id and stays clear
        // of the vanilla ore offsets (+1000..+5000).
        if short.ends_with("_ore") {
            lf_worldgen::register_ore_hook(lf_worldgen::OreHook {
                block_id: id,
                y_min: 8,
                y_max: 50,
                threshold: 0.62,
                noise_offset: 10_000.0 + (id % 40_000) as f32,
            });
        }
    }
    // plain items
    for item in &data.items {
        lf_game::items::register_mod_item(
            item.id.clone(),
            item.name.clone(),
            lf_game::items::ItemKind::Material,
            64,
        );
    }
    // smelting
    for recipe in &data.smelting_recipes {
        lf_game::smelting::register_mod_smelt(recipe.input.clone(), recipe.output.clone());
    }
}

/// Load every mod under `dir` and apply it to the live registries.
/// Returns the loaded mods (empty when the directory is missing).
pub fn load_mods_dir(dir: &Path) -> Vec<ModData> {
    let mut mods = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return mods,
    };
    let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if !path.is_dir() {
            continue;
        }
        if let Ok(mut data) = load_mod(&path) {
            if let Ok(content) = std::fs::read_to_string(path.join("data/smelting.toml")) {
                if let Ok(file) = toml::from_str::<SmeltingFile>(&content) {
                    data.smelting_recipes = file.smelting;
                }
            }
            apply_mod(&data);
            mods.push(data);
        }
    }
    mods
}

/// Goal Section 4: the unmissable boot line confirming the mod pipeline via
/// the bundled smoke_test mod — "are mods working right now" answerable at
/// a glance, no log archaeology. Returns the line when the smoke mod is
/// among the loaded set; each boot site logs it with its own logger.
pub fn smoke_line(mods: &[ModData]) -> Option<&'static str> {
    mods.iter()
        .any(|m| m.manifest.id == "smoke_test")
        .then_some("[MOD SMOKE TEST] OK — smoke_test mod loaded successfully")
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

    let smelting_path = data_dir.join("smelting.toml");
    if smelting_path.exists() {
        let content = std::fs::read_to_string(&smelting_path).unwrap_or_default();
        if let Ok(file) = toml::from_str::<SmeltingFile>(&content) {
            data.smelting_recipes = file.smelting;
        }
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn smelting_toml_parses() {
        let dir = tempdir().unwrap();
        let mod_path = dir.path().join("smelter");
        std::fs::create_dir_all(mod_path.join("data")).unwrap();
        std::fs::write(mod_path.join("mod.toml"), r#"
id = "smelter"
name = "Smelter"
version = "1.0.0"
api_version = "1"
side = "both"
dependencies = []
permissions = []
"#).unwrap();
        std::fs::write(mod_path.join("data/smelting.toml"), r#"
[[smelting]]
input = "smelter:raw_stuff"
output = "smelter:ingot"
xp = 0.2
"#).unwrap();
        let data = load_mod(&mod_path).unwrap();
        assert_eq!(data.smelting_recipes.len(), 1);
        assert_eq!(data.smelting_recipes[0].output, "smelter:ingot");
        apply_mod(&data);
        assert_eq!(
            lf_game::smelting::smelt_result("smelter:raw_stuff"),
            Some("smelter:ingot")
        );
    }

    #[test]
    fn full_pipeline_registers_blocks_items_smelting() {
        let dir = tempdir().unwrap();
        let mod_path = dir.path().join("ember_ores");
        std::fs::create_dir_all(mod_path.join("data")).unwrap();
        std::fs::write(mod_path.join("mod.toml"), r#"
id = "ember_ores"
name = "Ember Ores"
version = "1.0.0"
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
        std::fs::write(mod_path.join("data/smelting.toml"), r#"
[[smelting]]
input = "ember_ores:ember_ore"
output = "ember_ores:ember_ingot"
xp = 0.7
"#).unwrap();

        let data = load_mod(&mod_path).unwrap();
        apply_mod(&data);

        // block registered and behaves
        let block_id = mod_block_id("ember_ores", "ember_ore");
        assert!(lf_voxel::registry::mod_block(block_id).is_some(), "block registered");
        assert!(lf_voxel::registry::is_solid(lf_voxel::BlockState(block_id)));
        assert_eq!(
            lf_voxel::registry::block::name(block_id),
            "ember_ores:ember_ore"
        );
        // world accepts placement of the modded block
        let mut world = lf_voxel::World::new();
        world.ensure_chunk(0, 0);
        world.set_block(3, 40, 3, lf_voxel::BlockState(block_id)).unwrap();
        assert_eq!(world.get_block(3, 40, 3).id(), block_id);
        // breaking it drops the mod item
        assert_eq!(
            lf_game::items::block_drop(block_id).as_deref(),
            Some("ember_ores:ember_ore")
        );
        // item + smelting registered
        assert!(lf_game::items::item_def("ember_ores:ember_ingot").is_some());
        assert_eq!(
            lf_game::smelting::smelt_result("ember_ores:ember_ore"),
            Some("ember_ores:ember_ingot")
        );
        // ore hook auto-registered by apply_mod (README contract)
        assert!(
            lf_worldgen::registered_ore_hooks().iter().any(|h| h.block_id == block_id),
            "*_ore block auto-registers a worldgen vein"
        );
        let gen = lf_worldgen::WorldGen::new(lf_worldgen::Seed(12345));
        let col = gen.generate_chunk(0, 0);
        let mut found = 0;
        for lx in 0..16 {
            for lz in 0..16 {
                for y in 8..50 {
                    if col.get(lx, y, lz).id() == block_id {
                        found += 1;
                    }
                }
            }
        }
        assert!(found > 0, "mod ore should generate in the world");
    }

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

    /// Goal Section 4: the bundled smoke_test mod keeps loading correctly
    /// (real mods/ folder, not a fixture) — its block and item land in the
    /// live registries and smoke_log flags it.
    #[test]
    fn smoke_test_mod_loads_from_the_real_folder() {
        let repo_mods = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join("mods/smoke_test");
        let data = load_mod(&repo_mods).expect("mods/smoke_test parses");
        assert_eq!(data.manifest.id, "smoke_test");
        assert_eq!(data.blocks.len(), 1);
        assert_eq!(data.items.len(), 1);
        apply_mod(&data);
        let id = mod_block_id("smoke_test", "ok_block");
        assert!(lf_voxel::registry::mod_block(id).is_some(),
            "smoke_test:ok_block must be registered (id {id})");
        assert!(lf_game::items::item_def("smoke_test:ok_token").is_some(),
            "smoke_test:ok_token must be a registered item");
    }
}
