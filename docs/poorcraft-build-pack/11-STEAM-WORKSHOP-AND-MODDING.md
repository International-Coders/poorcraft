# Steam Workshop & Mod-Friendliness — Detail for Steps 37–39

## What already exists (don't rebuild this)

`lf_modapi` already has runtime registries for mod blocks, items, recipes,
and smelting, plus worldgen ore-vein hooks (`*_ore` auto-registration
found real per the P25 audit). The client already loads `mods/` at boot,
with `ember_ores` and `amberium` as working examples, and a full-pipeline
test (parse → register → place → break → smelt) already proves the
runtime side works. This stage adds two things on top: **Steam Workshop
distribution**, and **making mod authorship easy for a person or an AI
coding session with zero other context.**

## Step 37 — Workshop upload/download pipeline

- Use the Steamworks UGC API (`ISteamUGC`, via the same `steamworks`
  crate already gated behind the `steam` feature) to:
  - Package an existing `mods/<name>/` folder (its TOML files + any
    textures) into a Workshop item and upload it.
  - On client boot, in addition to loading local `mods/`, check for and
    load subscribed Workshop items, placing their content into the same
    runtime registries local mods use — a Workshop mod and a local mod
    should be indistinguishable to `lf_modapi` once loaded.
- Since a real public Workshop upload can't be exercised in CI, use a
  local UGC sandbox / mock for automated testing, and document a real
  manual upload-and-resubscribe test for verification before considering
  this step done, the same testing-reality caveat as Step 34–36.

## Step 38 — Mod manifest and API documentation rewrite

This is the "mods need to be a very easy way for the AI or people to
develop it" ask, made concrete. Rewrite `mods/README.md` from scratch as
a complete reference, structured so an AI coding assistant with *no other
context on this repo* could read only this file and produce a working
mod:

- **Every TOML field documented**, for each of: blocks, items, recipes,
  smelting, ore veins — field name, type, whether required, default
  value, and a one-line example value.
- **One complete minimal example per content type** — a full, valid,
  copy-pasteable block definition, item definition, recipe, smelting
  entry, and ore-vein hook, each short enough to read in one glance.
- **One complete "small but real" example mod** (a step up from the
  single-block minimal examples) — ideally documenting exactly what
  `ember_ores` or `amberium` already does, annotated, so the existing
  working examples double as the documentation's own proof that the
  instructions are accurate.
- **A "common mistakes" section** aimed at exactly the kind of error an
  AI assistant or new modder makes: ID collisions with base-game blocks,
  missing texture references, recipe ingredient IDs that don't exist yet,
  forgetting to register a smelting output.
- **Concrete Done check**: hand only this rewritten file (no other repo
  context) to a fresh AI session or a person unfamiliar with the project,
  ask them to write a small mod, and confirm it passes the existing
  full-pipeline test on the first try.

## Step 39 — Mod scaffolding tool

- Add an `xtask new-mod <name>` command that generates a starter
  `mods/<name>/` folder with one working example block, item, and recipe
  already filled in and internally consistent (valid IDs, a texture
  placeholder, a smelting entry if relevant) — a known-good starting
  point instead of a blank folder or a copy-paste-and-edit workflow.
- This directly serves both audiences named in the request: a person
  gets a template to edit instead of guessing the schema from scratch,
  and an AI coding session gets a deterministic, testable starting point
  it can extend rather than hallucinate.
- **Done check**: running the scaffold command produces a mod that loads
  without errors and passes the full-pipeline test immediately, before
  any manual editing.

## Why documentation and scaffolding come before more mod *features*

The runtime mod system is already real and tested (per the P25 audit).
The gap isn't capability, it's approachability — Steps 38 and 39 exist
because the fastest way to get more, better mods (including from future
AI coding sessions working on this exact repo) is to make the on-ramp
obvious, not to add more registries before anyone can easily use the ones
that exist.
