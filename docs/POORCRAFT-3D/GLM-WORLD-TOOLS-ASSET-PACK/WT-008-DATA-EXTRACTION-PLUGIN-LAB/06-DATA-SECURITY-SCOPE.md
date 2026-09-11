# Data Security And Scope

Extraction tools are powerful. Keep them scoped.

## Scope Rules

- local filesystem output only;
- explicit output directories;
- no public network listener;
- no secrets;
- no unrelated home-directory scanning;
- temp worlds for tests;
- player saves read-only unless the owner asks otherwise;
- every mutation is logged.

## Privacy Fields

Do not export:

- OS username unless already part of the explicit path;
- environment variables;
- tokens;
- unrelated file paths;
- network addresses beyond loopback.

## Failure

If a tool cannot guarantee scope, it must refuse to run.
