//! UDP multiplayer client: connects to a loreforge-server, sends local state,
//! and surfaces remote players / block edits / chat.

use std::collections::{HashMap, VecDeque};
use std::net::UdpSocket;

use lf_game::host::EditKind;
use lf_game::survival::{Inventory, ItemStack};
use lf_protocol::{ClientMessage, MineClaim, ProtocolCodec, ServerMessage, PROTOCOL_VERSION};

/// THE MINE-CLAIM LAW: only a player MINED edit claims its hand on the
/// wire, and the claim is honest — the held item id, or `None` for a bare
/// hand (a bare-handed dig of a no-tool block is a legal yield, online
/// and off). Places and every simulation edit (machine, fluid, falling,
/// console, server mirror) claim nothing, so the server can never pay a
/// yield for an edit no player dug. The server evaluates its OWN copy of
/// the harvest law against the block it actually had before granting
/// ([`ServerMessage::ItemGrant`]) — the claim is what the server gates,
/// not a demand.
pub fn mine_claim_for(reason: EditKind, held: Option<&ItemStack>) -> Option<MineClaim> {
    match reason {
        EditKind::Mine => Some(MineClaim {
            held: held.map(|s| s.item_id.clone()),
        }),
        _ => None,
    }
}

/// THE REPLAY-WINDOW LAW: remote edits for chunks that have not streamed in
/// yet buffer here instead of being lost. The server replays its whole edit
/// history to a newcomer right after Welcome, while the client's streamer is
/// still generating chunks in the background — and `World::set_block`
/// refuses edits for a missing chunk (the host records a reject and moves
/// on). Without this buffer every replayed edit outside the already-meshed
/// ring silently vanished: the second client never saw the dig it was
/// standing next to five minutes later. The streamer flushes a chunk's
/// queue — oldest first, the server's history order — the moment that chunk
/// arrives, before the column is meshed.
pub struct RemoteEditBuffer {
    per_chunk: HashMap<(i32, i32), VecDeque<(i32, i32, i32, u32)>>,
    /// Chunk keys in first-buffered order; overflow evicts oldest whole.
    /// Invariant: exactly the keys of `per_chunk`, no empty queues.
    order: VecDeque<(i32, i32)>,
    total: usize,
    cap: usize,
    /// Edits dropped wholesale when the cap forced an eviction.
    pub evicted: usize,
}

/// Total buffered edits across all chunks. A reconnect re-replays the
/// server's full history, so one connection's replay must fit; past the cap
/// the oldest chunk's queue is dropped whole (reconnect refetches it).
pub const MAX_PENDING_REMOTE_EDITS: usize = 16384;

impl RemoteEditBuffer {
    pub fn new() -> Self {
        Self::with_cap(MAX_PENDING_REMOTE_EDITS)
    }

    fn with_cap(cap: usize) -> Self {
        Self {
            per_chunk: HashMap::new(),
            order: VecDeque::new(),
            total: 0,
            cap,
            evicted: 0,
        }
    }

    /// Buffer one remote edit for its (not yet loaded) chunk.
    pub fn buffer(&mut self, x: i32, y: i32, z: i32, block: u32) {
        let key = Self::chunk_of(x, z);
        if self.total >= self.cap {
            // Evict the oldest chunk's whole queue (FIFO by first buffer).
            if let Some(oldest) = self.order.pop_front() {
                if let Some(queue) = self.per_chunk.remove(&oldest) {
                    self.total -= queue.len();
                    self.evicted += queue.len();
                }
            }
        }
        let queue = self.per_chunk.entry(key).or_insert_with(|| {
            self.order.push_back(key);
            VecDeque::new()
        });
        queue.push_back((x, y, z, block));
        self.total += 1;
    }

