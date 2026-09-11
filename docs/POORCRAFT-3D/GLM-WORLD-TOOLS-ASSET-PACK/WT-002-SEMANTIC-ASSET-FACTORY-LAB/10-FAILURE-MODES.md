# Failure Modes

Reject the work if any of these happen:

- "asset pack" means screenshots only;
- generated buildings cannot be entered;
- NPCs are meshes with no interaction;
- machines animate but cannot process input/output;
- GLM uses a static mock instead of a runtime scene;
- screenshot filenames exist but are old or from the wrong build;
- wireframe is simulated in 2D instead of derived from mesh data;
- sidecar JSON omits build hash, seed, viewport, command, or asset id;
- GPU optimization is claimed without markers and before/after data;
- vendor SDK integration is promised without checking platform/API support;
- assets copy another game's names, silhouettes, or protected identity;
- tests only parse JSON and never reject bad semantic data.
