# Asset Pipeline

Generated assets live in `docs/POORCRAFT-3D/assets/generated/`. They are source
concept sheets and slice references, not proof that runtime UI exists.

## Required Runtime Steps

1. Decide whether each component should be image-backed or renderer-native.
2. Slice image-backed assets into stable named runtime files or atlas regions.
3. Put runtime assets under `poorcraft3d/assets/ui/`.
4. Keep a manifest that maps names to file/atlas coordinates.
5. Load assets through a tested catalog.
6. Draw text, counts, binding labels, cooldowns, and bar fills in-engine.
7. Screenshot every UI state that uses the assets.

## Asset Families

- Logo: title/menu brand only.
- HUD bars: frame and fill material reference.
- Key glyphs: keycap frame and mouse art reference.
- Action icons: action vocabulary and hotbar icons.
- Resource icons: inventory/crafting icons.
- Panel frames: title, pause, settings, modal, tooltip, toast.
- Faction strategy: minimap, realm, diplomacy, war, settlement markers.

## Bad Outcomes

- A beautiful generated sheet sits in docs and the game still has clipped text.
- Key labels are baked into art and become wrong after rebinding.
- Bars are full PNGs instead of live values.
- UI works at 1920x1080 but overlaps on Steam Deck.
