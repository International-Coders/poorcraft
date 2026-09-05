//! P3D-506 continued: the crafting system — recipes that combine
//! materials into new items, deterministic and UI-independent.
//!
//! A [`Recipe`] maps ingredient item codes + counts to one output item.
//! `craft` atomically checks the inventory, consumes inputs, and adds
//! the output — returning the output count. All recipes live in the
//! static [`RECIPES`] table; adding new recipes is append-only.

use crate::items::{item_kind, item_name, Inventory, ItemId, ItemKind};

/// A crafting recipe: inputs → output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recipe {
    /// Stable recipe code (for save/persistence).
    pub code: u16,
    /// (item, count) pairs required.
    pub ingredients: &'static [(u16, u32)],
    /// The output item.
    pub output: u16,
    /// How many of the output item per craft.
    pub output_count: u32,
}

/// All known recipes, in code order. Adding new recipes is append-only.
pub const RECIPES: &[Recipe] = &[
    Recipe { code: 1, ingredients: &[(1, 3), (2, 2)], output: 10, output_count: 1 }, // wood×3 + stone×2 → stone_pick
    Recipe { code: 2, ingredients: &[(2, 5), (1, 2)], output: 11, output_count: 1 }, // stone×5 + wood×2 → iron_pick
    Recipe { code: 3, ingredients: &[(5, 3)], output: 20, output_count: 1 },         // soil×3 → bread
    Recipe { code: 4, ingredients: &[(3, 2)], output: 4, output_count: 1 },           // sand×2 → snow (glass-smelting stand-in)
    Recipe { code: 5, ingredients: &[(1, 1), (5, 2)], output: 5, output_count: 2 },  // wood×1 + soil×2 → soil×2 (compost)
];

/// Find a recipe by code.
pub fn recipe_by_code(code: u16) -> Option<&'static Recipe> {
    RECIPES.iter().find(|r| r.code == code)
}

/// Find a recipe by its output item.
pub fn recipe_for_output(output: u16) -> Option<&'static Recipe> {
    RECIPES.iter().find(|r| r.output == output)
}

/// Can the inventory afford this recipe?
pub fn can_craft(inv: &Inventory, recipe: &Recipe) -> bool {
    recipe
        .ingredients
        .iter()
        .all(|&(item, count)| inv.count(ItemId(item)) >= count)
}

/// Atomically craft: consume ingredients, add output. Returns the output
/// count, or None if the inventory lacks any ingredient (inventory is
/// UNTOUCHED on failure — the P3D-501 atomicity law).
pub fn craft(inv: &mut Inventory, recipe: &Recipe) -> Option<u32> {
    if !can_craft(inv, recipe) {
        return None;
    }
    for &(item, count) in recipe.ingredients {
        let removed = inv.remove(ItemId(item), count);
        debug_assert_eq!(removed, count, "consume after verify must be exact");
    }
    let leftover = inv.add(ItemId(recipe.output), recipe.output_count);
    debug_assert_eq!(leftover, 0, "output must fit after ingredient removal");
    Some(recipe.output_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Craft a stone_pick: consumes exactly 3 wood + 2 stone, produces
    /// 1 stone_pick, and the inventory reflects the transaction.
    #[test]
    fn p3d506_craft_stone_pick() {
        let mut inv = Inventory::new(8);
        inv.add(ItemId(1), 3); // wood
        inv.add(ItemId(2), 2); // stone
        let recipe = recipe_by_code(1).expect("recipe 1 exists");
        assert_eq!(craft(&mut inv, recipe), Some(1));
        assert_eq!(inv.count(ItemId(1)), 0, "wood consumed");
        assert_eq!(inv.count(ItemId(2)), 0, "stone consumed");
        assert_eq!(inv.count(ItemId(10)), 1, "stone_pick produced");
    }

    /// Crafting with insufficient ingredients returns None and leaves
    /// the inventory UNTOUCHED (atomicity law).
    #[test]
    fn p3d506_craft_refuses_insufficient_ingredients() {
        let mut inv = Inventory::new(8);
        inv.add(ItemId(1), 2); // need 3 wood, only have 2
        inv.add(ItemId(2), 2); // enough stone
        let before = inv.clone();
        let recipe = recipe_by_code(1).expect("recipe 1 exists");
        assert_eq!(craft(&mut inv, recipe), None);
        assert_eq!(inv, before, "failed craft must not touch the inventory");
    }

    /// All recipes are well-formed: known items, positive counts,
    /// unique codes, unique outputs (one recipe per output item).
    #[test]
    fn p3d506_recipes_are_well_formed() {
        let mut codes = std::collections::BTreeSet::new();
        let mut outputs = std::collections::BTreeSet::new();
        for r in RECIPES {
            assert!(codes.insert(r.code), "duplicate recipe code {}", r.code);
            assert!(!outputs.contains(&r.output), "duplicate output {}", r.output);
            outputs.insert(r.output);
            assert!(r.output_count > 0);
            assert!(!r.ingredients.is_empty());
            for &(item, count) in r.ingredients {
                assert!(count > 0, "zero-count ingredient in recipe {}", r.code);
                // Item must exist in the catalog (item_name panics on unknown).
                let _ = item_name(ItemId(item));
            }
            // Output must exist in the catalog too.
            let _ = item_name(ItemId(r.output));
        }
    }

    /// The crafting loop: harvest materials, craft a pick, use the pick
    /// to harvest rock, craft an iron_pick — the full progression.
    #[test]
    fn p3d506_crafting_progression_loop() {
        let mut inv = Inventory::new(8);
        // Phase 1: gather wood + soil by hand, craft bread (food).
        inv.add(ItemId(5), 6); // soil×6
        let bread = recipe_by_code(3).unwrap();
        assert_eq!(craft(&mut inv, bread), Some(1));
        assert_eq!(inv.count(ItemId(20)), 1, "bread crafted");

        // Phase 2: gather enough for a stone_pick.
        inv.add(ItemId(1), 3); // wood×3
        inv.add(ItemId(2), 2); // stone×2
        let pick = recipe_by_code(1).unwrap();
        assert_eq!(craft(&mut inv, pick), Some(1));
        assert_eq!(inv.count(ItemId(10)), 1, "stone_pick crafted");

        // Phase 3: gather for iron_pick.
        inv.add(ItemId(2), 5); // stone×5
        inv.add(ItemId(1), 2); // wood×2
        let iron = recipe_by_code(2).unwrap();
        assert_eq!(craft(&mut inv, iron), Some(1));
        assert_eq!(inv.count(ItemId(11)), 1, "iron_pick crafted");
    }

    /// Craft results are deterministic: same inventory + recipe → same
    /// outcome every time.
    #[test]
    fn p3d506_crafting_is_deterministic() {
        let build = || {
            let mut inv = Inventory::new(8);
            inv.add(ItemId(1), 5);
            inv.add(ItemId(2), 3);
            inv
        };
        let recipe = recipe_by_code(1).unwrap();
        let mut a = build();
        let mut b = build();
        let ra = craft(&mut a, recipe);
        let rb = craft(&mut b, recipe);
        assert_eq!(ra, rb);
        assert_eq!(a, b, "same inputs diverged");
    }
}