    /// Drain one chunk's buffered edits, oldest first. An empty answer for
    /// a chunk that was never buffered (or already flushed) is normal.
    pub fn take_chunk(&mut self, cx: i32, cz: i32) -> Vec<(i32, i32, i32, u32)> {
        let Some(queue) = self.per_chunk.remove(&(cx, cz)) else {
            return Vec::new();
        };
        self.order.retain(|&k| k != (cx, cz));
        self.total -= queue.len();
        queue.into_iter().collect()
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// World column -> chunk key (the mesher's own partition).
    pub fn chunk_of(x: i32, z: i32) -> (i32, i32) {
        (x.div_euclid(16), z.div_euclid(16))
    }
}

impl Default for RemoteEditBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Edits buffer per chunk and flush oldest-first in the server's
    /// history order; a flushed chunk stays flushed (no ghost keys).
    #[test]
    fn buffered_edits_flush_in_history_order() {
        let mut buf = RemoteEditBuffer::with_cap(64);
        // Chunk (0,0) gets two edits, chunk (1,0) one, interleaved.
        buf.buffer(5, 70, 3, 1);
        buf.buffer(20, 71, 4, 2);
        buf.buffer(6, 72, 2, 3);
        assert_eq!(buf.total(), 3);
        assert!(!buf.is_empty());

        let first = buf.take_chunk(0, 0);
        assert_eq!(first, vec![(5, 70, 3, 1), (6, 72, 2, 3)], "chunk (0,0) FIFO");
        assert_eq!(buf.total(), 1, "only chunk (1,0)'s edit remains");
        assert!(buf.take_chunk(0, 0).is_empty(), "flushed chunk stays flushed");
        assert_eq!(buf.take_chunk(9, 9), Vec::<(i32, i32, i32, u32)>::new(),
            "never-buffered chunk drains empty");

        assert_eq!(buf.take_chunk(1, 0), vec![(20, 71, 4, 2)]);
        assert!(buf.is_empty());
    }

    /// THE BOUNDED-REPLAY LAW: past the cap the oldest chunk's whole queue
    /// is evicted (never the newest), the eviction is counted, and the
    /// order invariant (order == per_chunk keys) survives.
    #[test]
    fn overflow_evicts_the_oldest_chunk_whole() {
        let mut buf = RemoteEditBuffer::with_cap(3);
        buf.buffer(-1, 70, -1, 1); // chunk (-1,-1): 1 edit
        buf.buffer(0, 70, 0, 2); // chunk (0,0): 1 edit
        buf.buffer(0, 71, 0, 3); // chunk (0,0): 2 edits (within cap)
        assert_eq!(buf.total(), 3);
        assert_eq!(buf.evicted, 0);

        buf.buffer(16, 70, 16, 4); // chunk (1,1) pushes total to 4 > cap
        assert_eq!(buf.evicted, 1, "the oldest chunk's queue evicted whole");
        assert!(buf.take_chunk(-1, -1).is_empty(), "oldest chunk lost to eviction");
        assert_eq!(buf.take_chunk(0, 0).len(), 2, "newer chunk queue survives");
        assert_eq!(buf.take_chunk(1, 1), vec![(16, 70, 16, 4)]);
        assert!(buf.is_empty());
    }

    /// Chunk keys follow the mesher's own partition (div_euclid), negative
    /// columns included — a buffered edit must flush into the chunk the
    /// streamer will actually deliver.
    #[test]
    fn chunk_keys_match_the_mesher_partition() {
        assert_eq!(RemoteEditBuffer::chunk_of(5, 3), (0, 0));
        assert_eq!(RemoteEditBuffer::chunk_of(-1, -1), (-1, -1));
        assert_eq!(RemoteEditBuffer::chunk_of(-16, 15), (-1, 0));
        assert_eq!(RemoteEditBuffer::chunk_of(16, -17), (1, -2));
    }

    fn held(id: &str) -> ItemStack {
        ItemStack { item_id: id.to_string(), count: 1 }
    }

    /// THE PACK-SYNC LAWS (protocol v6): the mirror bootstraps the join
    /// (an empty pack is still a claim — the ledger must learn the pack
    /// is EMPTY, not stay unseeded), uploads only drift, aggregates
    /// split stacks into one sorted claim, and a server-side delta
    /// forces the next upload past the cadence.
    #[test]
    fn the_mirror_bootstraps_and_only_uploads_drift() {
        let t0 = std::time::Instant::now();
        let mut mirror = PackMirror::new();
        let inv = Inventory::new();

        let first = mirror.sync_message(&inv, t0).expect("the join uploads the bootstrap claim");
        assert_eq!(first, ClientMessage::PackSync { items: vec![] },
            "an empty pack is still claimed — the ledger learns the pack is empty");
        assert!(mirror.sync_message(&inv, t0).is_none(), "no drift, no upload");

        let mut inv = Inventory::new();
        inv.add_item("wood", 1);
        assert!(mirror.sync_message(&inv, t0).is_none(),
            "drift inside the cadence waits — the claim is not spammed");
        mirror.request_sync();
        let second = mirror.sync_message(&inv, t0).expect("a forced upload ignores the cadence");
        assert_eq!(second, ClientMessage::PackSync { items: vec![("wood".into(), 1)] });
        assert!(mirror.sync_message(&inv, t0).is_none(), "the claim is remembered");
    }

