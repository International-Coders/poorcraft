# Dual-scale biome layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make POORCRAFT 3D biomes read as wide climate countries with compatible enclaves, at today's map size.

**Architecture:** Stretch temperature/humidity FBM wavelengths for continental belts; add a faster pocket noise that overrides `biome(region)` to a compatible neighbor when strong. Keep elevation continuity and the existing `Biome` enum.

**Tech Stack:** Rust, `pc3d_world::gen`, existing FNV lattice noise.

**Spec:** `docs/superpowers/specs/2026-09-28-biome-layout-design.md`

## Global Constraints

- Deterministic: seed + integer coords only.
- No new biome enum values this pass.
- Do not grow stream radius / atlas size this pass.
- Ocean/Coast/SnowPeaks elevation/cold invariants must still hold.
- Soft edge blends are out of scope.

## File map

- Modify: `poorcraft-novo/crates/pc3d_world/src/gen.rs` — `macro_field`, `biome`, new helpers, new tests
- Touch later (not this plan's first commit): flora densities / detail atlas, windowed proofs
- Docs: spec above; DEVLOG entry after green tests

---

### Task 1: Belt + pocket laws (failing tests)

**Files:**
- Modify: `poorcraft-novo/crates/pc3d_world/src/gen.rs` (tests module)
- Test: same file

**Interfaces:**
- Consumes: `WorldGen::biome`, `WorldGen::macro_field`
- Produces: `p3d103_climate_belts_are_countries`, `p3d103_compatible_pockets_exist`

- [ ] **Step 1: Write the failing belt test** — inland land region shares biome with ≥60% of a ±6 neighborhood for several seeds.
- [ ] **Step 2: Write the failing pocket test** — over a wide sweep, count regions whose biome differs from the mode of their 8-neighbors; require a minimum count of compatible enclaves.
- [ ] **Step 3: Run tests — expect FAIL** before implementation.

### Task 2: Dual-scale fields + pocket override

**Files:**
- Modify: `poorcraft-novo/crates/pc3d_world/src/gen.rs`

**Interfaces:**
- Consumes: existing `fbm` / `value_noise`
- Produces: `biome(region)` using continental climate + `pocket_biome` override; `biome_of` remains threshold table for a field (may gain `base_biome_of` alias)

- [ ] **Step 1: Slow climate wavelengths** for temp/humidity in `macro_field` (larger FBM cells than elevation).
- [ ] **Step 2: Pocket noise + compatible override** in `biome(region)`.
- [ ] **Step 3: Adjust coherence test** so Forest/Plains/Wetland allow pocket exceptions; keep Ocean/Coast/SnowPeaks/Mountains/Highlands elevation rules.
- [ ] **Step 4: Run `cargo test -p pc3d_world --lib gen`** — all green.

### Task 3: Bookkeeping

- [ ] **Step 1: DEVLOG entry** for the job.
- [ ] **Step 2: Leave assets/windowed proofs** for the next slice once layout is proven.
