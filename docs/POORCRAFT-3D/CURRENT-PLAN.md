# POORCRAFT 3D — Playability Reset and Continuous Build Plan

Status: **ACTIVE**  
Owner direction recorded: **2026-10-04**  
Replaces the execution order in `28-BETA-0.2-PLAN.md` until the playability
gates below are green. It does not cancel the long-term world, faction, magic,
industry, castle, or multiplayer goals.

## 1. The decision

POORCRAFT 3D already has a promising lightweight engine, a strong menu style,
good performance on older Intel hardware, and many implemented systems. The
played game is difficult to understand because its systems are exposed as a
large collection of letter keys, the hotbar shows building materials the
player does not own, terrain and water can render without a believable shared
surface, and the first crafting path is hidden behind panels.

The next milestone is therefore a **playability reset**, not another breadth
push. The game will use a familiar survival-sandbox grammar:

- the selected item or tool determines what the player can do;
- left and right mouse buttons perform the primary and alternate actions;
- the hotbar contains owned items, not an always-available debug palette;
- inventory and crafting live in one discoverable screen;
- building and terrain editing require crafted tools;
- the first 20 minutes teach the game through visible world interactions;
- expansion resumes only after a new player can complete that loop without
  reading the controls page or source code.

The target feel is familiar to players of Valheim and Minecraft while keeping
POORCRAFT 3D's own smooth terrain, settlements, factions, magic, machines, and
realm identity.

## 2. Evidence from the 2026-10-04 packaged-build playtest

The supplied `poorcraft3d-macos.dmg` was mounted and its packaged binary was
played, then its deterministic rebuild, water, and UI routes were run.

| Finding | Evidence | Consequence |
| --- | --- | --- |
| Menu direction is worth preserving | Title, New World, Settings, Pause, and HUD captures are readable and stylistically coherent | Improve the shell incrementally; do not replace its visual identity |
| Spawn is visually broken | Giant trunk/branch geometry intersects the camera and menu background | Add spawn-clearance and camera-occlusion gates before content expansion |
| Starting state contradicts itself | HUD says `PACK EMPTY` while slots 1–5 expose Soil/Grass/Sand/Rock/Snow for building | Hotbar must be derived from inventory/equipment only |
| Controls are a memory test | Settings lists G dig, K pack, C craft, M machines, P attack, H/T forge, B save, L load, I inspect, and more | Replace letter-per-system controls with a selected-tool action grammar |
| Crafting exists but is not an understandable loop | The panel lists recipes, all `SHORT`, without showing how to gather the first required item | Build a guided gather → tool → station → recipe chain |
| River is not grounded | The water proof renders long blue strips beyond the loaded terrain; the route still reports PASS | Water and terrain need a shared visibility/LOD contract plus a semantic grounding gate |
| Cave proof is not visually valid | The rebuild cave capture is mostly clipped gray geometry | Add camera-validity and visible-interior gates |
| Automated proof can overclaim | Water coverage/remesh checks pass even while the river floats | Every visual gate must check the intended subject and obvious failure modes |
| Performance is viable | Rebuild route: p50 16.65 ms, p95 21.13 ms, average 55.5 fps. Water route: p50 9.07 ms, p95 11.96 ms, average 114.8 fps | Protect the lightweight engine and measure every rendering change |
| Package identity is misleading | Branch is `b9ff766`; app and DMG display `5c0f0d1` because packaging happened from a dirty pre-commit tree | Dirty builds must say `-dirty`; release evidence must name the source revision |
| Source truth was stale | `cargo test --workspace` found a format-version test still hard-coded for world v1 after world v2 shipped | A green claim requires a fresh command result from the current checkout |

## 3. Player-facing control grammar

The same inputs must work everywhere unless a visible panel owns them.

| Input | Default action |
| --- | --- |
| WASD | Move |
| Mouse | Look |
| Space | Jump |
| Shift | Sprint |
| 1–8 / wheel | Select an owned hotbar item |
| Left mouse | Primary action of selected item: attack, chop, mine, dig, place, or confirm |
| Right mouse | Alternate action: block, aim, terrain mode, or open build-piece selection |
| E | Interact, pick up, open, talk, use station |
| Tab | Inventory, equipment, and personal crafting |
| M | World/realm map |
| J | Journal and tracked objectives |
| Esc | Close panel or pause |
| F3 | Debug summary only in developer-enabled builds |
| F10 or backtick | Developer console only when developer mode is enabled |

Save belongs to autosave and the pause menu. Load belongs to the title/pause
menus. Digging, building, attacking, eating, crafting, machine operation, and
forge steps do not receive permanent one-letter shortcuts.