    #[test]
    fn the_mirror_aggregates_split_stacks_and_sorts_the_claim() {
        let t0 = std::time::Instant::now();
        let mut mirror = PackMirror::new();
        let mut inv = Inventory::new();
        inv.add_item("wood", 32);
        inv.add_item("wood", 32); // a second stack of the same item
        inv.add_item("stone", 1);
        inv.add_item("apple", 3);
        inv.slots[36] = Some(held("iron_helmet")); // worn armor is carried too

        let claim = mirror.sync_message(&inv, t0).expect("the drift uploads");
        let ClientMessage::PackSync { items } = claim else {
            panic!("the mirror sends PackSync, got {claim:?}");
        };
        assert_eq!(
            items,
            vec![
                ("apple".into(), 3),
                ("iron_helmet".into(), 1),
                ("stone".into(), 1),
                ("wood".into(), 64),
            ],
            "split stacks aggregate, the claim is sorted, counts are u32"
        );
    }

    /// THE MINE-CLAIM LAW: a MINE claims its honest hand — tool id when
    /// one is held, `None` for a bare hand — and every non-mine edit
    /// claims nothing, so only player digs can ever pay.
    #[test]
    fn only_mine_edits_claim_and_the_claim_is_the_honest_hand() {
        use lf_game::host::EditKind;

        let pick = held("stone_pickaxe");
        let claim = mine_claim_for(EditKind::Mine, Some(&pick)).expect("a mine claims");
        assert_eq!(claim.held.as_deref(), Some("stone_pickaxe"), "the claim names the held tool");

        let bare = mine_claim_for(EditKind::Mine, None).expect("a bare hand still claims the mine");
        assert_eq!(bare.held, None, "a bare-handed claim is honest about the empty hand");

        for reason in [EditKind::Place, EditKind::Machine, EditKind::Fluid,
                       EditKind::Falling, EditKind::Console, EditKind::Server] {
            assert!(mine_claim_for(reason, Some(&pick)).is_none(),
                    "{reason:?} edits claim nothing — only digs can pay");
        }
    }
}

pub struct NetClient {
    socket: UdpSocket,
    pub player_id: Option<u64>,
    pub remote_players: HashMap<u64, RemotePlayer>,
    pub chat_log: Vec<String>,
    pub connected: bool,
    /// THE PACK MIRROR: what we last told the server we carry — the seed
    /// of the server's canonical ledger for us (protocol v6).
    pub pack_mirror: PackMirror,
    last_send: std::time::Instant,
}

/// THE PACK MIRROR (protocol v6): the client's side of the pack-sync
/// law. The server holds a canonical LEDGER of what each player carries;
/// this mirror keeps the client's last uploaded claim and detects drift.
/// Uploads happen on join (the bootstrap — the ledger starts empty) and
/// whenever the live pack's aggregated contents drift from the last
/// claim, rate-limited to [`lf_protocol::PACK_SYNC_MIN_INTERVAL`] —
/// except when a server-side delta (an ItemGrant, an accepted trade)
/// just landed, which forces the next upload so the server's ledger is
/// re-claimed WITH the delta inside one round trip (a stale upload can
/// only ever remove server-known deltas, never add phantom items — the
/// window errs safe).
pub struct PackMirror {
    last_synced: Option<Vec<(String, u32)>>,
    last_upload: Option<std::time::Instant>,
    forced: bool,
}

impl PackMirror {
    pub fn new() -> Self {
        Self { last_synced: None, last_upload: None, forced: false }
    }

    /// THE PACK SNAPSHOT LAW: the claim is the aggregated (item, count)
    /// of every carried stack, sorted — slot layout is presentation, the
    /// ledger is contents. Counts aggregate to u32 (a full pack holds
    /// far more than 255 of one item).
    pub fn snapshot(inv: &Inventory) -> Vec<(String, u32)> {
        let mut agg: HashMap<String, u32> = HashMap::new();
        for stack in inv.slots.iter().flatten() {
            *agg.entry(stack.item_id.clone()).or_insert(0) += stack.count as u32;
        }
        let mut out: Vec<(String, u32)> = agg.into_iter().collect();
        out.sort();
        out
    }

    /// Force the next drift check to upload regardless of the cadence.
    pub fn request_sync(&mut self) {
        self.forced = true;
    }

    /// Compare the live pack against the last claim; answer the message
    /// to send, if any (and record it). Call every frame while connected.
    pub fn sync_message(&mut self, inv: &Inventory, now: std::time::Instant) -> Option<ClientMessage> {
        let snap = Self::snapshot(inv);
        if self.last_synced.as_ref() == Some(&snap) {
            self.forced = false;
            return None;
        }
        if !self.forced
            && self.last_upload.is_some_and(|t| now.duration_since(t) < lf_protocol::PACK_SYNC_MIN_INTERVAL)
        {
            return None; // rate-limited; the drift persists and retries
        }
        self.last_synced = Some(snap.clone());
        self.last_upload = Some(now);
        self.forced = false;
        Some(ClientMessage::PackSync { items: snap })
    }
}

