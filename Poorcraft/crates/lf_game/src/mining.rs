//! Mining rules: hardness per block, tool multipliers, harvest gating and
//! time-to-break computation.

use crate::items::{ItemKind, ToolKind, item_def, tier_speed};
use crate::survival::ItemStack;

/// Base hardness in seconds-by-hand (Minecraft-like scale).
pub fn hardness(block_id: u32) -> f32 {
    use lf_voxel::registry::block;
    match block_id {
        block::DIRT | block::GRASS | block::SAND | block::SNOW | block::MYCELIUM => 0.5,
        block::LEAVES => 0.2,
        block::LOG => 2.0,
        block::STONE => 1.5,
        block::COAL_ORE => 3.0,
        block::IRON_ORE => 3.0,
        block::CRAFTING_TABLE => 2.5,
        block::TORCH => 0.05,
        block::WATER | block::AIR => 0.0,
        _ => 1.5,
    }
}

/// Minimum tool tier required to harvest (drops). 255 = unbreakable.
/// None = harvestable by hand.
pub fn required_tool(block_id: u32) -> Option<(ToolKind, u8)> {
    use lf_voxel::registry::block;
    match block_id {
        block::STONE => Some((ToolKind::Pickaxe, 0)),
        block::COAL_ORE => Some((ToolKind::Pickaxe, 0)),
        block::IRON_ORE => Some((ToolKind::Pickaxe, 1)), // needs stone pick
        _ => None,
    }
}

/// Seconds to break `block_id` with the held item. `None` = unbreakable.
pub fn break_time(block_id: u32, held: Option<&ItemStack>) -> Option<f32> {
    use lf_voxel::registry::block;
    if block_id == block::AIR || block_id == block::WATER {
        return None;
    }
    let h = hardness(block_id);
    let mut speed = 1.0;
    if let Some(stack) = held {
        if let Some(def) = item_def(&stack.item_id) {
            if let ItemKind::Tool(kind, _) = def.kind {
                if kind.effective_on().contains(&block_id) {
                    speed = tier_speed(match def.kind {
                        ItemKind::Tool(_, t) => t,
                        _ => 0,
                    });
                }
            }
        }
    }
    // Minecraft formula shape: hardness * 1.5 when proper tool, * 5 without
    // a required one (still breakable, just slow / no drops handled outside).
    let penalty = if required_tool(block_id).is_some() && !tool_satisfies(block_id, held) { 3.33 } else { 1.0 };
    Some((h * 1.5 / speed * penalty).max(0.05))
}

/// Does the held tool meet the harvest requirement for this block?
pub fn tool_satisfies(block_id: u32, held: Option<&ItemStack>) -> bool {
    match required_tool(block_id) {
        None => true,
        Some((kind, min_tier)) => {
            match held.and_then(|s| item_def(&s.item_id)) {
                Some(def) => match def.kind {
                    ItemKind::Tool(k, tier) => k == kind && tier >= min_tier,
                    _ => false,
                },
                None => false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lf_voxel::registry::block;

    fn stack(id: &str) -> ItemStack {
        ItemStack { item_id: id.to_string(), count: 1 }
    }

    #[test]
    fn dirt_is_fast_iron_ore_needs_stone_pick() {
        let hand = None;
        assert!(break_time(block::DIRT, hand).unwrap() < 1.0);
        // iron ore by hand: slow and no harvest
        assert!(!tool_satisfies(block::IRON_ORE, hand));
        let stone_pick = stack("stone_pickaxe");
        assert!(tool_satisfies(block::IRON_ORE, Some(&stone_pick)));
        let wood_pick = stack("wooden_pickaxe");
        assert!(!tool_satisfies(block::IRON_ORE, Some(&wood_pick)));
    }

    #[test]
    fn proper_tools_are_faster() {
        let hand = break_time(block::STONE, None).unwrap();
        let wood = break_time(block::STONE, Some(&stack("wooden_pickaxe"))).unwrap();
        let stone = break_time(block::STONE, Some(&stack("stone_pickaxe"))).unwrap();
        assert!(wood < hand, "wood pick should beat hand: {wood} vs {hand}");
        assert!(stone < wood, "stone pick should beat wood: {stone} vs {wood}");
    }

    #[test]
    fn axe_speeds_logs_shovel_speeds_dirt() {
        let hand = break_time(block::LOG, None).unwrap();
        let axe = break_time(block::LOG, Some(&stack("wooden_axe"))).unwrap();
        assert!(axe < hand);
        let hand_dirt = break_time(block::GRASS, None).unwrap();
        let shovel = break_time(block::GRASS, Some(&stack("wooden_shovel"))).unwrap();
        assert!(shovel < hand_dirt);
    }

    #[test]
    fn torch_breaks_instantly() {
        assert!(break_time(block::TORCH, None).unwrap() < 0.1);
    }
}
