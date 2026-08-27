# Skin & Texture Manifest

Complete list of every new texture asset this design pack requires.
Each entry has: asset path (where it lives in `lf_assets`), dimensions,
style notes, and which code files register it.

All textures are **16×16 pixel art** unless otherwise noted. All block
textures must tile seamlessly (edge-continuous) and repeat correctly per
block unit (see `BLOCK_SKIN_SPEC.md` for the UV tiling rules).

---

## Block textures (register in `lf_voxel/src/registry.rs` + `lf_assets`)

### Faction-themed blocks (12 new)

| Texture name | Style notes |
|---|---|
| `accord_stone.png` | Smooth mid-grey stone with faint geometric inlay lines (corner-to-corner thin groove, like architectural stone). Neutral, formal. |
| `accord_pillar.png` | Tall carved column face — vertical fluted groove lines, slightly lighter than accord_stone at the edges. Should read as a column when stacked vertically. |
| `ironborn_brick.png` | Dark warm-brown brick with iron-flecked mortar lines. Brick pattern offset every other row. Gritty, industrial. |
| `ironborn_grate.png` | Metal grate — dark iron with a grid of circular holes. Semi-transparent (uses the glass transparent pass). Warm dark metallic base. |
| `ember_covenantwood.png` | Very dark timber planks (almost charcoal) with small carved runic symbols in the grain — subtle, not garish. Warm undertone. |
| `ember_glowstone.png` | Amber-orange self-illuminating stone. Warm glow-flecked surface, like heated stone. Should not be neon — muted amber, not electric. Emits light level 8. |
| `freeholds_thatch.png` | Woven straw/thatch. Warm tan-yellow. Diagonal weave pattern visible. Rough texture variation across the tile. |
| `freeholds_daub.png` | Pale off-white plaster. Some variation in tone (slightly streaky, slightly textured) — not perfectly flat. |
| `ashen_marble.png` | Pale grey polished stone with subtle dark veining. Clean, refined. The veins should be thin lines, not blobs. |
| `ashen_bookshelf.png` | Books packed horizontally on a shelf. Multiple spine colors (grey, off-white, dark blue). Same pattern as a standard bookshelf tile but in the Ashen palette. |
| `nameless_rotwood.png` | Decaying dark wood. Split-grain lines, grey-brown with darker rot patches. Should look abandoned. |
| `nameless_scorched.png` | Charred/scorched stone. Black-grey with lighter ash-grey highlights at edges. Some orange tinge visible at cracks. |

### Environmental/biome-exclusive blocks (8 new)

| Texture name | Style notes |
|---|---|
| `mushroom_cap.png` | Red with white-spot polka dots (classic mushroom cap pattern). Clean, bright red, white spots clearly distinct. |
| `coral_block.png` | Warm pink-orange with irregular coral branch texture. Organic, not geometric. |
| `permafrost.png` | Dark bluish soil with visible ice crystal flecks. Cold colour palette — dark grey-blue base. |
| `volcanic_basalt.png` | Very dark grey-black rough stone. Heat-crack lines glowing faintly orange-red at the cracks. Not fully glowing — just the crack lines. |
| `deep_slate.png` | Very dark blue-grey smooth stone. Almost black but with a distinct blue tint. Clean, minimal texture detail. |
| `mesa_terracotta.png` | Warm orange-red layered terracotta. Visible horizontal banding — slightly different values per layer band across the tile. |
| `gilded_grass.png` | Golden-tinted dry grass. Yellow-green with dry straw highlights. Top face only — side face uses standard dirt/soil. |
| `bog_peat.png` | Very dark, wet-looking dark brown-black soil. Some small root-like markings. Not grass — pure peat surface. |

### Decoration blocks (new)

