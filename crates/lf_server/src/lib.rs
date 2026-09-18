//! Multiplayer server: an authoritative-lite simulation over UDP. Owns the
//! canonical block world (worldgen + client edits), relays player states,
//! block updates and chat. Run standalone via the loreforge-server binary
//! or in-process from tests.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use lf_game::survival::Inventory;
use lf_protocol::{ClientMessage, ProtocolCodec, ServerMessage, PROTOCOL_VERSION};
use lf_voxel::{BlockState, World};
use lf_worldgen::{Seed, WorldGen};

pub const DEFAULT_PORT: u16 = 25565;

struct Player {
    name: String,
    addr: SocketAddr,
    pos: [f32; 3],
    yaw: f32,
    /// THE JOIN-MODE LAW (protocol v8): the joiner's claimed game mode,
    /// fixed for the session (a world is created in one mode). A CREATIVE
    /// joiner's placements are ungated — creative is infinite by its own
    /// law — while a SURVIVAL joiner pays for every placement at the
    /// place-payment gate. A client claim of prior-session state (the
    /// PackSync bootstrap tier), not a server-verified fact.
    creative: bool,
}

/// THE ESCROW LAW: a trade completes only when BOTH canonical ledgers can
/// pay — the offerer holds every `give`, the accepter holds every `want`,
/// and each receipt fits (simulated removes-first on a clone, so freed
/// slots count). Success moves both ledgers exactly; ANY failure moves
/// nothing and names the failing side. The same player cannot trade with
/// themselves (a one-sided escrow to one ledger is a duplication machine,
/// not a trade).
fn escrow(
    from: &mut Inventory,
    to: &mut Inventory,
    give: &[(String, u8)],
    want: &[(String, u8)],
) -> Result<(), String> {
    for (id, n) in give {
        if from.count_of(id) < *n as u32 {
            return Err(format!("the offerer no longer holds {}x{}", n, id));
        }
    }
    for (id, n) in want {
        if to.count_of(id) < *n as u32 {
            return Err(format!("the accepter no longer holds {}x{}", n, id));
        }
    }
    // Room: simulate the whole swap removes-first on clones — payment
    // frees slots the receipt may use.
    let mut trial_from = from.clone();
    let mut trial_to = to.clone();
    for (id, n) in give {
        trial_from.remove_count(id, *n as u32);
    }
    for (id, n) in want {
        trial_to.remove_count(id, *n as u32);
    }
    for (id, n) in want {
        if trial_from.add_item(id, *n) > 0 {
            return Err(format!("the offerer has no room for {}x{}", n, id));
        }
    }
    for (id, n) in give {
        if trial_to.add_item(id, *n) > 0 {
            return Err(format!("the accepter has no room for {}x{}", n, id));
        }
    }
    // Apply exactly what the trials proved.
    for (id, n) in give {
        from.remove_count(id, *n as u32);
        to.add_item(id, *n);
    }
    for (id, n) in want {
        to.remove_count(id, *n as u32);
        from.add_item(id, *n);
    }
    Ok(())
}

/// THE CRAFT REPLAY WINDOW: a client-chosen `req_id` answered recently is
/// remembered, so a duplicated datagram (or a re-sent request) is refused
/// with a no-op refusal instead of executing the craft a second time —
/// the crafting twin of the dig pays-once law. Bounded FIFO over
/// (player, req) pairs; the refusal moves nothing on either side.
const CRAFT_REPLAY_WINDOW: usize = 512;

/// THE FURNACE ACCOUNT (protocol v9): one furnace's committed goods for
/// one player — what their LEDGER paid into this fire and what it may
/// still take back out of it. `inputs` and `fuels` hold committed slot
/// goods by item id; `outputs` holds BACKED bars (each one paid for by a
/// committed input plus `SMELT_TIME` of burn); `burn` is the banked burn
/// time in seconds, funded when fuel is committed and spent by smelts.
/// Nothing here exists without a ledger payment behind it.
#[derive(Clone, Debug, Default)]
pub struct FurnaceAccount {
    pub inputs: HashMap<String, u32>,
    pub fuels: HashMap<String, u32>,
    pub outputs: HashMap<String, u32>,
    pub burn: f32,
}

impl FurnaceAccount {
    /// Total committed input count for one item.
    fn input_of(&self, item: &str) -> u32 {
        self.inputs.get(item).copied().unwrap_or(0)
    }

    /// Total committed (still unburned) fuel-slot count for one item.
    fn fuel_of(&self, item: &str) -> u32 {
        self.fuels.get(item).copied().unwrap_or(0)
    }

    /// Total banked seconds represented by the committed fuel items.
    /// A kind with no burn time (junk in the fuel slot) banks none and is
    /// never spent by the reconciliation below.
    fn fuel_seconds_banked(&self) -> f32 {
        self.fuels.iter()
            .map(|(id, n)| lf_game::smelting::fuel_seconds(id) * *n as f32)
            .sum()
    }

    /// THE BURN RECONCILIATION: keep the committed fuel items within the
    /// banked seconds — as the bank is spent by smelts, whole fuel items
    /// leave the commitments (they burned; they cannot be taken back
    /// out). The spend order is deterministic — positive-burn kinds by
    /// item id — so two servers replaying the same ops agree.
    fn reconcile_burn(&mut self) {
        let mut kinds: Vec<String> = self.fuels.keys()
            .filter(|id| lf_game::smelting::fuel_seconds(id) > 0.0)
            .cloned()
            .collect();
        kinds.sort();
        for id in kinds {
            while self.fuel_seconds_banked() > self.burn {
                match self.fuels.get_mut(&id) {
                    Some(n) if *n > 0 => {
                        *n -= 1;
                        if *n == 0 {
                            self.fuels.remove(&id);
                        }
                    }
                    _ => break,
                }
            }
        }
    }
}

/// What a granted smelt op moves through the LEDGER: `take` is consumed
/// from the player's canonical pack (a deposit's payment), `give` is
/// added to it (a withdrawal's grant). Both are pre-verified by the
/// caller against the ledger before they are applied — the account op
/// itself never fails on ledger grounds, so a commit is unconditional.
#[derive(Clone, Debug, PartialEq)]
pub struct SmeltLedgerMove {
    pub take: Vec<(String, u32)>,
    pub give: Vec<(String, u32)>,
}

/// THE FURNACE-OP LAW (protocol v9), the pure heart of server-side
/// smelting — the same three doors the client's furnace UI plays:
/// - DEPOSIT: goods leave the ledger and enter the commitments. Fuel
///   banks its burn seconds the moment it is committed (an item with no
///   burn time banks none — junk in the fuel slot sits, withdrawable,
///   never burned).
/// - WITHDRAW: goods leave the commitments and return to the ledger —
///   but a fuel item is withdrawable only while its seconds remain
///   banked: an unburned item leaves, a burned one cannot be taken back.
/// - SMELT-DONE: one committed input plus `SMELT_TIME` of banked burn
///   becomes one BACKED output bar of the realm's own smelt law
///   (`lf_game::smelting::smelt_result` — the same table the client's
///   furnace plays; what the fire cannot smelt, it does not yield).
/// ANY refusal moves nothing anywhere and says why.
pub fn smelt_op(
    account: &mut FurnaceAccount,
    op: &lf_protocol::SmeltOp,
) -> Result<SmeltLedgerMove, String> {
    use lf_protocol::{SmeltOp, SmeltSlot};
    match op {
        SmeltOp::Deposit { slot, item, count, .. } => {
            if *count == 0 {
                return Err("nothing to commit".into());
            }
            match slot {
                SmeltSlot::Input => {
                    *account.inputs.entry(item.clone()).or_insert(0) += count;
                }
                SmeltSlot::Fuel => {
                    *account.fuels.entry(item.clone()).or_insert(0) += count;
                    account.burn += lf_game::smelting::fuel_seconds(item) * *count as f32;
                }
                SmeltSlot::Output => {
                    *account.outputs.entry(item.clone()).or_insert(0) += count;
                }
            }
            Ok(SmeltLedgerMove {
                take: vec![(item.clone(), *count)],
                give: vec![],
            })
        }
        SmeltOp::Withdraw { slot, item, count, .. } => {
            if *count == 0 {
                return Err("nothing to take".into());
            }
            match slot {
                SmeltSlot::Input => {
                    let held = account.input_of(item);
                    if held < *count {
                        return Err(format!("the furnace holds no {}x{} {}", count, held, item));
                    }
                    *account.inputs.get_mut(item).unwrap() -= count;
                }
                SmeltSlot::Fuel => {
                    let held = account.fuel_of(item);
                    if held < *count {
                        return Err(format!("the furnace holds no {}x{} {}", count, held, item));
                    }
                    let seconds = lf_game::smelting::fuel_seconds(item) * *count as f32;
                    if account.burn < seconds {
                        return Err(format!("the fire has burned into the {}", item));
                    }
                    *account.fuels.get_mut(item).unwrap() -= count;
                    account.burn -= seconds;
                }
                SmeltSlot::Output => {
                    let held = account.outputs.get(item).copied().unwrap_or(0);
                    if held < *count {
                        return Err(format!("the furnace has not yielded {}x{} {}", count, held, item));
                    }
                    *account.outputs.get_mut(item).unwrap() -= count;
                }
            }
            Ok(SmeltLedgerMove {
                take: vec![],
                give: vec![(item.clone(), *count)],
            })
        }
        SmeltOp::SmeltDone { input, .. } => {
            let Some(out) = lf_game::smelting::smelt_result(input) else {
                return Err(format!("the fire cannot smelt {}", input));
            };
            if account.input_of(input) < 1 {
                return Err(format!("the furnace holds no {}", input));
            }
            if account.burn < lf_game::smelting::SMELT_TIME {
                return Err("the fire lacks burn".into());
            }
            *account.inputs.get_mut(input).unwrap() -= 1;
            if account.input_of(input) == 0 {
                account.inputs.remove(input);
            }
            account.burn -= lf_game::smelting::SMELT_TIME;
            account.reconcile_burn();
            *account.outputs.entry(out.to_string()).or_insert(0) += 1;
            Ok(SmeltLedgerMove { take: vec![], give: vec![] })
        }
    }
}

/// THE EAT-OP LAW (protocol v10), the pure heart of server-side eating:
/// a bite is PAID FOOD — the realm's own catalog decides what food is
/// (`lf_game::items::item_def`: what the realm does not call food, it
/// will not feed), a bite is at least one, and the LEDGER must hold the
/// count. A legal bite consumes exactly `count` from the ledger; ANY
/// refusal moves nothing and says why. (Hunger is a client-simmed stat —
/// the server gates the CONSUMPTION, not the stat; the eater applies the
/// bite's hunger and sound only when the verdict lands.)
pub fn eat_op(inv: &mut Inventory, item: &str, count: u32) -> Result<(), String> {
    let Some(def) = lf_game::items::item_def(item) else {
        return Err(format!("the realm knows no {}", item));
    };
    if !matches!(def.kind, lf_game::items::ItemKind::Food(_)) {
        return Err(format!("the realm does not call {} food", item));
    }
    if count == 0 {
        return Err("nothing to eat".into());
    }
    if inv.count_of(item) < count {
        return Err(format!("the ledger holds no {}x{}", count, item));
    }
    let removed = inv.remove_count(item, count);
    debug_assert_eq!(removed, count, "the pre-check guarantees the pay");
    Ok(())
}

fn craft_replay_seen(seen: &mut VecDeque<(u64, u64)>, seen_set: &mut std::collections::HashSet<(u64, u64)>, player: u64, req_id: u64) -> bool {
    if !seen_set.insert((player, req_id)) {
        return true;
    }
    seen.push_back((player, req_id));
    while seen.len() > CRAFT_REPLAY_WINDOW {
        if let Some(oldest) = seen.pop_front() {
            seen_set.remove(&oldest);
        }
    }
    false
}

pub struct Server {
    socket: Arc<UdpSocket>,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    local_addr: SocketAddr,
}

