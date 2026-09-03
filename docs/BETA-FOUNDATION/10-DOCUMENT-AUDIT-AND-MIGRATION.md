# Document Audit and Migration

## Scope reviewed

At loop 356 the repository contains 89 Markdown files totaling 16,856 lines:

| Group | Files | Role after this audit |
|---|---:|---|
| repository root | 9 | operational state, history, decisions, release truth |
| `docs/NIGHTLY-BETA/` | 14 | active detailed acceptance reference |
| `docs/V1REBRAND/` | 12 | historical product/phase design |
| `docs/poorcraft-build-pack/` | 14 | historical 40-step execution pack |
| `docs/ui-world-craft/` | 9 | UI/worldgen/crafting reference |
| `docs/lore-and-visuals/` | 18 | canonical lore/faction/art reference |
| `docs/ai-npc-assets/` | 7 | earlier AI/NPC/asset implementation reference |
| other `docs/` files | 5 | master plan, assets, ideas, roadmap, Steam |
| `mods/README.md` | 1 | current mod authoring guide |

The full histories were reviewed by entry/heading and the relevant water,
castle, NPC, asset, engine, and multiplayer passages were checked against
current source. Large prompt files and completed logs are retained as evidence,
not reread requirements for every future job.

## Root documents

- `AGENTS.md`: binding workflow and safety law.
- `STATE.md`: current executable pointer and latest evidence; never a product
  vision substitute.
- `BACKLOG.md`: broad completed/deferred inventory with stale historical
  sections; check source before trusting old boxes.
- `CHANGELOG.md`: feature history; useful for locating when behavior changed.
- `DEVLOG.md`: command/evidence history; newest entries are the handoff source.
- `DECISIONS.md`: accepted technical constraints; add decisions when new
  persistent/network/format semantics land.
- `AUDIT.md`: valuable 2026-08-26 reality baseline, now partially superseded by
  later fixes.
- `STATUS.md` and `RELEASE.md`: stale counts and limitations in places; update
  only alongside real verified release work.

## Current pack

`docs/NIGHTLY-BETA/` remains the best existing detailed quality contract for
HUD, crafting, seeds, biomes, castle placement, NPC moral history, assets,
proofs, performance, data shapes, and handoff. N01–N07 are implemented history.
This Beta Foundation changes these later assumptions:

- water force is a beta pillar, not merely adjacency/flow-level polish;
- the authoritative simulation seam precedes deeper replicated systems;
- major capitals are much farther apart than the current 12×12-chunk lattice;
- six existing realms are the beta core; two new realms are deferred;
- castles require an authored modular 3D/voxel asset compiler and manifest;
- NPC purpose, perception, castle alarm, and follower recovery are one causal
  system;
- multiplayer beta includes fluids, machines, living entities, transactions,
  reconnect, and a real two-account Steam proof.

Use the new B01–B30 order in `08-BETA-DELIVERY-ROADMAP.md` when it conflicts
with old N08–N24 sequencing.

## Historical packs and retained value

### `docs/V1REBRAND/`

Retain its vision of craft-first physical progression, ordinary-hardware
support, bounded magic, capped nuclear technology, construction tools, paths,
and Steam release honesty. Its P28–P39 execution plan has largely shipped and
must not be restarted as a current queue.

### `docs/poorcraft-build-pack/`

Retain its reality-audit method, renderer/UX detail, proof discipline, power
tier intent, Steam/Workshop checklist, and full-loop release check. The
40-step plan is historical; many unchecked boxes are stale relative to later
commits.

### `docs/ui-world-craft/`

Retain the LOREFORGE UI language, progressive world-creation flow, seed rules,
terrain hierarchy, biome five-second test, modal workbench principles, and
anti-generated-looking polish rules. Current N01–N07 code supersedes several
implementation gaps described there.

### `docs/lore-and-visuals/`

Retain Valdenmoor history, cosmology, six faction identities, relationships,
quests, companion economy, dialogue framework, skin rules, and visual polish.
Treat `lore/*.toml` plus loader tests as runtime canonical data. Replace the
old universal standing-event interpretation with witnessed/contextual policy
without discarding the faction writing.

### `docs/ai-npc-assets/`

Retain intentional-behavior principles, bounded pathfinding, activity posture,
memory seeds, black-screen diagnostic lessons, headless smoke concept, asset
generator safety, and CTM reference. Its NPC plan is too direct/simple for the
new castle-life gate, and its current-state counts are historical.

### Other plans

- `docs/ASSET-RENDERING-PLAN.md`: keep the per-part character, hero item,
  authored material, shadow, and LOD stages; extend them with the castle
  module/mesh manifest here.
- `docs/ROADMAP-100.md` and `docs/IDEAS-600.md`: idea banks only. Do not let
  their breadth preempt the beta causal chains.
- `docs/STEAM.md`: technical evidence for Spacewar, lobbies, and Steam sockets;
  update after real main-path/two-account verification because some "ready"
  wording is broader than current player-facing integration.
- `docs/MASTER-PLAN.md`: historical loops 330+ fix list; now points here.
- `mods/README.md`: current data-only mod boundary. Future manifest/protocol
  work must keep server/client deterministic IDs and safe content handshakes.

## Drift-prevention rules

1. New product decisions belong in this folder; accepted technical decisions
   also get a concise `DECISIONS.md` entry when code lands.
2. `STATE.md` points to exactly one executable job, not multiple roadmaps.
3. Completed code changes update old checklists only when those checklists are
   still declared active; do not spend jobs rewriting every historical pack.
4. Every current-state number carries a date/loop/commit or is generated.
5. Every use of "implemented," "authoritative," "Steam-ready," "3D asset,"
   "living NPC," or "physics" states the exact behavioral boundary.
6. Runtime data, source, tests, and reproducible proofs outrank prose.
7. The beta label comes only from B29/B30 evidence.

## Next documentation maintenance

When the first B-job ships, update `STATE.md` to the new queue and append the
normal history. Do not make a separate commit merely to mark this planning pack
as progress. As code lands, amend the relevant specification only when an
evidence-backed design decision changes; otherwise leave it stable so sessions
share one target.
