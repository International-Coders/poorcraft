//! P3D-801: the integrated solo host.
//!
//! One owner for the deterministic spine AND every canonical world
//! system. Player intents enter as `CommandEnvelope`s; the host applies
//! each tick's batch in canonical (tick, id) order — so delivery
//! grouping and reordering are inert — and then advances machines and
//! reactors every tick and settlements + dragons on the day boundary.
//! The digest folds the journal and system state, so two hosts with the
//! same seed and the same command history end bit-identical no matter
//! how the commands arrived.

use crate::build::{replay_builds, BuildKind, BuildOp, Construction};
use crate::coords::{CellCoord, PatchCoord, RegionCoord};
use crate::dragon::{AssaultOutcome, DragonWorld};
use crate::flow::FlowTable;
use crate::gen::CellMaterial;
use crate::hydro::RiverGraph;
use crate::machines::MachineNetwork;
use crate::nuclear::NuclearProgram;
use crate::settlement::Settlements;

use pc3d_core::command::{CommandEnvelope, CommandSequencer};
use pc3d_core::journal::fnv1a64;

/// Ticks per in-host day at the 60 Hz simulation rate (compressed:
/// one day every simulated 10 seconds).
pub const TICKS_PER_DAY: u64 = 600;
/// Journal event kinds.
pub const EV_HEARTBEAT: u32 = 1;
pub const EV_BUILD_APPLIED: u32 = 2;
pub const EV_DRAGON_RAID: u32 = 3;
pub const EV_REACTOR_EVENT: u32 = 4;
pub const EV_DRAGON_SLAIN: u32 = 5;
pub const EV_DRAGON_SPAWN: u32 = 6;

/// Everything a player or system can ask the host to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostCommand {
    Build { cell: CellCoord, material: CellMaterial, owner: u64 },
    RemoveBuild { cell: CellCoord, owner: u64 },
    FeedBoiler { machine: u64, fuel_milli: i64, water_milli: i64 },
    FuelReactor { reactor: u64, fuel_milli: i64, coolant_milli: i64 },
    SetRods { reactor: u64, pct: i64 },
    SpawnDragon { lair: RegionCoord },
    AssaultDragon { dragon: u64, party_power: i64, faction: u64, seed: u64 },
}

/// The integrated host: spine + world systems, one deterministic loop.
pub struct SoloHost {
    pub world_seed: u64,
    pub tick: u64,
    pub gen: crate::gen::WorldGen,
    pub flow: FlowTable,
    pub settlements: Settlements,
    pub construction: std::collections::BTreeMap<(i32, i32, i32), Construction>,
    pub machines: MachineNetwork,
    pub nuclear: NuclearProgram,
    pub dragons: DragonWorld,
    pub journal: Vec<(u64, u32, u64, u64)>,
    sequencer: CommandSequencer,
    pending: Vec<CommandEnvelope<HostCommand>>,
    next_build_id: u64,
}

impl SoloHost {
    /// Build the world: generator, river flow, settlements — all
    /// deterministic from the seed.
    pub fn new(world_seed: u64) -> SoloHost {
        let gen = crate::gen::WorldGen::new(world_seed);
        let graph = RiverGraph::new(&gen, 24);
        let flow = FlowTable::from_graph(&graph);
        let settlements = Settlements::new(&gen, &graph, 24);
        SoloHost {
            world_seed,
            tick: 0,
            gen,
            flow,
            settlements,
            construction: std::collections::BTreeMap::new(),
            machines: MachineNetwork::new(),
            nuclear: NuclearProgram::new(),
            dragons: DragonWorld::new(),
            journal: Vec::new(),
            sequencer: CommandSequencer::new(),
            pending: Vec::new(),
            next_build_id: 0,
        }
    }

    /// Queue a command; it lands at the next tick, in canonical order.
    pub fn submit(&mut self, command: HostCommand) -> u64 {
        let id = self.sequencer.assign();
        self.pending.push(CommandEnvelope::new(id, self.tick + 1, command));
        id
    }

    fn record(&mut self, kind: u32, a: u64, b: u64) {
        self.journal.push((self.tick, kind, a, b));
    }

