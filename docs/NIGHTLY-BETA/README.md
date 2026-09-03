# LOREFORGE Nightly Alpha-to-Beta Goal Pack

This folder is the execution contract for an autonomous Z.ai ZCode `/goal`
run. It does not ask an agent to "make the game better" in one unsafe change.
It turns that ambition into small, green, reviewable jobs which improve the
existing game instead of replacing working systems.

Start with [`00-ZCODE-GOAL.md`](00-ZCODE-GOAL.md). Paste its fenced prompt
into ZCode after `/goal`, from the repository root. The agent must then read
this whole pack, `AGENTS.md`, the project state files, and the relevant source
before choosing a job.

## Pack map

1. [`00-ZCODE-GOAL.md`](00-ZCODE-GOAL.md) — paste-ready autonomous goal.
2. [`01-CURRENT-REALITY.md`](01-CURRENT-REALITY.md) — audited baseline and
   visible alpha problems.
3. [`02-BETA-DEFINITION.md`](02-BETA-DEFINITION.md) — the product gate and
   non-negotiable quality bar.
4. [`03-HUD-AND-CRAFTING.md`](03-HUD-AND-CRAFTING.md) — game HUD, inventory,
   workbench, input, and accessibility specification.
5. [`04-WORLDGEN-SEEDS-AND-BIOMES.md`](04-WORLDGEN-SEEDS-AND-BIOMES.md) —
   deterministic but meaningfully different worlds.
6. [`05-CASTLES-FACTIONS-AND-STRATEGY.md`](05-CASTLES-FACTIONS-AND-STRATEGY.md)
   — original realm castles, settlement simulation, garrisons, and map play.
7. [`06-NPC-AI-REPUTATION-AND-LIFE.md`](06-NPC-AI-REPUTATION-AND-LIFE.md) —
   navigation, memory, witnesses, moral history, and faction reactions.
8. [`07-ASSET-BIBLE-AND-MANIFEST.md`](07-ASSET-BIBLE-AND-MANIFEST.md) — every
   required asset family, metadata, animation, and proof requirement.
9. [`08-ZAI-VISION-AND-DEEP-TESTS.md`](08-ZAI-VISION-AND-DEEP-TESTS.md) —
   screenshot recognition protocol plus behavioral and property tests.
10. [`09-PERFORMANCE-AND-REPO-HYGIENE.md`](09-PERFORMANCE-AND-REPO-HYGIENE.md)
    — profiling, disk cleanup, dead-file proof, and safety rules.
11. [`10-OVERNIGHT-JOB-QUEUE.md`](10-OVERNIGHT-JOB-QUEUE.md) — ordered N01–N24
    jobs sized for one commit each.
12. [`11-DATA-CONTRACTS.md`](11-DATA-CONTRACTS.md) — stable data shapes for
    assets, factions, castles, moral events, and vision reports.
13. [`12-RELEASE-RECOVERY-AND-HANDOFF.md`](12-RELEASE-RECOVERY-AND-HANDOFF.md)
    — green checkpoints, failure recovery, runtimes, push, and morning report.

Run `make night-plan-check` before starting and after changing this pack.

## Operating principle

One job means one coherent result: implementation, behavioral tests, visual
proof when visible, project bookkeeping, runtimes when game code changed,
commit, and `git push github HEAD`. If the night ends, it ends between jobs at
a green pushed checkpoint. Token budget is not a reason to combine unrelated
systems or weaken evidence.

This design takes inspiration from faction-town strategy games, voxel
sandboxes, survival RPGs, and immersive simulations. All names, textures,
characters, buildings, dialogue, icons, and code must be original LOREFORGE
work. Never copy Heroes of Might and Magic art, writing, town layouts, unit
names, music, UI, or other protected content.
