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
pub mod castle;
pub mod castle_law;
pub mod civic;
pub mod combat;
pub mod companion;
pub mod constraints;
pub mod coords;
pub mod craft;
pub mod debug_overlay;
pub mod diagnose;
pub mod dragon;
pub mod economy;
pub mod edit;
pub mod engineering;
pub mod entities;
pub mod faction;
pub mod flora;
pub mod flow;
pub mod garrison;
pub mod gen;
pub mod host;
pub mod hydro;
pub mod ideology;
pub mod items;
pub mod journey;
pub mod karma_evidence;
pub mod kits;
pub mod ley;
pub mod lod;
pub mod machines;
pub mod magic;
pub mod nav;
pub mod npc;
pub mod npc_death;
pub mod quest;
pub mod nuclear;
pub mod oversight;
pub mod perception;
pub mod player;
pub mod player_settlement;
pub mod proof;
pub mod query;
pub mod relationships;
pub mod replicate;
pub mod scale;
pub mod scales;
pub mod session;
pub mod settlement;
pub mod settlement_plan;
pub mod soak;
pub mod stream;
pub mod survival;
pub mod terrain;
pub mod valve_computing;
pub mod war;

pub use bounds::{WorldBounds, WorldBoundsXz};
pub use build::{
    effective_answer, replay_builds, BuildBlock, BuildKind, BuildOp, Construction, PlaceError,
    RemoveError,
};
pub use castle::{plan_capital, CastleLayout, CastleModule, ModuleKind, PlacedModule, Port};
pub use castle_law::{Alarm, CastleLaw, GateState, Law, LawKind, Punishment};
pub use civic::{CivicBoard, CivicProject, Commissioner};
pub use combat::{
    Creature, CreatureKind, CreatureSystem, DungeonRoom, Hit, CREATURE_COOLDOWN, MELEE_RANGE,
};
pub use companion::{Companion, CompanionCommand, FOLLOW_DISTANCE};
pub use constraints::{
    check_river_corridors, check_seed_reproducibility, run_biome_constraints, ConstraintResult,
};
pub use coords::{Axis, CellCoord, LocalPos, PatchCoord, RegionCoord, WorldPos};
pub use craft::{can_craft, craft, recipe_by_code, recipe_for_output, Recipe, RECIPES};
pub use debug_overlay::{lod_color, render_overlay, rows_for, PatchDebugRow};
pub use diagnose::{run_diagnosis, run_full_diagnosis, CheckResult, Diagnosis};
pub use dragon::{AssaultOutcome, Dragon, DragonWorld};
pub use economy::{consume_food, execute_trade, produce, EconomicState, TradeRoute};
pub use edit::{
    affected_patches, apply_edit, replay, Brush, EditKind, EditOp, Snapshot, COMPACT_THRESHOLD,
};
pub use engineering::{Pipe, Valve, ValveNetwork, WaterWheel};
pub use entities::{cell_center_mm, Entity, EntityId, EntityKind, EntityRegistry};
pub use garrison::Garrison;
pub use gen::{Biome, CellMaterial, MacroField, WorldGen};
pub use host::{HostCommand, SoloHost, TICKS_PER_DAY};
pub use ideology::{Ideology, PlayerFaction};
pub use journey::{run_journey, JourneyReport};
pub use karma_evidence::{AxisEvidence, KarmaAxis, MultiAxisKarma};
pub use kits::{kit_for, plan_capital_kit, FactionKit, KITS};
pub use ley::{Attunement, LeyCaster, Ritual, RitualError};
pub use machines::{
    Machine, MachineKind, MachineNetwork, PowerType, Wire, WireError, BATTERY_CAP, EDGE_THROUGHPUT,
};
pub use nav::{cross_patch_path, NavPatch, MAX_NAV_NODES};
pub use npc::{
    schedule_phase, Activity, Intent, Needs, NpcBrain, Role, SchedulePhase, IDLE_END, SLEEP_END,
    WORK_END,
};
pub use npc_death::NpcRoster;
pub use nuclear::{Contamination, NuclearProgram, Reactor, Siterror, MAX_REACTORS};
pub use oversight::{biome_viability, OversightPanel, OversightSummary, Project};
pub use perception::{
    witness, Evidence, Karma, Knowledge, MoralEvent, MoralKind, DISPOSITION_MAX, DISPOSITION_MIN,
    KNOWLEDGE_CAPACITY, REPORT_CONFIDENCE, SIGHT_RADIUS, WITNESSED_CONFIDENCE,
};
pub use player::{
    MoveInput, Player, BODY_HEIGHT, EYE_HEIGHT, HALF_WIDTH, SIM_DT, SWIM_SPEED, WALK_SPEED,
};
pub use player_settlement::{FoundError, PlayerSettlement, PlayerSettlements};
pub use proof::{current_shade, render_flow_map, river_stroke_width};
pub use query::{patches_in_region, patches_touching, regions_touching, QueryError};
pub use relationships::{CityRelationship, RelationshipKind, RelationshipSystem};
pub use replicate::{interest_snapshot, Ack, Mirror, ReliableChannel, RepSnapshot};
pub use scale::{scale_proof, ScaleRow};
pub use scales::{
    CELL_METERS, CELL_MM, MAX_QUERY_PATCHES, MM_PER_METER, PATCH_CELL_AXIS, PATCH_METERS, PATCH_MM,
    REGION_METERS, REGION_MM, REGION_PATCH_AXIS,
};
pub use session::{
    Lobby, LobbyManager, LoopbackTransport, PeerId, Session, SessionError, Transport,
};
pub use settlement::{Aggregate, Settlement, SettlementState, Settlements, MIN_SITE_SPACING};
pub use settlement_plan::{
    Anchors, BuildingKind, BuildingSlot, PlanValidation, RoadSegment, Service, ServiceSummary,
    SettlementPlan,
};
pub use soak::{run_soak, SoakReport};
pub use survival::{eat_from, fishing_catch, harvest_into, Onboarding, FISH};
pub use valve_computing::{GateKind, LogicCircuit, LogicGate, Signal, ValveController};
pub use war::{advance, assign_npcs, WarAssignment, WarObjective};
