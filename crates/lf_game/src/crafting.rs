//! Shaped crafting over a 2x2 or 3x3 grid.

use crate::survival::ItemStack;

#[derive(Clone, Debug)]
pub struct Recipe {
    pub output: String,
    pub output_count: u8,
    /// Rows of ingredient ids; None = empty cell. 2x2 or 3x3.
    pub pattern: Vec<Vec<Option<&'static str>>>,
}

impl Recipe {
    pub fn grid_size(&self) -> usize {
        self.pattern.len()
    }
}

/// The recipe book. Patterns are matched with translation (the pattern may
/// sit anywhere inside the grid) but not mirrored.
pub fn recipes() -> &'static [Recipe] {
    fn r(output: &str, count: u8, pattern: Vec<Vec<Option<&'static str>>>) -> Recipe {
        Recipe { output: output.to_string(), output_count: count, pattern }
    }
    static RECIPES: std::sync::OnceLock<Vec<Recipe>> = std::sync::OnceLock::new();
    &RECIPES.get_or_init(|| {
    let mut book = Vec::new();
    // log -> 4 planks (shapeless-ish: single cell)
    book.push(r("planks", 4, vec![vec![Some("log")]]));
    // 2 planks (vertical) -> 4 sticks
    book.push(r("stick", 4, vec![vec![Some("planks")], vec![Some("planks")]]));
    // 2x2 planks -> crafting table
    book.push(r("crafting_table", 1, vec![
        vec![Some("planks"), Some("planks")],
        vec![Some("planks"), Some("planks")],
    ]));
    // coal over stick -> 4 torches
    book.push(r("torch", 4, vec![vec![Some("coal")], vec![Some("stick")]]));
    // iron around a torch -> lantern (the block existed with light 15 but
    // was unobtainable — audit Step 1)
    book.push(r("lantern", 1, vec![
        vec![Some("iron_ingot")],
        vec![Some("torch")],
    ]));
    // Water Age (P29, gated to the Water era by Era::required_for):
    // wheel = planks frame + iron axle; battery = iron shell + copper
    book.push(r("water_wheel", 1, vec![
        vec![Some("planks"), Some("iron_ingot"), Some("planks")],
        vec![Some("planks"), None, Some("planks")],
        vec![Some("planks"), Some("stick"), Some("planks")],
    ]));
    book.push(r("battery", 1, vec![
        vec![Some("iron_ingot"), Some("copper_wire"), Some("iron_ingot")],
        vec![Some("copper_wire"), Some("coal"), Some("copper_wire")],
        vec![Some("iron_ingot"), Some("copper_wire"), Some("iron_ingot")],
    ]));
    // Steam Age (P30, Steam-era gated): copper pipes, the boiler drum,
    // the engine that drinks its steam
    book.push(r("pipe", 6, vec![
        vec![Some("copper_ingot"), None, Some("copper_ingot")],
        vec![None, Some("copper_ingot"), None],
    ]));
    book.push(r("boiler", 1, vec![
        vec![Some("iron_ingot"), Some("iron_ingot"), Some("iron_ingot")],
        vec![Some("iron_ingot"), Some("furnace"), Some("iron_ingot")],
        vec![Some("iron_ingot"), Some("iron_gear"), Some("iron_ingot")],
    ]));
    book.push(r("steam_engine", 1, vec![
        vec![Some("iron_ingot"), Some("copper_wire"), Some("iron_ingot")],
        vec![Some("machine_frame"), Some("iron_gear"), Some("machine_frame")],
        vec![Some("iron_ingot"), Some("copper_wire"), Some("iron_ingot")],
    ]));
    // Oil Age (P31): extraction kit is Industrial-gated; the combustion
    // generator is Oil-era (Era::required_for) and drinks refined fuel.
    book.push(r("pump", 1, vec![
        vec![Some("iron_ingot"), Some("pipe"), Some("iron_ingot")],
        vec![Some("iron_gear"), Some("machine_frame"), Some("iron_gear")],
        vec![Some("pipe"), Some("iron_ingot"), Some("pipe")],
    ]));
    book.push(r("refinery", 1, vec![
        vec![Some("iron_ingot"), Some("glass"), Some("iron_ingot")],
        vec![Some("pipe"), Some("machine_frame"), Some("pipe")],
        vec![Some("copper_ingot"), Some("furnace"), Some("copper_ingot")],
    ]));
    book.push(r("combustion_generator", 1, vec![
        vec![Some("iron_ingot"), Some("iron_gear"), Some("iron_ingot")],
        vec![Some("copper_wire"), Some("machine_frame"), Some("copper_wire")],
        vec![Some("iron_ingot"), Some("iron_ingot"), Some("iron_ingot")],
    ]));
    // Nuclear (P32, Nuclear-era gated): the containment vessel itself
    book.push(r("reactor", 1, vec![
        vec![Some("iron_ingot"), Some("glass"), Some("iron_ingot")],
        vec![Some("machine_frame"), Some("basic_circuit"), Some("machine_frame")],
        vec![Some("iron_ingot"), Some("iron_gear"), Some("iron_ingot")],
    ]));
    // Magic foundation (P33): the enchanting table, and the two crossover
    // blocks — magic that plays along with the machines instead of
    // replacing them (doc 05)
    book.push(r("enchanting_table", 1, vec![
        vec![Some("stone"), Some("book"), Some("stone")],
        vec![Some("glitch_dust"), Some("null_shard"), Some("glitch_dust")],
        vec![Some("stone"), Some("book"), Some("stone")],
    ]));
    book.push(r("lumen_block", 2, vec![
        vec![Some("glitch_dust"), Some("glass"), Some("glitch_dust")],
        vec![Some("glass"), Some("torch"), Some("glass")],
        vec![Some("glitch_dust"), Some("glass"), Some("glitch_dust")],
    ]));
    // Construction (P34): shapes + builder's kit
    book.push(r("stone_slab", 6, vec![
        vec![Some("stone"), Some("stone"), Some("stone")],
    ]));
    book.push(r("planks_slab", 6, vec![
        vec![Some("planks"), Some("planks"), Some("planks")],
    ]));
    book.push(r("stone_stairs", 4, vec![
        vec![Some("stone"), None, None],
        vec![Some("stone"), Some("stone"), None],
        vec![Some("stone"), Some("stone"), Some("stone")],
    ]));
    book.push(r("scaffold", 8, vec![
        vec![Some("planks"), None, Some("planks")],
        vec![None, Some("stick"), None],
        vec![Some("planks"), None, Some("planks")],
    ]));
    book.push(r("chisel", 1, vec![
        vec![None, Some("iron_ingot"), None],
        vec![None, Some("stick"), None],
    ]));
    // Step 27: the item belt backbone
    book.push(r("belt", 6, vec![
        vec![Some("iron_ingot"), Some("iron_gear"), Some("iron_ingot")],
    ]));
    // P37 ornate professional-tier recipes (path-gated via paths::gate_for)
    book.push(r("precision_gear", 1, vec![
        vec![Some("machine_frame"), Some("iron_gear"), Some("machine_frame")],
        vec![Some("copper_wire"), Some("glitch_dust"), Some("copper_wire")],
    ]));
    book.push(r("master_blueprint", 1, vec![
        vec![Some("blueprint"), Some("book"), Some("blueprint")],
        vec![Some("null_shard"), Some("glitch_dust"), Some("null_shard")],
    ]));
    book.push(r("battlestaff", 1, vec![
        vec![Some("dragon_scale"), Some("glitch_dust"), Some("dragon_scale")],
        vec![Some("stick"), Some("null_shard"), Some("stick")],
    ]));
    book.push(r("master_chisel", 1, vec![
        vec![Some("dragon_scale"), Some("iron_ingot"), Some("dragon_scale")],
        vec![Some("stick"), Some("rune_of_haste"), Some("stick")],
    ]));
    // Smart building (P35)
    book.push(r("conduit", 8, vec![
        vec![Some("copper_wire"), Some("iron_ingot"), Some("copper_wire")],
    ]));
    book.push(r("elevator", 2, vec![
        vec![Some("iron_ingot"), Some("machine_frame"), Some("iron_ingot")],
        vec![Some("planks"), Some("copper_wire"), Some("planks")],
    ]));
    book.push(r("ac_unit", 1, vec![
        vec![Some("iron_ingot"), Some("copper_wire"), Some("iron_ingot")],
        vec![Some("machine_frame"), Some("glass"), Some("machine_frame")],
    ]));
    book.push(r("computer", 1, vec![
        vec![Some("glass"), Some("basic_circuit"), Some("glass")],
        vec![Some("machine_frame"), Some("basic_circuit"), Some("machine_frame")],
    ]));
    book.push(r("blueprint", 1, vec![
        vec![None, Some("iron_ingot"), None],
        vec![Some("iron_ingot"), Some("book"), Some("iron_ingot")],
    ]));
    book.push(r("warding_pylon", 1, vec![
        vec![Some("null_shard"), Some("machine_frame"), Some("null_shard")],
        vec![Some("machine_frame"), Some("glitch_dust"), Some("machine_frame")],
        vec![Some("null_shard"), Some("machine_frame"), Some("null_shard")],
    ]));
    // 3 iron ingots in a V -> bucket (scoop/place water sources)
    book.push(r("bucket", 1, vec![
        vec![Some("iron_ingot"), None, Some("iron_ingot")],
        vec![None, Some("iron_ingot"), None],
    ]));
    // Faction + decoration blocks (lore-and-visuals C1)
    book.push(r("accord_stone", 4, vec![
        vec![Some("stone"), Some("stone")],
        vec![Some("stone"), Some("stone")],
    ]));
    book.push(r("accord_pillar", 3, vec![
        vec![Some("accord_stone")],
        vec![Some("accord_stone")],
        vec![Some("accord_stone")],
    ]));
    book.push(r("ironborn_brick", 4, vec![
        vec![Some("stone"), Some("raw_iron")],
        vec![Some("raw_iron"), Some("stone")],
    ]));
    book.push(r("ironborn_grate", 2, vec![
        vec![Some("iron_ingot"), None, Some("iron_ingot")],
        vec![None, Some("iron_ingot"), None],
        vec![Some("iron_ingot"), None, Some("iron_ingot")],
    ]));
    book.push(r("ember_covenantwood", 4, vec![
        vec![Some("log"), Some("log")],
    ]));
    book.push(r("ember_glowstone", 1, vec![
        vec![Some("ember_covenantwood")],
        vec![Some("coal"), Some("coal")],
    ]));
    book.push(r("freeholds_thatch", 4, vec![
        vec![Some("stick"), Some("stick"), Some("stick")],
    ]));
    book.push(r("freeholds_daub", 4, vec![
        vec![Some("dirt"), Some("dirt")],
        vec![Some("sand"), Some("sand")],
    ]));
    book.push(r("ashen_marble", 2, vec![
        vec![Some("stone"), Some("stone"), Some("stone")],
        vec![None, Some("coal"), None],
    ]));
    book.push(r("ashen_bookshelf", 1, vec![
        vec![Some("ashen_marble"), Some("book"), Some("ashen_marble")],
        vec![Some("ashen_marble"), Some("book"), Some("ashen_marble")],
    ]));
    book.push(r("carved_oak", 2, vec![
        vec![Some("planks"), Some("planks")],
        vec![None, Some("stick")],
    ]));
    book.push(r("carved_stone", 2, vec![
        vec![Some("stone"), Some("stone")],
        vec![None, Some("stick")],
    ]));
    book.push(r("carved_iron", 2, vec![
        vec![Some("iron_ingot"), Some("iron_ingot")],
        vec![None, Some("stone")],
    ]));
    // stained glass: 3 glass + one tint item (uses real materials)
    for (tint, out) in [
        ("apple", "stained_glass_red"),
        ("raw_copper", "stained_glass_orange"),
        ("sulfur", "stained_glass_yellow"),
        ("leaves", "stained_glass_green"),
        ("null_shard", "stained_glass_blue"),
        ("glitch_dust", "stained_glass_purple"),
        ("coal", "stained_glass_black"),
        ("snow", "stained_glass_white"),
    ] {
        book.push(r(out, 3, vec![
            vec![Some("glass"), Some("glass"), Some("glass")],
            vec![None, Some(tint), None],
        ]));
    }
    // banners: 2 sticks + the faction's material
    for (mat, out) in [
        ("accord_stone", "banner_accord"),
        ("ironborn_brick", "banner_ironborn"),
        ("ember_covenantwood", "banner_covenant"),
        ("freeholds_thatch", "banner_freeholds"),
        ("ashen_marble", "banner_ashen"),
        ("nameless_rotwood", "banner_nameless"),
    ] {
        book.push(r(out, 1, vec![
            vec![Some(mat), Some(mat), Some(mat)],
            vec![None, Some("stick"), None],
            vec![None, Some("stick"), None],
        ]));
    }
    book.push(r("lantern_hanging", 1, vec![
        vec![Some("iron_ingot"), None, Some("iron_ingot")],
        vec![None, Some("lantern"), None],
    ]));
    // Ironborn quest recipe: the press faceplates (ironborn_q2)
    book.push(r("iron_plate", 5, vec![
        vec![Some("iron_ingot"), Some("iron_ingot"), Some("iron_ingot")],
        vec![None, Some("coal"), None],
    ]));
    // Covenant channeler wage: concentrated Anima in crystal form
    book.push(r("anima_crystal", 1, vec![
        vec![Some("ember_glowstone"), Some("coal")],
    ]));
    // pickaxes (3x3): material row + 2 sticks
    for (mat, out) in [("planks", "wooden_pickaxe"), ("stone", "stone_pickaxe")] {
        book.push(r(out, 1, vec![
            vec![Some(mat), Some(mat), Some(mat)],
            vec![None, Some("stick"), None],
            vec![None, Some("stick"), None],
        ]));
    }
    // axes (3x3)
    for (mat, out) in [("planks", "wooden_axe"), ("stone", "stone_axe")] {
        book.push(r(out, 1, vec![
            vec![Some(mat), Some(mat), None],
            vec![Some(mat), Some("stick"), None],
            vec![None, Some("stick"), None],
        ]));
    }
    // shovels + swords (3x3); iron tools use ingots
    for (mat, out) in [
        ("planks", "wooden_shovel"),
        ("stone", "stone_shovel"),
        ("iron_ingot", "iron_shovel"),
        ("planks", "wooden_sword"),
        ("stone", "stone_sword"),
        ("iron_ingot", "iron_sword"),
    ] {
        book.push(r(out, 1, vec![
            vec![None, Some(mat), None],
            vec![None, Some("stick"), None],
            vec![None, Some("stick"), None],
        ]));
    }
    // pickaxes and axes in iron too
    book.push(r("iron_pickaxe", 1, vec![
        vec![Some("iron_ingot"), Some("iron_ingot"), Some("iron_ingot")],
        vec![None, Some("stick"), None],
        vec![None, Some("stick"), None],
    ]));
    book.push(r("iron_axe", 1, vec![
        vec![Some("iron_ingot"), Some("iron_ingot"), None],
        vec![Some("iron_ingot"), Some("stick"), None],
        vec![None, Some("stick"), None],
    ]));
    // furnace: 8 stone ring
    book.push(r("furnace", 1, vec![
        vec![Some("stone"), Some("stone"), Some("stone")],
        vec![Some("stone"), None, Some("stone")],
        vec![Some("stone"), Some("stone"), Some("stone")],
    ]));
    // chest: 8 planks ring
    book.push(r("chest", 1, vec![
        vec![Some("planks"), Some("planks"), Some("planks")],
        vec![Some("planks"), None, Some("planks")],
        vec![Some("planks"), Some("planks"), Some("planks")],
    ]));
    // bow: sticks + string-substitute (leaves fiber)
    book.push(r("bow", 1, vec![
        vec![Some("stick"), Some("leaves"), None],
        vec![Some("stick"), None, Some("leaves")],
        vec![Some("stick"), Some("leaves"), None],
    ]));
    // arrows: flint-substitute (stone tip)
    book.push(r("arrow", 4, vec![
        vec![Some("stone"), Some("stick"), Some("leaves")],
    ]));
    // armor via rings of material
    book.push(r("bronze_chestplate", 1, vec![
        vec![Some("bronze_ingot"), Some("bronze_ingot"), Some("bronze_ingot")],
        vec![Some("bronze_ingot"), Some("bronze_ingot"), Some("bronze_ingot")],
        vec![Some("bronze_ingot"), Some("bronze_ingot"), Some("bronze_ingot")],
    ]));
    book.push(r("steel_chestplate", 1, vec![
        vec![Some("steel_ingot"), Some("steel_ingot"), Some("steel_ingot")],
        vec![Some("steel_ingot"), Some("steel_ingot"), Some("steel_ingot")],
        vec![Some("steel_ingot"), Some("steel_ingot"), Some("steel_ingot")],
    ]));
    // industrial intermediates
    book.push(r("copper_wire", 6, vec![vec![Some("copper_ingot"), Some("copper_ingot"), Some("copper_ingot")]]));
    book.push(r("iron_gear", 1, vec![
        vec![Some("iron_ingot"), None, Some("iron_ingot")],
        vec![None, Some("iron_ingot"), None],
        vec![Some("iron_ingot"), None, Some("iron_ingot")],
    ]));
    book.push(r("machine_frame", 1, vec![
        vec![Some("iron_ingot"), Some("iron_gear"), Some("iron_ingot")],
        vec![Some("iron_gear"), None, Some("iron_gear")],
        vec![Some("iron_ingot"), Some("iron_gear"), Some("iron_ingot")],
    ]));
    book.push(r("basic_circuit", 1, vec![
        vec![Some("copper_wire"), Some("copper_wire"), None],
        vec![Some("tin_ingot"), Some("iron_ingot"), None],
    ]));
    // machines: frame + specific part
    book.push(r("coal_generator", 1, vec![
        vec![Some("machine_frame"), Some("iron_gear"), Some("stone")],
        vec![Some("furnace"), None, Some("stone")],
        vec![Some("machine_frame"), Some("iron_gear"), Some("stone")],
    ]));
    book.push(r("electric_furnace", 1, vec![
        vec![Some("machine_frame"), Some("copper_wire"), Some("furnace")],
        vec![Some("basic_circuit"), None, Some("furnace")],
        vec![Some("machine_frame"), Some("copper_wire"), Some("furnace")],
    ]));
    book.push(r("crusher", 1, vec![
        vec![Some("machine_frame"), Some("iron_gear"), Some("iron_gear")],
        vec![Some("basic_circuit"), None, Some("iron_gear")],
        vec![Some("machine_frame"), Some("iron_gear"), Some("iron_gear")],
    ]));
    book.push(r("assembler", 1, vec![
        vec![Some("machine_frame"), Some("basic_circuit"), Some("copper_wire")],
        vec![Some("crafting_table"), None, Some("copper_wire")],
        vec![Some("machine_frame"), Some("basic_circuit"), Some("copper_wire")],
    ]));
    book.push(r("research_bench", 1, vec![
        vec![Some("book"), Some("book"), Some("book")],
        vec![Some("basic_circuit"), Some("crafting_table"), Some("basic_circuit")],
        vec![Some("machine_frame"), Some("machine_frame"), Some("machine_frame")],
    ]));
    // smithing table
    book.push(r("smithing_table", 1, vec![
        vec![Some("iron_ingot"), Some("iron_ingot"), Some("iron_ingot")],
        vec![Some("planks"), Some("crafting_table"), Some("planks")],
        vec![Some("planks"), Some("planks"), Some("planks")],
    ]));
    book
    })
}

