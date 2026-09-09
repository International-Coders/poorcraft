# GLM UI Rework Pack

Date: 2026-09-09

This folder is a drop-in handoff pack for GLM 5.3 / Z-code sessions working on
POORCRAFT 3D. The current owner alpha is not acceptable as an alpha UI: it has
a temporary bitmap text overlay, clipped debug HUD text, no real bars, no
hotbar, no title menu, no save-slot UI, no settings screen, and weak screenshot
automation around the owner-facing path.

The pack is intentionally strict. GLM is authorized to improve the game, create
debug modules, add local MCP-style inspection tools, create instrumentation
plugins, export runtime data, and add screenshot scenes. It is not authorized
to claim a UI task is done without build, test, screenshot, and human-readable
evidence.

## Start Here

1. Read `GLM_SYSTEM_PROMPT.txt`.
2. Paste `FIRST_MESSAGE_TO_GLM.txt` into the Z-code session.
3. Keep `DO_NOT_CLAIM_DONE_UNTIL.txt` visible while working.
4. Execute `09-IMPLEMENTATION-QUEUE.md` in order unless the owner overrides.
5. Use the JSON files as machine-readable contracts, not decoration.

## Folder Map

- `00-OWNER-REALITY-CHECK.md` — why the current UI is not acceptable.
- `01-GLM-OPERATING-RULES.md` — mandatory behavior for GLM/Z-code.
- `02-UI-REBUILD-SPEC.md` — full runtime UI architecture target.
- `03-HUD-HOTBAR-SPEC.md` — exact gameplay HUD requirements.
- `04-CONTROLS-AND-MOUSE-SPEC.md` — mouse, Escape, keys, rebinding.
- `05-SCREENSHOT-AND-PLAYTEST-PROTOCOL.md` — proof loop for every UI task.
- `06-MCP-GAME-INSPECTOR-SPEC.md` — local inspector/MCP design.
- `07-DATA-EXTRACTION-AUTHORIZATION.md` — what debug data may be exported.
- `08-ASSET-PIPELINE.md` — how to use generated assets safely.
- `09-IMPLEMENTATION-QUEUE.md` — ordered vibe-code work queue.
- `10-FAILURE-MODES.md` — common ways agents fool themselves.
- `11-ACCEPTANCE-GATES.md` — gates for calling the UI usable.
- `12-SESSION-LOG-TEMPLATE.md` — required per-session log format.
- `13-BASELINE-SCREENSHOT-NOTES.md` — current screenshot audit.

## Machine Contracts

- `glm_ui_rework_manifest.json`
- `ui_acceptance_gates.json`
- `mcp_game_inspector.schema.json`
- `telemetry_contract.json`
- `screenshot_scenes.json`
- `ui_strings.en.json`
- `zcode_task_queue.json`
- `data_exports.json`

## Baseline

The inspected baseline image is:

- `baseline/current-windowed-slice-showcase.png`

The top-left debug text is clipped and unreadable. This image should be treated
as a failure baseline, not a success proof.
