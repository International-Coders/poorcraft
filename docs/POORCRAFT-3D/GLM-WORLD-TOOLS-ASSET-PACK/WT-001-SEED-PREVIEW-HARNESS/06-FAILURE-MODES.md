# Failure Modes

- Preview map is a static image.
- Same seed produces different preview.
- Different seed produces indistinguishable preview.
- UI shows seed but world creation uses another seed.
- Random seed is not displayed.
- Reroll can return the same seed silently.
- Spawn marker is hidden or off-map.
- Spawn safety says safe without checking water/slope/solid.
- Screenshot has no metadata sidecar.
- Create button works when preview is invalid.
- Map colors are pretty but not tied to worldgen.
- Tests only parse JSON and never compare actual preview values.
