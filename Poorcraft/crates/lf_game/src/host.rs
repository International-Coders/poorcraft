//! B03 integrated host, slice 1: block edits
//! (docs/BETA-FOUNDATION/08-BETA-DELIVERY-ROADMAP.md, Stage A).
//!
//! `SimHost` is the seam between "the client wanted an edit" and "the world
//! changed". Client systems no longer mutate the world directly; they queue
//! a command (with a monotonic id, a sim tick, and the reason for the edit)
//! and the host applies pending commands to the `lf_voxel::World` in
//! canonical (tick, id) order, recording one domain event per outcome.
//!
//! In singleplayer the client applies pending commands in the same frame it
//! queues them, so behavior is unchanged — but every edit is now
//! idempotent-able (duplicate ids are rejected), auditable (the event log is
//! the edit history), and portable (B08 moves this exact type to the
//! dedicated server; the client code does not change).

use crate::crafting::{self, CraftBlock, CraftOutcome};
use crate::sim::{CommandEnvelope, CommandSequencer, EventLog, SimHash, TickClock};
use crate::survival::Inventory;
use lf_voxel::{BlockState, World};

/// Why an edit was issued. Becomes part of the edit's domain event so the
/// log can answer "what broke this block" without stringly payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    Mine,
    Place,
    /// Machine-driven swap (e.g. refinery/forming output cells).
    Machine,
    /// Fluid sim cells (drain/fill during flow steps).
    Fluid,
    /// Falling-block landing / support removal.
    Falling,
    /// Console / debug / onboarding helpers.
    Console,
    /// A server-authoritative mirror update in multiplayer (BlockUpdate).
    Server,
}

impl EditKind {
    fn code(self) -> u64 {
        match self {
            EditKind::Mine => 1,
            EditKind::Place => 2,
            EditKind::Machine => 3,
            EditKind::Fluid => 4,
            EditKind::Falling => 5,
        EditKind::Console => 6,
        EditKind::Server => 7,
    }
    }
}

/// The host's command vocabulary. Slice 1 carried block edits; slice 2
/// adds inventory/crafting transactions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostCommand {
    SetBlock {
        x: i32,
        y: i32,
        z: i32,
        state: BlockState,
        reason: EditKind,
    },
    Craft {
        ingredients: Vec<(String, u8)>,
        output: String,
        output_count: u8,
        qty: u32,
    },
}

/// Domain-event kinds recorded by the host.
pub const EV_BLOCK_SET: u32 = 1;
pub const EV_BLOCK_REJECT: u32 = 2;
pub const EV_CRAFT: u32 = 3;
pub const EV_CRAFT_REJECT: u32 = 4;

/// Outcome of one host-applied craft command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CraftReceipt {
    pub id: u64,
    /// Items granted — `Some` only when the craft applied.
    pub granted: Option<u32>,
    /// Blocked reason code (1 = missing ingredient, 2 = no room, 0 = replay).
    pub blocked: Option<u8>,
    /// Player-facing reason line for blocked crafts.
    pub reason: Option<String>,
}

/// Packs (x, y, z) into one event word: 21 signed bits per axis
/// (±1,048,575 — the same magnitude the seed lab exercises).
pub fn pack_xyz(x: i32, y: i32, z: i32) -> u64 {
    let c = |v: i32| ((v.clamp(-1_048_575, 1_048_575) + 1_048_575) as u64) & 0x1F_FFFF;
    c(x) | (c(y) << 21) | (c(z) << 42)
}

/// The local authoritative simulation host.
#[derive(Clone, Debug)]
pub struct SimHost {
    pub clock: TickClock,
    seq: CommandSequencer,
    pub log: EventLog,
    pending: Vec<CommandEnvelope<HostCommand>>,
    pending_crafts: Vec<CommandEnvelope<HostCommand>>,
    /// Ids already applied — cross-batch duplicate protection (a replayed
    /// command after packet loss applies once, ever).
    applied_ids: std::collections::BTreeSet<u64>,
    pub applied: u64,
    pub rejected: u64,
}

impl Default for SimHost {
    fn default() -> Self {
        Self::new()
    }
}

impl SimHost {
    pub fn new() -> Self {
        SimHost {
            clock: TickClock::new(),
            seq: CommandSequencer::new(),
            log: EventLog::new(),
            pending: Vec::new(),
            pending_crafts: Vec::new(),
            applied_ids: std::collections::BTreeSet::new(),
            applied: 0,
            rejected: 0,
        }
    }

