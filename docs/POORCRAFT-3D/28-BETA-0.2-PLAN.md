# POORCRAFT 3D — Beta 0.2 Plan: "The Wide World"

Status: PLAN (owner-approved direction requested — see §7 for the three
decisions this plan needs). Grounded in the design pack: 00 (constitution),
02 (pillars), 03 (engine boundaries), 04+15 (terrain), 08 (assets),
09 (multiplayer), 10 (beta scope), 13 (open decisions), 18 (decision
register), 26 (gap matrix), 27 (roadmap), the 2026-09-28 biome-layout
spec, and the measured truth in STATE.md.

---

## 0. What Beta 0.2 means (the bar, measurable)

Beta 0.1 (today) = the Vertical-Slice Beta: one fun solo hour, V1–V6
gates, a 38-stage battery proving it. Beta 0.2 = **the world gets wide,
the toolkit gets deep, and the shell gets beautiful** — without breaking
one green gate. The four owner asks, translated into honest, countable
bars:

| Owner ask | Beta 0.2 bar (measurable) |
| --- | --- |
| "10X assets" | **Wired** assets (manifest row + consumer + in-world proof, per the 08-doc gate) go from ~47 to **≥ 470**; player-visible asset *kinds* from ~30 to **≥ 300**; raw compiled GLBs ≥ 13,000 via the deterministic factory. Count is printed by `make p3d-asset-inventory` (W5.1) and gated in `make p3d-beta`. |
| "10X functions" | A numbered **Function Catalog** (W3.0) — every player-reachable verb, screen, panel action, machine, recipe family, and setting is a catalog row with a proof. Today ≈ 45 rows. Beta 0.2 closes at **≥ 250 rows wired and proved** (5–6×), with the catalog structured so the next 150 (→ 10×, ≈ 450) are data-driven additions, not code rewrites. |
| "Map as big as originally wanted" | The design pack never fixed a km² — it fixed a **feel**: D-026, capitals 10–25 minutes apart. Beta 0.2 executes the biome spec's scale-up checklist: `WORLDGEN_LAYOUT_VERSION` stamped, climate fields ×2 (~100 km weather systems), interest rings 96/320/1024 → **192/640/2048 m**, a far-horizon that meets the sky (the "floating slab" dies), and **six realm capitals** seeded per D-026 spacing. The map stays procedurally unbounded; the *played* map becomes a ~12×12 km six-realm world. |
| "Menu and HUD WELL improved" | The UI-ASSET-IMPLEMENTATION-GUIDE's 5-step sequence executed: UiAssetCatalog → UiDrawList → title frame → gameplay HUD → screenshot proofs. **9 status bars** (6 live, 3 reserved-hidden), **≥ 60 catalog icons** in the HUD, 9-slice panels, save-slot browser, mouse-sensitivity + invert-Y driving the camera, three proof resolutions (1280×720 / 1280×800 / 1920×1080). |
| "Engine prepared for a lot of future additions" | Five enabling foundations, each with a proof: a **worker-thread pool** behind the streaming/meshing queues; **Arc-shared world** (Rc → Arc) so generation leaves the main thread; **WORLDGEN_LAYOUT_VERSION + FUNCTION_CATALOG version stamps** (safe migration law); the **data-driven content boundary** (P3D-506: TOML recipes/items/biome tables the game reads at boot — the mod door); the **authoritative host split** (sim host callable in-process AND headless — the co-op door). |

Everything stays inside the constitution: breadth safeguard holds (no
nuclear / armies / Steam productization *delivery* in 0.2 — but their
doors get built), depth before catalog size, performance is part of the
fantasy.

---

## 1. Workstream W1 — The Wide World (map scale + six realms)

The scale-up checklist already exists and was approved as design
(`docs/superpowers/specs/2026-09-28-biome-layout-design.md:95-107`).
This workstream executes it and adds the far-horizon work.

### W1.1 — Version stamp + scale dials (engine prep, do first)
- Stamp `WORLDGEN_LAYOUT_VERSION` in pc3d_world; saves record it; a
  mismatch regenerates untouched terrain and SAYS SO in the load flow
  (the "no silent format drift" law, 03-doc).
- Introduce the three scale dials as constants in one place:
  `CLIMATE_CELLS` (192 → 384), `ELEVATION_BASE_CELLS` (48 → 96),
  interest rings (96/320/1024 → 192/640/2048).
