# Screenshot Gates

WT-001 must add screenshot cases.

## Required PNGs

- `seed_preview_empty.png`
- `seed_preview_random.png`
- `seed_preview_typed.png`
- `seed_preview_rerolled.png`
- `seed_preview_deck_1280x800.png`
- `seed_preview_invalid_spawn_or_warning.png` if invalid state exists.

## Required Sidecars

Each screenshot needs:

- layout JSON;
- preview metadata JSON;
- pixel-check report JSON.

## Pixel Checks

- preview panel is nonblank;
- seed text is inside safe margins;
- spawn marker visible;
- create button enabled only when preview valid;
- no text overlaps preview map;
- warning strip visible for invalid preview;
- screenshot dimensions match requested viewport.

## Human Check

Open the PNG. If the map looks like random noise and tells the player nothing,
WT-001 is not done.
