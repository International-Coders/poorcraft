# Z-code Commands

GLM should add or wire commands like these.

## Test Commands

```bash
cargo test --manifest-path poorcraft3d/Cargo.toml -p pc3d_world seed_preview
cargo test --manifest-path poorcraft3d/Cargo.toml -p pc3d_render seed_preview
```

## Screenshot Commands

```bash
cargo run --release --manifest-path poorcraft3d/Cargo.toml -p poorcraft3d -- --ui-seed-preview-shots
```

or integrate into:

```bash
make p3d-ui-shots
```

## Inspector Commands

```bash
poorcraft3d inspect-seed --seed "owner-test" --world-type normal --out shots/seed_owner_test.json
poorcraft3d seed-preview --seed random --out shots/seed_random.png --json shots/seed_random.json
```

Exact command names may change. The outputs may not.
