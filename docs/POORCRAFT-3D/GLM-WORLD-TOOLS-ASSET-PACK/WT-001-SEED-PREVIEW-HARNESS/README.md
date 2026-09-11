# WT-001 Seed Preview Harness

This sub-pack turns the broad world/tools rules into a concrete first
implementation job: a screenshotable start-menu New World seed preview harness.

The goal is not a decorative menu mockup. The goal is a deterministic seed tool
that a player and GLM can both inspect:

- enter seed;
- randomize/reroll seed;
- render map preview;
- export JSON metadata;
- screenshot the screen;
- prove same seed is stable;
- prove different seed changes the map;
- prove spawn safety;
- prove preview metadata agrees with worldgen.

## Files

- `PROMPT_TO_GLM.txt` — paste this into Z-code for WT-001.
- `00-IMPLEMENTATION-BRIEF.md` — exact job scope.
- `01-RUST-API-SKETCH.md` — proposed Rust modules and functions.
- `02-START-MENU-WIREFRAME.md` — UI layout and state.
- `03-PREVIEW-ALGORITHM.md` — seed preview data algorithm.
- `04-SCREENSHOT-GATES.md` — screenshot scenes and pixel checks.
- `05-ZCODE-COMMANDS.md` — commands GLM should add or run.
- `06-FAILURE-MODES.md` — things that make WT-001 fake.
- `07-ACCEPTANCE-CHECKLIST.md` — done criteria.
- `seed_preview_execution_plan.json` — machine-readable implementation plan.
- `seed_preview_ui_wireframe.json` — UI element contract.
- `seed_preview_outputs.schema.json` — required output sidecar schema.
- `seed_preview_test_matrix.json` — required tests and screenshot cases.
