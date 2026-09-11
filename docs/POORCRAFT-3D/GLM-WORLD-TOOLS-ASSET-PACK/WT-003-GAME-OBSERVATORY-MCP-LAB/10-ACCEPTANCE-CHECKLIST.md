# Acceptance Checklist

WT-003 is done only when:

- [ ] observatory contracts parse;
- [ ] screenshot command writes PNG + state JSON;
- [ ] wireframe command writes PNG + sidecar JSON;
- [ ] overlay command labels bounds/anchors/materials;
- [ ] asset dump includes mesh/material/bounds/LOD/collision/nav/anchors;
- [ ] input replay can drive title, New World, Escape, and at least one world
      interaction route;
- [ ] runtime state export includes player, UI, world, visible assets,
      interactions, NPCs, machines, perf, and GPU markers;
- [ ] GPU marker audit verifies pass names;
- [ ] comparison command can produce pass/fail/inconclusive verdicts;
- [ ] every output includes build hash, command, seed, viewport, scene, and
      proof paths.
