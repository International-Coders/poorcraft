# Screenshot And Playtest Protocol

Every UI task must add or reuse a screenshot scene. The scene must be runnable
without manual clicking.

## Required Commands

- `cargo build --release --manifest-path poorcraft3d/Cargo.toml -p poorcraft3d`
- `cargo test --manifest-path poorcraft3d/Cargo.toml -p <touched-crate>`
- a screenshot command that writes PNGs under `poorcraft3d/apps/poorcraft3d/shots/`
- `make p3d-dmg` after owner-facing runtime changes

## Required Screenshot States

- title screen;
- title hover/focus state;
- new world screen;
- load world screen;
- settings screen;
- gameplay HUD;
- gameplay HUD with debug non-full bars;
- pause menu;
- confirmation modal;
- debug inspector overlay;
- Steam Deck 1280x800 layout.

## Pixel Checks

Each screenshot scene must assert:

- nonblank world or panel pixels;
- text is inside safe margins;
- HUD elements do not overlap;
- at least one alpha-blended UI element is present;
- selected/focused state differs from normal state;
- UI remains readable over bright sky and dark terrain.

## Human Inspection

After pixel checks pass, open the PNG and write a one-paragraph note. If it
looks awful, the task is not done.
