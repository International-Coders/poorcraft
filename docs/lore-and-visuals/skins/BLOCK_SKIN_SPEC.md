# Block Skin Specification — UV Tiling & Biome Color Grade

## The tiling rule (fix if not yet fixed)

Every block face UV must cover exactly 0.0–1.0 in both axes for a
**single block face**, regardless of how wide or tall the rendered
geometry is. When faces are merged (greedy meshing) or when a multi-block
structure is built from many blocks, each block's face still gets its own
0.0–1.0 UV range — the texture sampler is set to REPEAT so it tiles across
the merged surface naturally.

**Wrong (stretch):** a 4-wide merged quad gets UVs (0.0, 0.0) to (4.0, 1.0)
mapped to (0.0, 0.0) to (1.0, 1.0) in the shader → one texture stretched.

**Correct (tile):** a 4-wide merged quad gets UVs (0.0, 0.0) to (4.0, 1.0)
with the texture sampler set to AddressMode::Repeat → four copies of the
texture tiled naturally.

The fix location: `lf_voxel`'s meshing code, specifically wherever UV
coordinates are assigned to face geometry. Check both the standard per-face
mesher and any greedy-meshing path.

## Face direction rules (all blocks)

Blocks with a distinct top/side/bottom face (dirt, grass, logs, etc.):
- **Top face:** uses the texture's "top" atlas layer.
- **Bottom face:** uses the "bottom" atlas layer if distinct; otherwise
  same as top.
- **Side faces (N/S/E/W):** all four use the "side" atlas layer.

Blocks with a uniform face (stone, accord_stone, iron_ore, etc.):
- All 6 faces use the same atlas layer.

The texture atlas layering in `lf_assets::texture_index_for_block`
already handles this — new blocks should follow the same three-layer
(top/side/bottom) or single-layer pattern as existing blocks.

## New blocks with directional faces

| Block | Top | Side | Bottom |
|---|---|---|---|
| `gilded_grass` | `gilded_grass.png` (golden grass blades) | Standard dirt side | Standard dirt |
| `ember_covenantwood` | `ember_covenantwood.png` (end-grain with rune) | `ember_covenantwood.png` (plank face with rune) | Same as top |
| `ashen_bookshelf` | `ashen_marble.png` | `ashen_bookshelf.png` | `ashen_marble.png` |
| `accord_pillar` | `accord_stone.png` | `accord_pillar.png` | `accord_stone.png` |

All other new blocks are single-texture (all 6 faces use the same texture).

## Biome color grade integration

The biome color-grade post-process pass (in `lf_engine`) affects how ALL
block textures appear on screen — it is a full-frame effect, not a
per-block-texture modification. No block texture itself should be pre-
graded for a specific biome. The grading is handled entirely by the
post-process layer.

This means: every block texture should be designed for the neutral
(temperate) biome's color environment. The post-process grade will shift
it appropriately in desert/snow/swamp/etc. biomes.

**Exception — ember_glowstone:** its emission color (the light it emits)
should NOT be affected by the color grade, since emitted light is a source
property, not a surface property. The engine should apply the grade after
light accumulation, or exclude the emission term from the grade pass.

## Faction block registration checklist (for each new faction block)

For each new block in the skin manifest, ensure all of the following are
done in the same commit (the existing catalog consistency test will catch
any dangling reference):

1. Add texture constant to `lf_assets/src/lib.rs`
2. Add block ID constant to `lf_voxel/src/registry.rs` (ID ≥ 100 for
   blocks introduced as "built-in mods," matching the existing pattern)
3. Add solidity/opacity/light rules to the registry
4. Add drop item to `lf_game/src/items.rs` (what the block drops on break)
5. Add crafting recipe to `lf_game/src/items.rs` or the relevant recipe
   TOML file
6. For light-emitting blocks (ember_glowstone): add emitter entry in the
   light propagation table in `lf_voxel`'s light module
7. For transparent blocks (ironborn_grate, stained_glass variants): add
   to the transparent-pass block list in `lf_engine`'s render pipeline,
   same pattern as existing glass blocks

## Naming convention for new block IDs

Follow the existing pattern in `registry.rs`. Faction blocks:
`ACCORD_STONE`, `ACCORD_PILLAR`, `IRONBORN_BRICK`, etc. Biome blocks:
`MUSHROOM_CAP`, `CORAL_BLOCK`, `PERMAFROST`, etc. Decoration blocks:
`CARVED_OAK`, `CARVED_STONE`, `STAINED_GLASS_RED`, `BANNER_ACCORD`, etc.