// --- runtime mod recipes ---
fn mod_recipes() -> &'static std::sync::RwLock<Vec<Recipe>> {
    static RECIPES: std::sync::OnceLock<std::sync::RwLock<Vec<Recipe>>> = std::sync::OnceLock::new();
    RECIPES.get_or_init(|| std::sync::RwLock::new(Vec::new()))
}

/// Vanilla + mod recipes in one list, for recipe browsers.
pub fn all_recipes() -> Vec<Recipe> {
    let mut all = recipes().to_vec();
    all.extend(mod_recipes().read().unwrap().iter().cloned());
    all
}

/// Register a mod recipe. Ingredient ids are leaked into the bounded mod
/// namespace set (idempotent by output+pattern equality check).
pub fn register_mod_recipe(output: String, output_count: u8, pattern: Vec<Vec<Option<String>>>) -> bool {
    // leak ingredient ids into the bounded mod namespace set up front
    let static_pattern: Vec<Vec<Option<&'static str>>> = pattern
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|c| c.map(|s| Box::leak(s.into_boxed_str()) as &'static str))
                .collect()
        })
        .collect();
    let mut recipes = mod_recipes().write().unwrap();
    if recipes.iter().any(|r| r.output == output && r.pattern == static_pattern) {
        return true;
    }
    recipes.push(Recipe { output, output_count, pattern: static_pattern });
    true
}

