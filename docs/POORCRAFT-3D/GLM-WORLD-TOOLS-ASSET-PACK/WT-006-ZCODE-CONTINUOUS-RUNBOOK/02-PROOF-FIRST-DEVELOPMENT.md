# Proof-first Development

Every claim needs evidence from the current build.

## Required Evidence Types

- unit tests for pure logic;
- integration tests for state changes;
- screenshot PNG for UI/visual changes;
- wireframe PNG for mesh/asset claims;
- layout JSON for UI claims;
- runtime state JSON for interaction claims;
- perf JSON for optimization claims;
- route transcript JSON for input/playability claims.

## Success Language

Allowed:

- "passed";
- "not proven";
- "failed with this reason";
- "deferred with this exact next step."

Forbidden:

- "should work";
- "looks good" without screenshot path;
- "optimized" without before/after data;
- "asset created" without import/runtime evidence.
