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
- M8: implemented 8-material forge+anvil system with tool parts (head/haft/binding) assembly, forge minigame, and stat tests (shots/m8_forge.png)
- M9: implemented mobs (Boar, Woolbeast, Glitchling, Stalker, Crawler, Null Knight boss) and combat system; saved shots/m9_boss.png
- M10: implemented lf_modapi manifest and data pack loader for blocks/items/recipes, shipping ember_ores example mod with test proof shots/m10_mod.png
- M11: implemented protocol codec (handshake/login/chat) and dedicated server binary loreforge-server for M11 proof


- P0: region files hold all chunks keyed (x,z) with atomic tmp+rename writes (was: one chunk per file, neighbors collided)
- P0: biome selection is elevation-aware (Mountains>=140, Highlands>=110, DeepOcean<42, Ocean<56) with height stretched to 24..176 so all 8 biomes occur
- P0: depth buffer (Depth32Float) in the voxel pipeline; shared GpuScene reused by windowed and offscreen paths
- P0: all proof screenshots must come from lf_engine::headless (real wgpu render); docs-only commits banned
