# MCP Game Inspector Spec

GLM is authorized to create a local inspector interface so agents can query and
control the game during UI work.

## Purpose

The inspector exists to answer:

- What screen is active?
- Is the pointer captured?
- What UI rectangles were drawn?
- Which controls are currently bound?
- What bars/hotbar/state values are being displayed?
- What world/camera/player state is visible?
- What GPU/frame timing happened?
- Which screenshot was just written?

## Command Shape

Use JSON request/response commands. A local CLI is acceptable first; a true MCP
server can replace it later.

Minimum commands:

- `ui_state`
- `input_state`
- `set_screen`
- `set_debug_hud_values`
- `capture_screenshot`
- `dump_ui_layout`
- `dump_frame_stats`
- `dump_visible_world`
- `dump_mesh_wireframe`
- `replay_input`

## Safety

Inspector commands must never modify real owner saves unless an explicit test
save directory is passed.
