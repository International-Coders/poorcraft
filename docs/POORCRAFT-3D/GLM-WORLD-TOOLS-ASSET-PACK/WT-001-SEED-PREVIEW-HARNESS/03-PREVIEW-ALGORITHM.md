# Preview Algorithm

The preview must be deterministic and honest.

## Minimum Preview Layers

- Height: color ramp or shaded relief.
- Water: rivers/coasts/low water areas.
- Biome: compact palette.
- Spawn: marker and safety label.
- Features: at least nearest known hints when available.

## Sampling

Start with a fixed square around the origin or chosen spawn search area. Sample
the same worldgen authority used by runtime. Do not use unrelated preview noise.

## Seed Resolution

- Empty seed + random button: use entropy and show resolved seed.
- Text seed: stable hash.
- Numeric seed: exact numeric parse.
- Reroll: new seed not equal to current seed.

## Spawn Safety

Spawn safety must check at least:

- not water;
- not inside solid;
- slope below threshold;
- enough nearby walkable area;
- not inside a known blocked structure footprint.

## Honesty

If a layer is placeholder, mark it as placeholder in the JSON sidecar and UI.
