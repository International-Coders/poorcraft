# Ordered Overnight Job Queue (N01–N24)

Run the preflight, then take the first incomplete job whose prerequisites are
green. One job equals one commit and push. Split a job if it cannot be fully
proved in one pass; do not combine jobs to make the checklist move faster.

## Preflight (not a commit)

- Read `AGENTS.md`, state/backlog/changelog/devlog/audit, and this pack.
- Run `git status --short`; name every pre-existing dirty path and preserve it.
- Run `make night-plan-check`.
- Confirm current test/vistest counts from evidence, not this document.
- If baseline is red, diagnose and fix only if the failure is in scope; record
  exact blocker otherwise. Never stack feature changes on an unknown red base.

## N01 — First-minute onboarding and pinned objective

Implement the persisted contextual tutorial state machine from
`03-HUD-AND-CRAFTING.md`, including dynamic key labels, dismiss/reset, compact
HUD placement, and normal/small-window proofs. This is the current STATE.md
priority and must land before broader redesign.

## N02 — Transactional workbench and queue correctness

Make crafting validate/consume/produce atomically across craft-one, craft-all,
queue, cancel, full output, rapid input, mod recipe, and save/load. Add property
and integration tests before changing presentation.

## N03 — Workbench visual hierarchy and input recovery

Build the modal normal/compact layouts, world scrim, ingredient ownership,
disabled reasons, stable focus, and E/Escape recovery. Add all workbench proofs
specified in `03-HUD-AND-CRAFTING.md` and inspect them with Z.ai.

## N04 — Contextual HUD, combat, settlement, and reputation feedback

Implement priority-safe transient channels without returning debug clutter to
the default HUD. Prove normal, small, wide, danger, build, and faction states.

## N05 — Seed identity and 64-seed regression laboratory

Centralize `WorldIdentity`, fix UI/save/multiplayer consumption gaps, and add
determinism/order/negative-coordinate tests plus the machine-readable 64-seed
diversity report. Increment generator version only if generated chunks change.

## N06 — Macro terrain, climate, rivers, and spawn diversity

Use the N05 metrics to repair real low-diversity causes. Improve correlated
macro shape and spawn-quality choice without adding noisy microvariation.
Render seed/spawn atlases and record calibrated before/after measurements.

## N07 — Biome identity and transition pass

Give each retained biome a gameplay/visual data row; merge or deepen aliases;
reduce surface confetti; add transition, resource, vegetation, fog/weather,
and contact-sheet proofs. Z.ai must classify unlabeled crops reliably.

## N08 — Castle siting, layout grammar, and asset manifest pipeline

Separate candidate scoring/layout planning from voxel writes; add placement
reports, module ports/nav anchors, asset manifest validation, and 128-site
properties. Fix floating-pedestal and entrance/road failures before faction
expansion.

## N09 — Accord civic castle vertical slice

Ship an identifiable terrain-integrated Accord castle with complete minimum
kit, ruler/guard/worker/trader roles, schedule anchors, gate policy, market,
garrison posts, one dwelling, and proofs. This establishes the full vertical
slice every later realm must match.

## N10 — Ironborn forge-fort

Ship terraced mountain siting, production district, forge activity, heavy
defenses, distinctive assets/roles, ore economy, gate/garrison behaviors, and
multi-seed proofs without cloning N09's footprint.

## N11 — Ember Covenant living citadel

Ship grove-sensitive placement, root/ring grammar, magic/alchemy service,
living-land protection reactions, distinctive assets/roles/dwelling, and
multi-seed proofs.

## N12 — Free Holds hillfort

Ship farm/water/long-hall spatial grammar, hospitality law, food/mount economy,
distinctive assets/roles/dwelling, and multi-seed proofs.

## N13 — Ashen Order archive city

Ship stepped archive/observatory/vault grammar, knowledge services, restricted
areas, scholar routines, distinctive assets/roles/dwelling, and multi-seed
proofs.

## N14 — Nameless sunken refuge

Deepen the existing hostile faction as exiles who reject compacts. Ship broken
vertical/sunken grammar, stealth/salvage gameplay, conditional access,
distinctive assets/roles/dwelling, and multi-seed proofs.

## N15 — Gravebound Court tomb-city

Add the original ordered-undead faction only after the data contracts are
ready. Ship lore, palettes/silhouettes, tomb-city grammar, death-resource
economy, undead roles/creatures, grave/oath policies, and multi-seed proofs.
Resolve its difference from Nameless explicitly.

## N16 — Cinder Host caldera fortress

Add the original infernal contract faction. Ship lore, basalt/brass/chained
grammar, bargain economy, infernal roles/creatures, contract policies, safe
magma siting, and multi-seed proofs. Resolve its difference from Covenant.

## N17 — Moral event ledger, witnesses, and rumors

Implement contextual immutable moral events, local witness sensing, knowledge
source/confidence, bounded rumor delivery, chronicle integration, persistence,
and tests for unwitnessed/witness-interrupted/reported crimes.

## N18 — Faction policy engine and human-assassin reactions

Interpret N17 events through realm policy into standing, fear, respect,
personal memory, warrant, dialogue posture, trade/gate/guard behavior, and UI
reasons. Ship the full context matrix including Gravebound/Cinder conditional
affinity and contract/oath overrides.

## N19 — Bounded hierarchical NPC navigation

Add cached castle graphs plus local voxel routing, doors/gates, reservations,
crowd avoidance, stuck recovery, time budgets, F3 metrics, persistence, and
adversarial route simulations. No visible teleport repair.

## N20 — Daily life, alarms, work, and memory presentation

Connect schedules/needs to real anchors and production; ship patrol,
investigate, shelter, challenge, mourn/celebrate, work props/animations, and
cause-readable scenes across a simulated full day.

## N21 — Dwellings, recruitment, commanders, garrisons, and territory

Implement the smallest coherent strategy layer from
`05-CASTLES-FACTIONS-AND-STRATEGY.md`: bounded replenishment, persistent
casualties, real posts/orders, resource-site connectivity, castle upgrades at
safe sockets, and discovered-map influence. Use one full vertical slice, then
data-drive the other realms; do not create a disconnected army spreadsheet.

## N22 — Asset closure and visual consistency sweep

Complete manifest rows/consumers/proofs for all beta-critical castle kits, 64
NPC role entries, 32 creature-family entries, UI icons, fallbacks, animation
states, source/license data, and actual-scale recognition. Spawn-or-cut dormant
catalog types. Fix placeholder/near-duplicate/alpha/UV failures.

## N23 — Performance and repository hygiene

Profile and repair the largest measured frame/memory/stall issues, then execute
the safe cleanup audit. Reclaim rebuildable build output as warranted, classify
tracked screenshots, remove only proved obsolete files, preserve saves and
dirty work, rebuild, test, vistest, and report before/after bytes honestly.

## N24 — Beta journey, save/migration soak, and candidate release

Ship the deterministic end-to-end journey, multi-seed castle/NPC soak,
save/reload and generator-version checks, input recovery, full visual review,
performance report, smoke, fresh runtimes, and beta-known-issues list. The beta
label is earned only if `02-BETA-DEFINITION.md` passes; otherwise name the
remaining first failing gate and keep an alpha/pre-beta label.

## Expansion after N24

Do not hide remaining ambition. Move non-beta breadth—more creature families,
castle upgrade tiers, raids/sieges, diplomacy arcs, additional animations,
localization, controller support, multiplayer authority, and mod exposure—into
BACKLOG with evidence and prerequisites. Continue future goals from that queue,
not by silently expanding an active job.
