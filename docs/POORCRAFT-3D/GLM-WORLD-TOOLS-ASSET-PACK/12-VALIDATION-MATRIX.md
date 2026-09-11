# Validation Matrix

Use this matrix for every asset/tool/world-menu task.

## Asset Validation

- Manifest row exists.
- Generated/authored source exists.
- Asset loads.
- Triangle budget enforced.
- Sockets/anchors exported.
- Collision matches visible shape.
- Nav openings match doors/paths.
- Screenshot proof exists.
- Wireframe proof exists.
- Gameplay semantic law passes.

## Start Menu Validation

- Title visible.
- Build hash visible.
- Random seed button works.
- Seed text field stable.
- Preview map changes with seed.
- Create world writes metadata.
- Load world reads metadata.
- Settings reachable.

## GPU Validation

- Frame stats JSON exists.
- Pass markers are named.
- UI pass separately identifiable.
- Scene-without-UI and UI alpha/color architecture is planned or implemented.
- Pause/menu/loading disables frame-generation path if it exists.

## Inspector Validation

- JSON responses parse.
- Commands never touch real saves unless explicitly requested.
- Screenshot sidecars point to real files.
- Mesh/wireframe bounds are finite.
