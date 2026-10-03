# Dual-scale biome layout (POORCRAFT 3D)

Date: 2026-09-28  
Status: approved for near-term implementation  
Scope: `pc3d_world` terrain macro fields + biome selection; biome-dressing assets follow in the same campaign after layout laws are green.

## Intent

Make the seeded world read as **wide climate countries** with **compatible enclaves** inside them — not confetti biomes, not one flat band. Keep today's playable map size for this pass. Document (below) how to grow toward Valheim-scale later without rewriting the contract.

## Near-term (this pass)

### Architecture

Keep the 256 m `RegionCoord` grid and the existing `Biome` enum
(`Ocean`, `Coast`, `Plains`, `Forest`, `Wetland`, `Highlands`, `Mountains`,
`SnowPeaks`).

Drive biomes from **two noise scales** on top of the current elevation field:

1. **Continental climate** — much longer wavelength for temperature and
   humidity (and a slow warp on elevation if needed) so neighbors share a
   belt for many regions.
2. **Pocket field** — shorter wavelength that, when strong enough, flips
   the region to a **compatible** enclave biome (Forest pocket in Plains,
   Wetland bowl in humid lowland, Highlands blister on mid elevation).

`WorldGen::biome(region)` remains the single authority flora, materials,
and settlement already consult. Determinism stays seed → fields → biome;
no wall-clock or OS input.

### Compatibility rules (pockets)

| Base belt | Allowed pocket |
| --- | --- |
| Plains | Forest, Wetland (only if elevation still low) |
| Forest | Plains, Wetland (low elev) |
| Highlands | Plains, Forest, Mountains (rare / high pocket) |
| Mountains | Highlands, SnowPeaks (only if cold enough) |
| SnowPeaks | Mountains |
| Ocean / Coast | none (water edge stays water) |

Pockets never create land in Ocean or ocean in Mountains.

### Height and materials

- `surface_height_mm` / `effective_surface_mm` stay the ground authority.
- Surface materials and flora keep reading `biome()` — layout changes must
  be visible underfoot and in canopy without a parallel biome channel.
- Soft pixel blends at region edges are **out of scope** for this pass
  (deferred after layout proves itself).

### Assets (same campaign, after layout laws)

Dress biomes the new layout emphasizes: Forest / Plains / Mountains /
SnowPeaks / Wetland get clearer flora density and detail-atlas contrast.
No full settlement-kit redesign in the first terrain slice.

### Proofs

- Unit laws in `pc3d_world::gen`:
  - same-seed determinism unchanged;
  - neighbor elevation correlation (existing law) still holds;
  - **belt law**: for a fixed seed, a random inland land region shares
    its biome with a majority of a ±K neighborhood (K large enough to
    prove countries, not confetti);
  - **pocket law**: over a wide sweep, at least some inland land regions
    differ from the majority of their 8-neighbors (enclaves exist);
  - Ocean / Coast still require low elevation; SnowPeaks still require
    high cold elevation.
- Windowed (follow-up): capture a belt interior and a pocket; flora /
  materials must differ. Not required to close the gen laws.

### Non-goals (this pass)

- Growing view distance / streamed radius / atlas size.
- Soft shader blends across biome boundaries.
- New biome enum values.
- River / watershed redesign (P3D-301).
- UI / HUD work.

## Later: Valheim-scale map (docs only now)

When we grow the world, do **not** invent a second biome system. Scale the
same dual-field contract:

| Knob | Near-term | Later target |
| --- | --- | --- |
| Climate FBM cell size (regions) | continental vs today, still fits current play radius | ×2–×4 again so sailing/walking between lands is a journey |
| Pocket cell size | local enclaves in a session | keep relative to climate (pockets stay local inside huge belts) |
| Stream / interest radius | unchanged | raise with climate wavelength so the player can *see* the country |
| Seed stability | accept biome remaps for unreleased P3D worlds | freeze a `WORLDGEN_LAYOUT_VERSION`; old seeds either migrate or stay on prior version |
| Player experience | distinct biomes in one play session | Valheim-like: long travel between named lands, pockets for local drama |

### Scale-up checklist (when opening that job)

1. Bump a named layout version constant and record it in saves if any
   patch journals assume biome of a region.
2. Retune climate/pocket wavelengths; re-run belt + pocket laws with
   larger K.
3. Raise surface-stream / flora interest rings to match.
4. Re-bake showcase seed search (`find_showcase`) — capitals must still
   find river + cave within walking distance *or* the slice contract
   must be updated honestly.
5. Refresh windowed biome proofs and capability inventory if flora GLBs
   change.
6. Do **not** soft-blend until the large map still reads as countries.

## Open decisions locked by this doc

- Approach: dual-scale climate + compatible pockets (not Voronoi nations,
  not threshold-only retune).
- Map size this pass: current.
- Soft blends: later.
- Assets: biome dress after gen laws green.
