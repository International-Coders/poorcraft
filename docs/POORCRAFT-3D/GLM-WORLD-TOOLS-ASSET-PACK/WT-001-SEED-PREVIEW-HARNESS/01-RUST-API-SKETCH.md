# Rust API Sketch

Prefer small pure modules first.

## Suggested Data Types

```rust
pub struct SeedPreviewRequest {
    pub seed_text: String,
    pub world_type: PreviewWorldType,
    pub size_px: u32,
}

pub struct SeedPreview {
    pub resolved_seed: u64,
    pub seed_label: String,
    pub world_type: PreviewWorldType,
    pub spawn: SpawnPreview,
    pub biome_counts: Vec<BiomePreviewCount>,
    pub feature_hints: Vec<FeatureHint>,
    pub pixels_rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
```

## Suggested Functions

```rust
pub fn resolve_seed(text: &str, entropy: Option<u64>) -> ResolvedSeed;
pub fn preview_seed(req: &SeedPreviewRequest) -> SeedPreview;
pub fn encode_preview_png(preview: &SeedPreview, path: &Path) -> Result<()>;
pub fn write_preview_json(preview: &SeedPreview, path: &Path) -> Result<()>;
```

## Placement

Good candidates:

- world/seed logic: `pc3d_world`;
- UI harness/rendering: `pc3d_render`;
- command wiring: `apps/poorcraft3d`.

Keep the preview data pure and testable before connecting it to UI rendering.
