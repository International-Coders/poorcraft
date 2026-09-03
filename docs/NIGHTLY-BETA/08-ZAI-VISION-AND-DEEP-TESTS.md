# Z.ai Vision Review and Deep Test Protocol

Z.ai image recognition is a semantic reviewer, not a substitute for rendered
pixel assertions or game logic tests. Every visible job uses both.

## Capture rules

- Use `cargo run --release -p xtask -- vistest shots` for canonical proofs.
- Add or change a registered scene when the behavior is new. A loose screenshot
  without a reproducible scene is exploratory evidence only.
- Encode egui before readback; keep the existing nonuniform/multicolor gate.
- Fix camera, time, weather, UI state, seed, and resolution in the scene.
- For variable systems, use a declared seed corpus rather than a lucky seed.
- Capture normal, small, and wide UI where layout can change.
- Open every changed PNG at full size. Never claim a visual from console text.

## Vision review record

For every changed scene, append a structured record to the job's DEVLOG entry
or an artifact report:

```text
scene:
image_path:
seed/resolution/state:
expected_objects:
questions:
observed_objects:
layout_or_geometry_defects:
readability_at_game_scale:
comparison_to_baseline:
confidence_0_to_1:
verdict: PASS | FAIL | NEEDS_HUMAN
required_follow_up:
```

A pass needs direct answers, not "looks good." Confidence below 0.80 on a
required object or action is `NEEDS_HUMAN`, never an automatic pass.

## Review questions by domain

### HUD/crafting

- What is the player's most urgent state?
- What can be interacted with and which input performs it?
- What is the primary modal action?
- Which ingredient is missing and how many are owned/needed?
- Are two UI layers competing, clipped, or covered?
- Is any meaning conveyed by color alone?

### Biomes/seeds

- Classify climate, terrain form, vegetation, water, and landmark from an
  unlabeled crop. Which cues support the classification?
- Do different seed panels differ in macro shape, not only flower positions?
- Are repeated patterns, overpopulation, abrupt seams, floating foliage, or
  implausible rivers visible?

### Castles

- Identify realm from silhouette/material/landmark without label.
- Find the main entrance and road.
- Does the foundation meet terrain plausibly on every visible edge?
- Are rooms/walls floating, buried, repeated, clipped, or inaccessible?
- Is there visible life and a reason for each major district?

### NPCs/assets

- Identify role, faction, action, facing, and emotional/alert state.
- Do feet contact ground, limbs connect, held props align, and crowds avoid
  interpenetration?
- Is the icon/entity readable at actual gameplay scale?
- Is the asset visually original and consistent with the project?

## Adversarial visual tests

- bright day, dark night, rain/snow, underwater tint, and colored light;
- low/medium/high quality, RT mode where relevant;
- 640×420, 800×600, 1280×800, and wide aspect;
- pale/dark/busy backgrounds and every faction palette;
- long item/NPC/place names and maximum stack counts;
- full inventory, missing ingredients, hostile/friendly thresholds;
- castle on flat, hillside, coast/river-adjacent, and rejected bad candidates;
- NPC crowd at a gate, blocked workstation, alarm, and save/reload state.

## Logic/property/integration suite

Maintain a matrix in each job plan:

| Claim | Smallest reliable proof |
|---|---|
| same seed reproduces | pure deterministic hash test |
| different seeds diverge | multi-seed statistical/property test + atlas |
| recipe is atomic | inventory transaction unit/property test |
| UI is readable | vistest rectangles/pixels + Z.ai semantic review |
| NPC reaches work | navigation/schedule integration simulation |
| NPC knows a crime | witness/rumor graph test |
| castle is reachable | generator placement report + nav graph test |
| castle looks integrated | multi-seed rendered review |
| save retains state | tempfile round-trip; never `worlds/` |
| performance is bounded | benchmark/counter threshold on representative load |

Mutation question for every test: if the relevant implementation is removed,
does this test fail for the intended reason? If not, rewrite it.

## Beta journey test

Create a deterministic headless journey with a visual companion set:

1. create world from seed text and confirm stored identity;
2. spawn safely, complete onboarding move/look/gather;
3. mine, receive drop, craft planks/tool, place shelter block;
4. save/reload and verify inventory/tutorial/world identity;
5. find a path/kingdom, enter a castle through its gate;
6. trade and complete one local task;
7. commit a witnessed hostile act in a forked test state;
8. deliver the rumor and verify guard/faction response;
9. verify a Gravebound or Cinder policy interprets a human assassination in
   context rather than by a universal alignment flag;
10. save/reload again and verify no event or reward duplicates.

The headless test proves state. Vistest scenes prove the critical UI and world
frames. A short OS-level smoke proves the real window remains alive and input
can recover.

## Conflict resolution

- Pixel/assertion failure: fail even if Z.ai says the image looks good.
- Z.ai finds clipping/ambiguity while pixel gates pass: fail and add a better
  pixel/geometry assertion where feasible.
- Z.ai low confidence but human sees a pass: record human review and improve
  the question/crop before relying on automation.
- Baseline is already defective: record the defect, make the new test fail on
  it, fix it in the same job or honestly reduce job scope.
