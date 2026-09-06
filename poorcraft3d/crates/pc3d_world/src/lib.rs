//! POORCRAFT 3D world substrate — the spatial language
//! (P3D-101, docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md).
//!
//! Pure integer geometry: world positions in millimeters, the
//! region/patch/cell hierarchy from the terrain blueprint, bounds algebra,
//! and bounded spatial queries. No dependencies, no IO, no generation —
//! this is the ground every later system stands on, and the one place the
//! world's scales are declared.

pub mod bounds;
pub mod build;
pub mod edit;
pub mod economy;
pub mod ideology;
pub mod karma_evidence;
pub mod npc_death;
pub mod entities;
pub mod companion;
pub mod relationships;
pub mod civic;
pub mod combat;
pub mod constraints;
pub mod castle;
pub mod castle_law;
pub mod coords;
pub mod craft;
pub mod debug_overlay;
pub mod diagnose;
pub mod faction;
pub mod flow;
pub mod engineering;
pub mod garrison;
pub mod gen;
pub mod war;
pub mod hydro;
pub mod items;
pub mod lod;
pub mod magic;
pub mod nav;
pub mod player_settlement;
pub mod npc;
pub mod perception;
pub mod oversight;
pub mod player;
pub mod proof;
pub mod query;
pub mod scales;
pub mod survival;
pub mod settlement;
pub mod settlement_plan;
pub mod stream;
pub mod terrain;

pub use bounds::{WorldBounds, WorldBoundsXz};
pub use companion::{Companion, CompanionCommand, FOLLOW_DISTANCE};
pub use diagnose::{run_diagnosis, run_full_diagnosis, CheckResult, Diagnosis};
pub use craft::{can_craft, craft, recipe_by_code, recipe_for_output, Recipe, RECIPES};
pub use debug_overlay::{lod_color, rows_for, render_overlay, PatchDebugRow};
pub use npc_death::NpcRoster;
pub use karma_evidence::{AxisEvidence, KarmaAxis, MultiAxisKarma};
pub use ideology::{Ideology, PlayerFaction};
pub use economy::{consume_food, execute_trade, produce, EconomicState, TradeRoute};
pub use entities::{cell_center_mm, Entity, EntityId, EntityKind, EntityRegistry};
pub use engineering::{Pipe, Valve, ValveNetwork, WaterWheel};
pub use war::{advance, assign_npcs, WarAssignment, WarObjective};
pub use garrison::Garrison;
pub use gen::{Biome, CellMaterial, MacroField, WorldGen};
pub use build::{effective_answer, replay_builds, BuildBlock, BuildKind, BuildOp, Construction, PlaceError, RemoveError};
pub use edit::{apply_edit, affected_patches, replay, Brush, EditKind, EditOp, Snapshot, COMPACT_THRESHOLD};
pub use civic::{CivicBoard, CivicProject, Commissioner};
pub use combat::{Creature, CreatureKind, CreatureSystem, DungeonRoom, Hit, CREATURE_COOLDOWN, MELEE_RANGE};
pub use constraints::{check_river_corridors, check_seed_reproducibility, run_biome_constraints, ConstraintResult};
pub use relationships::{CityRelationship, RelationshipKind, RelationshipSystem};
pub use castle::{plan_capital, CastleLayout, CastleModule, ModuleKind, PlacedModule, Port};
pub use castle_law::{Alarm, CastleLaw, GateState, Law, LawKind, Punishment};
pub use coords::{Axis, CellCoord, LocalPos, PatchCoord, RegionCoord, WorldPos};
pub use player_settlement::{FoundError, PlayerSettlement, PlayerSettlements};
pub use nav::{cross_patch_path, NavPatch, MAX_NAV_NODES};
pub use npc::{schedule_phase, Activity, Intent, Needs, NpcBrain, Role, SchedulePhase, IDLE_END, SLEEP_END, WORK_END};
pub use perception::{witness, Evidence, Karma, Knowledge, MoralEvent, MoralKind, REPORT_CONFIDENCE, SIGHT_RADIUS, WITNESSED_CONFIDENCE, KNOWLEDGE_CAPACITY, DISPOSITION_MIN, DISPOSITION_MAX};
pub use oversight::{biome_viability, OversightPanel, OversightSummary, Project};
pub use player::{MoveInput, Player, BODY_HEIGHT, EYE_HEIGHT, HALF_WIDTH, SIM_DT, SWIM_SPEED, WALK_SPEED};
pub use proof::{current_shade, render_flow_map, river_stroke_width};
pub use query::{patches_in_region, patches_touching, regions_touching, QueryError};
pub use survival::{eat_from, fishing_catch, harvest_into, Onboarding, FISH};
pub use settlement::{Aggregate, Settlement, SettlementState, Settlements, MIN_SITE_SPACING};
pub use settlement_plan::{Anchors, BuildingKind, BuildingSlot, PlanValidation, RoadSegment, Service, ServiceSummary, SettlementPlan};
pub use scales::{
    CELL_MM, CELL_METERS, MAX_QUERY_PATCHES, MM_PER_METER, PATCH_CELL_AXIS, PATCH_MM,
    PATCH_METERS, REGION_MM, REGION_METERS, REGION_PATCH_AXIS,
};