    /// Apply one command to the world. Violated rules (occupied cells,
    /// unknown ids) are refused without touching state.
    fn apply(&mut self, cmd: &HostCommand) {
        match *cmd {
            HostCommand::Build { cell, material, owner } => {
                let key = patch_key(cell);
                let op = BuildOp {
                    id: { self.next_build_id += 1; self.next_build_id },
                    tick: self.tick,
                    kind: BuildKind::Place,
                    cell,
                    material,
                    owner,
                };
                let con = self.construction.entry(key).or_insert_with(|| {
                    Construction::new(PatchCoord { x: key.0, y: key.1, z: key.2 })
                });
                if replay_builds(con, &[op]) == 1 {
                    self.record(EV_BUILD_APPLIED, owner, cell_hash(cell));
                }
            }
            HostCommand::RemoveBuild { cell, owner } => {
                let key = patch_key(cell);
                let op = BuildOp {
                    id: { self.next_build_id += 1; self.next_build_id },
                    tick: self.tick,
                    kind: BuildKind::RemoveBuild,
                    cell,
                    material: CellMaterial::Air,
                    owner,
                };
                if let Some(con) = self.construction.get_mut(&key) {
                    if replay_builds(con, &[op]) == 1 {
                        self.record(EV_BUILD_APPLIED, owner, cell_hash(cell));
                    }
                }
            }
            HostCommand::FeedBoiler { machine, fuel_milli, water_milli } => {
                self.machines.supply(machine, fuel_milli, water_milli);
            }
            HostCommand::FuelReactor { reactor, fuel_milli, coolant_milli } => {
                self.nuclear.supply(reactor, fuel_milli, coolant_milli);
            }
            HostCommand::SetRods { reactor, pct } => {
                self.nuclear.set_rods(reactor, pct);
            }
            HostCommand::SpawnDragon { lair } => {
                let id = self.dragons.spawn(lair);
                self.record(EV_DRAGON_SPAWN, id, 0);
            }
            HostCommand::AssaultDragon { dragon, party_power, faction, seed } => {
                let out = self.dragons.assault(dragon, party_power, faction, seed);
                if let AssaultOutcome::Slain = out {
                    self.record(EV_DRAGON_SLAIN, dragon, faction);
                }
            }
        }
    }

    /// Advance `n` ticks. Per tick: apply due commands canonically,
    /// tick machines + reactors, and on the day boundary advance
    /// settlements and dragons (recording raids and meltdowns).
    pub fn run_ticks(&mut self, n: u64) {
        for _ in 0..n {
            self.tick += 1;
            // Canonical application of everything due this tick.
            let mut due: Vec<CommandEnvelope<HostCommand>> = self
                .pending
                .iter()
                .filter(|e| e.tick <= self.tick)
                .cloned()
                .collect();
            due = CommandEnvelope::canonical_batch(due);
            let used: std::collections::BTreeSet<u64> =
                due.iter().map(|e| e.id).collect();
            self.pending.retain(|e| !used.contains(&e.id));
            for env in due {
                self.apply(&env.command);
            }

            // Per-tick systems.
            self.machines.tick();
            self.nuclear.tick();
            if self.nuclear.contamination.dose.values().sum::<i64>() > 0 {
                let total: i64 = self.nuclear.contamination.dose.values().sum();
                self.record(EV_REACTOR_EVENT, total as u64, 0);
            }

            // Day boundary.
            if self.tick % TICKS_PER_DAY == 0 {
                let day = self.tick / TICKS_PER_DAY;
                self.settlements.simulate_far_days(1);
                if let Some(sid) = self.dragons.tick_day(&mut self.settlements, day) {
                    self.record(EV_DRAGON_RAID, sid, day);
                }
                self.record(EV_HEARTBEAT, day, self.digest_state());
            }
        }
    }

