# Upgrade Decision Framework

This document turns “keep improving the game” into useful engineering rather
than permanent churn. It applies whenever the idle agent is choosing between a
gameplay feature, system-depth pass, optimization, Rust dependency change, new
crate, renderer change, tooling improvement, asset pass, or platform effort.

## The admission test

A candidate job is eligible only when it can answer all of these:

1. **Player value:** what can a player do, perceive, understand, or trust after
   this change that they could not before?
2. **Evidence:** what current code, runtime behavior, profile, screenshot,
   failing test, owner report, or documented gap demonstrates the need?
3. **System connection:** which two or more layers does it strengthen—world,
   survival, NPCs, factions, settlements, magic, industry, UI, multiplayer,
   tools, performance, or portability?
4. **Bounded scope:** can one coherent vertical slice be implemented, tested,
   and shipped without leaving the repository red or half-migrated?
5. **Cost:** what are the CPU, GPU, memory, storage, startup, build-time, save,
   network, maintenance, and platform costs?
6. **Lore fit:** does the work preserve the canon locks and make Valdenmoor
   more legible rather than attaching a generic system with no world reason?
7. **Proof:** which automated tests, real gameplay route, screenshot/pixel
   analysis, benchmark, package inspection, or multi-client proof will decide
   whether the job succeeded?

If the only justification is “newer,” “popular,” “more realistic,” “more
content,” or “the model can build it,” record the idea without implementing it.

## Priority ladder

Use the first source that produces a real, unblocked job:

1. Reconcile an interrupted or dirty in-scope job without touching unrelated
   user work.
2. Resolve `STATE.md` blockers, then its `next_task`.
3. Fix a player-visible crash, broken control, corrupted save, authority bug,
   proof failure, or severe performance regression.
4. Complete an honestly deferred slice already named in the newest `DEVLOG.md`
   or `BACKLOG.md`.
5. Deepen an existing orphaned system so it becomes visible and useful in the
   first-person game.
6. Improve measured frame pacing, memory, streaming, loading, package size,
   network traffic, simulation cost, battery/thermal behavior, or build health.
7. Improve portability, controller/Deck use, platform isolation, headless
   server operation, or low-spec fallbacks.
8. Audit libraries and toolchain health; migrate only when evidence clears the
   dependency gate below.
9. Add original content only when it exercises existing systems and obeys the
   depth-before-catalog rule.

## Performance jobs

Optimization begins with a baseline, not a rewrite. Record the scene, seed,
hardware/backend when known, build profile, frame count, and relevant counters.
Prefer representative player journeys over microbenchmarks alone.

Measure the bottleneck that the job claims to improve:

- CPU frame and simulation p50/p95/max;
- GPU pass timings or the closest available vendor-neutral markers;
- allocations, resident memory, cache/chunk/mesh counts, and upload bytes;
- startup and world-load time;
- save size and save/load latency;
- network bytes, message counts, reconciliation, and interest-set size;
- binary/package size and feature contribution;
- battery/thermal behavior on Steam Deck or comparable low-power hardware.

After the change, repeat the same measurement as a before-and-after comparison.
Reject a speedup that breaks
determinism, save correctness, visual readability, input behavior, simulation
authority, lore, or supported platforms. If results are noise-sized, do not
claim a win.

## Rust dependency gate

Library review is welcome; dependency churn is not. Before adding, replacing,
or substantially upgrading a crate, write a short decision note containing:

- the exact problem in current code;
- the current crate/version and proposed crate/version;
- official documentation and release notes consulted;
- license compatibility and redistribution implications;
- maintenance activity, security advisories, unsafe-code exposure, and
  transitive dependency impact;
- MSRV/toolchain and target support, especially Windows, Linux, macOS, ARM64,
  Steam Deck, and headless server builds;
- compile-time, binary-size, memory, runtime, and API-complexity effects;
- migration scope, rollback plan, and representative tests/benchmarks;
- why a small local implementation is or is not safer.

Do not replace version-locked `wgpu`, `winit`, or `egui` components casually.
Their compatibility constraints are project law until a complete migration
branch proves the renderer, UI, screenshots, runtimes, and target builds.

Use the narrowest feature set. Disable unnecessary default features when that
is supported and measured. Run duplicate/transitive dependency inspection
before introducing a second implementation of the same concern.

## Creating a new Rust crate

A new in-repository crate is justified when it creates a stable, reusable
boundary with at least one of these benefits:

- removes a platform dependency from deterministic simulation or server code;
- allows a major optional system to compile out cleanly;
- isolates unsafe/FFI/platform code behind a small audited API;
- enables independent tests/benchmarks for a proven hotspot;
- prevents dependency-direction violations;
- provides a reusable data format or tool shared by client/server/asset tools.

Do not create a crate merely to shorten a file, mirror a design diagram, or
give one function a new package. Every new crate needs an owner, dependency
direction, feature policy, public API boundary, tests, and effect on compile
time and binary size.

## Portability and console-readiness

Portability is an architectural property, not a claim that unsupported
hardware was tested. The near-term verified targets are desktop platforms,
Steam Deck/Linux, and the dedicated server. Console readiness means keeping
the road open until licensed SDKs, hardware, accounts, and toolchains are
available.

Prefer:

- portable Rust and `wgpu` APIs over vendor-only runtime requirements;
- a raster baseline with optional expensive effects;
- explicit controller actions rather than keyboard-scancode game rules;
- platform services behind narrow traits/adapters;
- little-endian/versioned data formats with bounded decoding;
- asset compression and quality tiers chosen per target;
- headless simulation/server builds without window or GPU dependencies;
- no x86-only assumptions; preserve ARM64-friendly code and dependencies;
- deterministic fixed-step simulation independent of display refresh;
- feature flags that remove unavailable integrations cleanly.

Never claim a PlayStation, Xbox, Nintendo, mobile, or other console build
without the actual authorized SDK/toolchain, target compile, hardware run, and
platform compliance work. An abstraction or CI experiment is “readiness,” not
a shipped port.

## Gameplay and lore depth

A system is not complete because it has data structures or a menu. Close the
loop in the world. Examples:

- water becomes terrain-conforming flow, a wheel, visible power, a worker
  response, and a save/reload or multiplayer proof;
- faction standing becomes witnessed acts, dialogue, access, guards, trade,
  quests, territory, and chronicle consequence;
- a castle becomes gates, routes, residents, work, laws, defense, resources,
  faction identity, and player choices;
- magic becomes Anima manipulation with resource cost, physical feedback,
  faction interpretation, crafting/industry interaction, and counterplay;
- an asset becomes original source, validated export, LOD/collision/anchors,
  runtime use, interaction semantics, and real-window proof.

Prefer one completed loop over ten new catalogs.

## Stop and continuation rules

Each idle invocation may stop only at a green checkpoint: no half-migration,
red tests, staged unrelated files, or unreported proof failure. The invocation
must write a precise next task into `STATE.md` so another idle run can continue
cold.

There is no final design milestone after which improvement ends. If all named
work is complete, perform a bounded discovery audit across player experience,
performance, portability, correctness, accessibility, system depth, content
coherence, and maintenance. Ship the highest-value proven gap. If no safe,
valuable change can be justified, report that honestly; never generate filler
commits to simulate motion.
