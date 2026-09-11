# Tools And Proof Gates

Every feature category must have its own proof gate.

## Required Tool Families

- seed preview screenshot and metadata;
- observatory runtime state export;
- asset factory validation;
- semantic playtest routes;
- screenshot/wireframe/overlay capture;
- perf and GPU marker audit;
- save/load round trip;
- mod validation;
- multiplayer local integration;
- accessibility/UI layout checks.

## Gate Shape

Each gate writes:

- command line;
- build hash;
- seed;
- viewport;
- input route;
- screenshots;
- JSON state;
- verdict;
- failure reason if not pass.

## Owner Rule

If GLM cannot show files proving it, GLM should say "not proven yet" and keep
working.
