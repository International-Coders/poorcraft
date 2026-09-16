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
                            &mut craft_seen, &mut craft_seen_set, src, msg);
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
    src: SocketAddr,
    msg: ClientMessage,
) {
    match msg {
        ClientMessage::Hello { name, protocol_version } => {
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
            players.insert(id, Player { name: name.clone(), addr: src, pos: [0.0, 80.0, 0.0], yaw: 0.0 });
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
        ClientMessage::SetBlock { x, y, z, block, mine } => {
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
            if (0..256).contains(&y) && lf_voxel::registry::is_known_block(block) {
                let (cx, _lx) = (x.div_euclid(16), x.rem_euclid(16));
                let (cz, _lz) = (z.div_euclid(16), z.rem_euclid(16));
                if world.chunk(cx, cz).is_none() {
                    world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
                }
                // THE YIELD-GRANT LAW: what the block was is what the server
                // had — read BEFORE the canonical world changes.
                let old = world.get_block(x, y, z).0;
                world.set_block(x, y, z, BlockState(block));
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
                    if old != lf_voxel::registry::block::AIR {
                        let held = claim
                            .held
                            .map(|id| lf_game::survival::ItemStack { item_id: id, count: 1 });
                        if lf_game::mining::tool_satisfies(old, held.as_ref()) {
                            if let Some(item) = lf_game::items::block_drop(old) {
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
            name: "alice".into(), protocol_version: PROTOCOL_VERSION,
        })).unwrap();
        pump(150);
        c2.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
            name: "bob".into(), protocol_version: PROTOCOL_VERSION,
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

        c2.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock { x: 5, y: 70, z: -3, block: 1, mine: None })).unwrap();
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
                name: name.into(), protocol_version: PROTOCOL_VERSION,
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
                name: name.into(), protocol_version: PROTOCOL_VERSION,
            })).unwrap();
        }
        pump(150);
        let _ = drain(&sender);
        let _ = drain(&observer);

        // A mod-block edit is accepted and relayed — to the peer only.
        sender.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock { x: 2, y: 70, z: 2, block: probe_id, mine: None })).unwrap();
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
        sender.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock { x: 3, y: 70, z: 3, block: unknown_id, mine: None })).unwrap();
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
            name: "early".into(), protocol_version: PROTOCOL_VERSION,
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
            a.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock { x, y, z, block, mine: None })).unwrap();
        }
        pump(300);

        let b = UdpSocket::bind("127.0.0.1:0").unwrap();
        b.set_nonblocking(true).unwrap();
        b.connect(addr).unwrap();
        b.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
            name: "newcomer".into(), protocol_version: PROTOCOL_VERSION,
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
                name: name.into(), protocol_version: PROTOCOL_VERSION,
            })).unwrap();
        }
        pump(150);
        let _ = drain(&editor);
        let _ = drain(&peer);

        // A PLACE never grants — not to the placer, not to the peer.
        editor.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
            x: 2, y: 200, z: 2, block: block::STONE, mine: None,
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
            mine: Some(lf_protocol::MineClaim { held: Some("stone_pickaxe".into()) }),
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
            name: "gater".into(), protocol_version: PROTOCOL_VERSION,
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
                x, y: 200, z: 9, block: block::IRON_ORE, mine: None,
            })).unwrap();
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
                x, y: 200, z: 9, block: block::AIR,
                mine: Some(lf_protocol::MineClaim { held: held.map(|s| s.to_string()) }),
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
                name: name.into(), protocol_version: PROTOCOL_VERSION,
            })).unwrap();
        }
        pump(150);
        let _ = drain(&editor);
        let _ = drain(&late);

        // First miner breaks the staged block: exactly one grant. (No
        // self-echo wait — the placer never hears its own place; loopback
        // UDP keeps the staged pair in order.)
        editor.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
            x: 12, y: 200, z: 12, block: block::DIRT, mine: None,
        })).unwrap();
        let mine = ClientMessage::SetBlock {
            x: 12, y: 200, z: 12, block: block::AIR,
            mine: Some(lf_protocol::MineClaim { held: None }),
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
            mine: Some(lf_protocol::MineClaim { held: Some("stone_pickaxe".into()) }),
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
                name: name.into(), protocol_version: PROTOCOL_VERSION,
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
            x: 2, y: 200, z: 2, block: block::STONE, mine: None,
        })).unwrap();
        alice.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock {
            x: 2, y: 200, z: 2, block: block::AIR,
            mine: Some(lf_protocol::MineClaim { held: Some("stone_pickaxe".into()) }),
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
                name: name.into(), protocol_version: PROTOCOL_VERSION,
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
                name: name.into(), protocol_version: PROTOCOL_VERSION,
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
                name: name.into(), protocol_version: PROTOCOL_VERSION,
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
                name: name.into(), protocol_version: PROTOCOL_VERSION,
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
                name: name.into(), protocol_version: PROTOCOL_VERSION,
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
                name: name.into(), protocol_version: PROTOCOL_VERSION,
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
                name: name.into(), protocol_version: PROTOCOL_VERSION,
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
}
