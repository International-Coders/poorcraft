# Interaction Anchor Specs

Anchors are named points or volumes that connect art to gameplay. GLM must
export anchors for every accepted asset.

## Universal Anchor Fields

- `id`: stable asset-local identifier;
- `kind`: door, talk, forge_input, forge_output, loot, bed, nav_entry,
  workspot, fuel, power_port, fluid_port, climb, seat, sign, quest, spawn;
- `position_m`: local-space meters;
- `rotation_y_deg`: facing direction;
- `radius_m`: interaction radius;
- `requires_clearance`: true for doors, nav, climb, player workspots;
- `runtime_action`: command or system that consumes the anchor.

## Required Anchors By Asset Type

- house: `door.main`, `nav.entry`, `interior.center`, `bed.optional`,
  `storage.optional`, `light.optional`;
- NPC: `talk.front`, `look_at.head`, `trade.optional`, `follow.socket`,
  `combat.target`, `quest.optional`;
- forge: `forge.input`, `forge.output`, `forge.heat`, `workspot.anvil`,
  `fuel.input`, `smoke.exit`, `danger.hot_surface`;
- chest: `loot.open`, `hinge.visual`, `collision.closed`, `collision.open`;
- bridge: `nav.entry_a`, `nav.entry_b`, `support.left`, `support.right`,
  `fall_risk.optional`;
- door: `use.handle`, `hinge`, `nav.closed_blocker`, `nav.open_passage`.

## Anchor Proofs

The inspector must render anchor names and volumes. The playtest must attempt
at least one action through each required anchor class in the batch.
