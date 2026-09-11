# glTF, Blender, And Import Rules

This file turns official asset-delivery guidance into POORCRAFT rules.

## Official Sources To Check

- Khronos glTF 2.0 specification:
  https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html
- Khronos glTF overview:
  https://www.khronos.org/gltf/
- Khronos glTF PBR:
  https://www.khronos.org/gltf/pbr
- Khronos KTX:
  https://www.khronos.org/ktx/
- Khronos glTF Validator:
  https://github.com/KhronosGroup/glTF-Validator
- Blender glTF 2.0 manual:
  https://docs.blender.org/manual/en/4.4/addons/import_export/scene_gltf2.html
- Blender command-line arguments:
  https://docs.blender.org/manual/en/dev/advanced/command_line/arguments.html

## POORCRAFT Import Rules

- Prefer `.glb` for runtime delivery because it carries scene data and binary
  buffers together.
- Use glTF-style PBR material metadata: base color, metallic, roughness,
  normal, occlusion, emissive, alpha mode.
- Use node extras/custom properties for POORCRAFT anchors if the import path
  supports them; otherwise pair the asset with a sidecar JSON.
- Run a validator report for generated glTF/GLB assets.
- Blender background mode is allowed for automated conversion/render proof.
- KTX 2.0/BasisU texture compression is a later optimization path; start with
  PNG proof assets, then add compressed runtime variants when budgets demand it.

## Hard Rejections

- no invalid quaternions, NaN bounds, missing buffers, missing material ids, or
  unknown texture references;
- no embedded branded content;
- no asset accepted only because Blender can open it;
- no runtime import without POORCRAFT semantic sidecar or node extras.