impl Server {
    /// Start a server bound to `bind` (e.g. "127.0.0.1:0" for tests) with
    /// the given world seed.
    pub fn start(bind: &str, seed: u64) -> std::io::Result<Self> {
        let socket = Arc::new(UdpSocket::bind(bind)?);
        socket.set_nonblocking(true)?;
        let local_addr = socket.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let worker_socket = Arc::clone(&socket);
        let worker_stop = Arc::clone(&stop);
        let handle = thread::spawn(move || {
            run(worker_socket, worker_stop, seed);
        });
        Ok(Self { socket, stop, handle: Some(handle), local_addr })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(socket: Arc<UdpSocket>, stop: Arc<AtomicBool>, seed: u64) {
    let gen = WorldGen::new(Seed(seed));
    let mut world = World::new();
    let mut players: HashMap<u64, Player> = HashMap::new();
    // THE CANONICAL LEDGERS: one per player, the server's own copy of
    // what each adventurer holds. Seeded by PackSync (the client's
    // honest claim — the server cannot know prior-session history),
    // fed by every server-known flow (mined-yield grants, escrow
    // moves), and the ONLY thing the trade gates consult.
    let mut inventories: HashMap<u64, Inventory> = HashMap::new();
    let mut edits: Vec<(i32, i32, i32, u32)> = Vec::new();
    let mut next_id: u64 = 1;
    let mut offers: HashMap<u64, lf_protocol::TradeOfferRecord> = HashMap::new();
    let mut next_offer_id: u64 = 1;
    let mut craft_seen: VecDeque<(u64, u64)> = VecDeque::new();
    let mut craft_seen_set: std::collections::HashSet<(u64, u64)> = std::collections::HashSet::new();
    // THE FURNACE COMMITMENTS (protocol v9): one account per
    // (player, furnace block) — what that adventurer's LEDGER paid into
    // that fire. Dropped when the player leaves (Goodbye).
    let mut smelt_accounts: HashMap<(u64, (i32, i32, i32)), FurnaceAccount> = HashMap::new();
    let mut smelt_seen: VecDeque<(u64, u64)> = VecDeque::new();
    let mut smelt_seen_set: std::collections::HashSet<(u64, u64)> = std::collections::HashSet::new();
    let mut eat_seen: VecDeque<(u64, u64)> = VecDeque::new();
    let mut eat_seen_set: std::collections::HashSet<(u64, u64)> = std::collections::HashSet::new();
    let mut last_snapshot = std::time::Instant::now();
    let mut buf = [0u8; 2048];

    while !stop.load(Ordering::Relaxed) {
        let mut activity = false;
        for _ in 0..64 {
            match socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    activity = true;
                    if let Some(msg) = ProtocolCodec::decode_client(&buf[..len]) {
                        handle_message(&socket, &mut players, &mut inventories, &mut world, &gen,
                            &mut edits, &mut next_id, &mut offers, &mut next_offer_id,
                            &mut craft_seen, &mut craft_seen_set,
                            &mut smelt_accounts, &mut smelt_seen, &mut smelt_seen_set,
                            &mut eat_seen, &mut eat_seen_set,
                            src, msg);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        // Broadcast player snapshots ~20/s.
        if last_snapshot.elapsed() >= Duration::from_millis(50) {
            last_snapshot = std::time::Instant::now();
            let states: Vec<(u64, [f32; 3], f32)> = players
                .iter()
                .map(|(id, p)| (*id, p.pos, p.yaw))
                .collect();
            let msg = ProtocolCodec::encode_server(&ServerMessage::PlayerStates { states });
            for p in players.values() {
                let _ = socket.send_to(&msg, p.addr);
            }
        }

        if !activity {
            thread::sleep(Duration::from_millis(2));
        }
    }
}

/// Resolve the sender's player id from its address (`None` = never
/// said Hello — every stateful message ignores such senders).
fn id_of(players: &HashMap<u64, Player>, src: SocketAddr) -> Option<u64> {
    players.iter().find(|(_, p)| p.addr == src).map(|(id, _)| *id)
}

fn handle_message(
    socket: &UdpSocket,
    players: &mut HashMap<u64, Player>,
    inventories: &mut HashMap<u64, Inventory>,
    world: &mut World,
    gen: &WorldGen,
    edits: &mut Vec<(i32, i32, i32, u32)>,
    next_id: &mut u64,
    offers: &mut HashMap<u64, lf_protocol::TradeOfferRecord>,
    next_offer_id: &mut u64,
    craft_seen: &mut VecDeque<(u64, u64)>,
    craft_seen_set: &mut std::collections::HashSet<(u64, u64)>,
    smelt_accounts: &mut HashMap<(u64, (i32, i32, i32)), FurnaceAccount>,
    smelt_seen: &mut VecDeque<(u64, u64)>,
    smelt_seen_set: &mut std::collections::HashSet<(u64, u64)>,
    eat_seen: &mut VecDeque<(u64, u64)>,
    eat_seen_set: &mut std::collections::HashSet<(u64, u64)>,
    src: SocketAddr,
    msg: ClientMessage,
) {
    match msg {
        ClientMessage::Hello { name, protocol_version, creative } => {
            if protocol_version != PROTOCOL_VERSION {
                let reply = ProtocolCodec::encode_server(&ServerMessage::Reject {
                    reason: format!("version mismatch: server {}", PROTOCOL_VERSION),
                });
                let _ = socket.send_to(&reply, src);
                return;
            }
            // A reconnect from the same address replaces the old session
            // wholesale — ledger included (the fresh PackSync re-seeds it).
            players.retain(|_, p| p.addr != src);
            let id = *next_id;
            *next_id += 1;
            let roster: Vec<(u64, String)> = players.iter().map(|(pid, p)| (*pid, p.name.clone())).collect();
            players.insert(id, Player { name: name.clone(), addr: src, pos: [0.0, 80.0, 0.0], yaw: 0.0, creative });
            inventories.insert(id, Inventory::new());
            let welcome = ProtocolCodec::encode_server(&ServerMessage::Welcome {
                your_id: id,
                seed: gen.seed(), // the true world seed (P23)
                players: roster,
            });
            let _ = socket.send_to(&welcome, src);
            let joined = ProtocolCodec::encode_server(&ServerMessage::PlayerJoined { id, name });
            for p in players.values() {
                if p.addr != src {
                    let _ = socket.send_to(&joined, p.addr);
                }
            }
            // replay the canonical edit history so the newcomer catches up
            for &(x, y, z, block) in edits.iter() {
                let upd = ProtocolCodec::encode_server(&ServerMessage::BlockUpdate { x, y, z, block });
                let _ = socket.send_to(&upd, src);
            }
        }
        ClientMessage::Position { pos, yaw, .. } => {
            if let Some(p) = players.values_mut().find(|p| p.addr == src) {
                p.pos = pos;
                p.yaw = yaw;
            }
        }
        ClientMessage::SetBlock { x, y, z, block, mine, place } => {
            // THE WIDE-STATE WIRE (v8): the u32 is a full BlockState — the
            // shape/fluid nibbles ride the high bits, so a placed slab
            // arrives a slab (it used to arrive a full cube and the peers
            // plus the editor's own chunk reload disagreed). Every law
            // below reads the id via BlockState::id().
            let state = BlockState(block);
            let block_id = state.id();
            let creative = players.values().find(|p| p.addr == src).map(|p| p.creative);
            // THE NO-SELF-ECHO LAW: the editor applied its edit optimistically
            // before sending it, so echoing an ACCEPTED edit back would
            // double-apply (a second host event for one player action plus a
            // redundant relight/remesh of the column). Peers receive the
            // update; the editor only hears from the server again when it
            // REJECTED the op — the corrective echo below — or when the op
            // was a MINE that pays (the ItemGrant, also editor-alone).
            //
            // validate: within height, and a real block (vanilla or a mod
            // block registered from a loaded mods/ dir)
            if (0..256).contains(&y) && lf_voxel::registry::is_known_block(block_id) {
                let (cx, _lx) = (x.div_euclid(16), x.rem_euclid(16));
                let (cz, _lz) = (z.div_euclid(16), z.rem_euclid(16));
                if world.chunk(cx, cz).is_none() {
                    world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
                }
                // THE SMUGGLE GUARD: one edit carries one claim. A mine
                // claim on a place (or a place claim on a mine) is a
                // forged op — refused with the corrective echo, nothing
                // applied, nothing broadcast.
                if mine.is_some() && place.is_some() {
                    let truth = world.get_block(x, y, z).0;
                    let fix = ProtocolCodec::encode_server(&ServerMessage::BlockUpdate { x, y, z, block: truth });
                    let _ = socket.send_to(&fix, src);
                    let reject = ProtocolCodec::encode_server(&ServerMessage::Reject {
                        reason: "an edit cannot both mine and place".into(),
                    });
                    let _ = socket.send_to(&reject, src);
                    return;
                }
                // THE PLACE-PAYMENT GATE (v8): a SURVIVAL joiner's
                // placement pays the ledger. The claimed item must be
                // able to place this block (lf_game::items::
                // placement_pays — the shared law; every admission is a
                // closed loop under the mining law, so paying and mining
                // back never mints) and the ledger must hold one, which
                // the gate consumes. ANY refusal moves nothing: no world
                // edit, no broadcast — the editor alone hears the
                // corrective echo plus a reasoned Reject. A CREATIVE
                // joiner's placements are ungated (creative is infinite
                // by its own law). A claim-free edit is a simulation
                // edit (fluids, falling, machines, spell effects) — the
                // client-simmed tier, accepted ungated as shipped; its
                // forged-claim residual is that tier's trust, deferred
                // with it.
                let mut refused: Option<String> = None;
                if let Some(claim) = &place {
                    if creative != Some(true) {
                        let item = claim.item.as_str();
                        if !lf_game::items::placement_pays(item, state) {
                            refused = Some(format!("{} cannot place that block", item));
                        } else {
                            match id_of(players, src).and_then(|id| inventories.get_mut(&id)) {
                                Some(inv) if inv.count_of(item) >= 1 => {
                                    inv.remove_count(item, 1);
                                }
                                _ => refused = Some(format!("the ledger holds no {}", item)),
                            }
                        }
                    }
                }
                if let Some(reason) = refused {
                    let truth = world.get_block(x, y, z).0;
                    let fix = ProtocolCodec::encode_server(&ServerMessage::BlockUpdate { x, y, z, block: truth });
                    let _ = socket.send_to(&fix, src);
                    let reject = ProtocolCodec::encode_server(&ServerMessage::Reject { reason });
                    let _ = socket.send_to(&reject, src);
                    return;
                }
                // THE YIELD-GRANT LAW: what the block was is what the server
                // had — read BEFORE the canonical world changes.
                let old = world.get_block(x, y, z).0;
                world.set_block(x, y, z, state);
                edits.push((x, y, z, block));
                let upd = ProtocolCodec::encode_server(&ServerMessage::BlockUpdate { x, y, z, block });
                for p in players.values() {
                    if p.addr == src {
                        continue;
                    }
                    let _ = socket.send_to(&upd, p.addr);
                }
                // THE YIELD IS THE SERVER'S TO GIVE: only a player MINE
                // (claimed on the wire, bare hands included) that actually
                // broke a real block pays, and it pays to the editor ALONE.
                // The harvest gate and the drop table are lf_game's — the
                // same law the client plays, one source. A place, a
                // simulation edit, a dig into already-air (a re-sent packet,
                // or a peer who mined the same block first), and a rejected
                // op all grant nothing.
                if let Some(claim) = mine {
                    let old_id = BlockState(old).id();
                    if old_id != lf_voxel::registry::block::AIR {
                        let held = claim
                            .held
                            .map(|id| lf_game::survival::ItemStack { item_id: id, count: 1 });
                        if lf_game::mining::tool_satisfies(old_id, held.as_ref()) {
                            if let Some(item) = lf_game::items::block_drop(old_id) {
                                // The yield pays the ledger first: the
                                // canonical copy of the miner's pack grows by
                                // the same drop the grant carries. Overflow
                                // (a full ledger) is the block's spill — on
                                // the ground, not held — so it is never
                                // counted here either.
                                if let Some(miner) = id_of(players, src) {
                                    if let Some(inv) = inventories.get_mut(&miner) {
                                        inv.add_item(&item, 1);
                                    }
                                }
                                let grant = ProtocolCodec::encode_server(&ServerMessage::ItemGrant {
                                    items: vec![(item, 1)],
                                });
                                let _ = socket.send_to(&grant, src);
                            }
                        }
                    }
                }
            } else if (0..256).contains(&y) {
                // THE CORRECTIVE ECHO: the op was rejected (e.g. a mod block
                // the server has not registered), but the optimistic editor
                // DID apply it locally. Answer with the server's true block
                // at that position — the same lazy generation the accept
                // path uses to be able to answer at all — so the editor
                // reverts to canonical state instead of diverging silently.
                let (cx, _lx) = (x.div_euclid(16), x.rem_euclid(16));
                let (cz, _lz) = (z.div_euclid(16), z.rem_euclid(16));
                if world.chunk(cx, cz).is_none() {
                    world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
                }
                let truth = world.get_block(x, y, z).0;
                let fix = ProtocolCodec::encode_server(&ServerMessage::BlockUpdate {
                    x, y, z, block: truth,
                });
                let _ = socket.send_to(&fix, src);
            }
            // y outside 0..256: the client's own world.set_block guard
            // refused the optimistic apply too, so there is nothing to
            // correct and nothing to broadcast.
        }
        ClientMessage::PackSync { items } => {
            // THE PACK-SYNC LAW: the client's claim replaces its ledger
            // wholesale. Rebuild through add_item so the ledger stays a
            // physically legal 36-slot pack — a claim larger than the
            // pack can hold is truncated at the pack's own law, never
            // counted. Unknown senders are ignored (no Hello, no ledger).
            if let Some(id) = id_of(players, src) {
                let mut inv = Inventory::new();
                for (item, count) in items {
                    // u32 claims add in u8-sized batches until the pack's
                    // own law stops them (a full ledger drops the rest —
                    // an oversized claim is truncated, never counted).
                    let mut remaining = count;
                    while remaining > 0 {
                        let batch = remaining.min(u8::MAX as u32) as u8;
                        let moved = batch - inv.add_item(&item, batch);
                        if moved == 0 { break; }
                        remaining -= moved as u32;
                    }
                }
                inventories.insert(id, inv);
            }
        }
        ClientMessage::CraftRequest { req_id, ingredients, output, output_count, qty } => {
            // THE CRAFT IS THE SERVER'S TO MAKE (protocol v7): while
            // connected, a workbench craft is a REQUEST — the canonical
            // ledger pays and the verdict (granted/refused + reason) goes
            // to the crafter ALONE. Two gates stand between the request
            // and the ledger: THE CRAFT-SPEC GATE (a spec the realm's own
            // recipe book does not name is refused — a connected client
            // cannot fabricate output from nothing; the same
            // `crafting::spec_matches_book` the laws test) and the
            // transactional engine itself (`crafting::execute` — validate,
            // consume, produce atomically against the LEDGER; a blocked
            // craft moves nothing and names why). A replayed req_id (a
            // duplicated datagram) is answered with a no-op refusal, so a
            // craft pays once. Unknown senders (no Hello, no ledger) are
            // ignored like every stateful message.
            let Some(id) = id_of(players, src) else { return };
            if craft_replay_seen(craft_seen, craft_seen_set, id, req_id) {
                let replay = ProtocolCodec::encode_server(&ServerMessage::CraftVerdict {
                    req_id, granted: false, consumed: vec![], output: None,
                    reason: Some("already answered".into()),
                });
                let _ = socket.send_to(&replay, src);
                return;
            }
            let verdict = if !lf_game::crafting::spec_matches_book(&ingredients, &output, output_count) {
                ServerMessage::CraftVerdict {
                    req_id, granted: false, consumed: vec![], output: None,
                    reason: Some("no recipe by that name is in the realm's book".into()),
                }
            } else {
                match inventories.get_mut(&id) {
                    Some(inv) => match lf_game::crafting::execute(inv, &ingredients, &output, output_count, qty) {
                        lf_game::crafting::CraftOutcome::Crafted { output, granted } => {
                            // THE VERDICT IS THE DELTA: exactly what the
                            // ledger consumed and what it produced (a
                            // granted craft's consumption is ledger-bounded,
                            // so the u64 product always fits u32).
                            let consumed: Vec<(String, u32)> = ingredients.iter()
                                .map(|(id, n)| (id.clone(), (*n as u64 * qty as u64) as u32))
                                .collect();
                            ServerMessage::CraftVerdict {
                                req_id, granted: true, consumed,
                                output: Some((output, granted)), reason: None,
                            }
                        }
                        lf_game::crafting::CraftOutcome::Blocked(b) =>
                            ServerMessage::CraftVerdict {
                                req_id, granted: false, consumed: vec![], output: None,
                                reason: Some(b.reason()),
                            },
                    },
                    None => return,
                }
            };
            let _ = socket.send_to(&ProtocolCodec::encode_server(&verdict), src);
        }
        ClientMessage::SmeltRequest { req_id, op } => {
            // THE FURNACE IS THE SERVER'S FIRE (protocol v9): while
            // connected, a furnace's economy is a sequence of gated ops
            // against the player's canonical LEDGER and this furnace's
            // own commitment account. A DEPOSIT consumes the ledger (the
            // pack's claim includes the held cursor, so the goods cannot
            // ride in two places at once); a WITHDRAW pays out of the
            // account's commitments and grants the goods back to the
            // ledger; a SMELT-DONE transforms one committed input plus
            // SMELT_TIME of banked burn into one backed bar of the
            // realm's own smelt law. A replayed req_id (a duplicated
            // datagram) is answered with a no-op refusal, so an op pays
            // once. EVERY refusal moves nothing and answers the smelter
            // ALONE (a reasoned SmeltVerdict); peers hear nothing about
            // another player's furnace. Unknown senders (no Hello, no
            // ledger) are ignored like every stateful message.
            let Some(id) = id_of(players, src) else { return };
            if craft_replay_seen(smelt_seen, smelt_seen_set, id, req_id) {
                let replay = ProtocolCodec::encode_server(&ServerMessage::Smelt(
                    lf_protocol::SmeltVerdict {
                        req_id, granted: false,
                        reason: Some("already answered".into()),
                    },
                ));
                let _ = socket.send_to(&replay, src);
                return;
            }
            let verdict_reason: Option<String> = {
                // Ledger pre-checks before the account op: a deposit must
                // be payable, a withdrawal's grant must fit the ledger's
                // own 36-slot law (trial on a clone — the escrow idiom).
                let op_reason = match &op {
                    lf_protocol::SmeltOp::Deposit { item, count, .. } => {
                        match inventories.get(&id) {
                            Some(inv) if inv.count_of(item) >= *count => None,
                            Some(_) => Some(format!("the ledger holds no {}x{}", count, item)),
                            None => Some("no ledger".into()),
                        }
                    }
                    lf_protocol::SmeltOp::Withdraw { item, count, .. } => {
                        match inventories.get(&id) {
                            Some(inv) => {
                                let mut trial = inv.clone();
                                let mut remaining = *count;
                                while remaining > 0 {
                                    let batch = remaining.min(u8::MAX as u32) as u8;
                                    let moved = batch - trial.add_item(item, batch);
                                    if moved == 0 { break; }
                                    remaining -= moved as u32;
                                }
                                if remaining > 0 {
                                    Some(format!("the ledger's pack has no room for {}x{}", count, item))
                                } else {
                                    None
                                }
                            }
                            None => Some("no ledger".into()),
                        }
                    }
                    lf_protocol::SmeltOp::SmeltDone { .. } => None,
                };
                if op_reason.is_some() {
                    op_reason
                } else {
                    let pos = match &op {
                        lf_protocol::SmeltOp::Deposit { pos, .. }
                        | lf_protocol::SmeltOp::Withdraw { pos, .. }
                        | lf_protocol::SmeltOp::SmeltDone { pos, .. } => *pos,
                    };
                    // One commitment account per (player, furnace): two
                    // fires never pool each other's fuel.
                    let account = smelt_accounts.entry((id, pos)).or_default();
                    match smelt_op(account, &op) {
                        Ok(moves) => {
                            // The commit is unconditional here: the ledger
                            // pre-checks verified both directions before
                            // the account moved.
                            let inv = inventories.get_mut(&id).expect("pre-checked");
                            for (item, count) in &moves.take {
                                inv.remove_count(item, *count);
                            }
                            for (item, count) in &moves.give {
                                let mut remaining = *count;
                                while remaining > 0 {
                                    let batch = remaining.min(u8::MAX as u32) as u8;
                                    let moved = batch - inv.add_item(item, batch);
                                    if moved == 0 { break; }
                                    remaining -= moved as u32;
                                }
                            }
                            None
                        }
                        Err(reason) => Some(reason),
                    }
                }
            };
            let verdict = ProtocolCodec::encode_server(&ServerMessage::Smelt(
                lf_protocol::SmeltVerdict { req_id, granted: verdict_reason.is_none(), reason: verdict_reason },
            ));
            let _ = socket.send_to(&verdict, src);
        }
        ClientMessage::EatRequest { req_id, item, count } => {
            // THE BITE IS THE SERVER'S TO FEED (protocol v10): while
            // connected, a SURVIVAL bite is a REQUEST — the canonical
            // ledger pays and the verdict (granted/refused + reason)
            // goes to the eater ALONE. Two gates stand between the
            // request and the ledger: THE FOOD GATE (the realm's own
            // catalog decides what food is — `eat_op`'s item_def door;
            // a connected client cannot eat its pickaxe or a stone) and
            // THE LEDGER ITSELF (the bite must be paid — `count` leaves
            // the pack exactly). A replayed req_id (a duplicated
            // datagram) is answered with a no-op refusal, so a bite pays
            // once (the craft pays-once law's eating twin). Peers hear
            // nothing about another player's appetite. Unknown senders
            // (no Hello, no ledger) are ignored like every stateful
            // message.
            let Some(id) = id_of(players, src) else { return };
            if craft_replay_seen(eat_seen, eat_seen_set, id, req_id) {
                let replay = ProtocolCodec::encode_server(&ServerMessage::Eat(
                    lf_protocol::EatVerdict {
                        req_id, granted: false,
                        reason: Some("already answered".into()),
                    },
                ));
                let _ = socket.send_to(&replay, src);
                return;
            }
            let verdict = match inventories.get_mut(&id) {
                Some(inv) => match eat_op(inv, &item, count) {
                    Ok(()) => ServerMessage::Eat(lf_protocol::EatVerdict {
                        req_id, granted: true, reason: None,
                    }),
                    Err(reason) => ServerMessage::Eat(lf_protocol::EatVerdict {
                        req_id, granted: false, reason: Some(reason),
                    }),
                },
                None => return,
            };
            let _ = socket.send_to(&ProtocolCodec::encode_server(&verdict), src);
        }
        ClientMessage::TradeOffer { to, give, want } => {
            // P37 escrow: register the offer, notify the recipient.
            // THE OFFER GATE (protocol v6): the offerer's canonical
            // ledger must hold every offered item — a phantom offer is
            // refused to the offerer alone and the target hears nothing.
            let Some(from_id) = id_of(players, src) else { return };
            let from_name = players.get(&from_id).map(|p| p.name.clone()).unwrap_or_default();
            if players.get(&to).is_none() {
                let reply = ProtocolCodec::encode_server(&ServerMessage::Reject {
                    reason: "trade target is not online".into(),
                });
                let _ = socket.send_to(&reply, src);
                return;
            }
            if from_id == to {
                let reply = ProtocolCodec::encode_server(&ServerMessage::Reject {
                    reason: "you cannot trade with yourself".into(),
                });
                let _ = socket.send_to(&reply, src);
                return;
            }
            let covers_offer = inventories.get(&from_id).is_some_and(|inv| {
                give.iter().all(|(id, n)| inv.count_of(id) >= *n as u32)
            });
            if !covers_offer {
                let reply = ProtocolCodec::encode_server(&ServerMessage::Reject {
                    reason: "your pack does not hold the offered goods".into(),
                });
                let _ = socket.send_to(&reply, src);
                return;
            }
            let offer_id = *next_offer_id;
            *next_offer_id += 1;
            offers.insert(offer_id, lf_protocol::TradeOfferRecord {
                offer_id, from: from_id, to, give: give.clone(), want: want.clone(),
            });
            let reply = ProtocolCodec::encode_server(&ServerMessage::TradeOffered {
                offer_id, from: from_id, from_name, give, want,
            });
            if let Some(target) = players.get(&to) {
                let _ = socket.send_to(&reply, target.addr);
            }
        }
        ClientMessage::TradeAccept { offer_id } => {
            // THE ESCROW IS THE SERVER'S: only the target may complete an
            // offer (a third party neither completes nor dissolves what is
            // not theirs), and the swap is validated against BOTH
            // canonical ledgers by `escrow` before anything moves —
            // success moves both ledgers exactly; failure moves nothing
            // and answers the failing side with the reason.
            let Some(sender_id) = id_of(players, src) else { return };
            let Some(offer) = offers.remove(&offer_id) else { return };
            if sender_id != offer.to {
                offers.insert(offer_id, offer);
                return;
            }
            // Take both ledgers out (the offer gate forbids from == to),
            // run the escrow, and put them back whatever the verdict —
            // a failed escrow's ledgers return UNMOVED.
            let from_ledger = inventories.remove(&offer.from);
            let to_ledger = inventories.remove(&offer.to);
            let (mut from_inv, mut to_inv) = match (from_ledger, to_ledger) {
                (Some(f), Some(t)) => (f, t),
                (f, t) => {
                    // A party left: the offer dissolves, nothing moves.
                    if let Some(inv) = f { inventories.insert(offer.from, inv); }
                    if let Some(inv) = t { inventories.insert(offer.to, inv); }
                    let gone = ProtocolCodec::encode_server(&ServerMessage::TradeResolved {
                        offer_id, accepted: false, items: vec![],
                    });
                    if let Some(t) = players.get(&offer.to) { let _ = socket.send_to(&gone, t.addr); }
                    if let Some(f) = players.get(&offer.from) { let _ = socket.send_to(&gone, f.addr); }
                    return;
                }
            };
            let verdict = escrow(&mut from_inv, &mut to_inv, &offer.give, &offer.want);
            inventories.insert(offer.from, from_inv);
            inventories.insert(offer.to, to_inv);
            match verdict {
                Ok(()) => {
                    let to_accepter = ProtocolCodec::encode_server(&ServerMessage::TradeResolved {
                        offer_id, accepted: true, items: offer.give.clone(),
                    });
                    let to_offerer = ProtocolCodec::encode_server(&ServerMessage::TradeResolved {
                        offer_id, accepted: true, items: offer.want.clone(),
                    });
                    if let Some(t) = players.get(&offer.to) {
                        let _ = socket.send_to(&to_accepter, t.addr);
                    }
                    if let Some(f) = players.get(&offer.from) {
                        let _ = socket.send_to(&to_offerer, f.addr);
                    }
                }
                Err(reason) => {
                    let to_accepter = ProtocolCodec::encode_server(&ServerMessage::TradeResolved {
                        offer_id, accepted: false, items: vec![],
                    });
                    let to_offerer = ProtocolCodec::encode_server(&ServerMessage::TradeResolved {
                        offer_id, accepted: false, items: vec![],
                    });
                    let reject = ProtocolCodec::encode_server(&ServerMessage::Reject { reason });
                    if let Some(t) = players.get(&offer.to) {
                        let _ = socket.send_to(&to_accepter, t.addr);
                        let _ = socket.send_to(&reject, t.addr);
                    }
                    if let Some(f) = players.get(&offer.from) {
                        let _ = socket.send_to(&to_offerer, f.addr);
                        let _ = socket.send_to(&reject, f.addr);
                    }
                }
            }
        }
        ClientMessage::TradeCancel { offer_id } => {
            let sender_id = players.iter().find(|(_, p)| p.addr == src).map(|(id, _)| *id).unwrap_or(0);
            if let Some(offer) = offers.remove(&offer_id) {
                if offer.from == sender_id || offer.to == sender_id {
                    let msg = ProtocolCodec::encode_server(&ServerMessage::TradeResolved {
                        offer_id, accepted: false, items: vec![],
                    });
                    if let Some(t) = players.get(&offer.to) {
                        let _ = socket.send_to(&msg, t.addr);
                    }
                    if let Some(f) = players.get(&offer.from) {
                        let _ = socket.send_to(&msg, f.addr);
                    }
                } else {
                    offers.insert(offer_id, offer);
                }
            }
        }
        ClientMessage::Chat { text } => {
            let from = players.values().find(|p| p.addr == src).map(|p| p.name.clone()).unwrap_or_default();
            let chat = ProtocolCodec::encode_server(&ServerMessage::Chat { from, text });
            for p in players.values() {
                let _ = socket.send_to(&chat, p.addr);
            }
        }
        ClientMessage::Goodbye => {
            let leaving = id_of(players, src);
            if let Some(id) = leaving {
                players.remove(&id);
                inventories.remove(&id);
                // THE FURNACE COMMITMENTS BURN OUT: a leaver's furnace
                // accounts drop with their ledger — the next join starts
                // with nothing committed to any fire.
                smelt_accounts.retain(|(pid, _), _| *pid != id);
                let left = ProtocolCodec::encode_server(&ServerMessage::PlayerLeft { id });
                for p in players.values() {
                    let _ = socket.send_to(&left, p.addr);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lf_protocol::ProtocolCodec;

    fn pump(ms: u64) {
        thread::sleep(Duration::from_millis(ms));
    }

    fn drain(socket: &UdpSocket) -> Vec<ServerMessage> {
        let mut out = Vec::new();
        let mut buf = [0u8; 2048];
        loop {
            match socket.recv(&mut buf) {
                Ok(len) => {
                    if let Some(msg) = ProtocolCodec::decode_server(&buf[..len]) {
                        out.push(msg);
                    }
                }
                Err(_) => break,
            }
        }
        out
    }

    /// Poll a non-blocking socket until `pred` matches a decoded message or
    /// `ms` elapses. The server generates terrain on first touch, so fixed
    /// sleeps are not enough under parallel test load.
    fn drain_until(socket: &UdpSocket, ms: u64, pred: impl Fn(&ServerMessage) -> bool) -> Option<ServerMessage> {
        let deadline = std::time::Instant::now() + Duration::from_millis(ms);
        let mut buf = [0u8; 2048];
        while std::time::Instant::now() < deadline {
            match socket.recv(&mut buf) {
                Ok(len) => {
                    if let Some(msg) = ProtocolCodec::decode_server(&buf[..len]) {
                        if pred(&msg) {
                            return Some(msg);
                        }
                    }
                }
                Err(_) => thread::sleep(Duration::from_millis(5)),
            }
        }
        None
    }

    /// Full local integration: two clients join, exchange chat, one edits a
    /// block, the other receives the update, positions snapshot.
    #[test]
    fn two_clients_chat_and_block_sync() {
        let mut server = Server::start("127.0.0.1:0", 12345).expect("start server");
        let addr = server.local_addr();

        let c1 = UdpSocket::bind("127.0.0.1:0").unwrap();
        let c2 = UdpSocket::bind("127.0.0.1:0").unwrap();
        c1.set_nonblocking(true).unwrap();
        c2.set_nonblocking(true).unwrap();
        c1.connect(addr).unwrap();
        c2.connect(addr).unwrap();

        c1.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
            name: "alice".into(), protocol_version: PROTOCOL_VERSION, creative: false,
        })).unwrap();
        pump(150);
        c2.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
            name: "bob".into(), protocol_version: PROTOCOL_VERSION, creative: false,
        })).unwrap();
        pump(150);

        let c1_msgs = drain(&c1);
        assert!(c1_msgs.iter().any(|m| matches!(m, ServerMessage::Welcome { .. })), "alice welcome");
        assert!(c1_msgs.iter().any(|m| matches!(m, ServerMessage::PlayerJoined { name, .. } if name == "bob")),
            "alice sees bob join");

        c1.send(&ProtocolCodec::encode_client(&ClientMessage::Chat { text: "hi bob".into() })).unwrap();
        pump(150);
        let c2_msgs = drain(&c2);
        assert!(c2_msgs.iter().any(|m| matches!(m, ServerMessage::Chat { from, text } if from == "alice" && text == "hi bob")),
            "bob receives chat");

        c2.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock { x: 5, y: 70, z: -3, block: 1, mine: None, place: None })).unwrap();
        assert!(drain_until(&c1, 5000, |m| matches!(m,
            ServerMessage::BlockUpdate { x: 5, y: 70, z: -3, block: 1 })).is_some(),
            "alice receives block update");

        c1.send(&ProtocolCodec::encode_client(&ClientMessage::Position { pos: [10.0, 80.0, 10.0], yaw: 0.0, pitch: 0.0 })).unwrap();
        pump(200);
        let c2_msgs = drain(&c2);
        assert!(c2_msgs.iter().any(|m| matches!(m, ServerMessage::PlayerStates { states }
            if states.iter().any(|(_, pos, _)| pos == &[10.0, 80.0, 10.0]))),
            "bob sees alice position");

        server.stop();
    }

