# Z-code Rules

GLM must obey these rules while using this pack.

## Core Loop

1. Pick one task from `zcode_world_tools_task_queue.json`.
2. Read the matching Markdown and JSON contracts.
3. Add or improve one tool, asset family, or gameplay-semantic slice.
4. Add tests and screenshot/wireframe/data proofs.
5. Inspect the outputs.
6. Update the pack if new rules are learned.

## No Decorative Assets

Every generated or authored asset must declare:

- gameplay purpose;
- interaction affordance;
- collision/nav behavior;
- required anchors/sockets;
- UI prompts;
- data export fields;
- screenshot proof scene;
- failure conditions.

## Required Proof Types

- Rendered screenshot.
- Wireframe or mesh summary.
- Semantic JSON dump.
- Interaction test.
- Performance row if the asset can appear many times.

## If A Tool Is Missing

Build the tool. Do not guess.

Examples:

- Need to know if a house has a door? Add an asset inspector.
- Need to know if a forge can be used? Add a gameplay probe.
- Need to know if AMD/NVIDIA markers exist? Add marker dump output.
- Need to know if a random seed preview is meaningful? Add seed-preview
  screenshots and diversity tests.
