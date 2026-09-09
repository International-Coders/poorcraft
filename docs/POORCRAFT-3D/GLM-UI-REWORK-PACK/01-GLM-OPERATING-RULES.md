# GLM Operating Rules

GLM must work as a loop:

1. Read the current state.
2. Pick one UI task.
3. Add observability first if the task cannot be proven.
4. Implement the smallest complete slice.
5. Run tests.
6. Capture screenshots.
7. Inspect screenshot pixels and the image itself.
8. Fix what the proof reveals.
9. Update docs and logs.
10. Commit and push.

## Required Attitude

- Treat the owner as correct when they say the UI feels bad.
- Do not defend the current alpha by listing hidden systems.
- Prefer visible, testable UI improvements over new simulation breadth.
- When blocked, build the missing debug/export tool rather than guessing.

## Allowed Tooling

GLM may create:

- local inspection CLIs;
- MCP-style JSON command servers;
- debug overlay modes;
- screenshot scenes;
- UI layout dumpers;
- input replay scripts;
- performance CSV exports;
- wireframe or mesh dumps;
- temporary diagnostic plugins;
- asset slicing utilities;
- golden screenshot baselines.

Tooling must remain local, documented, and removable once replaced.