Context-specific keys such as rotate or snap may appear only while the relevant
tool is equipped, and the preview must show them onscreen.

## 4. Tool-owned verbs

| Equipped item | Primary action | Alternate action |
| --- | --- | --- |
| Empty hands | Pick up loose objects; gather light plants/branches | Inspect/context action |
| Axe | Chop trees and wooden objects | Block or alternate swing |
| Pickaxe | Mine stone and ore | Inspect mineral/target |
| Shovel | Lower soil/sand; dig a trench | Raise/fill using carried material |
| Hoe | Level toward a visible target plane | Cycle flatten/smooth/slope mode |
| Hammer | Place or repair selected construction piece | Open build-piece wheel/menu |
| Weapon | Attack | Block/aim/alternate attack |
| Food/potion | Consume | Inspect details |

Terrain editing must show a translucent preview of the resulting surface
before the player commits. Green means the edit is valid; amber names a cost
or warning; red names the reason it cannot be performed. The simulation uses
the same preview result when applying the edit, so the preview cannot lie.

## 5. Milestones and gates

### P0 — Trustworthy baseline and smaller workspace

Deliverables:

1. Make the full current workspace test command green.
2. Remove duplicate Make targets and add one safe `p3d-clean` command.
3. Make dirty package builds identify themselves as dirty.
4. Keep LOREFORGE clearly labeled as the preserved older project and
   `poorcraft-novo/` as the current game.
5. Remove generated Cargo caches from handoff copies; keep releases, saves,
   source, assets, and evidence.
6. Add semantic visual checks:
   - water pixels cannot extend over unsupported/void pixels within the active
     terrain visibility ring;
   - the first-person spawn center cannot be occupied by large geometry;
   - the cave camera must see a minimum amount of lit interior surface;
   - title/gameplay captures fail when one object covers an excessive fraction
     of the center view.

Exit gate: current tests pass, the packaged build identifies its real source
state, and a clean checkout can be understood from `poorcraft-novo/README.md`.

### P1 — Honest new-game start

Deliverables:

1. Spawn on validated walkable terrain with a clear forward view, no intersecting
   prop, and a reachable first resource cluster.
2. Start with empty hands and no free terrain palette. Any deliberate starter
   item must be an owned inventory item and documented in the world preset.
3. Place loose branches, stones, and basic food near—but not directly in—the
   spawn path.
4. Show one short contextual objective: gather a branch and stone, then open
   inventory.
5. Ensure each new seed passes spawn clearance, slope, water-distance, and
   resource-reachability laws.

Exit gate: a fresh world reaches its first crafted tool in five minutes without
the controls page.

### P2 — Input and hotbar rebuild

Deliverables:

1. Introduce an action resolver based on selected item, target type, distance,
   player state, and button intent.
2. Route mouse input through that resolver. Remove gameplay dependence on G,
   F, R, P, X, H, T, B, L, I, K, and C as permanent verbs.
3. Build hotbar slots from the real inventory/equipment state. Empty slots are
   empty; stack counts and durability are visible.
4. Merge pack, equipment, and personal recipes into the Tab screen.
5. Generate every prompt from the active binding map and current action result.
6. Add rebind support after the default map is stable.

Exit gate: every first-session action can be completed with movement, mouse,
E, Tab, hotbar selection, and Esc.

### P3 — Terrain and water repair

Deliverables:

1. Build the river bed from the same sampled terrain authority used by
   collision and rendering.
2. Clip/stream water to the terrain visibility region; terrain and water share
   LOD boundaries and fog behavior.
3. Implement RiverCarve so channel elevation and the visible water surface agree.
4. Equipable shovel and hoe actions: lower, fill/raise, level, smooth, and
   optional slope mode.
5. Preview the exact post-edit mesh, affected area, material cost, and water
   invalidation before commit.
6. Bound remeshing and water recalculation to dirty patches/sections.

Exit gate: six seeds show no unsupported water from six ground-level and two
high-level views; edited land matches its preview and remains walkable.

### P4 — Understandable crafting and building

The first-session chain is:

1. gather loose wood, stone, and food;
2. craft a basic axe or hammer from personal recipes;
3. gather enough wood to place a workbench;
4. use the workbench to craft a hoe/shovel and better tools;
5. level a small site with the hoe, place a shelter with the hammer;
6. add fire/food and survive the first night;
7. find ore/river power and enter the wider machine or magic paths.

Deliverables:

- recipe cards show output, owned/required ingredients, station, tool tier,
  craft time, and the purpose unlocked;