    /// Queue one block edit; returns its command id. QUEUING NEVER MUTATES
    /// THE WORLD — the edit happens in `apply_pending` or not at all.
    pub fn queue_set_block(&mut self, x: i32, y: i32, z: i32, state: BlockState, reason: EditKind) -> u64 {
        let id = self.seq.assign();
        self.pending.push(CommandEnvelope::new(
            id,
            self.clock.tick,
            HostCommand::SetBlock { x, y, z, state, reason },
        ));
        id
    }

    /// Apply every pending command in canonical (tick, id) order against
    /// `world`, recording one event per outcome. Returns how many applied.
    /// Duplicate ids (already applied) are counted as rejected, not applied.
    pub fn apply_pending(&mut self, world: &mut World) -> usize {
        let batch = std::mem::take(&mut self.pending);
        let mut applied = 0;
        for env in CommandEnvelope::canonical_batch(batch) {
            if !self.applied_ids.insert(env.id) {
                self.rejected += 1;
                self.log.record(self.clock.tick, EV_BLOCK_REJECT, [env.id, 0]);
                continue;
            }
            let (x, y, z, state, reason) = match env.command {
                HostCommand::SetBlock { x, y, z, state, reason } => (x, y, z, state, reason),
                HostCommand::Craft { .. } => unreachable!("block queue holds only SetBlock"),
            };
            match world.set_block(x, y, z, state) {
                Some((_cx, _cz)) => {
                    self.applied += 1;
                    applied += 1;
                    self.log.record(
                        self.clock.tick,
                        EV_BLOCK_SET,
                        [pack_xyz(x, y, z), state.0 as u64 | (reason.code() << 32)],
                    );
                }
                None => {
                    self.rejected += 1;
                    self.log.record(self.clock.tick, EV_BLOCK_REJECT, [env.id, 1]);
                }
            }
        }
        applied
    }

    /// Queue one crafting transaction; returns its command id. QUEUING
    /// NEVER MUTATES THE INVENTORY.
    pub fn queue_craft(&mut self, ingredients: Vec<(String, u8)>, output: String,
                       output_count: u8, qty: u32) -> u64 {
        let id = self.seq.assign();
        self.pending_crafts.push(CommandEnvelope::new(
            id,
            self.clock.tick,
            HostCommand::Craft { ingredients, output, output_count, qty },
        ));
        id
    }

    /// Apply every pending craft transaction in canonical (tick, id) order
    /// against `inv`; returns one receipt per applied command, in order.
    /// Replayed ids are rejected with `blocked: Some(0)`.
    pub fn apply_pending_crafts(&mut self, inv: &mut Inventory) -> Vec<CraftReceipt> {
        let batch = std::mem::take(&mut self.pending_crafts);
        let mut receipts = Vec::new();
        for env in CommandEnvelope::canonical_batch(batch) {
            if !self.applied_ids.insert(env.id) {
                self.rejected += 1;
                self.log.record(self.clock.tick, EV_CRAFT_REJECT, [env.id, 0]);
                receipts.push(CraftReceipt { id: env.id, granted: None, blocked: Some(0), reason: None });
                continue;
            }
            let (ingredients, output, output_count, qty) = match env.command {
                HostCommand::Craft { ingredients, output, output_count, qty } =>
                    (ingredients, output, output_count, qty),
                HostCommand::SetBlock { .. } => unreachable!("craft queue holds only Craft"),
            };
            match crafting::execute(inv, &ingredients, &output, output_count, qty) {
                CraftOutcome::Crafted { granted, .. } => {
                    self.applied += 1;
                    self.log.record(
                        self.clock.tick,
                        EV_CRAFT,
                        [fnv1a(output.as_bytes()), qty as u64 | ((granted as u64) << 32)],
                    );
                    receipts.push(CraftReceipt { id: env.id, granted: Some(granted), blocked: None, reason: None });
                }
                CraftOutcome::Blocked(b) => {
                    self.rejected += 1;
                    let code = match b {
                        CraftBlock::MissingIngredient { .. } => 1u8,
                        CraftBlock::NoRoom { .. } => 2u8,
                    };
                    self.log.record(self.clock.tick, EV_CRAFT_REJECT, [env.id, code as u64]);
                    receipts.push(CraftReceipt {
                        id: env.id,
                        granted: None,
                        blocked: Some(code),
                        reason: Some(b.reason().to_string()),
                    });
                }
            }
        }
        receipts
    }

