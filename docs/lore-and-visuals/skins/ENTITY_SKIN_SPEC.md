# Entity Skin Specification

## How entity skins work in this codebase

Entity rendering is in `lf_engine`. The existing mobs (Geode Guardian,
Cinder Crawler, Null Knight, villagers) use a simple box-model renderer
with a flat texture skin applied per model region. Before adding any new
skins, read the existing entity rendering code and confirm the skin format
(likely a texture that's partitioned into face regions — top, bottom,
front, back, left, right of each body part). New skins must match the
same format.

## Villager faction skins — implementation approach

The simplest approach: add a `faction_skin_id` field to the villager data
struct, look up the appropriate skin texture by that ID at render time,
and substitute it for the default villager skin. No new geometry — only
a texture swap.

Skin selection priority at render time:
1. If the villager has a `faction_skin_id` → use `villager_<faction>.png`
2. Otherwise → use the existing default villager skin

The faction of a worldgen-placed villager NPC is set in their archetype
data (NPC roster file); the skin follows from that.

## Companion skins — trust badge overlay

The trust badge (visible at trust ≥ 50) can be implemented as either:
- A second sprite/quad rendered on the companion's chest, OR
- A second texture variant per companion archetype (`companion_*_trusted.png`)
  that is swapped in when trust crosses 50.

The second approach (separate texture variant) is simpler to implement
and avoids a multi-draw for each companion. Use that approach unless the
rendering architecture makes a texture overlay significantly easier.

Trust badge visual: a small circular badge texture region, 4×4 pixels of
the 16×16 skin, centered on the chest front face. The badge should be
distinctly warm-colored (amber or gold) against whatever the companion's
clothing color is — it should be readable at a glance.

## Mob silhouette differentiation — design rules

A player should be able to identify any mob type at 20+ blocks distance
by silhouette alone, even if they can't read the texture detail. The
current 6 mobs should satisfy these rules:

1. **Height**: Null Knight is the tallest (~2.5 blocks visual height).
   Common wander mob is shorter (~1.5 blocks). Cinder Crawler is the
   lowest (~0.7 blocks, wider than tall).
2. **Width**: Cinder Crawler is the widest relative to height.
   Geode Guardian is large and roughly cubic with angular protrusions.
3. **Posture**: Chase mob is upright and forward-leaning (aggressive
   stance implied by model or animation offset). Ranged mob is crouched.
   Wander mob is slightly hunched.

If the current models don't satisfy these rules, the body dimensions need
adjustment, not just the texture.

## Biome tint variants — implementation approach

Biome tint variants are **not separate model instances**. They are applied
at render time by passing the current biome's tint palette to the entity
shader, which applies a palette-shift transformation to the entity's base
skin colors.

The tint should only affect "clothing/body" color regions, not the
entity's eyes or any glow effect (e.g., Cinder Crawler's ember glow
should stay ember-orange regardless of biome tint — it's a heat property,
not a skin property).

Implementation: add a per-biome `mob_tint: [f32; 3]` (RGB shift, additive)
to the biome table in `lf_worldgen/src/biome.rs`, default [0,0,0] for
neutral. Apply it in the entity rendering shader. Three non-zero examples:
- Desert: [+0.1, +0.04, –0.05] (warm sandy shift)
- Snow: [–0.05, –0.02, +0.12] (cool blue shift)
- Swamp: [–0.04, +0.06, –0.04] (muddy green shift)

Tune the specific values per visual playtest — these are starting points,
not final values.

## Named NPC unique skins

The Unmarked and Archivist Maren Voss (the two named NPCs from the NPC
roster) each need a unique skin, not just a variant of the generic
faction skin. Their uniqueness should be readable in a single detail
per character:

- **The Unmarked**: ash-grey hair region in the face/head texture (no
  other villager has this); no visible faction symbol anywhere on the skin.
- **Archivist Maren Voss**: a visible journal texture detail on the side
  of their body (as if tucked under an arm), and slightly more formal
  robe geometry (if the model supports it) or robes with an additional
  decorative hem pattern at the bottom.

These are 16×16 skins like all others — the "uniqueness" is in how that
space is used, not in different geometry.
