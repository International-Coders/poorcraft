//! The crafting workbench data model (ui-world-craft Section F).
//!
//! Crafting here is a conversation, not a form: the workbench shows what
//! the player can make, checks materials against the real inventory, and
//! crafts in batches. Recipes are aggregated ingredient lists — position
//! never matters, so the old 3x3 grid metaphor is gone. Recipe VISIBILITY
//! is earned: a base set is always known, the rest unlock when the player
//! first picks up a key ingredient or reaches the era that produces them.

use std::collections::HashSet;

use lf_game::research::Era;

// ------------------------------------------------------------------
// Categories

/// The eight crafting categories, in sidebar order. Each has a short
/// label, an iconic item used as its icon, and a greeting line shown in
/// the detail panel when nothing is selected — the workbench speaks.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Category {
    Materials,
    Tools,
    Building,
    Food,
    Machines,
    Magic,
    Armor,
    Deco,
}

pub const CATEGORIES: [Category; 8] = [
    Category::Materials,
    Category::Tools,
    Category::Building,
    Category::Food,
    Category::Machines,
    Category::Magic,
    Category::Armor,
    Category::Deco,
];

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Materials => "Materials",
            Category::Tools => "Tools",
            Category::Building => "Building",
            Category::Food => "Food",
            Category::Machines => "Machines",
            Category::Magic => "Magic",
            Category::Armor => "Armor",
            Category::Deco => "Deco",
        }
    }

    /// An item id whose icon represents the category.
    pub fn icon_item(self) -> &'static str {
        match self {
            Category::Materials => "iron_ingot",
            Category::Tools => "iron_pickaxe",
            Category::Building => "planks",
            Category::Food => "apple",
            Category::Machines => "steam_engine",
            Category::Magic => "enchanting_table",
            Category::Armor => "bronze_chestplate",
            Category::Deco => "carved_oak",
        }
    }

    /// The category's voice, shown when no recipe is selected. Data, not
    /// renderer strings — the world has a personality, the UI delivers it.
    pub fn greeting(self) -> &'static str {
        match self {
            Category::Materials => "The foundation of everything you'll build. Iron doesn't apologize for being iron.",
            Category::Tools => "Built to last, if you maintain it.",
            Category::Building => "A block that can become anything.",
            Category::Food => "Sustains the body. Simple as that.",
            Category::Machines => "The Ironborn's contribution to the age.",
            Category::Magic => "Anima, shaped by intention.",
            Category::Armor => "Between you and the world.",
            Category::Deco => "Because places should feel like places.",
        }
    }
}

/// Which category a recipe belongs to, by its OUTPUT. Item-kind first,
/// then a table for the ids whose kind doesn't tell the story.
pub fn categorize(output: &str) -> Category {
    use lf_game::items::{item_def, ItemKind, ToolKind};
    if let Some(def) = item_def(output) {
        match def.kind {
            ItemKind::Tool(ToolKind::Pickaxe | ToolKind::Axe | ToolKind::Shovel | ToolKind::Sword | ToolKind::Bow, _) => {
                return Category::Tools;
            }
            ItemKind::Food(_) => return Category::Food,
            ItemKind::Armor(_) => return Category::Armor,
            ItemKind::Material | ItemKind::Block(_) => {}
        }
    }
    match output {
        // materials: raw -> processed
        "planks" | "stick" | "iron_ingot" | "copper_ingot" | "tin_ingot"
        | "bronze_ingot" | "steel_ingot" | "aluminum_ingot" | "uranium_ingot"
        | "iron_plate" | "copper_wire" | "iron_gear" | "precision_gear"
        | "basic_circuit" | "glass" | "bucket" | "fuel_rod" => Category::Materials,
        // tools & weapons beyond ItemKind::Tool
        "arrow" | "chisel" | "master_chisel" | "battlestaff" | "blueprint"
        | "master_blueprint" => Category::Tools,
        // machines & power
        "furnace" | "smithing_table" | "coal_generator" | "electric_furnace"
        | "crusher" | "assembler" | "water_wheel" | "battery" | "pipe"
        | "boiler" | "steam_engine" | "pump" | "refinery"
        | "combustion_generator" | "reactor" | "belt" | "research_bench"
        | "conduit" | "elevator" | "ac_unit" => Category::Machines,
        // magic & enchanting
        "enchanting_table" | "lumen_block" | "warding_pylon" | "scroll_of_firebolt"
        | "scroll_of_gale_step" | "scroll_of_ward" | "scroll_of_hearthlight"
        | "rune_of_haste" | "rune_of_warding" | "anima_crystal" => Category::Magic,
        // decoration
        "carved_oak" | "carved_stone" | "carved_iron" | "statue" | "computer" => Category::Deco,
        // building blocks (everything placeable that isn't a machine)
        "crafting_table" | "chest" | "torch" | "lantern" | "lantern_hanging"
        | "stone_slab" | "planks_slab" | "stone_stairs" | "scaffold"
        | "accord_stone" | "accord_pillar" | "ironborn_brick" | "ironborn_grate"
        | "ember_covenantwood" | "ember_glowstone" | "freeholds_thatch"
        | "freeholds_daub" | "ashen_marble" | "ashen_bookshelf" => Category::Building,
        _ => Category::Materials,
    }
}