/// Try to craft from a grid (row-major, 2x2 or 3x3) of stacks.
/// Returns the crafted result if a recipe matches; does not consume inputs.
pub fn match_recipe(grid: &[Option<ItemStack>]) -> Option<(String, u8)> {
    let size = match grid.len() {
        4 => 2usize,
        9 => 3,
        _ => return None,
    };
    let cell = |x: usize, y: usize| -> Option<&str> {
        grid[y * size + x].as_ref().map(|s| s.item_id.as_str())
    };
    let mods = mod_recipes().read().unwrap().clone();
    for recipe in recipes().iter().chain(mods.iter()) {
        let rh = recipe.pattern.len(); // rows = height
        let rw = recipe.pattern.iter().map(|r| r.len()).max().unwrap_or(0); // widest row = width
        if rw > size || rh > size {
            continue;
        }
        // Try every translation.
        for oy in 0..=(size - rh) {
            for ox in 0..=(size - rw) {
                let mut ok = true;
                // pattern cells must match
                for py in 0..rh {
                    for px in 0..rw {
                        let want = recipe.pattern[py].get(px).copied().flatten();
                        let got = cell(ox + px, oy + py);
                        if want.is_none() && got.is_none() {
                            continue;
                        }
                        match (want, got) {
                            (Some(w), Some(g)) if w == g => {}
                            _ => { ok = false; }
                        }
                    }
                }
                if !ok {
                    continue;
                }
                // cells outside the pattern footprint must be empty
                let mut clear = true;
                for y in 0..size {
                    for x in 0..size {
                        let inside = x >= ox && x < ox + rw && y >= oy && y < oy + rh;
                        if !inside && cell(x, y).is_some() {
                            clear = false;
                        }
                    }
                }
                if clear {
                    return Some((recipe.output.clone(), recipe.output_count));
                }
            }
        }
    }
    None
}