| Texture name | Style notes |
|---|---|
| `carved_oak.png` | Oak wood with carved decorative groove pattern. Warmer than standard oak planks. |
| `carved_stone.png` | Standard stone with carved geometric relief. Mid-grey, more refined than cobblestone. |
| `carved_iron.png` | Iron-metal face with hammered/carved relief pattern. Industrial ornamental. |
| `stained_glass_red.png` | Translucent red — warm crimson tint, same geometry as glass block. |
| `stained_glass_orange.png` | Translucent orange-amber. |
| `stained_glass_yellow.png` | Translucent yellow. |
| `stained_glass_green.png` | Translucent green. |
| `stained_glass_blue.png` | Translucent blue. |
| `stained_glass_purple.png` | Translucent purple. |
| `stained_glass_black.png` | Very dark translucent — almost opaque but with slight light pass-through. |
| `stained_glass_white.png` | Frosted/milky white translucent. |
| `banner_accord.png` | #4a7ab5 blue-grey background, white scale symbol. Flat quad rendering (sign-style). |
| `banner_ironborn.png` | #8b4513 iron-brown, white hammer-anvil symbol. |
| `banner_covenant.png` | #c4602a ember-orange, dark flame-in-circle symbol. |
| `banner_freeholds.png` | #6b8e23 earthy green, white wheat stalks. |
| `banner_ashen.png` | #b0b0b0 pale grey, dark open book symbol. |
| `banner_nameless.png` | #2d2d2d near-black, grey broken-chain symbol. |
| `lantern_hanging.png` | Same as the existing lantern texture (reuse) — the difference is placement logic (ceiling/chain mount), not art. |

---

## Entity/NPC skins (register in `lf_engine`'s entity rendering)

### Villager faction variants (6 skins)

All villager faction skins use the existing villager body geometry.
The distinction is clothing color and a small faction symbol visible
on the chest/back:

| Skin name | Base color | Accent | Symbol detail |
|---|---|---|---|
| `villager_accord.png` | Blue-grey tunic | White trim | Small scale on chest |
| `villager_ironborn.png` | Iron-brown leather | Dark seams | Hammer on back |
| `villager_covenant.png` | Dark ember-wood robe | Orange trim | Flame circle on chest |
| `villager_freeholds.png` | Earthy green work clothes | Tan trim | Wheat stalks on sleeve |
| `villager_ashen.png` | Pale grey robes | Off-white trim | Open book on chest |
| `villager_nameless.png` | Near-black patched clothing | No trim | Broken chain on back |

### Companion skins (6 skins)

Distinct from generic villagers — more detailed/individualized.
Each should have a visible trust-indicator detail (a pin/badge on the
chest) that is added at trust ≥ 50. Implement as a second sprite layer
or a toggle texture overlay:

| Skin name | Description |
|---|---|
| `companion_accord_warden.png` | Accord blue, light armor, sword-and-shield silhouette hint in texture. Slightly more armored than a generic villager. |
| `companion_ironborn_artisan.png` | Ironborn brown leather apron, visible tool belt at waist. |
| `companion_covenant_channeler.png` | Dark ember robe with orange glow-trim at cuffs. Slender silhouette. |
| `companion_freeholds_scout.png` | Earthy green practical clothes, hood/cap. Lighter build. |
| `companion_ashen_scribe.png` | Pale grey robes, visibly carrying a journal (texture detail). Tall, slightly stooped posture (if supported by the model). |
| `companion_nameless_rover.png` | Near-black patched clothes. Thin, no faction symbol. Slightly furtive visual posture. |

### Mob refresh (6 existing mobs)

Each existing mob type must have a clearly distinct silhouette and
palette. Audit which two currently look too similar and differentiate:

| Mob | Color palette | Silhouette note |
|---|---|---|
| Standard wander mob | Medium earth-brown | Hunched, stocky |
| Chase/aggressive mob | Dark red-brown | Upright, slightly taller |
| Ranged mob | Dusty grey | Thin, crouched |
| Geode Guardian | Crystal blue-white | Large, angular — crystal facets on body |
| Cinder Crawler | Ember-orange and charcoal | Low to ground, crab-like silhouette |
| Null Knight | Near-black with grey void-glow at joints | Tall, armored, imposing |

### Biome-tinted mob variants (palette swaps, 3 variants per common mob)

Apply to the three "common" mob types (wander, chase, ranged) only —
not the unique bosses:

| Variant | Palette shift |
|---|---|
| Desert variant | Sandy yellow-tan base, less saturation |
| Snow variant | Blue-white tinted, slight shimmer at edges |
| Swamp variant | Muddy green-grey, slightly darker overall |

---

## UI skin assets

| Asset | Description |
|---|---|
| `ui_faction_scale_icon.png` | 8×8 or 16×16 small icon for the Accord (scale) |
| `ui_faction_hammer_icon.png` | 8×8 or 16×16 small icon for the Ironborn (hammer-anvil) |
| `ui_faction_flame_icon.png` | Small Covenant icon |
| `ui_faction_wheat_icon.png` | Small Free Holds icon |
| `ui_faction_book_icon.png` | Small Ashen Order icon |
| `ui_faction_chain_icon.png` | Small Nameless icon |
| `ui_trust_badge.png` | The companion trust-level badge shown on companion HUD tile (small, neutral symbol) |
