# Local Plugin Architecture

Start with simple shapes. A true plugin ABI can come later.

## Shapes

- CLI command: safest first step;
- dev-only module behind a feature flag;
- TOML/JSON mod sample pack;
- loopback-only MCP-style server;
- file watcher for generated evidence folders;
- offline validator that reads manifests and screenshots.

## Boundaries

- plugin code may inspect game state;
- plugin code may write explicit output folders;
- plugin code may create temp worlds;
- plugin code must not silently alter player saves;
- plugin code must not send data externally by default.

## Versioning

Every export includes:

- schema version;
- build hash;
- command;
- plugin id;
- scene;
- seed;
- viewport;
- timestamp;
- verdict.