/// Flavor text for a recipe's output — what the thing IS in the world.
/// Iconic items get their own line; everything else speaks in its
/// category's voice (the generic fallbacks live here as data too).
pub fn flavor_for(output: &str) -> &'static str {
    match output {
        "planks" => "Sawn from the log, square and honest. Every hall in Valdenmoor started here.",
        "stick" => "A branch with ambition.",
        "torch" => "Pitch and cloth. The oldest argument against the dark.",
        "crafting_table" => "A workbench of one's own. Everything after this is made ON something.",
        "furnace" => "Stone gut that eats fuel and gives back metal.",
        "iron_ingot" => "The Ironborn have smelted this in their forges since Era I. Their reputation is forged into every bar.",
        "bronze_ingot" => "Copper grown strong through an alliance with tin — the Accord approves of this kind of marriage.",
        "steel_ingot" => "Iron that has been through the fire twice and come out prouder.",
        "chest" => "Locks sell separately. Trust sells even harder.",
        "water_wheel" => "The river works so you don't have to. The Free Holds call this progress.",
        "steam_engine" => "Boiled water with opinions. It will shake your floor and haul your ore.",
        "reactor" => "The Ashen Archivists call it a lantern. Do not stand near the lantern.",
        "enchanting_table" => "Anima settles into written things. This one is written all the way through.",
        "bronze_chestplate" => "Bronze over the heart. Heavy, and heavier in the right way.",
        "bow" => "Bent wood and a strung promise.",
        "apple" => "Picked, not made. Some things arrive finished.",
        _ => category_default(categorize(output)),
    }
}

/// The per-category fallback flavor lines (CRAFTING_REVAMP.md generic set).
pub fn category_default(c: Category) -> &'static str {
    c.greeting()
}

/// Flavor for the detail panel's empty state: a selected category speaks
/// its own line.
pub fn flavor_for_or_greeting(c: Category) -> &'static str {
    c.greeting()
}

/// (output, ingredient ids) for every recipe the game knows — crafting,
/// smelting, alloying and crushing. Feeds the pickup-unlock rule without
/// dragging the whole UI catalog into the pickup path.
pub fn catalog_pairs() -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for r in lf_game::crafting::all_recipes() {
        let mut counts: Vec<(String, u8)> = Vec::new();
        for row in &r.pattern {
            for cell in row.iter().flatten() {
                match counts.iter_mut().find(|(id, _)| id == cell) {
                    Some(e) => e.1 += 1,
                    None => counts.push((cell.to_string(), 1)),
                }
            }
        }
        out.push((
            r.output.clone(),
            counts.into_iter().map(|(id, _)| id).collect(),
        ));
    }
    for (input, output) in lf_game::smelting::smelt_entries() {
        out.push((output.to_string(), vec![input]));
    }
    for (a, _an, b, _bn, out_id, _on) in lf_game::machines::alloy_recipes() {
        out.push((out_id.to_string(), vec![a.to_string(), b.to_string()]));
    }
    for (input, output, _n) in lf_game::machines::crush_entries() {
        out.push((output.to_string(), vec![input.to_string()]));
    }
    out
}

// ------------------------------------------------------------------
// Recipe visibility

/// The player's recipe book, persisted with the world. Rather than
/// materializing every recipe id, it remembers the ITEMS the player has
/// ever held; visibility derives from that plus the era rules — the same
/// behavior with less state to keep in sync (recipes from mods unlock for
/// free).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RecipeBook {
    /// Item ids the player has picked up at least once.
    pub seen_items: HashSet<String>,
}

/// Outputs every player knows from the first minute: the basic survival
/// set (CRAFTING_REVAMP.md "always visible").
pub const ALWAYS_VISIBLE: &[&str] = &[
    "planks", "stick", "torch", "crafting_table", "chest", "furnace",
    "wooden_pickaxe", "wooden_axe", "wooden_shovel", "wooden_sword",
    "stone_pickaxe", "stone_axe", "stone_shovel", "stone_sword",
    "apple", "porkchop", "mutton", "iron_ingot", "glass",
];