/// Consume one of each ingredient in the grid (after a successful match).
pub fn consume_ingredients(grid: &mut [Option<ItemStack>]) {
    for slot in grid.iter_mut() {
        if let Some(stack) = slot {
            stack.count -= 1;
            if stack.count == 0 {
                *slot = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(id: &str) -> Option<ItemStack> {
        Some(ItemStack { item_id: id.to_string(), count: 1 })
    }

    #[test]
    fn log_to_planks_anywhere_in_grid() {
        let mut grid = vec![None::<ItemStack>; 4];
        grid[3] = s("log");
        assert_eq!(match_recipe(&grid), Some(("planks".into(), 4)));
        let mut grid9 = vec![None::<ItemStack>; 9];
        grid9[0] = s("log");
        assert_eq!(match_recipe(&grid9), Some(("planks".into(), 4)));
    }

    #[test]
    fn sticks_need_vertical_planks() {
        let mut grid = vec![None::<ItemStack>; 4];
        grid[0] = s("planks");
        grid[2] = s("planks"); // vertical pair in column 0
        assert_eq!(match_recipe(&grid), Some(("stick".into(), 4)));
        // horizontal pair does not match
        let mut grid = vec![None::<ItemStack>; 4];
        grid[0] = s("planks");
        grid[1] = s("planks");
        assert_eq!(match_recipe(&grid), None);
    }

    #[test]
    fn pickaxe_requires_3x3_and_exact_shape() {
        let mut grid = vec![None::<ItemStack>; 9];
        for x in 0..3 {
            grid[x] = s("stone");
        }
        grid[4] = s("stick");
        grid[7] = s("stick");
        assert_eq!(match_recipe(&grid), Some(("stone_pickaxe".into(), 1)));
        // extra stray item blocks the match
        grid[8] = s("stone");
        assert_eq!(match_recipe(&grid), None);
        // 2x2 grid cannot hold a 3-wide recipe
        let small = vec![None::<ItemStack>; 4];
        assert_eq!(match_recipe(&small), None);
    }

    #[test]
    fn consume_clears_spent_stacks() {
        let mut grid = vec![None::<ItemStack>; 4];
        grid[0] = Some(ItemStack { item_id: "log".into(), count: 3 });
        consume_ingredients(&mut grid);
        assert_eq!(grid[0].as_ref().unwrap().count, 2);
        let mut single = vec![s("log"), None, None, None];
        consume_ingredients(&mut single);
        assert!(single[0].is_none());
    }

    #[test]
    fn mod_recipes_match() {
        register_mod_recipe("ember_ores:ember_block".into(), 1, vec![
            vec![Some("ember_ores:ember_ingot".into()), Some("ember_ores:ember_ingot".into())],
            vec![Some("ember_ores:ember_ingot".into()), Some("ember_ores:ember_ingot".into())],
        ]);
        let mut grid = vec![None::<ItemStack>; 4];
        grid[0] = s("ember_ores:ember_ingot");
        grid[1] = s("ember_ores:ember_ingot");
        grid[2] = s("ember_ores:ember_ingot");
        grid[3] = s("ember_ores:ember_ingot");
        assert_eq!(match_recipe(&grid), Some(("ember_ores:ember_block".into(), 1)));
    }

    #[test]
    fn torches_from_coal_and_stick() {
        let mut grid = vec![None::<ItemStack>; 4];
        grid[0] = s("coal");
        grid[2] = s("stick");
        assert_eq!(match_recipe(&grid), Some(("torch".into(), 4)));
    }

    /// Audit Step 1: the lantern block existed with light 15 but was
    /// unobtainable; it is now craftable (iron over torch).
    #[test]
    fn lantern_is_craftable() {
        let mut grid = vec![None::<ItemStack>; 4];
        grid[0] = s("iron_ingot");
        grid[2] = s("torch");
        assert_eq!(match_recipe(&grid), Some(("lantern".into(), 1)));
    }

    #[test]
    fn recipes_table_is_a_singleton() {
        // the book is built once and shared, never leaked per call
        let a = recipes();
        let b = recipes();
        assert!(std::ptr::eq(a.as_ptr(), b.as_ptr()));
        assert!(!a.is_empty());
    }
}
