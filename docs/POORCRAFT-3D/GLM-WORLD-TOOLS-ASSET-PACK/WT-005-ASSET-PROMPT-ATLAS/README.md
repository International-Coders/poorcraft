# WT-005 Asset Prompt Atlas

This sub-pack is a production atlas for generating many original POORCRAFT 3D
assets without flooding the repo with fake props. It gives GLM/Z-code prompt
families, batch rules, naming taxonomy, UI sprite/key/bar contracts, glTF/GLB
import rules, PBR material rules, animation/rig expectations, texture
compression notes, and acceptance gates.

Use it after WT-002 and WT-003: assets must be gameplay-semantic and inspected.

## Files

- `PROMPT_TO_GLM.txt` - paste this into Z-code for WT-005.
- `00-ASSET-PRODUCTION-BRIEF.md` - purpose and constraints.
- `01-STYLE-IDENTITY-RULES.md` - original visual identity and source policy.
- `02-BUILDING-PROMPT-LIBRARY.md` - house, workshop, tavern, castle, bridge,
  road, mine, port, farm, shrine prompts.
- `03-NPC-CREATURE-PROMPT-LIBRARY.md` - people, factions, animals, enemies,
  bosses, rigs, dialogue anchors.
- `04-MACHINE-ITEM-PROMPT-LIBRARY.md` - forge, tools, machines, magic,
  industry, resources, loot.
- `05-UI-SPRITE-ICON-KEY-BAR-LIBRARY.md` - HUD bars, keycaps, icons, map
  markers, accessibility variants.
- `06-WORLD-PROP-RESOURCE-LIBRARY.md` - trees, rocks, ruins, harvestables,
  dungeon props, road props, environmental storytelling.
- `07-BATCH-GENERATION-RULES.md` - how to create hundreds of assets safely.
- `08-GLTF-BLENDER-IMPORT-RULES.md` - GLB/PBR/texture/import validation rules.
- `09-ACCEPTANCE-CHECKLIST.md` - done criteria.
- `10-FAILURE-MODES.md` - ways asset generation becomes junk.
- JSON contracts for prompt atlas manifest, prompt batches, naming taxonomy,
  UI sprites, glTF import, materials, animation rigs, texture compression, and
  batch acceptance.