    /// The singleplayer convenience: queue a craft and apply it in the same
    /// step, returning its receipt (the same same-frame semantics as block
    /// edits). The transaction itself stays atomic — a blocked craft leaves
    /// the inventory untouched and is rejected WITH an event.
    pub fn craft_now(&mut self, inv: &mut Inventory, ingredients: Vec<(String, u8)>,
                     output: String, output_count: u8, qty: u32) -> CraftReceipt {
        let id = self.queue_craft(ingredients, output, output_count, qty);
        let mut receipts = self.apply_pending_crafts(inv);
        receipts.retain(|r| r.id == id);
        receipts.into_iter().next().unwrap_or(CraftReceipt { id, granted: None, blocked: Some(0), reason: None })
    }

    /// Host snapshot fingerprint: tick, counters, and the event log spine.
    /// Equal hosts that consumed equal command streams hash equal.
    pub fn snapshot_hash(&self) -> SimHash {
        let mut h = SimHash::new();
        h.mix_u64(self.clock.tick);
        h.mix_u64(self.applied);
        h.mix_u64(self.rejected);
        h.mix_u64(self.log.next_seq());
        h.mix_u64(self.log.hash().0);
        h
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn pending_crafts_len(&self) -> usize {
        self.pending_crafts.len()
    }
}

/// FNV-1a 64 over bytes (event payload for string ids).
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirt() -> BlockState {
        BlockState(9)
    }

    fn world_with_ground() -> World {
        let mut w = World::new();
        for cx in -1..=1 {
            for cz in -1..=1 {
                w.chunks.insert((cx, cz), lf_voxel::ChunkColumn::empty());
            }
        }
        for x in -2..=2 {
            for z in -2..=2 {
                w.set_block(x, 10, z, dirt());
            }
        }
        w
    }

    /// The core migration contract: the only way a client edit reaches the
    /// world is queue -> apply, and every application leaves exactly one
    /// event with the edit's coordinates, state, and reason.
    #[test]
    fn b03_edits_apply_only_through_the_host_and_are_event_sourced() {
        let mut world = world_with_ground();
        let mut host = SimHost::new();
        let id = host.queue_set_block(1, 11, 1, dirt(), EditKind::Place);
        assert_eq!(host.pending_len(), 1);
        // Queuing did not mutate the world.
        assert_eq!(world.get_block(1, 11, 1), BlockState(0));
        assert_eq!(host.apply_pending(&mut world), 1);
        assert_eq!(world.get_block(1, 11, 1), dirt());
        assert_eq!(host.applied, 1);
        assert_eq!(host.rejected, 0);
        assert_eq!(host.pending_len(), 0);
        let ev = host.log.iter().next().expect("one event");
        assert_eq!(ev.kind, EV_BLOCK_SET);
        assert_eq!(ev.seq, 0);
        assert_eq!(ev.payload[0], pack_xyz(1, 11, 1));
        assert_eq!(ev.payload[1] & 0xFFFF_FFFF, dirt().0 as u64);
        assert_eq!(ev.payload[1] >> 32, EditKind::Place.code());
        let _ = id;
    }

    /// A replayed command id (packet loss recovery, double-delivery)
    /// applies once, ever.
    #[test]
    fn b03_duplicate_command_ids_apply_once() {
        let mut world = world_with_ground();
        let mut host = SimHost::new();
        let id = host.queue_set_block(0, 11, 0, dirt(), EditKind::Place);
        assert_eq!(host.apply_pending(&mut world), 1);
        // Simulate the same command arriving again (different tick).
        host.pending.push(CommandEnvelope::new(
            id,
            host.clock.tick + 5,
            HostCommand::SetBlock { x: 0, y: 11, z: 0, state: BlockState(0), reason: EditKind::Mine },
        ));
        assert_eq!(host.apply_pending(&mut world), 0, "replay must be rejected");
        assert_eq!(world.get_block(0, 11, 0), dirt(), "world must be untouched by the replay");
        assert_eq!(host.rejected, 1);
        assert!(host.log.iter().any(|e| e.kind == EV_BLOCK_REJECT));
    }

    /// Edits outside loaded space are rejected WITH an event, never silently.
    #[test]
    fn b03_unloaded_edits_are_rejected_with_events() {
        let mut world = world_with_ground();
        let mut host = SimHost::new();
        host.queue_set_block(1, 9999, 1, dirt(), EditKind::Console);
        assert_eq!(host.apply_pending(&mut world), 0);
        assert_eq!(host.rejected, 1);
        let ev = host.log.iter().next().expect("reject event");
        assert_eq!(ev.kind, EV_BLOCK_REJECT);
        assert_eq!(ev.payload[1], 1, "reject reason: unloaded/out of range");
    }