impl Default for PackMirror {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct RemotePlayer {
    pub name: String,
    pub pos: [f32; 3],
    pub yaw: f32,
}

impl NetClient {
    pub fn connect(host: &str, name: &str) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.connect(host)?;
        socket.set_nonblocking(true)?;
        let hello = ProtocolCodec::encode_client(&ClientMessage::Hello {
            name: name.to_string(),
            protocol_version: PROTOCOL_VERSION,
        });
        socket.send(&hello)?;
        Ok(Self {
            socket,
            player_id: None,
            remote_players: HashMap::new(),
            chat_log: Vec::new(),
            connected: false,
            pack_mirror: PackMirror::new(),
            last_send: std::time::Instant::now() - std::time::Duration::from_secs(1),
        })
    }

    /// Send our state at ~20/s; call every frame.
    pub fn send_state(&mut self, pos: [f32; 3], yaw: f32, pitch: f32) {
        if self.last_send.elapsed() < std::time::Duration::from_millis(50) {
            return;
        }
        self.last_send = std::time::Instant::now();
        let msg = ProtocolCodec::encode_client(&ClientMessage::Position { pos, yaw, pitch });
        let _ = self.socket.send(&msg);
    }

    pub fn send_block(&self, x: i32, y: i32, z: i32, block: u32, mine: Option<MineClaim>) {
        let msg = ProtocolCodec::encode_client(&ClientMessage::SetBlock { x, y, z, block, mine });
        let _ = self.socket.send(&msg);
    }

    pub fn send_chat(&self, text: &str) {
        let msg = ProtocolCodec::encode_client(&ClientMessage::Chat { text: text.to_string() });
        let _ = self.socket.send(&msg);
    }

    /// THE ONE PACK-SYNC SENDER: detects drift against the live pack and
    /// uploads the claim. Call every frame while a session is connected;
    /// a no-op before the server has said Welcome.
    pub fn sync_pack(&mut self, inv: &Inventory) {
        if !self.connected {
            return;
        }
        if let Some(msg) = self.pack_mirror.sync_message(inv, std::time::Instant::now()) {
            let encoded = ProtocolCodec::encode_client(&msg);
            let _ = self.socket.send(&encoded);
        }
    }

    /// Drain incoming server messages (also prunes stale remotes).
    pub fn poll(&mut self) -> Vec<ServerMessage> {
        let mut received = Vec::new();
        let mut buf = [0u8; 2048];
        loop {
            match self.socket.recv(&mut buf) {
                Ok(len) => {
                    if let Some(msg) = ProtocolCodec::decode_server(&buf[..len]) {
                        // track basics here; detailed world edits returned to caller
                        match &msg {
                            ServerMessage::Welcome { your_id, players, .. } => {
                                self.player_id = Some(*your_id);
                                self.connected = true;
                                self.chat_log.push(format!("[connected as player {}]", your_id));
                                for (id, name) in players {
                                    self.remote_players.entry(*id).or_insert_with(|| RemotePlayer {
                                        name: name.clone(),
                                        pos: [0.0, 80.0, 0.0],
                                        yaw: 0.0,
                                    });
                                }
                            }
                            ServerMessage::PlayerStates { states } => {
                                let seen: Vec<u64> = states.iter().map(|(id, _, _)| *id).collect();
                                for (id, pos, yaw) in states {
                                    if Some(*id) == self.player_id {
                                        continue;
                                    }
                                    let entry = self.remote_players.entry(*id).or_insert(RemotePlayer {
                                        name: format!("player {}", id),
                                        pos: *pos,
                                        yaw: *yaw,
                                    });
                                    entry.pos = *pos;
                                    entry.yaw = *yaw;
                                }
                                let _ = &seen;
                            }
                            ServerMessage::PlayerJoined { id, name } => {
                                self.chat_log.push(format!("[{} joined]", name));
                                self.remote_players.entry(*id).or_insert(RemotePlayer {
                                    name: name.clone(),
                                    pos: [0.0, 80.0, 0.0],
                                    yaw: 0.0,
                                });
                            }
                            ServerMessage::PlayerLeft { id } => {
                                if let Some(p) = self.remote_players.remove(id) {
                                    self.chat_log.push(format!("[{} left]", p.name));
                                }
                            }
                            ServerMessage::Chat { from, text } => {
                                self.chat_log.push(format!("{}: {}", from, text));
                                if self.chat_log.len() > 64 {
                                    self.chat_log.drain(..32);
                                }
                            }
                            _ => {}
                        }
                        received.push(msg);
                    }
                }
                Err(_) => break,
            }
        }
        received
    }
}
