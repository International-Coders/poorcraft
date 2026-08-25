# LOREFORGE Mods

Mods live in directories under `mods/` and load at boot. The two bundled
examples (`ember_ores`, `amberium`) demonstrate the full surface.

## Layout
```
mods/my_mod/
  mod.toml            # manifest: id, name, version, api_version, side, dependencies, permissions
  data/
    blocks.toml       # [[blocks]] id, name, texture, hardness, harvest_level, light
    items.toml        # [[items]]  id, name
    smelting.toml     # [[smelting]] input, output, xp
```

## What registers where
- **Blocks** get a stable runtime id (`MOD_BLOCK_BASE + fnv1a(namespace:block)`)
  and become solid, opaque, minable blocks that drop their item form.
- **Items** join the live item registry (holdable, stackable, smeltable).
- **Smelting** entries map input items to outputs in any furnace.
- **Ore veins**: blocks whose name ends in `_ore` are auto-registered as
  worldgen veins between y=8..50 (threshold 0.62).

## ids
Use your namespace everywhere: `ember_ores:ember_ingot`. Vanilla ids are the
plain names from the catalog (`stone`, `iron_ingot`, ...).

## Testing
`cargo test -p lf_modapi` covers the whole pipeline: parse -> register ->
place a modded block in a world -> break it -> smelt the drop.