- Proof: `--atlas` across 8 seeds still passes coherence/seam/belt laws;
  `make p3d-beta` stages 5–11 green; new law test: two seeds' capital
  spacing ∈ [10, 25] minutes' walk (D-026).

### W1.2 — The far horizon (kill the floating slab)
- Ring 3 "Far" LOD extends to the new 2048 m ring with a cheaper
  impostor mesh (per-region average height plate + color); the fog curve
  blends the far ring INTO the sky gradient (fog color = sky color at the
  horizon pitch — one shader constant family).
- Proof: a new visual gate `windowed_far_horizon.png` — the horizon band
  contains terrain pixels, not void, at 6 vantages; frame p50 on the deck
  bench stays ≤ 8 ms mid.

### W1.3 — Six realm capitals (D-026 spacing)
- Extend the settlement/castle planner: 6 capitals placed by the seeded
  watershed graph (one per biome family: coast, plains, forest,
  wetlands, highlands, snow), spacing enforced by a new law test
  (10–25 min walk ≈ 720–1800 s at the walk rate → 860–2160 m apart).
- Each capital gets its faction kit (see W2.3) and a name from the
  seeded name stream.
- Proof: `--play-city` renders the near capital (existing gate) + a new
  `--realm-map` windowless export: a PNG world map with the 6 capitals
  and river graph drawn (the marketing shot AND the law's evidence).

### W1.4 — RiverCarve (queued design, belongs here)
- Per-seed RiverGraph cached once at world build; the height function
  depresses a smooth valley under each river edge segment; the water
  strips conform to the carved bed.
- Proof: new law test "water strip height ∈ [bed, bed+0.5 m] along the
  whole edge"; `--play-water` gate unchanged and green.

---

## 2. Workstream W2 — 10× Assets (the factory meets the gate)

Law first (08-doc, non-negotiable): an asset counts ONLY with a manifest
row + provenance + consumer + proof. Raw factory output that nothing
consumes does not move the number.

### W2.1 — The inventory gate (measurement, do first)
- `make p3d-asset-inventory`: extends the existing inventory validator —
  prints wired vs present vs orphan counts, and fails if an asset has a
  consumer claim with no proof. This number IS the workstream's score.

### W2.2 — Flora/prop variety to 300+ visible kinds
- Extend assetgen's variant factory: 21 families → **45+ families**
  (6 biome-dressed families each: coast/wetland/plains/forest/highland/
  snow variants of trees, shrubs, rocks, logs, reeds, mushrooms),
  13,000+ compiled GLBs, determinism byte-compare kept.
- Flora placement tables get per-biome dressings (the W1 world reads
  different at a glance per realm).
- Proof: the wilderness gate counts ≥ 8 distinct plant kinds per biome
  family in-frame (pixel-kind census, like the existing green_frac
  gates); `--play-wilderness` green.

### W2.3 — Faction kits ×6 (the capital builders)
- The settlement kit (11 modules) becomes 6 faction kits (walls, gate,
  keep, homes, workshop, watchtower, banner — 7 modules × 6 factions =
  42 wired modules), each with faction palette + silhouette identity
  (08-doc's identity list: proportions/roofs/walls/colors/symbols).
- city.rs finally wires the GLB kits (STATE blocker closed): capitals
  draw their faction kit, not procedural boxes.
- Proof: `--play-city` per faction (6 seeds) — subject gate + "kit modules
  drawn" counter; the V5 gate flips PARTIAL → WORKS.

### W2.4 — Creatures + NPCs (the world's population)
- 12 creature kinds (goblin ×3 variants, wolf, deer, boar, bird ×3,
  fish ×3 — fish feed the fishing verb) with lod0 ≤ 1800 tris each,
  3 variants per kind (36 GLBs).
- NPC variety: 3 static humanoids → 8 (adds mage, merchant, farmer,
  noble, guard-captain) with per-faction palettes (40 GLBs).
- Proof: npc/city gates + a creature-count law in the wilderness gate.

### W2.5 — UI icon set (feeds W4)
- WT-005's `ui_100` batch: 100+ icons (actions, resources, bars, map
  markers, faction symbols) generated as PNGs into
  `assets/compiled/ui/` + the UiAssetCatalog manifest (W4.1 consumes).
- Proof: catalog validator — every icon referenced by the HUD exists,
  every icon on disk is referenced (the inventory law, UI edition).

---

## 3. Workstream W3 — 10× Functions (the catalog)

### W3.0 — The Function Catalog (measurement, do first)
- `docs/POORCRAFT-3D/FUNCTION-CATALOG.md`: numbered rows — every
  player-reachable verb, key, screen, panel action, machine, recipe
  family, setting, and menu item; each row names its proof (a gate, a
  route, or a ui_key test). Seed it from today's played truth (~45 rows).
- The catalog is a gate: `make p3d-beta` fails if a row's proof is red.
  New functions MUST add a row (the anti-overclaiming law, function
  edition).

### W3.1 — Journey step 7: two deep paths (the declared next_task)
- Path choice as a player-facing fork at the forge: ENGINEERING vs
  MYSTERIES (technology vs magic careers — 07-doc's parallel paths,
  delivered thin-but-real: each path = a distinct recipe family, a
  distinct machine line, a distinct quest thread, one signature tool).
- Proof: a new observatory route `route_path_choice` (choose → the world
  reacts: a machine appears OR a ley node hums) + catalog rows.

### W3.2 — Production depth (machines ×8, recipes ×40)
- Machines: boiler (exists) + water wheel, mill, crusher, loom, pump,
  kiln, wind turbine, alembic — each with a flow/fuel consumer law and a
  panel (the machine framework exists; this is breadth on rails).
- Recipes: 40 recipe rows across the two paths, data-driven (W5.4).
- Proof: per-machine route (`route_machine_<name>`), catalog rows, and a
  recipe-validation gate (TOML table ↔ item registry consistency).

### W3.3 — Survival depth (the played loop's missing verbs)
- Fishing wired to the river (first_catch onboarding finally fed — the
  scorecard's gap #2), cooking (campfire + 8 foods), farming (tilled
  soil, 4 crops, growth ticks), beds/sleep (skip night with a shelter
  law), armor slots (the reserved armor bar goes live).
- Proof: `route_fishing`, `route_cook`, `route_farm`, `route_sleep` —
  one observatory route each; onboarding marks extended.

### W3.4 — Social depth (steps 8–9 arrive early, thin)
- Companion actually helps (fetch/dig/fight — the companion crate is
  tested but thin in play): one real assist verb + a follow-combat law.
- Witnessed faction effect: an assault near a capital drops that
  faction's trust and the GATE GUARDS react (the karma chain's first
  visible consumer — the perception/karma crates already exist).
