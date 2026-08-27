# LOREFORGE Mods — the full authoring guide

Mods are plain TOML folders under `mods/` (or installed Workshop items —
see [UGC](#ugc--workshop)). They load at boot on both the client and the
dedicated server. Everything a mod can do is data; no code, no build.

## Quick start: scaffold one

```bash
cargo run -p xtask -- new-mod my_pack --name "My Pack"
# -> mods/my_pack/  (manifest + blocks + items, ready to edit)
```

## Is loading working? Check the smoke test

`mods/smoke_test/` registers one block and one item and nothing else.
When it loads, boot prints one unmissable line:

```
[MOD SMOKE TEST] OK — smoke_test mod loaded successfully
```

If that line is missing while the folder exists, loading is broken. A CI
test (`smoke_test_mod_loads_from_the_real_folder` in lf_modapi) keeps it
honest.

## Layout

```
mods/my_mod/
  mod.toml            # manifest (below)
  data/
    blocks.toml       # [[blocks]]
    items.toml        # [[items]]
    smelting.toml     # [[smelting]] input, output, xp
```

## Manifest (mod.toml)

```toml
id = "my_pack"              # your namespace, unique, no colons
name = "My Pack"
version = "1.0.0"
api_version = "1"           # the loader refuses mismatched majors
side = "both"               # both | client | server
dependencies = ["core"]
permissions = ["world.read", "world.write"]
```

## Blocks

```toml
[[blocks]]
id = "my_pack:glowing_banner"   # always namespaced
name = "Glowing Banner"
texture = "glowing_banner.png"  # declared name (see Textures)
hardness = 0.6                  # seconds by hand
harvest_level = 0               # 0 hand, 1 stone, 2 iron, ...
light = 12                      # 0..15 — REALLY emits light
```

- Runtime ids are stable: `100 + fnv1a(namespace:block)`.
- Mod blocks are solid, opaque, minable, and drop their item form.
- `light` reaches the light engine (registered blocks emit; the
  decoration pack's glowing banner is the living test).
- Names ending in `_ore` become worldgen veins (y 8..50, rare).

## Items & smelting

```toml
[[items]]
id = "my_pack:carving_kit"
name = "Carving Kit"

[[smelting]]
input = "my_pack:raw_amber"
output = "my_pack:amber_ingot"
```

Items are holdable, stackable, tradeable (protocol v4 escrow applies to
any id), and smeltable in any furnace.

## Decoration packs

A mod with only decorative blocks IS a decoration pack — see
`mods/decor_pack/` (banner with light, plinth, rug). There is no special
mode: declare blocks, set `light`, keep hardness low. Build with them,
run them through the chisel/enchanting economy like any block.

## UGC & Workshop

Workshop items are mod folders. On Steam they download into the UGC
store (feature-gated integration, `docs/STEAM.md`); everywhere else they
live in `ugc/` at the game root. Both are scanned by the same code
(`lf_steam::workshop::scan_installed`) and load exactly like bundled
mods. To install by hand: drop the folder into `ugc/`.

## Transport & multiplayer

Mods load identically on the dedicated UDP server (`loreforge-server`).
Block ids are deterministic across peers (same fnv1a mapping), so
modded worlds stay in sync; the server validates ids against the same
registry and drops unknown ones.

## Gate interactions

Vanilla gates (era / path standing) apply to vanilla recipes only. Mod
items are ungated by design — pack authors curate their own progression.

## Testing your mod

```bash
cargo test -p lf_modapi     # loader pipeline (parse/register/place/break/smelt)
cargo run --release -p loreforge   # watch for the smoke line + your blocks
```

The catalog consistency test catches dangling references (a mod item
pointing at a missing block, etc.).