    /// P37: the full trade escrow over real UDP — offer, deliver to both
    /// sides on accept, and a cancel path that frees the offer.
    #[test]
    fn trade_escrow_over_real_udp() {
        let mut server = Server::start("127.0.0.1:0", 12345).expect("start server");
        let addr = server.local_addr();
        let c1 = UdpSocket::bind("127.0.0.1:0").unwrap();
        let c2 = UdpSocket::bind("127.0.0.1:0").unwrap();
        c1.set_nonblocking(true).unwrap();
        c2.set_nonblocking(true).unwrap();
        c1.connect(addr).unwrap();
        c2.connect(addr).unwrap();
        // both hellos, then both packs: alice holds 4 iron + 1 coal, bob
        // holds the dragon scale — THE OFFER GATE (v6) admits only what
        // the canonical ledgers cover.
        for (sock, name) in [(&c1, "alice"), (&c2, "bob")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(150);
        c1.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("iron_ingot".into(), 4), ("coal".into(), 1)],
        })).unwrap();
        c2.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("dragon_scale".into(), 1)],
        })).unwrap();
        pump(150);
        let welcome1 = drain(&c1);
        let alice_id = welcome1.iter().find_map(|m| match m {
            ServerMessage::Welcome { your_id, players, .. } => {
                let _ = players;
                Some(*your_id)
            }
            _ => None,
        }).expect("alice has an id");
        let bob_id = drain(&c2).iter().find_map(|m| match m {
            ServerMessage::Welcome { your_id, .. } => Some(*your_id),
            _ => None,
        }).expect("bob has an id");

        // alice offers 4 iron for bob's dragon scale
        c1.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: bob_id,
            give: vec![("iron_ingot".into(), 4)],
            want: vec![("dragon_scale".into(), 1)],
        })).unwrap();
        let offered = drain_until(&c2, 5000, |m| matches!(m,
            ServerMessage::TradeOffered { from, give, want, .. }
                if *from == alice_id && give[0].0 == "iron_ingot" && want[0].0 == "dragon_scale"));
        assert!(offered.is_some(), "bob receives the offer");
        let offer_id = match offered.unwrap() {
            ServerMessage::TradeOffered { offer_id, .. } => offer_id,
            _ => unreachable!(),
        };

        // bob accepts: BOTH sides get their delivery
        c2.send(&ProtocolCodec::encode_client(&ClientMessage::TradeAccept { offer_id })).unwrap();
        let bob_res = drain_until(&c2, 5000, |m| matches!(m,
            ServerMessage::TradeResolved { accepted: true, items, .. } if items[0].0 == "iron_ingot"));
        assert!(bob_res.is_some(), "bob receives the iron");
        let alice_res = drain_until(&c1, 5000, |m| matches!(m,
            ServerMessage::TradeResolved { accepted: true, items, .. } if items[0].0 == "dragon_scale"));
        assert!(alice_res.is_some(), "alice receives the scale");

        // a cancelled offer frees both sides with no items
        c1.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: bob_id, give: vec![("coal".into(), 1)], want: vec![],
        })).unwrap();
        let offered2 = drain_until(&c2, 5000, |m| matches!(m, ServerMessage::TradeOffered { .. }));
        assert!(offered2.is_some());
        let offer2 = match offered2.unwrap() {
            ServerMessage::TradeOffered { offer_id, .. } => offer_id,
            _ => unreachable!(),
        };
        c1.send(&ProtocolCodec::encode_client(&ClientMessage::TradeCancel { offer_id: offer2 })).unwrap();
        let cancel1 = drain_until(&c1, 5000, |m| matches!(m,
            ServerMessage::TradeResolved { accepted: false, items, .. } if items.is_empty()));
        assert!(cancel1.is_some(), "the offerer is freed on cancel");
        let cancel2 = drain_until(&c2, 5000, |m| matches!(m,
            ServerMessage::TradeResolved { accepted: false, items, .. } if items.is_empty()));
        assert!(cancel2.is_some(), "the target is freed on cancel");

        server.stop();
    }

    /// P25 regression + THE NO-SELF-ECHO LAW + THE CORRECTIVE ECHO, over
    /// real UDP: mod blocks (ids >= 100 from a loaded mods/ dir) are
    /// accepted and relayed to PEERS (the old `block <= 18` cap rejected
    /// them all); the editor never receives its own accepted edit back
    /// (it applied optimistically — an echo would double-apply); and a
    /// REJECTED op (unknown id) answers the editor alone with the server's
    /// true block at that position so its optimistic world reverts instead
    /// of diverging silently.
    #[test]
    fn set_block_no_self_echo_and_corrective_reject() {
        use lf_voxel::registry::{is_known_block, register_mod_block, ModBlockDef};

        let probe_id = 9101;
        assert!(register_mod_block(probe_id, ModBlockDef {
            name: "server_test:udp_probe".into(),
            solid: true,
            opaque: true,
            drop: None, light: 0 }));
        assert!(is_known_block(probe_id), "precondition: probe registered");
        let unknown_id = lf_voxel::registry::MAX_VANILLA_BLOCK + 1;
        assert!(!is_known_block(unknown_id), "precondition: unknown id");

        let mut server = Server::start("127.0.0.1:0", 777).expect("start server");
        let addr = server.local_addr();
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        let observer = UdpSocket::bind("127.0.0.1:0").unwrap();
        sender.set_nonblocking(true).unwrap();
        observer.set_nonblocking(true).unwrap();
        sender.connect(addr).unwrap();
        observer.connect(addr).unwrap();
        for (sock, name) in [(&sender, "editor"), (&observer, "peer")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(150);
        let _ = drain(&sender);
        let _ = drain(&observer);

        // A mod-block edit is accepted and relayed — to the peer only.
        sender.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock { x: 2, y: 70, z: 2, block: probe_id, mine: None, place: None })).unwrap();
        assert!(drain_until(&observer, 5000, |m| matches!(m,
            ServerMessage::BlockUpdate { x: 2, y: 70, z: 2, block } if *block == probe_id)).is_some(),
            "mod block edit is accepted and relayed to the peer");
        pump(300);
        assert!(!drain(&sender).iter().any(|m| matches!(m,
            ServerMessage::BlockUpdate { x: 2, y: 70, z: 2, .. })),
            "THE NO-SELF-ECHO LAW: the editor receives nothing for its own accepted edit");

        // An unknown-id op is rejected — the editor alone gets the server's
        // true block at that position (never the unknown id), and the peer
        // hears nothing (nothing happened in the shared world).
        sender.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock { x: 3, y: 70, z: 3, block: unknown_id, mine: None, place: None })).unwrap();
        let fix = drain_until(&sender, 5000, |m| matches!(m,
            ServerMessage::BlockUpdate { x: 3, y: 70, z: 3, .. }));
        match fix {
            Some(ServerMessage::BlockUpdate { block, .. }) =>
                assert_eq!(block, 0, "the corrective echo carries the server's true (air) block"),
            other => panic!("expected a corrective echo, got {:?}", other),
        }
        pump(300);
        assert!(!drain(&observer).iter().any(|m| matches!(m,
            ServerMessage::BlockUpdate { x: 3, y: 70, z: 3, .. })),
            "a rejected op never touches the peers");

        server.stop();
    }

    /// THE NEWCOMER REPLAY FAMILY: every edit in the server's canonical
    /// history is replayed to a client that joins later — including edits
    /// in chunks the newcomer has not generated yet (the client-side
    /// replay window buffers those until the chunks stream in).
    #[test]
    fn newcomer_receives_the_full_edit_history() {
        let mut server = Server::start("127.0.0.1:0", 4242).expect("start server");
        let addr = server.local_addr();
        let a = UdpSocket::bind("127.0.0.1:0").unwrap();
        a.set_nonblocking(true).unwrap();
        a.connect(addr).unwrap();
        a.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
            name: "early".into(), protocol_version: PROTOCOL_VERSION, creative: false,
        })).unwrap();
        pump(150);
        let _ = drain(&a);

        // Three digs/placements across three chunks, one far from spawn.
        let history = [
            (5, 70, 5, 0u32),      // dug (air) near spawn
            (-33, 80, 12, 3u32),   // placed in chunk (-3, 0)
            (40, 90, -49, 1u32),   // placed in chunk (2, -4)
        ];
        for &(x, y, z, block) in history.iter() {
            a.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock { x, y, z, block, mine: None, place: None })).unwrap();
        }
        pump(300);

        let b = UdpSocket::bind("127.0.0.1:0").unwrap();
        b.set_nonblocking(true).unwrap();
        b.connect(addr).unwrap();
        b.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
            name: "newcomer".into(), protocol_version: PROTOCOL_VERSION, creative: false,
        })).unwrap();

        // The newcomer's replay must carry every historical edit.
        let mut seen = 0usize;
        let deadline = std::time::Instant::now() + Duration::from_millis(5000);
        let mut buf = [0u8; 2048];
        while seen < history.len() && std::time::Instant::now() < deadline {
            if let Ok(len) = b.recv(&mut buf) {
                if let Some(m) = ProtocolCodec::decode_server(&buf[..len]) {
                    if let ServerMessage::BlockUpdate { x, y, z, block } = m {
                        if history.iter().any(|&(hx, hy, hz, hb)| hx == x && hy == y && hz == z && hb == block) {
                            seen += 1;
                        }
                    }
                }
            } else {
                thread::sleep(Duration::from_millis(5));
            }
        }
        assert_eq!(seen, history.len(), "the newcomer's replay covers the full edit history");

        server.stop();
    }

    /// THE YIELD-GRANT LAW, over real UDP: an accepted MINE pays the
    /// canonical yield to the EDITOR ALONE — the peer sees the block
    /// update and never a grant — and a PLACE never pays anyone. The
    /// staged blocks sit at y=200, far above any terrain, so the law does
    /// not depend on where the seed put its stone.
    #[test]
    fn mined_yields_are_granted_to_the_editor_alone() {
        use lf_voxel::registry::block;

        let mut server = Server::start("127.0.0.1:0", 909).expect("start server");
        let addr = server.local_addr();
        let editor = UdpSocket::bind("127.0.0.1:0").unwrap();
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        editor.set_nonblocking(true).unwrap();
        peer.set_nonblocking(true).unwrap();
        editor.connect(addr).unwrap();
        peer.connect(addr).unwrap();
        for (sock, name) in [(&editor, "miner"), (&peer, "watcher")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(150);
        let _ = drain(&editor);
        let _ = drain(&peer);

        // A PLACE never grants — not to the placer, not to the peer.
        editor.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
            x: 2, y: 200, z: 2, block: block::STONE, mine: None, place: None,
        })).unwrap();
        assert!(drain_until(&peer, 5000, |m| matches!(m,
            ServerMessage::BlockUpdate { x: 2, y: 200, z: 2, block } if *block == block::STONE)).is_some(),
            "the peer sees the placed block");
        pump(300);
        assert!(!drain(&editor).iter().any(|m| matches!(m, ServerMessage::ItemGrant { .. })),
            "a place never pays the placer");
        assert!(!drain(&peer).iter().any(|m| matches!(m, ServerMessage::ItemGrant { .. })),
            "a place never pays the peer");

        // The MINE pays: the canonical stone yield, to the editor alone.
        editor.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
            x: 2, y: 200, z: 2, block: block::AIR,
            mine: Some(lf_protocol::MineClaim { held: Some("stone_pickaxe".into()) }), place: None,
        })).unwrap();
        let grant = drain_until(&editor, 5000, |m| matches!(m, ServerMessage::ItemGrant { .. }));
        match grant {
            Some(ServerMessage::ItemGrant { items }) =>
                assert_eq!(items, vec![("stone".into(), 1)], "the canonical stone yield"),
            other => panic!("the editor's mine must pay, got {:?}", other),
        }
        assert!(drain_until(&peer, 5000, |m| matches!(m,
            ServerMessage::BlockUpdate { x: 2, y: 200, z: 2, block } if *block == block::AIR)).is_some(),
            "the peer sees the dug block");
        pump(300);
        assert!(!drain(&peer).iter().any(|m| matches!(m, ServerMessage::ItemGrant { .. })),
            "THE YIELD-GRANT LAW: the peer never receives a grant");

        server.stop();
    }

    /// THE HARVEST GATE HOLDS SERVER-SIDE: the wire claim is evaluated by
    /// the same lf_game law the client plays — iron ore pays only to a
    /// stone-or-better pick, and a mine claimed with an inadequate tool
    /// pays nothing (the block still breaks: the update is canonical).
    #[test]
    fn the_harvest_gate_holds_server_side() {
        use lf_voxel::registry::block;

        let mut server = Server::start("127.0.0.1:0", 911).expect("start server");
        let addr = server.local_addr();
        let editor = UdpSocket::bind("127.0.0.1:0").unwrap();
        editor.set_nonblocking(true).unwrap();
        editor.connect(addr).unwrap();
        editor.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
            name: "gater".into(), protocol_version: PROTOCOL_VERSION, creative: false,
        })).unwrap();
        pump(150);
        let _ = drain(&editor);

        // Stage iron ore at a fresh spot above any terrain, then dig it
        // with the claimed hand; answer whether a grant arrived. (The
        // editor never sees its own place echoed — THE NO-SELF-ECHO LAW —
        // and loopback UDP keeps the pair in order, so the server stages
        // before it digs.)
        let mine = |sock: &UdpSocket, x: i32, held: Option<&str>| {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
                x, y: 200, z: 9, block: block::IRON_ORE, mine: None, place: None,
            })).unwrap();
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
                x, y: 200, z: 9, block: block::AIR,
                mine: Some(lf_protocol::MineClaim { held: held.map(|s| s.to_string()) }), place: None,
            })).unwrap();
            pump(250);
            drain(sock).into_iter().any(|m| matches!(m, ServerMessage::ItemGrant { .. }))
        };

        assert!(!mine(&editor, 4, Some("wooden_pickaxe")),
            "iron ore needs a stone-or-better pick: a wooden pick's mine pays nothing");
        assert!(mine(&editor, 6, Some("stone_pickaxe")),
            "a stone pick's iron mine pays");
        assert!(!mine(&editor, 8, None),
            "a bare-handed iron mine claims the mine but fails the gate");

        server.stop();
    }

    /// A DIG PAYS ONCE: a re-sent mine into already-air (a lost-packet
    /// duplicate, or a peer who mined the same block first) grants nothing,
    /// and a REJECTED op (unknown block id) never pays — the corrective
    /// echo still reverts the optimistic editor, and no grant rides with it.
    #[test]
    fn a_dig_pays_once_and_rejected_ops_never_pay() {
        use lf_voxel::registry::block;

        let mut server = Server::start("127.0.0.1:0", 913).expect("start server");
        let addr = server.local_addr();
        let editor = UdpSocket::bind("127.0.0.1:0").unwrap();
        let late = UdpSocket::bind("127.0.0.1:0").unwrap();
        editor.set_nonblocking(true).unwrap();
        late.set_nonblocking(true).unwrap();
        editor.connect(addr).unwrap();
        late.connect(addr).unwrap();
        for (sock, name) in [(&editor, "first"), (&late, "second")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(150);
        let _ = drain(&editor);
        let _ = drain(&late);

        // First miner breaks the staged block: exactly one grant. (No
        // self-echo wait — the placer never hears its own place; loopback
        // UDP keeps the staged pair in order.)
        editor.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
            x: 12, y: 200, z: 12, block: block::DIRT, mine: None, place: None,
        })).unwrap();
        let mine = ClientMessage::SetBlock {
            x: 12, y: 200, z: 12, block: block::AIR,
            mine: Some(lf_protocol::MineClaim { held: None }), place: None,
        };
        let bytes = ProtocolCodec::encode_client(&mine);
        editor.send(&bytes).unwrap();
        assert!(drain_until(&editor, 5000, |m| matches!(m, ServerMessage::ItemGrant { .. })).is_some(),
            "the first (and only) dig pays");
        pump(300);
        let _ = drain(&editor);

        // The same mine again — a duplicate packet, or the second player's
        // optimistic dig of a block that is already gone: canonical air,
        // BlockUpdate to peers, but NO second grant to anyone.
        editor.send(&bytes).unwrap();
        pump(300);
        assert!(!drain(&editor).iter().any(|m| matches!(m, ServerMessage::ItemGrant { .. })),
            "a re-sent dig into already-air never pays again");
        assert!(!drain(&late).iter().any(|m| matches!(m, ServerMessage::ItemGrant { .. })),
            "peers never see grants, duplicate or not");

        // A rejected op (unknown id) never pays: the corrective echo
        // reverts the editor, and no grant rides with it.
        let unknown = lf_voxel::registry::MAX_VANILLA_BLOCK + 1;
        late.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
            x: 14, y: 200, z: 14, block: unknown,
            mine: Some(lf_protocol::MineClaim { held: Some("stone_pickaxe".into()) }), place: None,
        })).unwrap();
        let fix = drain_until(&late, 5000, |m| matches!(m,
            ServerMessage::BlockUpdate { x: 14, y: 200, z: 14, .. }));
        assert!(matches!(fix, Some(ServerMessage::BlockUpdate { block: 0, .. })),
            "the corrective echo answers the optimistic editor");
        pump(300);
        assert!(!drain(&late).iter().any(|m| matches!(m, ServerMessage::ItemGrant { .. })),
            "a rejected op never pays");
        assert!(!drain(&editor).iter().any(|m| matches!(m, ServerMessage::ItemGrant { .. })),
            "a rejected op never pays anyone else either");

        server.stop();
    }

    /// THE ESCROW LAW, pure: a trade completes only when BOTH ledgers can
    /// pay — a short hold on either side moves NOTHING — and a successful
    /// swap moves both ledgers exactly, with payment freeing room the
    /// receipt may use (removes-first on the trial).
    #[test]
    fn the_escrow_moves_both_ledgers_or_neither() {
        let pack = |items: &[(&str, u8)]| {
            let mut inv = Inventory::new();
            for (id, n) in items {
                inv.add_item(id, *n);
            }
            inv
        };
        let give = vec![("iron_ingot".to_string(), 4u8)];
        let want = vec![("dragon_scale".to_string(), 1u8)];

        // The happy swap: both ledgers move exactly.
        let mut alice = pack(&[("iron_ingot", 4)]);
        let mut bob = pack(&[("dragon_scale", 1)]);
        escrow(&mut alice, &mut bob, &give, &want).expect("a covered swap completes");
        assert_eq!(alice.count_of("iron_ingot"), 0, "the offer paid exactly");
        assert_eq!(alice.count_of("dragon_scale"), 1, "the offerer received exactly");
        assert_eq!(bob.count_of("dragon_scale"), 0, "the accepter paid exactly");
        assert_eq!(bob.count_of("iron_ingot"), 4, "the accepter received exactly");

        // A short hold on EITHER side moves nothing at all.
        let mut alice = pack(&[("iron_ingot", 3)]);
        let mut bob = pack(&[("dragon_scale", 1)]);
        assert!(escrow(&mut alice, &mut bob, &give, &want).is_err(),
            "three irons cannot pay four");
        assert_eq!(alice.count_of("iron_ingot"), 3, "the failed escrow left the offerer alone");
        assert_eq!(bob.count_of("dragon_scale"), 1, "the failed escrow left the accepter alone");

        let mut alice = pack(&[("iron_ingot", 4)]);
        let mut bob = pack(&[]);
        assert!(escrow(&mut alice, &mut bob, &give, &want).is_err(),
            "an accepter who holds no scale cannot complete the swap");
        assert_eq!(alice.count_of("iron_ingot"), 4, "nothing moved on the failed accept");

        // Payment frees room: a full pack can still RECEIVE if it pays
        // first (the trial simulates removes before adds). `other` gives
        // 1 log; `full` pays 64 stone and receives the log.
        let mut full = pack(&[("stone", 64), ("dirt", 64)]);
        let mut other = pack(&[("log", 1)]);
        let give = vec![("log".to_string(), 1u8)];
        let want = vec![("stone".to_string(), 64u8)];
        escrow(&mut other, &mut full, &give, &want).expect("paying frees the receipt's room");
        assert_eq!(full.count_of("log"), 1, "the receipt landed in the freed room");
        assert_eq!(full.count_of("stone"), 0, "the payment went out");
    }

    /// THE PACK-SYNC LAW + THE OFFER GATE, over real UDP: the uploaded
    /// pack seeds the canonical ledger, a MINED yield pays into the same
    /// ledger (the grant's stone is gate-visible though it was never
    /// uploaded), and an offer beyond the ledger is refused to the
    /// offerer alone while the target hears nothing.
    #[test]
    fn the_pack_sync_seeds_the_ledger_and_the_gate_counts_mined_grants() {
        use lf_voxel::registry::block;

        let mut server = Server::start("127.0.0.1:0", 915).expect("start server");
        let addr = server.local_addr();
        let alice = UdpSocket::bind("127.0.0.1:0").unwrap();
        let bob = UdpSocket::bind("127.0.0.1:0").unwrap();
        alice.set_nonblocking(true).unwrap();
        bob.set_nonblocking(true).unwrap();
        alice.connect(addr).unwrap();
        bob.connect(addr).unwrap();
        for (sock, name) in [(&alice, "miner"), (&bob, "target")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(150);
        let _ = drain(&alice);
        let _ = drain(&bob);
        alice.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("iron_ingot".into(), 2)],
        })).unwrap();
        pump(150);

        // Mine a staged stone block with an honest claim: the grant pays.
        alice.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
            x: 2, y: 200, z: 2, block: block::STONE, mine: None, place: None,
        })).unwrap();
        alice.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
            x: 2, y: 200, z: 2, block: block::AIR,
            mine: Some(lf_protocol::MineClaim { held: Some("stone_pickaxe".into()) }), place: None,
        })).unwrap();
        let grant = drain_until(&alice, 5000, |m| matches!(m, ServerMessage::ItemGrant { items }
            if items.first().map(|(id, _)| id.as_str()) == Some("stone")));
        assert!(grant.is_some(), "the staged stone pays the miner");

        // THE LEDGER COUNTED THE GRANT: an offer of exactly the mined
        // stone passes the gate though the stone was never uploaded.
        // (alice joined first: her id is 1, bob's is 2.)
        let offer = |give: Vec<(String, u8)>| {
            alice.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
                to: 2, give, want: vec![],
            })).unwrap();
            pump(250);
        };
        offer(vec![("stone".into(), 1)]);
        assert!(drain(&bob).iter().any(|m| matches!(m, ServerMessage::TradeOffered { give, .. }
            if give.first().map(|(id, _)| id.as_str()) == Some("stone"))),
            "the mined stone is gate-visible: the ledger counted the grant");

        // Beyond the ledger — phantom in either direction — refuses.
        offer(vec![("stone".into(), 2)]);
        assert!(drain_until(&alice, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "two stones were never held: the offer is refused");
        offer(vec![("iron_ingot".into(), 3)]);
        assert!(drain_until(&alice, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "three irons exceed the uploaded two: refused");
        pump(300);
        assert!(!drain(&bob).iter().any(|m| matches!(m, ServerMessage::TradeOffered { .. })),
            "THE OFFER GATE: the target never hears a phantom offer");

        // Exactly what the ledger holds passes.
        offer(vec![("iron_ingot".into(), 2)]);
        assert!(drain(&bob).iter().any(|m| matches!(m, ServerMessage::TradeOffered { give, .. }
            if give.first().map(|(id, _)| id.as_str()) == Some("iron_ingot"))),
            "the uploaded two irons cover an offer of two");

        server.stop();
    }

    /// THE ESCROW MOVES BOTH LEDGERS, over real UDP: after a completed
    /// trade each side's ledger shows the swap — proven through the gate
    /// alone, with no ledger peeking: the paid-away goods refuse further
    /// offers, the received goods admit them.
    #[test]
    fn the_escrow_moves_both_ledgers_atomically_over_real_udp() {
        let mut server = Server::start("127.0.0.1:0", 917).expect("start server");
        let addr = server.local_addr();
        let a = UdpSocket::bind("127.0.0.1:0").unwrap();
        let b = UdpSocket::bind("127.0.0.1:0").unwrap();
        a.set_nonblocking(true).unwrap();
        b.set_nonblocking(true).unwrap();
        a.connect(addr).unwrap();
        b.connect(addr).unwrap();
        for (sock, name) in [(&a, "alice"), (&b, "bob")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(150);
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("iron_ingot".into(), 4)],
        })).unwrap();
        b.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("dragon_scale".into(), 1)],
        })).unwrap();
        pump(150);

        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("iron_ingot".into(), 4)], want: vec![("dragon_scale".into(), 1)],
        })).unwrap();
        let offered = drain_until(&b, 5000, |m| matches!(m, ServerMessage::TradeOffered { .. }));
        let offer_id = match offered {
            Some(ServerMessage::TradeOffered { offer_id, .. }) => offer_id,
            other => panic!("bob receives the covered offer, got {other:?}"),
        };
        b.send(&ProtocolCodec::encode_client(&ClientMessage::TradeAccept { offer_id })).unwrap();
        let bob_done = drain_until(&b, 5000, |m| matches!(m,
            ServerMessage::TradeResolved { accepted: true, items, .. } if items[0].0 == "iron_ingot"));
        assert!(bob_done.is_some(), "bob receives the iron over the escrow");
        let alice_done = drain_until(&a, 5000, |m| matches!(m,
            ServerMessage::TradeResolved { accepted: true, items, .. } if items[0].0 == "dragon_scale"));
        assert!(alice_done.is_some(), "alice receives the scale over the escrow");
        pump(300);

        // THE LEDGERS MOVED, proven through the gate alone. (Ids 1 and 2
        // are deterministic — join order.)
        let offer = |sock: &UdpSocket, to: u64, give: Vec<(String, u8)>| {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
                to, give, want: vec![],
            })).unwrap();
            pump(250);
        };
        offer(&a, 2, vec![("iron_ingot".into(), 4)]);
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "alice paid her four irons away: the ledger refuses a re-offer");
        offer(&a, 2, vec![("dragon_scale".into(), 1)]);
        assert!(drain(&b).iter().any(|m| matches!(m, ServerMessage::TradeOffered { .. })),
            "the scale alice RECEIVED is hers to offer");
        offer(&b, 1, vec![("dragon_scale".into(), 1)]);
        assert!(drain_until(&b, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "bob paid his scale away: the ledger refuses a re-offer");
        offer(&b, 1, vec![("iron_ingot".into(), 4)]);
        assert!(drain(&a).iter().any(|m| matches!(m, ServerMessage::TradeOffered { .. })),
            "the irons bob RECEIVED are his to offer");
        offer(&b, 1, vec![("iron_ingot".into(), 5)]);
        assert!(drain_until(&b, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "the received four irons do not cover five");

        server.stop();
    }

    /// A FAILED ACCEPT DISSOLVES WITHOUT MOVING ANYTHING, over real UDP:
    /// an accept whose counterparty cannot pay (bob holds no scale)
    /// answers BOTH sides with the refused verdict and the reason, and
    /// the offerer's ledger is untouched — her re-offer of the same goods
    /// passes, proving the failed escrow deducted nothing.
    #[test]
    fn an_accept_that_cannot_pay_dissolves_without_moving_anything() {
        let mut server = Server::start("127.0.0.1:0", 919).expect("start server");
        let addr = server.local_addr();
        let a = UdpSocket::bind("127.0.0.1:0").unwrap();
        let b = UdpSocket::bind("127.0.0.1:0").unwrap();
        a.set_nonblocking(true).unwrap();
        b.set_nonblocking(true).unwrap();
        a.connect(addr).unwrap();
        b.connect(addr).unwrap();
        for (sock, name) in [(&a, "alice"), (&b, "bob")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(150);
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("iron_ingot".into(), 4)],
        })).unwrap();
        pump(150); // bob uploads nothing — his ledger is empty

        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("iron_ingot".into(), 4)], want: vec![("dragon_scale".into(), 1)],
        })).unwrap();
        let offered = drain_until(&b, 5000, |m| matches!(m, ServerMessage::TradeOffered { .. }));
        let offer_id = match offered {
            Some(ServerMessage::TradeOffered { offer_id, .. }) => offer_id,
            other => panic!("the offer reaches bob, got {other:?}"),
        };
        b.send(&ProtocolCodec::encode_client(&ClientMessage::TradeAccept { offer_id })).unwrap();
        assert!(drain_until(&b, 5000, |m| matches!(m,
            ServerMessage::TradeResolved { accepted: false, items, .. } if items.is_empty())).is_some(),
            "bob's accept is refused — he holds no scale");
        assert!(drain_until(&b, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "the refusal names its reason to the failing side");
        assert!(drain_until(&a, 5000, |m| matches!(m,
            ServerMessage::TradeResolved { accepted: false, .. })).is_some(),
            "alice's offer dissolves with it");
        pump(300);

        // THE ATOMICITY PROOF: the same goods re-offer cleanly — the
        // failed escrow never deducted alice's ledger.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("iron_ingot".into(), 4)], want: vec![],
        })).unwrap();
        assert!(drain_until(&b, 5000, |m| matches!(m, ServerMessage::TradeOffered { .. })).is_some(),
            "the failed accept deducted nothing: the re-offer passes the gate");

        server.stop();
    }

    /// THE SELF-TRADE HOLE IS CLOSED, over real UDP: an offer to oneself
    /// is refused outright (a one-sided escrow to one ledger was a
    /// duplication machine under the ungated v4 accept), and a THIRD
    /// PARTY cannot complete (nor dissolve) an offer that is not theirs
    /// — the offer survives for its true target.
    #[test]
    fn self_trades_and_third_party_accepts_never_move_a_ledger() {
        let mut server = Server::start("127.0.0.1:0", 921).expect("start server");
        let addr = server.local_addr();
        let a = UdpSocket::bind("127.0.0.1:0").unwrap();
        let b = UdpSocket::bind("127.0.0.1:0").unwrap();
        let c = UdpSocket::bind("127.0.0.1:0").unwrap();
        for sock in [&a, &b, &c] {
            sock.set_nonblocking(true).unwrap();
            sock.connect(addr).unwrap();
        }
        for (sock, name) in [(&a, "alice"), (&b, "bob"), (&c, "carol")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(250);
        let mut ids = [0u64; 3];
        for (i, sock) in [&a, &b, &c].into_iter().enumerate() {
            ids[i] = drain(sock).iter().find_map(|m| match m {
                ServerMessage::Welcome { your_id, .. } => Some(*your_id),
                _ => None,
            }).expect("each client has an id");
        }
        let (alice_id, bob_id) = (ids[0], ids[1]);
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("diamond".into(), 1)],
        })).unwrap();
        pump(150);

        // THE SELF-TRADE REFUSAL: the offer to oneself is dead on arrival.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: alice_id, give: vec![("diamond".into(), 1)], want: vec![("diamond".into(), 1)],
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "trading with yourself is refused, not escrowed");

        // THE THIRD-PARTY LAW: carol cannot accept bob's offer — the
        // offer survives untouched for its true target.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: bob_id, give: vec![("diamond".into(), 1)], want: vec![],
        })).unwrap();
        let offered = drain_until(&b, 5000, |m| matches!(m, ServerMessage::TradeOffered { .. }));
        let offer_id = match offered {
            Some(ServerMessage::TradeOffered { offer_id, .. }) => offer_id,
            other => panic!("bob receives alice's offer, got {other:?}"),
        };
        c.send(&ProtocolCodec::encode_client(&ClientMessage::TradeAccept { offer_id })).unwrap();
        pump(300);
        assert!(!drain(&a).iter().any(|m| matches!(m, ServerMessage::TradeResolved { .. })),
            "a third party's accept resolves nothing for the offerer");
        assert!(!drain(&b).iter().any(|m| matches!(m, ServerMessage::TradeResolved { .. })),
            "a third party's accept resolves nothing for the target");

        // The offer still completes for its true target.
        b.send(&ProtocolCodec::encode_client(&ClientMessage::TradeAccept { offer_id })).unwrap();
        assert!(drain_until(&b, 5000, |m| matches!(m,
            ServerMessage::TradeResolved { accepted: true, .. })).is_some(),
            "the true target still completes the offer");

        server.stop();
    }

    /// THE CRAFT IS THE SERVER'S TO MAKE, over real UDP (protocol v7): a
    /// workbench craft is a request — the canonical ledger pays exactly
    /// what the recipe names, and the verdict carries the produce. The
    /// ledger's movement is proven through the trade gate alone: the
    /// crafted planks are hers to offer, the consumed logs are not.
    #[test]
    fn a_granted_craft_moves_the_ledger_exactly() {
        let mut server = Server::start("127.0.0.1:0", 923).expect("start server");
        let addr = server.local_addr();
        let a = UdpSocket::bind("127.0.0.1:0").unwrap();
        let b = UdpSocket::bind("127.0.0.1:0").unwrap();
        a.set_nonblocking(true).unwrap();
        b.set_nonblocking(true).unwrap();
        a.connect(addr).unwrap();
        b.connect(addr).unwrap();
        for (sock, name) in [(&a, "smith"), (&b, "target")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(150);
        let _ = drain(&a);
        let _ = drain(&b);
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("log".into(), 2)],
        })).unwrap();
        pump(150);

        // Two batches of the real planks spec: 2 logs -> 8 planks.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::CraftRequest {
            req_id: 1, ingredients: vec![("log".into(), 1)],
            output: "planks".into(), output_count: 4, qty: 2,
        })).unwrap();
        let verdict = drain_until(&a, 5000, |m| matches!(m, ServerMessage::CraftVerdict { req_id: 1, .. }));
        match verdict {
            Some(ServerMessage::CraftVerdict { granted: true, consumed, output: Some((out, n)), reason, .. }) => {
                assert_eq!((out.as_str(), n), ("planks", 8), "the ledger produced exactly output_count x qty");
                assert_eq!(consumed, vec![("log".to_string(), 2u32)],
                    "the verdict names exactly what the ledger consumed");
                assert!(reason.is_none());
            }
            other => panic!("the covered craft is granted, got {other:?}"),
        }
        assert!(drain_until(&b, 5000, |m| matches!(m, ServerMessage::CraftVerdict { .. })).is_none(),
            "the verdict goes to the crafter alone");

        // THE LEDGER MOVED, proven through the gate alone: the crafted
        // planks are hers to offer, the consumed logs are not. (alice's
        // id is 1, bob's is 2 — join order.)
        let offer = |give: Vec<(String, u8)>| {
            a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
                to: 2, give, want: vec![],
            })).unwrap();
            pump(250);
        };
        offer(vec![("planks".into(), 8)]);
        assert!(drain(&b).iter().any(|m| matches!(m, ServerMessage::TradeOffered { .. })),
            "the crafted planks are gate-visible: the ledger counted the produce");
        offer(vec![("log".into(), 1)]);
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "the consumed logs are gone: the ledger paid the craft");

        server.stop();
    }

    /// A BLOCKED CRAFT MOVES NOTHING, over real UDP: a ledger too short
    /// for the requested batches is refused with the engine's own reason,
    /// and the re-offer of the untouched goods proves the atomicity.
    #[test]
    fn a_blocked_craft_moves_nothing_and_says_why() {
        let mut server = Server::start("127.0.0.1:0", 925).expect("start server");
        let addr = server.local_addr();
        let a = UdpSocket::bind("127.0.0.1:0").unwrap();
        let b = UdpSocket::bind("127.0.0.1:0").unwrap();
        a.set_nonblocking(true).unwrap();
        b.set_nonblocking(true).unwrap();
        a.connect(addr).unwrap();
        b.connect(addr).unwrap();
        for (sock, name) in [(&a, "smith"), (&b, "target")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(150);
        let _ = drain(&a);
        let _ = drain(&b);
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("log".into(), 1)],
        })).unwrap();
        pump(150);

        // Two batches need two logs; the ledger holds one.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::CraftRequest {
            req_id: 2, ingredients: vec![("log".into(), 1)],
            output: "planks".into(), output_count: 4, qty: 2,
        })).unwrap();
        let verdict = drain_until(&a, 5000, |m| matches!(m, ServerMessage::CraftVerdict { .. }));
        match verdict {
            Some(ServerMessage::CraftVerdict { granted: false, consumed, output: None, reason: Some(reason), .. }) => {
                assert!(consumed.is_empty(), "a refusal carries no delta");
                assert!(reason.contains("log"), "the refusal names the short ingredient: {reason}");
            }
            other => panic!("the short craft is refused with its reason, got {other:?}"),
        }

        // THE ATOMICITY PROOF: the log was never consumed — the gate
        // still admits an offer of exactly the held one.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("log".into(), 1)], want: vec![],
        })).unwrap();
        assert!(drain_until(&b, 5000, |m| matches!(m, ServerMessage::TradeOffered { .. })).is_some(),
            "the blocked craft deducted nothing from the ledger");

        server.stop();
    }

    /// THE FABRICATION GATE, over real UDP: a spec no recipe names —
    /// output demanded from nothing, or from ingredients no recipe
    /// accepts — is refused by the book, and the ledger never gains the
    /// fabricated goods (proven through the gate: a phantom diamond
    /// offer refuses).
    #[test]
    fn a_fabricated_spec_is_refused_by_the_book() {
        let mut server = Server::start("127.0.0.1:0", 927).expect("start server");
        let addr = server.local_addr();
        let a = UdpSocket::bind("127.0.0.1:0").unwrap();
        let b = UdpSocket::bind("127.0.0.1:0").unwrap();
        a.set_nonblocking(true).unwrap();
        b.set_nonblocking(true).unwrap();
        a.connect(addr).unwrap();
        b.connect(addr).unwrap();
        for (sock, name) in [(&a, "forger"), (&b, "target")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(150);
        let _ = drain(&a);
        let _ = drain(&b);
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![],
        })).unwrap();
        pump(150);

        // Diamonds from nothing; a diamond from a log; a real output with
        // a smuggled extra ingredient — none of them is a recipe.
        let attempts = [
            (vec![], "diamond", 64u8),
            (vec![("log".into(), 1)], "diamond", 1),
            (vec![("coal".into(), 1), ("stick".into(), 1), ("diamond".into(), 1)], "torch", 4),
        ];
        for (i, (ingredients, output, count)) in attempts.iter().enumerate() {
            a.send(&ProtocolCodec::encode_client(&ClientMessage::CraftRequest {
                req_id: 100 + i as u64, ingredients: ingredients.clone(),
                output: output.to_string(), output_count: *count, qty: 1,
            })).unwrap();
            let verdict = drain_until(&a, 5000, |m| matches!(m, ServerMessage::CraftVerdict { req_id, .. }
                if *req_id == 100 + i as u64));
            match verdict {
                Some(ServerMessage::CraftVerdict { granted: false, output: None, reason: Some(reason), .. }) =>
                    assert!(reason.contains("book"), "the refusal names the book: {reason}"),
                other => panic!("fabricated spec {i} is refused, got {other:?}"),
            }
        }

        // THE LEDGER NEVER GAINED: a phantom offer of the fabricated
        // diamonds refuses at the gate.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("diamond".into(), 1)], want: vec![],
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "the fabricated diamonds never entered the ledger");
        pump(300);
        assert!(!drain(&b).iter().any(|m| matches!(m, ServerMessage::TradeOffered { .. })),
            "the target never hears the phantom offer either");

        server.stop();
    }

    /// A CRAFT PAYS ONCE, over real UDP: the same req_id delivered twice
    /// (a duplicated datagram, a re-sent packet) executes the craft once —
    /// the replay is answered with a no-op refusal — and the gate proves
    /// exactly one batch's worth exists.
    #[test]
    fn a_replayed_craft_request_pays_once() {
        let mut server = Server::start("127.0.0.1:0", 929).expect("start server");
        let addr = server.local_addr();
        let a = UdpSocket::bind("127.0.0.1:0").unwrap();
        let b = UdpSocket::bind("127.0.0.1:0").unwrap();
        a.set_nonblocking(true).unwrap();
        b.set_nonblocking(true).unwrap();
        a.connect(addr).unwrap();
        b.connect(addr).unwrap();
        for (sock, name) in [(&a, "smith"), (&b, "target")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(150);
        let _ = drain(&a);
        let _ = drain(&b);
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("log".into(), 4)],
        })).unwrap();
        pump(150);

        let request = ClientMessage::CraftRequest {
            req_id: 7, ingredients: vec![("log".into(), 1)],
            output: "planks".into(), output_count: 4, qty: 2,
        };
        let bytes = ProtocolCodec::encode_client(&request);
        a.send(&bytes).unwrap();
        let first = drain_until(&a, 5000, |m| matches!(m,
            ServerMessage::CraftVerdict { req_id: 7, granted: true, .. }));
        assert!(first.is_some(), "the first delivery is granted");
        // the duplicate delivery: answered, but it must not craft again
        a.send(&bytes).unwrap();
        let replay = drain_until(&a, 5000, |m| matches!(m,
            ServerMessage::CraftVerdict { req_id: 7, granted: false, .. }));
        assert!(replay.is_some(), "the replay is answered");
        pump(300);

        // Exactly one craft's produce exists: eight planks pass the gate,
        // nine do not — and only two of the four logs were consumed.
        let offer = |give: Vec<(String, u8)>| {
            a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
                to: 2, give, want: vec![],
            })).unwrap();
            pump(250);
        };
        offer(vec![("planks".into(), 8)]);
        assert!(drain(&b).iter().any(|m| matches!(m, ServerMessage::TradeOffered { .. })),
            "one craft's produce is in the ledger");
        offer(vec![("planks".into(), 9)]);
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "the replay never crafted a second time");
        offer(vec![("log".into(), 3)]);
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "exactly two logs were consumed by the one craft");

        server.stop();
    }

    // ===== THE PLACE-PAYMENT LAWS (protocol v8) =====

    /// Join helpers shared by the place-payment laws: a and b join as
    /// survival players (a first, so a's ledger id is 1).
    fn join_two(
        server: &mut Server,
        a_name: &str,
        b_name: &str,
    ) -> (UdpSocket, UdpSocket) {
        let addr = server.local_addr();
        let a = UdpSocket::bind("127.0.0.1:0").unwrap();
        let b = UdpSocket::bind("127.0.0.1:0").unwrap();
        a.set_nonblocking(true).unwrap();
        b.set_nonblocking(true).unwrap();
        a.connect(addr).unwrap();
        b.connect(addr).unwrap();
        for (sock, name) in [(&a, a_name), (&b, b_name)] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION, creative: false,
            })).unwrap();
        }
        pump(150);
        let _ = drain(&a);
        let _ = drain(&b);
        (a, b)
    }

    fn place(sock: &UdpSocket, x: i32, y: i32, z: i32, block: u32, item: Option<&str>) {
        sock.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
            x, y, z, block,
            mine: None,
            place: item.map(|i| lf_protocol::PlaceClaim { item: i.to_string() }),
        })).unwrap();
    }

    /// THE PAID-PLACEMENT MOVES-THE-LEDGER LAW: a survival placement
    /// claiming an item the ledger holds is accepted — the peer sees the
    /// edit land, the editor hears no echo (no-self-echo) — and the
    /// ledger PAID: two placed stones are two consumed stones (offers of
    /// seven refuse, six pass, through the trade gate alone).
    #[test]
    fn a_paid_placement_moves_the_ledger() {
        use lf_voxel::registry::block;
        let mut server = Server::start("127.0.0.1:0", 931).expect("start server");
        let (a, b) = join_two(&mut server, "smith", "target");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("stone".into(), 8)],
        })).unwrap();
        pump(150);

        place(&a, 6, 200, 6, block::STONE, Some("stone"));
        place(&a, 7, 200, 7, block::STONE, Some("stone"));
        let landed = drain_until(&b, 5000, |m| matches!(m,
            ServerMessage::BlockUpdate { x: 7, y: 200, z: 7, block: b2 } if *b2 == block::STONE));
        assert!(landed.is_some(), "the peer sees the second placement land");
        pump(250);
        assert!(!drain(&a).iter().any(|m| matches!(m, ServerMessage::Reject { .. })),
            "a paid placement is never refused");
        assert!(!drain(&a).iter().any(|m| matches!(m, ServerMessage::BlockUpdate { .. })),
            "the editor never hears its own accepted placement");

        // THE LEDGER PAID BOTH: eight uploaded, two consumed by the two
        // placements — six pass the trade gate, seven refuse.
        let offer = |give: Vec<(String, u8)>| {
            a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
                to: 2, give, want: vec![],
            })).unwrap();
            pump(250);
        };
        offer(vec![("stone".into(), 7)]);
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "seven stones were never held: two placements consumed two");
        offer(vec![("stone".into(), 6)]);
        assert!(drain(&b).iter().any(|m| matches!(m, ServerMessage::TradeOffered { .. })),
            "exactly six stones remain: the placements paid the ledger");

        server.stop();
    }

    /// THE SHORT-LEDGER REFUSAL LAW: a placement the ledger cannot pay
    /// moves nothing — the world edit never lands (the peer never sees
    /// it), and the editor alone hears the corrective echo (the server's
    /// true block: still air) plus a reasoned Reject.
    #[test]
    fn an_unpaid_placement_refuses_and_lands_nothing() {
        use lf_voxel::registry::block;
        let mut server = Server::start("127.0.0.1:0", 932).expect("start server");
        let (a, b) = join_two(&mut server, "smith", "target");

        place(&a, 8, 200, 8, block::STONE, Some("stone"));
        let echo = drain_until(&a, 5000, |m| matches!(m, ServerMessage::BlockUpdate { x: 8, y: 200, z: 8, block: 0 }));
        assert!(echo.is_some(), "the corrective echo carries the server's true (air) block");
        let reject = drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { reason } if reason.contains("ledger")));
        assert!(reject.is_some(), "the refusal names the ledger");
        pump(300);
        assert!(!drain(&b).iter().any(|m| matches!(m, ServerMessage::BlockUpdate { x: 8, .. })),
            "THE GATE: an unpaid placement never lands for anyone");

        server.stop();
    }

    /// THE MISMATCHED-CLAIM LAW: paying dirt for a stone refuses by the
    /// shared placement law — the ledger is untouched (five uploaded
    /// dirts all pass the gate afterward), the block never lands.
    #[test]
    fn a_mismatched_place_claim_refuses() {
        use lf_voxel::registry::block;
        let mut server = Server::start("127.0.0.1:0", 933).expect("start server");
        let (a, b) = join_two(&mut server, "smith", "target");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("dirt".into(), 5)],
        })).unwrap();
        pump(150);

        place(&a, 9, 200, 9, block::STONE, Some("dirt"));
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { reason } if reason.contains("cannot place"))).is_some(),
            "dirt cannot place a stone: the shared law refuses");
        pump(300);
        assert!(!drain(&b).iter().any(|m| matches!(m, ServerMessage::BlockUpdate { x: 9, .. })),
            "the mismatched placement never landed");

        // THE LEDGER IS UNTOUCHED by a refused placement.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("dirt".into(), 5)], want: vec![],
        })).unwrap();
        pump(250);
        assert!(drain(&b).iter().any(|m| matches!(m, ServerMessage::TradeOffered { .. })),
            "a refused placement consumed nothing");

        server.stop();
    }

    /// THE SMUGGLE GUARD: one edit carries one claim — a mine claim and
    /// a place claim on the same datagram is a forged op. It refuses
    /// (echo + reason), and the smuggled mine never pays a grant.
    #[test]
    fn the_smuggled_double_claim_refuses() {
        use lf_voxel::registry::block;
        let mut server = Server::start("127.0.0.1:0", 934).expect("start server");
        let (a, _b) = join_two(&mut server, "smith", "target");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("stone".into(), 8)],
        })).unwrap();
        pump(150);

        a.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
            x: 12, y: 200, z: 12, block: block::AIR,
            mine: Some(lf_protocol::MineClaim { held: Some("stone_pickaxe".into()) }),
            place: Some(lf_protocol::PlaceClaim { item: "stone".into() }),
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { reason } if reason.contains("both mine and place"))).is_some(),
            "the forged op is named");
        pump(300);
        assert!(!drain(&a).iter().any(|m| matches!(m, ServerMessage::ItemGrant { .. })),
            "the smuggled mine never pays");
        assert!(!drain(&a).iter().any(|m| matches!(m, ServerMessage::Reject { reason } if reason.contains("ledger"))),
            "the gate never charged the placement either");

        server.stop();
    }

    /// THE CREATIVE LAW: a creative joiner places without a claim and
    /// without a ledger — nothing is gated, nothing consumed — and the
    /// placed blocks are real (mining them back pays the canonical
    /// grants). Two placements, two landings, two grants.
    #[test]
    fn creative_places_ungated() {
        use lf_voxel::registry::block;
        let mut server = Server::start("127.0.0.1:0", 935).expect("start server");
        let addr = server.local_addr();
        // b joins first so the creative joiner's grants are observable
        // through a stable second socket; ids: b = 1, c = 2.
        let b = UdpSocket::bind("127.0.0.1:0").unwrap();
        let c = UdpSocket::bind("127.0.0.1:0").unwrap();
        b.set_nonblocking(true).unwrap();
        c.set_nonblocking(true).unwrap();
        b.connect(addr).unwrap();
        c.connect(addr).unwrap();
        b.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
            name: "target".into(), protocol_version: PROTOCOL_VERSION, creative: false,
        })).unwrap();
        c.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
            name: "creator".into(), protocol_version: PROTOCOL_VERSION, creative: true,
        })).unwrap();
        pump(150);
        let _ = drain(&b);
        let _ = drain(&c);

        place(&c, 13, 200, 13, block::DIRT, None);
        place(&c, 14, 200, 14, block::DIRT, None);
        assert!(drain_until(&b, 5000, |m| matches!(m,
            ServerMessage::BlockUpdate { x: 14, y: 200, z: 14, block: bl } if *bl == block::DIRT)).is_some(),
            "the creative placement lands ungated");
        pump(300);
        assert!(!drain(&c).iter().any(|m| matches!(m, ServerMessage::Reject { .. })),
            "no ledger, no claim, no refusal: creative is infinite by law");

        // The blocks are REAL: mining them back pays the canonical drops.
        for (x, z) in [(13, 13), (14, 14)] {
            c.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
                x, y: 200, z, block: block::AIR,
                mine: Some(lf_protocol::MineClaim { held: None }), place: None,
            })).unwrap();
            assert!(drain_until(&c, 5000, |m| matches!(m, ServerMessage::ItemGrant { items }
                if items.first().map(|(id, _)| id.as_str()) == Some("dirt"))).is_some(),
                "the creative-placed block was really there (the dig pays)");
        }

        server.stop();
    }

    /// THE SIM-TIER LAW (pinned consciously): a claim-free edit from a
    /// survival player is a simulation edit — fluids, falling blocks,
    /// machines, spell effects — and lands ungated, exactly as shipped.
    /// Its forged-claim residual is that tier's trust, deferred with it.
    #[test]
    fn the_simulation_edit_lands_without_a_claim() {
        use lf_voxel::registry::block;
        let mut server = Server::start("127.0.0.1:0", 936).expect("start server");
        let (a, b) = join_two(&mut server, "smith", "target");

        place(&a, 15, 200, 15, block::DIRT, None);
        assert!(drain_until(&b, 5000, |m| matches!(m,
            ServerMessage::BlockUpdate { x: 15, y: 200, z: 15, block: bl } if *bl == block::DIRT)).is_some(),
            "the simulation edit lands ungated");
        pump(300);
        assert!(!drain(&a).iter().any(|m| matches!(m, ServerMessage::Reject { .. })),
            "the client-simmed tier is never refused (as shipped)");

        server.stop();
    }

    /// THE WIDE-STATE LAW (v8): the wire carries the FULL BlockState — a
    /// slab-bottom stone placement arrives a slab to the peer and to the
    /// server's canonical world (it used to arrive a full cube, so peers
    /// and the editor's own chunk reload disagreed with the placement).
    #[test]
    fn the_placed_shape_rides_the_wire() {
        use lf_voxel::registry::block;
        let mut server = Server::start("127.0.0.1:0", 937).expect("start server");
        let (a, b) = join_two(&mut server, "smith", "target");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("stone".into(), 8)],
        })).unwrap();
        pump(150);

        let slab_stone = block::STONE | (1 << 28); // Shape::SlabBottom nibble
        place(&a, 16, 200, 16, slab_stone, Some("stone"));
        assert!(drain_until(&b, 5000, |m| matches!(m,
            ServerMessage::BlockUpdate { x: 16, y: 200, z: 16, block: bl } if *bl == slab_stone)).is_some(),
            "the peer receives the slab state, not a full cube");
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_none(),
            "the shape-bearing placement pays like any other");

        server.stop();
    }

    // ---- THE FURNACE ACCOUNT (protocol v9): the pure law ----

    use lf_protocol::{SmeltOp, SmeltSlot};

    fn deposit(pos: (i32, i32, i32), slot: SmeltSlot, item: &str, count: u32) -> SmeltOp {
        SmeltOp::Deposit { pos, slot, item: item.into(), count }
    }

    fn withdraw(pos: (i32, i32, i32), slot: SmeltSlot, item: &str, count: u32) -> SmeltOp {
        SmeltOp::Withdraw { pos, slot, item: item.into(), count }
    }

    /// THE FURNACE-OP LAW (unit): every payout was funded, every refusal
    /// moves nothing, and the burn reconciliation is deterministic. Three
    /// doors: a deposit funds commitments, a withdrawal pays out of them
    /// (a fuel item only while its seconds remain banked), and a
    /// smelt-done transforms exactly one committed input plus SMELT_TIME
    /// of burn into one backed bar of the realm's smelt law.
    #[test]
    fn the_furnace_account_law_gates_every_door() {
        let pos = (3, 64, -7);
        let mut acc = FurnaceAccount::default();

        // The empty fire yields nothing: no input, no burn.
        assert_eq!(
            smelt_op(&mut acc, &SmeltOp::SmeltDone { pos, input: "raw_iron".into() }),
            Err("the furnace holds no raw_iron".into()),
            "an unbacked smelt refuses by name",
        );
        // What the fire cannot smelt, it does not yield.
        assert!(smelt_op(&mut acc, &SmeltOp::SmeltDone { pos, input: "dirt".into() }).is_err(),
            "the realm's smelt law gates the input");
        // A funded deposit moves the ledger's take; the junk fuel banks
        // no seconds.
        let mv = smelt_op(&mut acc, &deposit(pos, SmeltSlot::Input, "raw_iron", 3)).unwrap();
        assert_eq!(mv.take, vec![("raw_iron".into(), 3)]);
        assert_eq!(mv.give, vec![]);
        let mv = smelt_op(&mut acc, &deposit(pos, SmeltSlot::Fuel, "coal", 2)).unwrap();
        assert_eq!(mv.take, vec![("coal".into(), 2)]);
        assert!((acc.burn - 160.0).abs() < 1e-4, "two coals bank 160s, got {}", acc.burn);
        smelt_op(&mut acc, &deposit(pos, SmeltSlot::Fuel, "dirt", 4)).unwrap();
        assert!((acc.burn - 160.0).abs() < 1e-4, "junk banks nothing");

        // Each completed smelt consumes one input and SMELT_TIME of burn
        // and backs exactly one bar of the law's own output.
        for _ in 0..3 {
            smelt_op(&mut acc, &SmeltOp::SmeltDone { pos, input: "raw_iron".into() }).unwrap();
        }
        assert_eq!(acc.inputs.get("raw_iron"), None, "all three inputs spent");
        assert!((acc.burn - 130.0).abs() < 1e-4, "30s spent smelting, got {}", acc.burn);
        // THE BURN RECONCILIATION: one coal was spent by the smelts (its
        // whole seconds left the bank), so only one withdrawable coal
        // remains — the deterministic kind order decides which kind a
        // mixed fire spends first.
        assert_eq!(acc.fuels.get("coal"), Some(&1), "one coal burned away, one remains");
        assert_eq!(acc.fuels.get("dirt"), Some(&4), "junk is never burned");
        assert_eq!(acc.outputs.get("iron_ingot"), Some(&3), "three bars backed");

        // The backed bars are withdrawable; a fourth is not.
        let mv = smelt_op(&mut acc, &withdraw(pos, SmeltSlot::Output, "iron_ingot", 3)).unwrap();
        assert_eq!(mv.give, vec![("iron_ingot".into(), 3)]);
        assert!(smelt_op(&mut acc, &withdraw(pos, SmeltSlot::Output, "iron_ingot", 1)).is_err(),
            "an unyielded bar refuses");
        // The unburned coal is withdrawable only while its seconds
        // remain banked.
        let mv = smelt_op(&mut acc, &withdraw(pos, SmeltSlot::Fuel, "coal", 1)).unwrap();
        assert_eq!(mv.give, vec![("coal".into(), 1)]);
        assert!((acc.burn - 50.0).abs() < 1e-4, "the coal's 80s left with it, got {}", acc.burn);
        assert!(smelt_op(&mut acc, &withdraw(pos, SmeltSlot::Fuel, "coal", 1)).is_err(),
            "a burned coal cannot be taken back out");
        // The junk withdraws freely (it never burned).
        smelt_op(&mut acc, &withdraw(pos, SmeltSlot::Fuel, "dirt", 4)).unwrap();
        // Zero-count ops refuse before anything moves.
        assert!(smelt_op(&mut acc, &deposit(pos, SmeltSlot::Input, "raw_iron", 0)).is_err());
        assert!(smelt_op(&mut acc, &withdraw(pos, SmeltSlot::Input, "raw_iron", 0)).is_err());
    }

    /// THE RECONCILIATION IS DETERMINISTIC (unit): a mixed fire spends
    /// positive-burn kinds by item id, so replaying the same ops anywhere
    /// leaves the same commitments — and the spend never dips below the
    /// banked seconds.
    #[test]
    fn the_burn_reconciliation_spends_kinds_in_a_deterministic_order() {
        let pos = (0, 0, 0);
        let mut acc = FurnaceAccount::default();
        // 1 log (15s) + 1 stick (5s) + 1 coal (80s): the id-sorted spend
        // order is coal, log, stick.
        smelt_op(&mut acc, &deposit(pos, SmeltSlot::Fuel, "stick", 1)).unwrap();
        smelt_op(&mut acc, &deposit(pos, SmeltSlot::Fuel, "coal", 1)).unwrap();
        smelt_op(&mut acc, &deposit(pos, SmeltSlot::Fuel, "log", 1)).unwrap();
        smelt_op(&mut acc, &deposit(pos, SmeltSlot::Input, "sand", 1)).unwrap();
        // One glass: 10s spent; bank 90s; the banked items hold 100s, so
        // the coal (id-first) leaves the commitments.
        smelt_op(&mut acc, &SmeltOp::SmeltDone { pos, input: "sand".into() }).unwrap();
        assert_eq!(acc.fuels.get("coal"), None, "the id-first kind burned away");
        assert_eq!(acc.fuels.get("log"), Some(&1));
        assert_eq!(acc.fuels.get("stick"), Some(&1));
        assert!((acc.burn - 90.0).abs() < 1e-4);
        assert_eq!(acc.outputs.get("glass"), Some(&1));
    }

    // ---- THE BITE LEDGER (protocol v10): the unit law ----

    /// THE EAT-OP LAW (unit): a bite is paid food — the realm's catalog
    /// decides what food is, a bite is at least one, and the ledger must
    /// hold the count. A legal bite consumes exactly the count; every
    /// refusal (unknown item, non-food, zero count, short ledger) moves
    /// nothing.
    #[test]
    fn the_eat_op_feeds_only_what_the_ledger_pays_for() {
        let mut inv = Inventory::new();
        assert_eq!(inv.add_item("apple", 10), 0);
        assert_eq!(inv.add_item("log", 3), 0);

        // Food the ledger holds: the bite pays exactly.
        eat_op(&mut inv, "apple", 1).unwrap();
        eat_op(&mut inv, "apple", 4).unwrap();
        assert_eq!(inv.count_of("apple"), 5, "five apples bitten, five remain");

        // What the realm does not call food refuses by name and moves
        // nothing (a log is an ingredient, not a meal; so is a tool).
        assert_eq!(
            eat_op(&mut inv, "log", 1).unwrap_err(),
            "the realm does not call log food",
        );
        assert_eq!(eat_op(&mut inv, "stone_pickaxe", 1).unwrap_err(),
            "the realm does not call stone_pickaxe food");
        assert_eq!(inv.count_of("log"), 3, "the refusal never bit the log");

        // An item the realm does not know at all refuses by name.
        assert_eq!(
            eat_op(&mut inv, "ghost_stew", 1).unwrap_err(),
            "the realm knows no ghost_stew",
        );

        // A bite is at least one; a bite the ledger cannot pay refuses
        // and moves nothing.
        assert_eq!(eat_op(&mut inv, "apple", 0).unwrap_err(), "nothing to eat");
        assert_eq!(
            eat_op(&mut inv, "apple", 6).unwrap_err(),
            "the ledger holds no 6xapple",
        );
        assert_eq!(inv.count_of("apple"), 5, "the unpaid bite moved nothing");
    }

    // ---- THE FURNACE LEDGER (protocol v9): the wire laws ----

    fn smelt(sock: &UdpSocket, req_id: u64, op: SmeltOp) {
        sock.send(&ProtocolCodec::encode_client(&ClientMessage::SmeltRequest { req_id, op })).unwrap();
    }

    /// THE FUNDED-DEPOSIT LAW: a furnace deposit consumes the player's
    /// canonical LEDGER and lands in that furnace's commitments — the
    /// verdict answers the depositor ALONE (the peer hears nothing about
    /// another player's fire), and the ledger really paid: an offer of
    /// more than remains refuses through the trade gate alone.
    #[test]
    fn a_funded_deposit_pays_the_ledger_and_answers_the_depositor_alone() {
        let mut server = Server::start("127.0.0.1:0", 941).expect("start server");
        let (a, b) = join_two(&mut server, "smith", "target");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("raw_iron".into(), 3)],
        })).unwrap();
        pump(150);

        smelt(&a, 1, deposit((9, 70, 9), SmeltSlot::Input, "raw_iron", 3));
        let verdict = drain_until(&a, 5000, |m| matches!(m, ServerMessage::Smelt(_)));
        assert!(matches!(verdict,
            Some(ServerMessage::Smelt(lf_protocol::SmeltVerdict { req_id: 1, granted: true, .. })),
        ), "the funded deposit is granted: {:?}", verdict);
        pump(300);
        assert!(!drain(&b).iter().any(|m| matches!(m, ServerMessage::Smelt(_))),
            "the peer never hears another player's furnace");

        // The ledger paid: 0 raw_iron remain, so even 1 refuses the offer
        // gate; committing a fourth was impossible in the first place.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("raw_iron".into(), 1)], want: vec![],
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "the deposit consumed the ledger's raw_iron");

        // A deposit the ledger cannot pay refuses, moves nothing, and
        // still answers the depositor alone.
        smelt(&a, 2, deposit((9, 70, 9), SmeltSlot::Input, "raw_iron", 1));
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Smelt(_))),
            Some(ServerMessage::Smelt(lf_protocol::SmeltVerdict { granted: false, reason: Some(_), .. })),
        ), "the unpaid deposit refuses with a reason");
        server.stop();
    }

    /// THE BACKED-BAR LAW: a bar only the fire yielded is the ledger's to
    /// give — the full loop deposits ore and fuel, smelts three times,
    /// and the ledger ends having consumed 3 ore + fuel and produced
    /// exactly 3 bars; a fourth withdrawal refuses. A smelt with no
    /// committed input or no banked burn yields NOTHING.
    #[test]
    fn the_fire_yields_only_what_the_ledger_funded() {
        let mut server = Server::start("127.0.0.1:0", 942).expect("start server");
        let (a, _b) = join_two(&mut server, "smith", "target");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("raw_iron".into(), 5), ("coal".into(), 2)],
        })).unwrap();
        pump(150);
        let pos = (4, 70, 4);

        // No commitments yet: the smelt yields nothing, by name.
        smelt(&a, 1, SmeltOp::SmeltDone { pos, input: "raw_iron".into() });
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Smelt(_))),
            Some(ServerMessage::Smelt(lf_protocol::SmeltVerdict { granted: false, .. })),
        ), "an unfunded fire yields nothing");

        // Commit 3 ore + 1 coal, then smelt three times.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::SmeltRequest {
            req_id: 2, op: deposit(pos, SmeltSlot::Input, "raw_iron", 3),
        })).unwrap();
        a.send(&ProtocolCodec::encode_client(&ClientMessage::SmeltRequest {
            req_id: 3, op: deposit(pos, SmeltSlot::Fuel, "coal", 1),
        })).unwrap();
        pump(150);
        let _ = drain(&a);
        for i in 4..=6 {
            smelt(&a, i, SmeltOp::SmeltDone { pos, input: "raw_iron".into() });
        }
        pump(300);
        let granted_n = drain(&a).iter().filter(|m| matches!(m,
            ServerMessage::Smelt(lf_protocol::SmeltVerdict { granted: true, .. }))).count();
        assert_eq!(granted_n, 3, "three smelts, three grants");

        // The three bars are backed: withdraw them all.
        smelt(&a, 7, withdraw(pos, SmeltSlot::Output, "iron_ingot", 3));
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Smelt(_))),
            Some(ServerMessage::Smelt(lf_protocol::SmeltVerdict { req_id: 7, granted: true, .. })),
        ), "the backed bars are withdrawable");
        // A fourth bar was never yielded: it refuses.
        smelt(&a, 8, withdraw(pos, SmeltSlot::Output, "iron_ingot", 1));
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Smelt(_))),
            Some(ServerMessage::Smelt(lf_protocol::SmeltVerdict { granted: false, .. })),
        ), "an unyielded bar refuses");

        // THE LEDGER IS EXACT: the pack claimed 5 ore + 2 coal; the
        // ledger consumed the committed 3 ore + 1 coal and received the
        // 3 bars — an offer of 2 ore + 1 coal + 3 bars passes, one bar
        // more refuses (through the trade gate alone).
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2,
            give: vec![("raw_iron".into(), 2), ("coal".into(), 1), ("iron_ingot".into(), 3)],
            want: vec![],
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_none(),
            "the ledger holds exactly the loop's net goods");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2,
            give: vec![("iron_ingot".into(), 4)],
            want: vec![],
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "a fourth bar was never minted");
        server.stop();
    }

    /// THE PAY-ONCE LAW (smelting): a replayed req_id — a duplicated
    /// datagram — is answered with a no-op refusal; the ledger moved for
    /// the first answer only (the offer gate proves the second deposit
    /// never charged).
    #[test]
    fn a_replayed_smelt_request_pays_once() {
        let mut server = Server::start("127.0.0.1:0", 943).expect("start server");
        let (a, _b) = join_two(&mut server, "smith", "target");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("coal".into(), 8)],
        })).unwrap();
        pump(150);
        let pos = (2, 70, 2);

        smelt(&a, 11, deposit(pos, SmeltSlot::Fuel, "coal", 2));
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Smelt(_))),
            Some(ServerMessage::Smelt(lf_protocol::SmeltVerdict { req_id: 11, granted: true, .. })),
        ));
        smelt(&a, 11, deposit(pos, SmeltSlot::Fuel, "coal", 2));
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Smelt(_))),
            Some(ServerMessage::Smelt(lf_protocol::SmeltVerdict {
                req_id: 11, granted: false, reason: Some(reason),
            })) if reason == "already answered",
        ), "the replay is a named no-op");
        // The ledger paid for ONE deposit: 6 coals remain.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("coal".into(), 7)], want: vec![],
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "the replay never charged twice");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("coal".into(), 6)], want: vec![],
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_none(),
            "the honest remainder passes");
        server.stop();
    }

    /// THE TWO-FIRES LAW: each furnace's commitments are its own — fuel
    /// committed to one fire never funds another's smelt.
    #[test]
    fn two_fires_never_pool_each_others_fuel() {
        let mut server = Server::start("127.0.0.1:0", 944).expect("start server");
        let (a, _b) = join_two(&mut server, "smith", "target");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("raw_iron".into(), 2), ("coal".into(), 4)],
        })).unwrap();
        pump(150);
        let fire_a = (10, 70, 10);
        let fire_b = (20, 70, 20);

        smelt(&a, 1, deposit(fire_a, SmeltSlot::Input, "raw_iron", 1));
        smelt(&a, 2, deposit(fire_a, SmeltSlot::Fuel, "coal", 4));
        pump(200);
        let _ = drain(&a);
        // Fire B holds nothing: its smelt refuses however rich fire A is.
        smelt(&a, 3, SmeltOp::SmeltDone { pos: fire_b, input: "raw_iron".into() });
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Smelt(_))),
            Some(ServerMessage::Smelt(lf_protocol::SmeltVerdict { granted: false, .. })),
        ), "fire B is not funded by fire A");
        // Fire A smelts its own.
        smelt(&a, 4, SmeltOp::SmeltDone { pos: fire_a, input: "raw_iron".into() });
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Smelt(_))),
            Some(ServerMessage::Smelt(lf_protocol::SmeltVerdict { req_id: 4, granted: true, .. })),
        ), "fire A yields from its own commitments");
        server.stop();
    }

    /// THE BURN-OUT LAW: a leaver's furnace commitments drop with their
    /// ledger — a returning adventurer commits anew.
    #[test]
    fn goodbye_burns_out_the_furnace_commitments() {
        let mut server = Server::start("127.0.0.1:0", 945).expect("start server");
        let (a, _b) = join_two(&mut server, "smith", "target");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("raw_iron".into(), 2), ("coal".into(), 2)],
        })).unwrap();
        pump(150);
        let pos = (5, 70, 5);

        smelt(&a, 1, deposit(pos, SmeltSlot::Input, "raw_iron", 2));
        smelt(&a, 2, deposit(pos, SmeltSlot::Fuel, "coal", 2));
        pump(200);
        let _ = drain(&a);
        a.send(&ProtocolCodec::encode_client(&ClientMessage::Goodbye)).unwrap();
        pump(200);
        a.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
            name: "smith".into(), protocol_version: PROTOCOL_VERSION, creative: false,
        })).unwrap();
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("raw_iron".into(), 2), ("coal".into(), 2)],
        })).unwrap();
        pump(200);
        let _ = drain(&a);

        smelt(&a, 3, withdraw(pos, SmeltSlot::Input, "raw_iron", 1));
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Smelt(_))),
            Some(ServerMessage::Smelt(lf_protocol::SmeltVerdict { granted: false, .. })),
        ), "the old fire's commitments burned out at Goodbye");
        server.stop();
    }

    // ---- THE BITE LEDGER (protocol v10): the wire laws ----

    fn eat(sock: &UdpSocket, req_id: u64, item: &str, count: u32) {
        sock.send(&ProtocolCodec::encode_client(&ClientMessage::EatRequest {
            req_id, item: item.into(), count,
        })).unwrap();
    }

    /// THE FUNDED-BITE LAW: a survival bite consumes the player's
    /// canonical LEDGER and answers the eater ALONE (the peer hears
    /// nothing about another player's appetite), and the ledger really
    /// paid: the trade gate proves what remains.
    #[test]
    fn a_funded_bite_pays_the_ledger_and_answers_the_eater_alone() {
        let mut server = Server::start("127.0.0.1:0", 946).expect("start server");
        let (a, b) = join_two(&mut server, "smith", "target");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("apple".into(), 2)],
        })).unwrap();
        pump(150);

        eat(&a, 1, "apple", 1);
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Eat(_))),
            Some(ServerMessage::Eat(lf_protocol::EatVerdict { req_id: 1, granted: true, .. })),
        ), "the funded bite is granted");
        pump(300);
        assert!(!drain(&b).iter().any(|m| matches!(m, ServerMessage::Eat(_))),
            "the peer never hears another player's bite");

        // The ledger paid one: only one apple remains, so an offer of
        // two refuses through the trade gate alone.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("apple".into(), 2)], want: vec![],
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "the bitten apple left the ledger");

        // The second bite is granted; a third (an unpaid bite) refuses
        // with a reason and moves nothing.
        eat(&a, 2, "apple", 1);
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Eat(_))),
            Some(ServerMessage::Eat(lf_protocol::EatVerdict { req_id: 2, granted: true, .. })),
        ));
        eat(&a, 3, "apple", 1);
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Eat(_))),
            Some(ServerMessage::Eat(lf_protocol::EatVerdict {
                granted: false, reason: Some(reason), ..
            })) if reason.contains("the ledger holds no"),
        ), "the unpaid bite refuses by name");
        // The refused bite moved nothing: the pack was already empty, so
        // an offer of one apple refuses exactly as it would have before
        // the refusal — a refusal is a no-op, never a grant, never a
        // charge.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("apple".into(), 1)], want: vec![],
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "an unpaid bite never conjures an apple");
        server.stop();
    }

    /// THE FOOD-GATE LAW: the realm's own catalog decides what a bite is
    /// — an EatRequest for anything else refuses by name and the ledger
    /// never moves (the offer gate proves the goods all remain).
    #[test]
    fn the_realm_does_not_feed_what_it_does_not_call_food() {
        let mut server = Server::start("127.0.0.1:0", 947).expect("start server");
        let (a, _b) = join_two(&mut server, "smith", "target");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("log".into(), 3), ("stone_pickaxe".into(), 1)],
        })).unwrap();
        pump(150);

        eat(&a, 1, "log", 1);
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Eat(_))),
            Some(ServerMessage::Eat(lf_protocol::EatVerdict {
                req_id: 1, granted: false, reason: Some(reason),
            })) if reason == "the realm does not call log food",
        ), "a log is an ingredient, not a meal");
        eat(&a, 2, "stone_pickaxe", 1);
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Eat(_))),
            Some(ServerMessage::Eat(lf_protocol::EatVerdict { granted: false, .. })),
        ), "a tool is not a meal either");
        eat(&a, 3, "ghost_stew", 1);
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Eat(_))),
            Some(ServerMessage::Eat(lf_protocol::EatVerdict { granted: false, .. })),
        ), "an unknown item is not a meal");
        eat(&a, 4, "apple", 0);
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Eat(_))),
            Some(ServerMessage::Eat(lf_protocol::EatVerdict { granted: false, .. })),
        ), "a bite is at least one");

        // The ledger never moved: every refused bite left the goods, so
        // an offer of all of them passes the offer gate (no Reject).
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("log".into(), 3), ("stone_pickaxe".into(), 1)], want: vec![],
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_none(),
            "a refused bite never consumed the ledger");
        server.stop();
    }

    /// THE BITE PAYS ONCE: a duplicated datagram (the same req_id twice)
    /// is answered with a no-op refusal, and the ledger moved exactly
    /// once — the craft pays-once law's eating twin.
    #[test]
    fn a_replayed_bite_pays_once() {
        let mut server = Server::start("127.0.0.1:0", 948).expect("start server");
        let (a, _b) = join_two(&mut server, "smith", "target");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::PackSync {
            items: vec![("porkchop".into(), 4)],
        })).unwrap();
        pump(150);

        eat(&a, 11, "porkchop", 1);
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Eat(_))),
            Some(ServerMessage::Eat(lf_protocol::EatVerdict { req_id: 11, granted: true, .. })),
        ));
        eat(&a, 11, "porkchop", 1);
        assert!(matches!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Eat(_))),
            Some(ServerMessage::Eat(lf_protocol::EatVerdict {
                req_id: 11, granted: false, reason: Some(reason),
            })) if reason == "already answered",
        ), "the replay is a named no-op");
        // The ledger paid for ONE bite: 3 porkchops remain.
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("porkchop".into(), 4)], want: vec![],
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_some(),
            "the replay never charged twice");
        a.send(&ProtocolCodec::encode_client(&ClientMessage::TradeOffer {
            to: 2, give: vec![("porkchop".into(), 3)], want: vec![],
        })).unwrap();
        assert!(drain_until(&a, 5000, |m| matches!(m, ServerMessage::Reject { .. })).is_none(),
            "the honest remainder passes");
        server.stop();
    }
}
