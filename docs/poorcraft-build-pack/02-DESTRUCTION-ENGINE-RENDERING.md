# Destruction, Engine & Rendering — Detail for Steps 3–11

## Why this is Stage B, right after the audit

You called the engine "a little bit too vague, too bad" and specifically
called out block breaking as not implemented in feel. This is the single
highest-leverage fix in the whole plan: a voxel game's core verb is
breaking and placing blocks, and if that doesn't feel good, nothing built
on top of it will land, no matter how many biomes or magic systems exist.

## Block-break feedback (Steps 3–4)

### Cracking overlay
- A block being actively mined should show a progressively-more-cracked
  texture overlay (typically 8–10 stages) composited on the block's
  faces, tied to `progress = elapsed_mining_time / total_time_for_tool`.
- This overlay needs its own texture-array layer(s) and a shader pass (or
  a fragment blend) distinct from the block's own texture — check
  `lf_assets` for where the existing texture atlas is built and add the
  crack stages there.

### Particle burst on break
- On block destruction, spawn a short-lived particle burst that samples
  color from the broken block's actual texture (not a generic gray/brown
  particle) — this is what makes stone breaking look different from wood
  breaking without needing bespoke per-block particle art.
- Reuse the existing item-drop entity's physics (gravity) as a base for
  particle motion if a lighter-weight particle system doesn't already
  exist — don't build a second physics system.

### Screen feedback on heavy impacts
- A very short screen-shake or camera punch on breaking especially hard
  blocks (or on tool-durability-breaking events) — subtle, not
  disorienting. This is a cheap, high-perceived-polish addition.

### Audio (Step 4)
- If the sound system genuinely doesn't exist yet (BACKLOG says
  "Sound/music (kira)" is deferred), this step is where it gets built —
  minimally: an `AudioManager` wrapping `kira`, loading a small set of
  category sounds (wood/stone/metal/glass break, wood/stone/metal/glass
  place), triggered from the same code path that spawns break particles.
- Don't scope this into a full music/ambience system yet — that's a
  separate future item. This step is specifically "breaking and placing
  blocks makes a sound," because that's the gap you named.

## Engine/rendering fixes (Steps 5–11)

### Per-vertex AO (Step 5)
- Standard voxel-AO technique: for each vertex of a face, sample the
  solidity of the up-to-3 neighboring blocks that touch that corner, and
  darken the vertex color based on how many are solid (0 solid = no
  darkening, 3 solid = darkest). This is computed at mesh-build time in
  `lf_voxel`'s meshing code, not per-frame in the shader — keep meshing
  performance in mind, this touches the same code the greedy-meshing item
  (Step 10) will touch later.
- This single change is usually what takes a voxel renderer from "looks
  like a tech demo" to "looks like a real game" — prioritize it highly.

### Chunk-border lighting (Step 6)
- Audit BFS light propagation, sky-light column factor, and day-night
  blending specifically at chunk seams. A common bug class: light
  computed independently per chunk without properly reading neighbor
  chunk state at the border, producing a visible seam or lighting pop-in
  as chunks stream.

### Camera/FOV audit (Step 7)
- The P25 audit already found and fixed a bug where the path tracer's
  camera basis was scaled by a double `to_radians()` call on an
  already-radians FOV value. Explicitly check the rasterized perspective
  projection matrix construction for the same mistake — it's an easy bug
  to reintroduce or to have missed the first time since it's a separate
  code path.

### Transparency/particle sort (Step 8)
- Existing water/glass transparency uses back-to-front column sort. New
  particles (break debris, later steam/smoke) need to either participate
  in that sort correctly or render in a separate, correctly-ordered pass.
  Test with a scene that has water, glass, and active particles
  overlapping.

### Performance target (Step 9)
- Pick one real, named device (e.g., "2021 MacBook Air M1, integrated
  GPU" or a specific mid-range Windows laptop with Intel integrated
  graphics) — write the actual model into `DECISIONS.md`. This turns
  Pillar 4 ("runs on what people own") from a vibe into a number everyone
  can check against.
- Profile at "Medium" quality (once Step 13's quality tiers exist) and
  record fps.

### Greedy meshing (Step 10)
- Only after Step 5 (AO) is in, since greedy meshing merges adjacent
  faces and needs to carry per-vertex AO data correctly through the merge
  or the visual quality gain from Step 5 gets undone.

### Live RT decision (Step 11)
- The existing compute-shader path tracer only supports capture-to-PNG
  (R key), not a live view. Decide explicitly: either invest in making it
  a real live toggle (meaningful compute/engineering cost — likely a
  multi-day investment) or formally cut it to "showcase capture feature
  only" and say so in `DECISIONS.md`, so nobody wonders again whether it's
  supposed to be live.

## What "done" looks like for all of Stage B

A player mining a block sees the crack progress, hears a sound, sees
particles matching the block's material, and the world around them is
lit consistently with soft corner shading — all while running acceptably
on the device named in Step 9. That combination is what turns "the engine
feels vague" into "the engine feels like a real game."
