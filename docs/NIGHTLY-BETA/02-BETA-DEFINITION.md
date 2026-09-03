# Beta Definition and Quality Gates

"Beta" means the intended core experience is coherent, completable, stable,
and testable. It does not mean every conceivable biome, faction, creature, or
decoration is finished.

## Player-facing beta gate

A fresh player can create a named world with a chosen or rolled seed, learn
movement/mining/crafting without external instructions, establish shelter,
advance through real recipes and machines, find a visibly distinct settlement
or castle, understand how its people view them, change that relationship by
observable actions, survive combat, save, quit, reload, and continue without
corruption or hidden state loss.

The following must all be true:

- First minute: contextual controls and one pinned starter objective appear,
  can be dismissed, never block input, and persist their completion state.
- HUD: readable at 640×420, 800×600, 1280×800, and a wide format; default
  play remains quiet; danger, interaction, objective, and reputation changes
  are unmistakable without opening debug UI.
- Crafting: no world/HUD collision, no invisible focus trap, no minting or
  duplication, clear missing-ingredient states, safe batch craft, queue
  feedback, category navigation, recipe discovery, and keyboard/mouse parity.
- Worldgen: same `(seed, generator_version, world_type)` reproduces the same
  audited samples; different seeds cross minimum diversity thresholds; all
  generated structures are reachable, supported, and terrain-integrated.
- Biomes: the major climate families can be identified from a screenshot
  without reading a label; transitions look intentional; resource differences
  affect play rather than only ground color.
- Castles: each realm's home is identifiable by silhouette, material, spatial
  grammar, inhabitants, and activity. No castle floats, buries required rooms,
  blocks its own gate, spawns too near the player, or severs rivers absurdly.
- NPCs: can reach essential home/work/safety anchors or visibly recover;
  remember witnessed player behavior; react according to faction values;
  never know unwitnessed events instantly; and cannot permanently deadlock a
  doorway or path.
- Reputation: self-defense, hostile combat, civilian murder, assassination,
  theft, aid, desecration, monster hunting, and contract fulfillment are
  distinct events. Undead and infernal realms may value ruthless behavior,
  but their response follows their own interests and evidence—not a universal
  "evil likes evil" switch.
- Assets: no checkerboards, missing layers, accidental opaque backgrounds,
  unreadable icons, copied third-party art, unexplained files, or unproved
  faction/role variants in beta-critical paths.
- Stability: load/save round trips, generator-version migration behavior,
  input recovery, smoke, full tests, and all visual proofs pass.
- Performance: no unbounded per-frame world rebuild, pathfinding storm, asset
  decode loop, or UI allocation spike. Target budgets are measured on this
  host and recorded, not invented.

## Evidence hierarchy

1. Deterministic unit/property tests for pure logic.
2. Integration tests for cross-crate and persistence behavior.
3. Vistest pixel assertions for rendered facts.
4. Z.ai image-recognition review for composition and semantic facts.
5. Human play/smoke for feel and operating-system input behavior.

Higher items do not replace lower ones. Image recognition cannot prove that a
recipe consumed the right inventory items; a unit test cannot prove that the
craft button is legible over terrain.

## Per-job definition of done

- The implementation has no orphaned public data or dead UI path.
- Tests fail when the new behavior is removed or broken.
- Every visible change has before/after or multi-seed proof and an actual
  image review recorded in `DEVLOG.md`.
- `cargo build --workspace` and `cargo test --workspace` are green.
- Visual jobs pass `cargo run --release -p xtask -- vistest shots`; changed
  images were opened and inspected, not only generated.
- Runtime-sensitive jobs pass `make smoke`; performance-sensitive jobs pass
  `make perf` with before/after numbers.
- State/backlog/changelog/devlog are honest; Makefile matches commands.
- Fresh runtimes exist when game code changed; the job is committed and
  pushed before another job begins.

## Stop-the-line conditions

Stop the current job and repair before continuing when any of these occurs:

- data loss, save incompatibility without migration, or test touching
  `worlds/`;
- screenshot claim contradicted by the image;
- duplicate item/block ID, missing texture mapping, or dead catalog entry;
- seed determinism regression;
- castle/structure clipping, floating, unreachable, or replacing protected
  player edits;
- NPC navigation consuming an unbounded frame budget;
- unrelated dirty files overwritten or committed;
- a red build, test, vistest, smoke, or required performance gate.

## Scope discipline

N01–N24 are stepping stones, not permission for a monolithic rewrite. An
overnight run that lands four deep, green jobs is better than twenty partial
systems. Anything honestly deferred is recorded as the next job with exact
files, failing evidence, and acceptance criteria.
