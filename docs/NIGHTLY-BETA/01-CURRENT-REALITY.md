# Current Reality: What Exists and What Still Reads as Alpha

This baseline was taken on 2026-09-03 at loop 348. Re-audit it at the start of
execution; never treat counts or file sizes as timeless facts.

## Proven foundation

- Workspace state reports 406 passing tests, 94 passing vistest proofs, a
  green smoke test, and warm terrain proof p50 of 53.7 ms.
- The renderer, voxel streaming, persistence, survival, mining, placement,
  inventory, recipes, machines, magic, combat, multiplayer foundations,
  weather, maps, factions, NPC schedules, companions, and mod loading are real
  code rather than design-only claims.
- Thirty biome variants, multi-channel seeded noise, rivers, caves, ores,
  faction structures, and one deterministic kingdom citadel per large region
  already exist.
- Six original factions already have standing data and art: Accord, Ironborn,
  Ember Covenant, Free Holds, Ashen Order, and Nameless.
- The item catalog proof currently shows 164 items; a procedural atlas, eight
  connected-texture strips, and six faction NPC skins are tracked assets.
- The HUD, inventory, workbench, map, journal, technology tree, faction HUD,
  kingdom compass, castle, NPC locomotion, and biome scenes all have visual
  proofs. Existing proofs are starting points, not automatic beta passes.

## Visual audit: concrete failures to solve

The following observations came from actually opening the current proof PNGs:

- `vistest_hud_preview.png`: functional and compact, but status language is
  visually thin, the default state provides little first-minute guidance,
  and important transient feedback competes at the bottom edge.
- `vistest_crafting_workbench.png`: the workbench is transparent over a busy
  world; the underlying HUD and inventory remain visible; ingredient,
  quantity, queue, and primary-action hierarchy lack modal clarity. This is a
  usability failure, not merely a palette preference.
- `vistest_kingdom_citadel.png`: the citadel is readable but generic and sits
  on a severe artificial block pedestal. It lacks a believable road/terrain
  transition, district identity, surrounding life, and faction-specific
  architecture.
- `vistest_npc_walkers.png`: locomotion exists, but the scene looks staged and
  sparse; characters need clearer roles, activities, navigation intent,
  reaction readability, and better integration with buildings.
- `vistest_biome_contact_sheet.png`: much of the visible frame is similar
  bright-green terrain with repeated gray plants. A proof named "contact
  sheet" does not yet communicate thirty unmistakable biome identities.
- `vistest_asset_catalog.png`: the breadth is good, but silhouettes, palettes,
  and finish are inconsistent and several icons resemble neutral placeholders.

## Code and design gaps relevant to this pack

- Standing is largely a per-faction number. The game needs witnessed events,
  moral categories, rumor propagation, memory, fear/respect, and context so
  factions do not react with global omniscience.
- NPCs can walk, hop, descend, patrol, flee, remember a few interactions, and
  react with lines. Full grid navigation, doors, crowd negotiation, work
  completion, home needs, and robust recovery remain incomplete.
- Existing faction structures are small set pieces. The kingdom citadel is a
  first grammar, not eight distinct living castles with economies, garrisons,
  paths, and strategic consequences.
- Same-seed determinism is tested in several places, but "different seeds feel
  like different worlds" needs statistical and rendered evidence across a
  seed corpus, not a single two-panel screenshot.
- The beta cannot depend on dozens of unverified generated assets. Every
  visual file needs an owner, consumer, license/source declaration, quality
  gate, and proof scene.

## Disk and repository baseline

Measured working-directory categories:

| Path | Approximate size | Meaning |
|---|---:|---|
| `target/` | 23 GB | Ignored Cargo build output; rebuildable, main size cause |
| `.git/` | 858 MB | Repository history/objects; never delete or rewrite casually |
| `shots/` | 64 MB | 158 tracked proof/manual PNGs; audit before pruning |
| `dist/` | 48 MB | Ignored runtimes; replace only with fresh verified artifacts |
| `assets/` | 2.5 MB | Small; tracked art plus current untracked audio work |
| `crates/` | 2.4 MB | Game source; not the size problem |
| `mods/` | 860 KB | Example content; not the size problem |
| `docs/` | 656 KB | Plans and specifications; not the size problem |
| `worlds/` | 364 KB | Ignored player saves; sacred, never clean as test output |

At audit time there was unrelated audio work in progress in
`crates/lf_audio/src/lib.rs`, `assets/sounds/`, and `tools/gen_sounds.py`.
Agents must inspect current status and preserve any unrelated dirty work.

## Correct interpretation

The game is not empty and should not be rewritten. It is a wide, impressive
alpha whose next risk is shallow integration: many systems exist once, but
their interaction, presentation, variation, and failure behavior are not yet
beta-grade. This pack therefore prioritizes depth, coherence, and proof.
