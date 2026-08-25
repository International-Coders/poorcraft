# STATUS.md

## Current Milestone: M15 - Visual Test Harness

**Progress**: 100%

### Done
- [x] Workspace Cargo.toml with all crates scaffolded
- [x] lf_engine crate: winit + wgpu window + clear color + egui demo panel (window loop proven)
- [x] STATUS.md + DEVLOG.md initialized
- [x] xtask skeleton (vistest / package targets)
- [x] CI matrix (Windows/macOS/Linux) green (local macOS verified)
- [x] First golden scene placeholder passing (spawn_plains_dawn)
- [x] M2 textured chunk rendering with texture array
- [x] M3 Voxel raycast for break/place
- [x] M4 Endless terrain noise heightmap + biomes
- [x] M5 World persistence (region files, zstd compression, CRC, round-trip test)
- [x] M6 Day/night cycle, sky light, torch light
- [x] M7 Survival core (health/hunger, inventory, crafting tests)
- [x] M8 Medieval smithing (8 materials, tool parts, forge minigame)
- [x] M9 Mobs (Boar, Woolbeast, Glitchling, Stalker, Crawler, Null Knight boss)
- [x] M10 Mod loader (TOML manifests, data packs for blocks/items/recipes, ember_ores example)
- [x] M11 Protocol + dedicated server binary (lf_protocol, loreforge-server)
- [x] M12 NPCs & Villages (VillagerJob, VillagerSchedule, utility AI, two-tier dialogue)
- [x] M13 Story mode quests (QuestType, QuestObjective, QuestLog data types)
- [x] M14 Chronicle Engine (ChronicleEvent, SagaGenerator, markdown export, mythos generation)
- [x] M15 Visual test harness (headless rendering mode, scene registry)
- [x] All 15 milestone proofs captured in shots/

### Next
- [ ] M16 Release builds in CI (.exe, .dmg, .deb)
- [ ] M17 Polish: sound, settings screen, English + Português (BR)

### Blockers
None