- Proof: `route_social` extended (witness → guard reaction), catalog
  rows.

### W3.5 — The realm map screen (joins W1.3)
- M currently opens machines; the MAP becomes its own screen: the
  `--realm-map` export rendered live in-engine (6 capitals, rivers,
  the player's marker, fog-of-war by visited regions).
- Proof: `ui_shots` gains `ui_realm_map`; the map matches the
  windowless export (pixel-consistency gate).

Counts this workstream: machines 2→10, recipe families 3→40+, verbs
~18→30+ (fishing/cook/farm/sleep/ride-map/path-pick/companion-assist…),
screens 6→9 (Map, Path, Codex), panels 9→12, settings 4→8. Catalog:
~45 → ~250 proven rows; the data-driven boundary (W5.4) is what makes
the remaining 200 cheap.

---

## 4. Workstream W4 — Menu & HUD (the shell)

### W4.1 — UiAssetCatalog + UiDrawList (the guide's step 1–2)
- The manifest-driven icon/panel system from
  `assets/UI-ASSET-IMPLEMENTATION-GUIDE.md`: every icon, bar, panel
  9-slice, and key glyph is a catalog row; the UI draws from the
  catalog, never from baked pixels.
- Proof: catalog validator + `ui_shots` at three resolutions.

### W4.2 — The title frame (step 3)
- Title screen: game logo (the generated transparent logo exists), menu
  buttons with 5 states, version + build hash, background = a slow
  camera drift over the realm map vista.
- Proof: `ui_title` gates at 3 resolutions + mouse-nav test (the
  `route_title_mouse` observatory route exists).

### W4.3 — The gameplay HUD (step 4)
- 9 bars per the guide (health/stamina/food/XP live + armor/mana/air/
  temperature/karma reserved-hidden until state exists), 9-slot hotbar
  with icon rendering + counts + selection ring, action prompt, fading
  toasts, compass strip with the realm capitals' bearings, minimap
  corner (the realm map, local crop).
- HUD elements come from the catalog; debug text moves behind a toggle
  (the 22-doc requirement).
- Proof: `ui_shots` HUD states + a hotbar interaction route (select,
  use, count decrement).

### W4.4 — Menus with real depth (save slots, settings, pause)
- Save-slot browser (3+ slots, each showing the world name/seed/day/
  playtime + the save's own minimap crop), pause menu per the 22-doc
  contract, settings page 2 (keybinds view + audio placeholders per the
  GLM-UI spec).
- Proof: `ui_shots` states + a save→slot-browser→load round-trip route.

---

## 5. Workstream W5 — Engine prepared for everything after

### W5.1 — `make p3d-asset-inventory` (W2's measurement + the orphan law)
### W5.2 — Worker threads behind the bounded queues
- A small job system (std::thread + channels, no new deps) runs patch
  generation and meshing off the main thread; the per-frame budgets stay
  (results uploaded bounded). The single-threaded Rc world becomes Arc.
- Proof: deck bench mid p50 ≤ 6 ms; teleport-recovery (stream) time
  halved; all determinism tests unchanged (pure functions, same order
  of application).
### W5.3 — WORLDGEN_LAYOUT_VERSION + FUNCTION_CATALOG stamps
  (migration laws; see W1.1, W3.0)
### W5.4 — The data-driven content boundary (P3D-506)
- Recipes, items, machine definitions, flora dressing tables, and quest
  seeds move to TOML tables read at boot (the LOREFORGE mods/ pattern,
  by decision P3D-506 which is pre-authorized). Validation gate: unknown
  item ids, cycles, or bad tables fail the boot loudly.
- Proof: a `mods/` example pack loads and adds one recipe + one item
  WITHOUT recompiling — the catalog rows prove it.
### W5.5 — The authoritative host split (the co-op door)
- The slice host's sim becomes callable headless (it already nearly is —
  SoloHost::run_ticks) with a replay-hash law: same command stream, same
  digest, in-process or headless. No wire protocol in 0.2 — the DOOR
  only (09-doc's staged order respected).
- Proof: a `--host-headless` flag runs a 60 s command stream and prints
  the same replay hash as the in-process run.

---

## 6. Sequencing & gates

```
Stage 0 (foundations, ~first): W5.1, W3.0, W5.3, W4.1
  → nothing else counts until the three meters exist.
Stage 1 (world): W1.1 → W1.2 → W1.4 → W1.3
Stage 2 (breadth): W5.4 → W2.2 → W2.5 → W3.2 (parallel with stage 3)
Stage 3 (population): W2.3 → W2.4 → W3.4 → W3.3
Stage 4 (shell): W4.2 → W4.3 → W4.4 → W3.5
Stage 5 (door): W5.2 → W5.5
CLOSE: full `make p3d-beta` + new gates (asset inventory ≥ 470 wired,
catalog ≥ 250 rows, realm-map gate, 3-resolution UI gates) — the
definition of Beta 0.2 shipped.
```

Rules carried from the dev-loop: every stage ships code + tests + proof,
updates STATE/BACKLOG/CHANGELOG/DEVLOG, keeps the battery green, and
stops only at green checkpoints. Any stage that would break V1–V6 gets
cut before it starts (the breadth safeguard, 27-doc).

## 7. Owner decisions this plan needs (13-doc's open list, answered or confirmed)

1. **P-001/P-002 scale confirmation** — adopt the W1 dials (192/640/2048 m
   rings, ×2 climate) as the Beta 0.2 world: YES/NO (the D-026 spacing is
   already owned; this just sizes the streaming around it).
2. **10× asset metric** — confirm "wired assets" (with consumers) as the
   counted metric, not raw factory output (the 08-doc gate says this is
   the only honest count).
3. **Path choice flavors** — ENGINEERING vs MYSTERIES as the two Beta 0.2
   careers (07-doc allows technology/magic/exploration/politics; 0.2
   ships two, the catalog holds the hooks for four).

## 8. What is explicitly OUT of Beta 0.2 (kept from the safeguard)

Multiplayer wire/Steam productization (the door ships, the product
doesn't), armies/nuclear/dragons as content, full fluid dynamics,
LLM-driven NPC dialogue, language rewrites, 128-player promises. The
H2 list is untouched — 0.2 just builds its doors.