- unavailable recipes explain one next requirement instead of only `SHORT`;
- nearby stations extend the same crafting screen;
- build pieces consume inventory materials and use a ghost preview with snap,
  rotation, support, slope, ownership, and collision feedback;
- the journal teaches at most one next action and never requires debug output.

Exit gate: an unfamiliar player can build a small valid shelter and make one
terrain edit in 20 minutes.

### P5 — Developer console and cheat engine

Developer mode is enabled by a launch flag or dedicated development build. It
is visibly marked and off in normal player releases.

Foundation:

- one typed command registry with parsing, autocomplete, help, argument
  validation, permissions, history, and structured results;
- commands call the same simulation APIs as gameplay or named debug-only APIs;
- every command is available to deterministic input replay where practical;
- using a state-changing cheat marks the save metadata so later bug reports
  disclose it without blocking the player from saving;
- the console can copy/export its command history and current runtime state.

First command set:

| Category | Commands |
| --- | --- |
| Player | `god`, `fly`, `noclip`, `heal`, `stamina`, `kill`, `respawn` |
| Inventory | `give`, `take`, `clearinventory`, `equip`, `durability` |
| World | `teleport`, `setspawn`, `settime`, `timescale`, `weather`, `revealmap` |
| Entities | `spawn`, `despawn`, `listentities`, `ai`, `faction` |
| Terrain | `terrain lower`, `terrain raise`, `terrain level`, `terrain smooth`, `undo` |
| Progression | `unlockrecipe`, `setskill`, `setcareer`, `quest` |
| Inspection | `inspect`, `dumpstate`, `profile`, `wireframe`, `collision`, `nav`, `water` |
| Evidence | `screenshot`, `capture-route`, `save`, `load`, `reload-assets` |

Exit gate: a developer can reproduce the first-session state, terrain defects,
and river defects from a short saved command script.

### P6 — Resume breadth from a stable core

After P0–P5, resume the strongest parts of the Wide World plan in this order:

1. six visually distinct faction capitals;
2. deeper NPC jobs, needs, factions, and witnessed consequences;
3. engineering and mysteries as real play paths;
4. machine, magic, farming, fishing, and combat depth;
5. larger asset variety tied to gameplay consumers;
6. authoritative co-op and later platform work.

Raw asset counts and catalog row counts are diagnostics, not milestone goals.
A feature counts when a player can find it, understand it, use it, observe its
consequence, save it, and load it.

## 6. Continuous work-card contract

Every implementation job starts with one card containing:

```text
ID / title:
Player-visible problem:
Trigger and before behavior:
Expected after behavior:
Files/systems likely involved:
Automated proof:
Manual play path:
Performance budget:
Save/version impact:
Explicitly out of scope:
```

A card closes only when:

1. implementation and focused tests pass;
2. the full affected workspace test command passes;
3. the manual play path was exercised from a packaged or release build;
4. before/after evidence is saved outside Cargo `target/`;
5. `STATE.md` and the monthly development log are updated;
6. known limitations and the next single card are named.

## 7. Immediate queue

| Order | Card | Outcome |
| --- | --- | --- |
| 1 | P0-001 Truth baseline | Green current tests, truthful build stamp, duplicate Make targets removed |
| 2 | P0-002 Visual honesty | River-support, spawn-occlusion, and cave-validity gates fail on the current bad scenes |
| 3 | P1-001 Safe spawn | Clear first view and reachable starter resources across seed set |
| 4 | P2-001 Owned hotbar | Remove the free terrain palette; show only carried/equipped items |
| 5 | P2-002 Action resolver | Mouse-driven tool actions and contextual prompts |
| 6 | P4-001 First tool loop | Gather → personal recipe → equip → use |
| 7 | P3-001 Grounded river | Shared terrain/water visibility and RiverCarve |
| 8 | P3-002 Hoe and shovel preview | Visible level/lower/raise simulation before commit |
| 9 | P4-002 Workbench and shelter | Readable station recipes and material-cost building |
| 10 | P5-001 Developer console | Registry, overlay, help, core inspection and movement cheats |

## 8. Protected qualities

- Keep the current menu's dark translucent panels, orange focus color, blocky
  type, seed preview, and explicit pause/quit flow unless user testing shows a
  specific failure.
- Keep Rust and the lightweight proprietary engine while it continues to meet
  frame-time and memory budgets on older hardware.
- Keep deterministic generation, bounded local updates, save compatibility
  decisions, and proof routes.
- Keep the original POORCRAFT/LOREFORGE project preserved as history.
- Keep the long-term hybrid identity: survival sandbox first, then situated
  RPG, settlements, realms, machines, magic, and empire.