    /// Application order is canonical (tick, id) regardless of delivery
    /// order, and the snapshot hash is stable when idle but sensitive to
    /// work. Same commands (same ids) delivered reversed hash equal.
    #[test]
    fn b03_apply_order_is_total_and_snapshot_hash_tracks_state() {
        let mut world = world_with_ground();
        let mut a = SimHost::new();
        let mut b = SimHost::new();
        // Identical command streams, identical ids...
        a.queue_set_block(0, 11, 0, BlockState(1), EditKind::Place);
        a.queue_set_block(1, 11, 0, BlockState(2), EditKind::Place);
        b.queue_set_block(0, 11, 0, BlockState(1), EditKind::Place);
        b.queue_set_block(1, 11, 0, BlockState(2), EditKind::Place);
        // ...but B's delivery order is reversed.
        b.pending.reverse();
        a.apply_pending(&mut world);
        b.apply_pending(&mut world);
        assert_eq!(a.applied, b.applied);
        assert_eq!(a.snapshot_hash(), b.snapshot_hash(), "same work must hash equal");
        let idle = a.snapshot_hash();
        a.queue_set_block(2, 11, 0, BlockState(3), EditKind::Place);
        a.apply_pending(&mut world);
        assert_ne!(a.snapshot_hash(), idle, "new work must move the fingerprint");
    }

    /// Slice 2: a craft is a host command too. Applied crafts record an
    /// EV_CRAFT event with the output hash, qty, and granted count.
    #[test]
    fn b03_crafts_apply_through_the_host_and_are_event_sourced() {
        let mut inv = Inventory::new();
        inv.add_item("log", 4);
        let mut host = SimHost::new();
        let receipt = host.craft_now(&mut inv, vec![("log".into(), 1)], "planks".into(), 4, 2);
        assert_eq!(receipt.granted, Some(8), "2 batches of 4 planks");
        assert_eq!(inv.count_of("log"), 2, "exactly 2 logs consumed");
        assert_eq!(inv.count_of("planks"), 8);
        assert_eq!(host.pending_crafts_len(), 0);
        let ev = host.log.iter().next().expect("craft event");
        assert_eq!(ev.kind, EV_CRAFT);
        assert_eq!(ev.payload[0], fnv1a(b"planks"));
        assert_eq!(ev.payload[1] & 0xFFFF_FFFF, 2, "qty");
        assert_eq!(ev.payload[1] >> 32, 8, "granted");
    }

    /// A blocked craft consumes nothing, is rejected WITH an event, and the
    /// receipt says why (1 = missing ingredient, 2 = no room).
    #[test]
    fn b03_blocked_crafts_reject_with_events_and_untouched_inventory() {
        let mut inv = Inventory::new();
        inv.add_item("log", 1);
        let mut host = SimHost::new();
        let short = host.craft_now(&mut inv, vec![("iron".into(), 3)], "anvil".into(), 1, 1);
        assert_eq!(short.granted, None);
        assert_eq!(short.blocked, Some(1));
        assert_eq!(inv.count_of("log"), 1, "blocked craft must not consume");
        assert_eq!(host.rejected, 1);

        // Fill the inventory to force NoRoom: a full grid of filler stacks.
        let mut full = Inventory::new();
        for i in 0..36 {
            full.add_item(&format!("filler{i}"), 64);
        }
        let mut host2 = SimHost::new();
        let no_room = host2.craft_now(&mut full, vec![], "planks".into(), 4, 100);
        assert_eq!(no_room.blocked, Some(2));
        assert_eq!(host2.rejected, 1);
        assert!(host2.log.iter().all(|e| e.kind == EV_CRAFT_REJECT));
    }

    /// A replayed craft command id applies once, ever.
    #[test]
    fn b03_replayed_craft_command_applies_once() {
        let mut inv = Inventory::new();
        inv.add_item("log", 8);
        let mut host = SimHost::new();
        let id = host.queue_craft(vec![("log".into(), 1)], "planks".into(), 4, 1);
        assert_eq!(host.apply_pending_crafts(&mut inv).len(), 1);
        host.pending_crafts.push(CommandEnvelope::new(
            id,
            host.clock.tick + 3,
            HostCommand::Craft {
                ingredients: vec![("log".into(), 1)],
                output: "planks".into(),
                output_count: 4,
                qty: 1,
            },
        ));
        let receipts = host.apply_pending_crafts(&mut inv);
        assert!(receipts.iter().all(|r| r.blocked == Some(0)), "replay must be rejected");
        assert_eq!(inv.count_of("planks"), 4, "no double grant");
        assert_eq!(inv.count_of("log"), 7);
    }
}