impl RecipeBook {
    /// Recipes the player can SEE (not necessarily craft) in this set.
    pub fn is_visible(&self, output: &str, ingredients: &[String], era: Era) -> bool {
        if ALWAYS_VISIBLE.contains(&output) {
            return true;
        }
        // era-tagged recipes surface when the era is reached. Untagged
        // (Primitive) recipes rely on the pickup rule — otherwise the
        // default era would reveal everything and visibility would mean
        // nothing.
        let req = Era::required_for(output);
        if req != Era::Primitive && req <= era {
            return true;
        }
        // picking up any listed ingredient reveals what it can become
        ingredients.iter().any(|i| self.seen_items.contains(i))
    }

    /// Record a first pickup; returns how many NOT-yet-visible recipe
    /// outputs this item is a key ingredient of (drives the HUD toast).
    pub fn unlock_on_pickup(&mut self, item: &str, catalog: &[(String, Vec<String>)], era: Era) -> usize {
        if self.seen_items.contains(item) {
            return 0;
        }
        self.seen_items.insert(item.to_string());
        catalog
            .iter()
            .filter(|(out, ings)| {
                if ALWAYS_VISIBLE.contains(&out.as_str()) {
                    return false;
                }
                let req = Era::required_for(out);
                // only pickup-driven recipes count: era-tagged ones wait
                // for the era, not the pickup
                let pickup_gated = req == Era::Primitive || req > era;
                pickup_gated && ings.iter().any(|i| i == item)
            })
            .count()
    }
}

// ------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_cover_the_catalog() {
        // known outputs land in the right bucket by kind or table
        assert_eq!(categorize("iron_pickaxe"), Category::Tools);
        assert_eq!(categorize("apple"), Category::Food);
        assert_eq!(categorize("bronze_chestplate"), Category::Armor);
        assert_eq!(categorize("steam_engine"), Category::Machines);
        assert_eq!(categorize("enchanting_table"), Category::Magic);
        assert_eq!(categorize("carved_oak"), Category::Deco);
        assert_eq!(categorize("planks"), Category::Materials);
        assert_eq!(categorize("stone_stairs"), Category::Building);
        // unknowns still resolve (nothing crashes the sidebar)
        assert_eq!(categorize("some_mod_thing"), Category::Materials);
    }

    #[test]
    fn flavor_exists_for_every_input() {
        for c in CATEGORIES {
            assert!(!c.greeting().is_empty());
        }
        assert!(flavor_for("iron_ingot").contains("Ironborn"));
        assert!(!flavor_for("totally_unknown_item").is_empty());
    }

    /// The core F3 behavior: basic recipes are visible immediately, era
    /// recipes wait for the era, the rest wait for a first pickup.
    #[test]
    fn recipe_visibility_earns_itself() {
        let mut book = RecipeBook::default();
        let primitive = Era::Primitive;
        // basic survival set: visible from minute one
        assert!(book.is_visible("planks", &["log".to_string()], primitive));
        assert!(book.is_visible("stone_pickaxe", &["stone".to_string(), "stick".to_string()], primitive));
        assert!(book.is_visible("furnace", &["stone".to_string()], primitive));
        // the lantern (iron + torch) is hidden until iron is ever picked up
        let lantern: Vec<String> = vec!["iron_ingot".into(), "torch".into()];
        assert!(!book.is_visible("lantern", &lantern, primitive), "locked recipes stay hidden");
        let unlocked = book.unlock_on_pickup(
            "iron_ingot",
            &[("lantern".to_string(), vec!["iron_ingot".to_string(), "torch".to_string()])],
            primitive,
        );
        assert_eq!(unlocked, 1, "first pickup unlocks exactly the lantern");
        assert!(book.is_visible("lantern", &lantern, primitive));
        // second pickup does not re-unlock
        let again = book.unlock_on_pickup(
            "iron_ingot",
            &[("lantern".to_string(), vec!["iron_ingot".to_string(), "torch".to_string()])],
            primitive,
        );
        assert_eq!(again, 0);
        // era-gated recipes stay hidden until the era arrives, even when
        // their ingredients are mundane and never picked up
        let battery: Vec<String> = vec!["never_seen_a".into(), "never_seen_b".into()];
        assert!(!book.is_visible("battery", &battery, primitive));
        assert!(book.is_visible("battery", &battery, Era::required_for("battery")));
        // the always-visible set never hides
        assert!(book.is_visible("torch", &["coal".to_string(), "stick".to_string()], primitive));
    }
}
