# DECISIONS
- M1: chose winit + wgpu for cross-platform rendering
- M2: chose palette compression for voxel sections
- M1: captured m1_window.png as proof for M1 window milestone
- M2: implemented 2D texture array and rendered textured voxel section for M2 proof
- M3: implemented Amanatides & Woo DDA raycast for voxel selection and outline highlight proof shots/m3_breakplace.png
- M4: integrated fastnoise-lite for deterministic terrain height, temperature, humidity, biomes, and soil strata
- M5: implemented zstd-compressed regional chunk file storage with bincode serialization and round-trip tests (shots/m5_save_load.png)
- M6: implemented 20-min day/night TimeOfDay with sky color interpolation, block light levels (torch=14, lantern=15), and light engine for M6 proof
- M7: added PlayerStats (health/hunger/saturation), Inventory (36+4+1 slots), item stacking up to 64, and unit tests for M7 proof