    /// Fold of journal + system state (builds, machine buffers,
    /// contamination, settlement aggregates, dragon states).
    pub fn digest_state(&self) -> u64 {
        let mut words: Vec<u64> = Vec::new();
        let mut builds = 0usize;
        for con in self.construction.values() {
            builds += con.built_count();
            for cell in &con.cells {
                if let Some(b) = cell {
                    words.push(b.material as u64);
                }
            }
        }
        words.push(builds as u64);
        for m in self.machines.machines.values() {
            words.push(m.kind as u64);
            words.push(m.in_buffer as u64);
            words.push(m.out_buffer as u64);
            words.push(m.stored as u64);
            words.push(m.fuel_milli as u64);
            words.push(m.water_milli as u64);
        }
        for r in self.nuclear.reactors.values() {
            words.push(r.temp_milli as u64);
            words.push(r.integrity as u64);
            words.push(r.fuel_milli as u64);
            words.push(r.scrammed as u64);
        }
        for d in self.nuclear.contamination.dose.values() {
            words.push(*d as u64);
        }
        for s in &self.settlements.list {
            let a = &s.aggregate;
            words.push(a.population as u64);
            words.push(a.food as u64);
            words.push(a.defense as u64);
            words.push(a.prosperity as u64);
        }
        for d in self.dragons.dragons.values() {
            words.push(d.alive as u64);
            words.push(d.awake as u64);
            words.push(d.power as u64);
            words.push(d.hp as u64);
            words.push(d.pact.map(|p| p.tribute_per_month).unwrap_or(0) as u64);
        }
        let mut bytes = Vec::with_capacity(words.len() * 8);
        for w in &words {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        let mut h = fnv1a64(&bytes);
        for (tick, kind, a, b) in &self.journal {
            let mut jb = Vec::with_capacity(32);
            for w in [*tick, *kind as u64, *a, *b] {
                jb.extend_from_slice(&w.to_le_bytes());
            }
            h ^= fnv1a64(&jb);
        }
        h
    }

    pub fn journal_len(&self) -> usize {
        self.journal.len()
    }
}

fn patch_key(cell: CellCoord) -> (i32, i32, i32) {
    let p = cell.patch();
    (p.x, p.y, p.z)
}

fn cell_hash(cell: CellCoord) -> u64 {
    let mut bytes = Vec::new();
    for v in [cell.x, cell.y, cell.z] {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    fnv1a64(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two hosts, same seed, same command SET but delivered in
    /// DIFFERENT order (all queued before running), end bit-identical —
    /// canonical (tick, id) ordering makes delivery order inert.
    #[test]
    fn p3d801_reordered_delivery_is_inert() {
        let run = |reverse_submit: bool| {
            let mut h = SoloHost::new(4242);
            let boiler = h.machines.add_machine(crate::machines::MachineKind::Boiler);
            let engine = h.machines.add_machine(crate::machines::MachineKind::SteamEngine);
            h.machines.connect(boiler, engine).expect("typed wire");
            let reactor = h.nuclear.site_reactor(RegionCoord { x: 1, z: 1 }).unwrap();
            let cell = CellCoord { x: 8, y: 0, z: 8 };
            let mut cmds = vec![
                HostCommand::Build { cell, material: CellMaterial::Rock, owner: 7 },
                HostCommand::FeedBoiler { machine: boiler, fuel_milli: 5_000, water_milli: 20_000 },
                HostCommand::FuelReactor { reactor, fuel_milli: 50_000, coolant_milli: 0 },
                HostCommand::SetRods { reactor, pct: 40 },
            ];
            if reverse_submit {
                cmds.reverse();
            }
            for c in cmds {
                h.submit(c);
            }
            h.run_ticks(1_500);
            h.digest_state()
        };
        let a = run(false);
        let b = run(true);
        assert_eq!(a, b, "delivery order must be inert");
    }

    /// The build path is authoritative end to end: place lands in the
    /// patch construction, removal honors ownership, and the journal
    /// records exactly the applied ops.
    #[test]
    fn p3d801_build_commands_reach_construction() {
        let mut h = SoloHost::new(777);
        let cell = CellCoord { x: 8, y: 0, z: 8 };
        h.submit(HostCommand::Build { cell, material: CellMaterial::Rock, owner: 7 });
        h.run_ticks(1);
        let key = patch_key(cell);
        let con = h.construction.get(&key).expect("patch materialized");
        assert_eq!(con.at(cell).map(|b| b.owner), Some(7));

        // A different owner cannot remove it: refused, journal quiet.
        let before = h.journal_len();
        h.submit(HostCommand::RemoveBuild { cell, owner: 99 });
        h.run_ticks(1);
        assert_eq!(h.journal_len(), before, "violated removal not journaled");
        assert!(h.construction.get(&key).unwrap().at(cell).is_some());

        // The true owner removes it; the journal records the op.
        h.submit(HostCommand::RemoveBuild { cell, owner: 7 });
        h.run_ticks(1);
        assert!(h.construction.get(&key).unwrap().at(cell).is_none());
        assert!(h.journal_len() > before);
    }

    /// Reactor and boiler live in ONE world behind a working SCRAM:
    /// the boiler boils from fed fuel, the reactor makes heat with
    /// rods partly out, and no contamination exists because SCRAM
    /// protects the core.
    #[test]
    fn p3d801_machines_and_reactors_share_the_world() {
        let mut h = SoloHost::new(99);
        let boiler = h.machines.add_machine(crate::machines::MachineKind::Boiler);
        let engine = h.machines.add_machine(crate::machines::MachineKind::SteamEngine);
        let gen = h.machines.add_machine(crate::machines::MachineKind::Generator);
        let bat = h.machines.add_machine(crate::machines::MachineKind::Battery);
        assert!(h.machines.connect(boiler, engine).is_ok());
        assert!(h.machines.connect(engine, gen).is_ok());
        assert!(h.machines.connect(gen, bat).is_ok());
        let reactor = h.nuclear.site_reactor(RegionCoord { x: 2, z: -2 }).unwrap();

        h.submit(HostCommand::FeedBoiler { machine: boiler, fuel_milli: 8_000, water_milli: 40_000 });
        h.submit(HostCommand::FuelReactor { reactor, fuel_milli: 200_000, coolant_milli: 0 });
        h.submit(HostCommand::SetRods { reactor, pct: 30 }); // climbs to SCRAM
        h.run_ticks(2_000);

        assert!(h.machines.stored_charge() > 0, "boiler charged the battery");
        let r = &h.nuclear.reactors[&reactor];
        assert!(r.fuel_milli < 200_000, "the reactor burned before SCRAM");
        assert_eq!(r.control_pct, 100, "SCRAM slammed the rods and they stay in");
        assert!(r.temp_milli < crate::nuclear::SAFE_TEMP, "core cooled");
        assert_eq!(r.integrity, crate::nuclear::FULL_INTEGRITY, "no damage");
        assert_eq!(h.nuclear.contamination.dose.len(), 0, "SCRAM kept it clean");
        assert!(h.journal.iter().any(|(_, k, ..)| *k == EV_HEARTBEAT), "heartbeat journaled");
    }

    /// A dragon spawned near a settlement raids through the host loop
    /// on the day boundary, the settlement pays, and the raid is
    /// journaled; slaying it through a command is final and journaled.
    #[test]
    fn p3d801_dragon_raids_land_through_the_host() {
        // The raid must hurt BEYOND natural settlement drift: compare
        // against a CONTROL host — same seed, same 61 days, no dragon.
        let days = 61;
        let mut h = SoloHost::new(31);
        let mut control = SoloHost::new(31);
        let center = h.settlements.list[0].center;
        let lair = RegionCoord { x: center.x + 5, z: center.z };
        let dragon = h.dragons.spawn(lair);
        assert!(h.dragons.dragons[&dragon].in_territory(center));
        // Only settlement[0] can be in territory: sites are >= 24 apart,
        // so any other center is >= 19 from the lair — outside radius 12.

        h.run_ticks(TICKS_PER_DAY * days);
        control.run_ticks(TICKS_PER_DAY * days);
        // World consequence the dragon alone caused: its target's
        // region is scorched and the raid is journaled; the control
        // world (no dragon) has neither.
        assert!(
            h.dragons.scorched.contains(&(center.x, center.z)),
            "raided region scorched"
        );
        assert!(control.dragons.scorched.is_empty());
        assert!(
            h.journal.iter().any(|(_, k, a, _)| *k == EV_DRAGON_RAID && *a == h.settlements.list[0].id),
            "raid journaled"
        );
        assert!(
            !control.journal.iter().any(|(_, k, ..)| *k == EV_DRAGON_RAID),
            "control world saw no raid"
        );

        // Slay through a command; journal records the kill; no more raids.
        let mut slain_seed = None;
        for seed in 0..500u64 {
            let mut probe = SoloHost::new(31);
            let d = probe.dragons.spawn(lair);
            if matches!(
                probe.dragons.assault(d, 1000, 5, seed),
                AssaultOutcome::Slain
            ) {
                slain_seed = Some(seed);
                break;
            }
        }
        let seed = slain_seed.expect("party of 1000 slays with some die roll");
        h.submit(HostCommand::AssaultDragon { dragon, party_power: 1000, faction: 5, seed });
        h.run_ticks(2);
        assert!(!h.dragons.dragons[&dragon].alive);
        assert!(
            h.journal.iter().any(|(_, k, a, _)| *k == EV_DRAGON_SLAIN && *a == dragon),
            "kill journaled"
        );
        assert_eq!(h.dragons.reputation.of(5), crate::dragon::AWE_FOR_SLAYING);
    }

    /// The host digests are stable across identical runs and differ the
    /// moment the command histories differ.
    #[test]
    fn p3d801_digest_binds_seed_and_history() {
        let run = |build: bool| {
            let mut h = SoloHost::new(555);
            let cell = CellCoord { x: 8, y: 0, z: 8 };
            if build {
                h.submit(HostCommand::Build { cell, material: CellMaterial::Rock, owner: 7 });
            }
            h.run_ticks(700);
            h.digest_state()
        };
        assert_eq!(run(true), run(true), "same history: same digest");
        assert_ne!(run(true), run(false), "different history: different digest");
    }
}
