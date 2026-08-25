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
    static RECIPES: &[Recipe] = &[];
    // Building a static table with owned Strings is awkward; return a leaked
    // vec built once.
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
    // glass-tier pickaxe doesn't exist; done
    let _ = RECIPES;
    Box::leak(book.into_boxed_slice())
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
    for recipe in recipes() {
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
    fn torches_from_coal_and_stick() {
        let mut grid = vec![None::<ItemStack>; 4];
        grid[0] = s("coal");
        grid[2] = s("stick");
        assert_eq!(match_recipe(&grid), Some(("torch".into(), 4)));
    }
}
