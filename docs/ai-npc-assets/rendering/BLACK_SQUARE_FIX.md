# Black Square Fix — Reference Document

## What you're looking for in the code

The black square that appears locked in the player's view during gameplay
is a rendering artifact, not a gameplay bug. It has a fixed position on
screen (not in the world), which immediately tells you it's either:
a) a UI/overlay element rendered at a wrong size or position, or
b) a stale GPU buffer or render pass being composited over the frame.

## Diagnostic checklist (run in this order before touching any code)

### Step 1 — Capture it

Add a temporary screenshot hotkey (or use the existing one) to capture a
frame where the black square is visible. Look at the PNG and measure:
- Exact pixel dimensions of the square.
- Exact pixel position (top-left corner).
- Whether it appears in vistest captures (headless) or only in interactive
  play.

If it appears in vistest: the cause is in the render pipeline.
If it only appears in interactive play: the cause is likely egui or input-
related (a hover rect, a focus indicator, a cursor interaction bounding box).

### Step 2 — Search for known egui debug features

In `lf_client/src/ui.rs`, search for:
```rust
ctx.set_debug_on_hover(true)  // if present, remove
style.debug.show_expand_width
style.debug.show_expand_height
style.debug.show_widget_hits
```

Also check if `egui::Context::set_style` is called anywhere with debug
flags enabled. These produce paint rects that look exactly like a black
square overlay.

### Step 3 — Inspect the render pass descriptors

In `lf_engine`, find every `wgpu::RenderPassDescriptor`. For each one,
check the `depth_stencil_attachment`:
```rust
depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
    view: &depth_view,
    depth_ops: Some(wgpu::Operations {
        load: wgpu::LoadOp::Load,  // <-- IS THIS CORRECT HERE?
        store: wgpu::StoreOp::Store,
    }),
    ...
})
```

Any pass using `LoadOp::Load` for depth that should be using
`LoadOp::Clear(1.0)` will produce depth-buffer artifacts that can
appear as opaque black regions.

The correct pattern:
- The FIRST pass that uses the depth buffer: `LoadOp::Clear(1.0)`
- Subsequent passes that need to READ depth from the previous pass:
  `LoadOp::Load`
- A pass that renders independently (e.g., the egui overlay): `LoadOp::Clear(1.0)`,
  and its depth attachment should be a SEPARATE depth texture from the
  scene depth, or the scene depth should be cleared for this pass.

If the egui pass shares the scene depth texture and uses `LoadOp::Load`,
previous geometry's depth values can cause egui's background rects to
be depth-tested against scene geometry and produce a black overdraw.

### Step 4 — Check the path tracer accumulation buffer

In `lf_engine/src/pathtrace.rs` (or wherever `Pathtracer` lives):
```rust
impl Pathtracer {
    pub fn on_scene_change(&mut self, queue: &wgpu::Queue) {
        // Does this method exist? If not, that's the bug.
        // It should zero-fill or invalidate the accumulation buffer.
    }
}
```

If the Pathtracer's accumulation texture is not cleared when the player
transitions from the title screen to a new world (or vice versa), a
black region from the previous scene can persist in the new scene.

### Step 5 — Check the chunk mesh upload path

In `lf_engine/src/mesh_batch.rs` (or wherever `MeshBatch` / `SceneResources`
handle chunk mesh uploads):

```rust
// Look for something like this:
pub fn upload_chunk_mesh(&mut self, chunk_pos: ChunkPos, vertices: &[Vertex]) {
    let slot = self.allocate_slot(chunk_pos);
    if vertices.is_empty() {
        // BUG: if we don't return here and still register the slot as drawable,
        // the GPU will draw a quad with uninitialized vertices = black square
        return; // <-- is this present?
    }
    // ... upload vertices to GPU buffer
    self.mark_drawable(slot); // <-- only called if vertices non-empty?
}
```

An empty mesh being marked drawable with a stale (zero-filled) GPU buffer
will render as a black rectangle.

## The pixel-analysis assertion to add

After the fix, add this to the `no_black_square` vistest scene's assertions.
This is pseudocode — adapt to the actual `lf_vistest` assertion API:

```rust
// Scan the centre 80% of the frame for large black rectangles.
// Fail if any contiguous region of pixels where all three channels < 8
// has a bounding box larger than 64×64.
fn assert_no_black_rect(image: &RgbImage, frame_name: &str) {
    let w = image.width() as usize;
    let h = image.height() as usize;
    let x_start = w / 10;
    let x_end = w - w / 10;
    let y_start = h / 10;
    let y_end = h - h / 10;

    // Simple scan: for each pixel in the centre region, check if it's
    // black. If a run of black pixels in a row exceeds 64, fail.
    for y in y_start..y_end {
        let mut black_run = 0usize;
        for x in x_start..x_end {
            let px = image.get_pixel(x as u32, y as u32);
            if px[0] < 8 && px[1] < 8 && px[2] < 8 {
                black_run += 1;
                if black_run > 64 {
                    panic!("{}: black run of {} pixels at row {}", frame_name, black_run, y);
                }
            } else {
                black_run = 0;
            }
        }
    }
}
```

This assertion is cheap, deterministic, and catches the exact bug that was
reported without being so strict it fails on dark rocks or shadows.
