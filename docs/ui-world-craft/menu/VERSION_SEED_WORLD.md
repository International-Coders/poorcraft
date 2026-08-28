# Version-Seeded Preview World — Reference Document

## The concept

Every version of LOREFORGE shows a different preview world on the title
screen. When the game updates, players immediately see something new and
recognizable — a snowy valley for v0.3, a mesa canyon for v0.4, a deep
forest for v0.5. This creates a genuine emotional connection to version
history, and it's entirely free: the same world generation that already
runs generates it, just with a version-derived seed.

## Seed derivation (authoritative spec)

The seed is derived from the `CARGO_PKG_VERSION` environment variable
baked in at compile time. The derivation must be:

1. **Stable across machines and builds of the same version.** The same
   version always produces the same seed, regardless of build machine,
   OS, or Rust version.
2. **Visually different between close versions.** v0.4.1 and v0.4.2 should
   produce noticeably different worlds — the seed mixing must be
   non-linear enough that a patch increment creates a large seed change.
3. **The seed must be displayed on the title screen** — in the version
   string area or as a small annotation below the version, so players
   can reference it when talking about a specific version's world.

Authoritative derivation function (Rust):
```rust
pub fn version_preview_seed() -> u64 {
    let version = env!("CARGO_PKG_VERSION"); // e.g. "0.4.2"
    let parts: Vec<u64> = version
        .splitn(3, '.')
        .map(|s| s.parse::<u64>().unwrap_or(0))
        .collect();
    let major = *parts.get(0).unwrap_or(&0);
    let minor = *parts.get(1).unwrap_or(&0);
    let patch = *parts.get(2).unwrap_or(&0);
    // Pack into a u64 with intentional large-factor mixing
    // so patch increments change the seed significantly
    let packed = major
        .wrapping_mul(1_000_000_007)
        .wrapping_add(minor.wrapping_mul(999_983))
        .wrapping_add(patch.wrapping_mul(999_979));
    // Final Fibonacci hash for good bit distribution
    packed.wrapping_mul(0x9e3779b97f4a7c15u64)
}
```

## Preview world properties

The preview world should:
- Load a small, fast-generating view region (32×32 chunks maximum — just
  enough to look interesting in a 90-second orbit).
- Use WorldType::Normal always (not Superflat or Amplified — the title
  screen should show what a "normal" world looks like).
- Not persist to disk. Generate in memory, display, discard on exit.
  Do not create a `worlds/preview/` save folder — that pollutes the saves.
- Be generated at startup, before the title screen appears, while a brief
  loading indicator is shown (a simple text line "Generating world…" in
  the `text-muted` color at the bottom of an otherwise-dark screen is fine;
  do not show a loading bar or spinner — they look template-ish).

## Camera path parameters

Stored as named constants in the title-screen module:
```rust
// Orbit parameters
const PREVIEW_ORBIT_PERIOD_SECS: f64 = 90.0;
const PREVIEW_ORBIT_X_RADIUS: f32 = 80.0;
const PREVIEW_ORBIT_Z_RADIUS: f32 = 60.0;  // elliptical, not circular
const PREVIEW_ORBIT_LOOK_OFFSET_X: f32 = 20.0; // camera looks at offset point
const PREVIEW_BASE_ALTITUDE: f32 = 40.0;   // blocks above median terrain height
// Gentle altitude oscillation
const PREVIEW_ALT_OSCILLATION_AMPLITUDE: f32 = 8.0;
const PREVIEW_ALT_OSCILLATION_PERIOD_SECS: f64 = 57.3; // prime, avoids synchrony
```

Using a prime-ish oscillation period (57.3s ≠ 45s = 90/2) ensures the
altitude oscillation never perfectly syncs with the orbit, producing
a subtly varied path that doesn't feel mechanical.

## What to display when the world is loading

While the preview world generates (should take < 3 seconds on any
reasonable machine):
- Screen: very dark background (#1a1410).
- Center of screen: the LOREFORGE logotype only. Same style as the title
  screen but without the buttons.
- Bottom center: "Entering world v{VERSION}..." in `text-muted` style.
- No progress bar. No spinner. No percentage. Loading is fast enough that
  a static loading message is correct — a dynamic indicator implies it
  might take a while, which sets the wrong expectation.

## The seed display

On the title screen, below the version string in the bottom-right corner:
```
LOREFORGE v0.4.2
Seed: 14,203,847,923
```
Both lines in `micro` size (9pt), `text-muted` color (#8a7f6e).

This is a detail players will notice and share. "Check out the v0.3 seed
world" becomes a real thing people say. That's the goal.
