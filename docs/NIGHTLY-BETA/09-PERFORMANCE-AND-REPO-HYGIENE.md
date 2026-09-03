# Performance, Folder Size, and Repository Hygiene

The source is small. The current size problem is overwhelmingly generated
build output. Cleanup must be evidence-based, recoverable, and separate from
gameplay changes.

## Baseline to refresh

At pack creation:

- `target/`: approximately 23 GB, ignored and rebuildable;
- `.git/`: approximately 858 MB (`git count-objects -vH` reported about
  413 MiB loose and 444 MiB packed objects);
- `shots/`: approximately 64 MB, 158 tracked files;
- `dist/`: approximately 48 MB, ignored and rebuildable;
- source/assets/docs/mods combined: only a few megabytes;
- `worlds/`: ignored player saves, never cleanup material.

Before acting, capture `du`, `git status`, tracked/untracked/ignored lists, and
the exact toolchain state. An agent must never interpret "untracked" as
"unused."

## Runtime performance audit

Measure before optimizing:

- raster and applicable RT frame p50/p95/min using `make perf`;
- chunk generation, meshing, lighting, atlas upload, minimap/map refresh;
- UI frame allocation and recipe filtering at full catalog size;
- NPC planner/pathfinding expansions and milliseconds for castle populations;
- save/load and autosave pause;
- memory after spawn, ten-minute travel, castle entry, and save/reload;
- binary/runtime artifact size.

Prefer counters and scoped timings behind F3 or tracing. Remove or feature-gate
temporary noisy instrumentation before shipping. Optimization acceptance is a
measured improvement or a necessary bound with no material visual/correctness
regression.

## Performance budgets

Calibrate exact numbers on the host; record device, resolution, preset, scene,
commit, and warm/cold state. Initial policy:

- no new per-frame O(world volume), full-atlas decode, or unbounded graph
  search;
- expensive world/UI caches rebuild only on relevant dirty state;
- NPC planning is time-sliced and visible in counters;
- streaming never blocks the render thread for long generation work;
- p95 matters; a good median cannot hide periodic multi-hundred-ms stalls;
- low/medium quality retains gameplay clarity, not merely fewer pixels.

## Safe cleanup ladder

### A. Identify

```bash
git status --short
git status --ignored --short
du -sh .git target dist shots assets crates mods docs worlds 2>/dev/null
git ls-files shots assets docs mods
git count-objects -vH
git clean -ndX
```

`git clean -ndX` is preview evidence only. Do not turn it into a broad delete;
it includes precious ignored `worlds/` and may include active work.

### B. Build output

`target/` is the main reclaimable category. If disk pressure justifies losing
incremental caches, use Cargo's own scoped clean command after recording size
and ensuring no build is running. Never use a recursive delete with a broad or
expanded environment-variable target. Rebuild and rerun the required tests
afterward. A cleanup that makes the next build impossible is not complete.

`dist/` contains ignored deliverables. Replace old artifacts only as part of
`make runtimes`; verify fresh files before considering the old set obsolete.

### C. Tracked screenshots

Classify every `shots/` file:

- canonical current `vistest_<scene>.png` with a registered scene;
- documentation/release evidence referenced by a tracked file;
- historical milestone evidence;
- exploratory/manual/audit duplicate with no consumer;
- stale proof contradicted by current behavior.

Generate a CSV/Markdown inventory with path, bytes, git tracked, code scene,
documentation references, newest equivalent, and recommendation. Remove only
proved duplicates/stale manual outputs in a dedicated cleanup commit. Keep at
least the canonical output for every registered scene. Re-run vistest after
pruning so required images are regenerated and visible.

### D. Assets and code

- Asset is used only when a registry/manifest/code path consumes it; a textual
  filename mention is not enough.
- Code/data entry is dead only after call/reference search, feature/build
  matrix, tests, and runtime registration audit.
- Spawn-or-cut dormant entities such as existing audit items in a gameplay job;
  do not silently delete design data to improve counts.
- Remove unused dependencies only after workspace and feature builds prove it.
- Preserve save migrations and serialized variants even if current code rarely
  constructs them.

### E. Git objects

First list largest reachable blobs and determine whether tracked binaries or
proof history explains `.git/`. Normal `git gc` may compact objects after a
clean pushed checkpoint. Never rewrite published history, strip blobs, delete
`.git`, expire reflogs aggressively, or force-push without a separate explicit
user decision and backup plan.

## Protected paths and data

Never delete or test-write into:

- `worlds/` or any player save;
- unrelated dirty/untracked files;
- `.git/` internals;
- hand-authored assets lacking replacement approval;
- current runtime artifacts before fresh replacements exist;
- state/bookkeeping evidence needed to understand prior loops.

## Hygiene job acceptance

- before/after byte counts by category;
- explicit file inventory attached to the commit;
- no player save changes;
- no unrelated dirty-file changes;
- build and full tests green after cleanup;
- canonical vistest scenes still render;
- fresh runtimes verified when build artifacts were cleared;
- removed tracked items listed in DEVLOG with why and recovery through Git;
- no claim that ignored cache deletion improved repository clone size.
