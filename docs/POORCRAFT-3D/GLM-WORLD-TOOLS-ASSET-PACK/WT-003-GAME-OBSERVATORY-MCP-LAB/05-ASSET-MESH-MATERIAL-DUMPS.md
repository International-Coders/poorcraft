# Asset Mesh And Material Dumps

WT-003 requires GLM to export facts about assets, not just images.

## Mesh Dump Fields

- `asset_id`;
- `source_path`;
- `generated_by`;
- `vertex_count`;
- `triangle_count`;
- `submeshes`;
- `bounds`;
- `lods`;
- `wireframe_edge_count`;
- `collision_shapes`;
- `nav_shapes`;
- `anchors`;
- `material_slots`.

## Material Dump Fields

- `material_id`;
- `semantic_role`;
- `base_color`;
- `alpha_mode`;
- `emissive`;
- `roughness`;
- `metallic`;
- `texture_paths`;
- `atlas_index`;
- `ui_alpha_tested`;
- `interactive_importance`.

## Consistency Checks

- every material slot has a known material id;
- every required interaction anchor is inside or near asset bounds;
- collision and nav shapes are finite;
- triangle counts fit budget;
- LOD0 and LOD1 use compatible anchor locations;
- material ids agree with the rendered material overlay.
