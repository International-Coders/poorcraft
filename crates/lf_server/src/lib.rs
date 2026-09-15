//! Multiplayer server: an authoritative-lite simulation over UDP. Owns the
//! canonical block world (worldgen + client edits), relays player states,
//! block updates and chat. Run standalone via the loreforge-server binary
//! or in-process from tests.

use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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
    let mut edits: Vec<(i32, i32, i32, u32)> = Vec::new();
    let mut next_id: u64 = 1;
    let mut offers: HashMap<u64, lf_protocol::TradeOfferRecord> = HashMap::new();
    let mut next_offer_id: u64 = 1;
    let mut last_snapshot = std::time::Instant::now();
    let mut buf = [0u8; 2048];

    while !stop.load(Ordering::Relaxed) {
        let mut activity = false;
        for _ in 0..64 {
            match socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    activity = true;
                    if let Some(msg) = ProtocolCodec::decode_client(&buf[..len]) {
                        handle_message(&socket, &mut players, &mut world, &gen, &mut edits,
                            &mut next_id, &mut offers, &mut next_offer_id, src, msg);
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

fn handle_message(
    socket: &UdpSocket,
    players: &mut HashMap<u64, Player>,
    world: &mut World,
    gen: &WorldGen,
    edits: &mut Vec<(i32, i32, i32, u32)>,
    next_id: &mut u64,
    offers: &mut HashMap<u64, lf_protocol::TradeOfferRecord>,
    next_offer_id: &mut u64,
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
            players.retain(|_, p| p.addr != src);
            let id = *next_id;
            *next_id += 1;
            let roster: Vec<(u64, String)> = players.iter().map(|(pid, p)| (*pid, p.name.clone())).collect();
            players.insert(id, Player { name: name.clone(), addr: src, pos: [0.0, 80.0, 0.0], yaw: 0.0 });
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
        ClientMessage::SetBlock { x, y, z, block } => {
            // THE NO-SELF-ECHO LAW: the editor applied its edit optimistically
            // before sending it, so echoing an ACCEPTED edit back would
            // double-apply (a second host event for one player action plus a
            // redundant relight/remesh of the column). Peers receive the
            // update; the editor only hears from the server again when it
            // REJECTED the op — the corrective echo below — so the editor's
            // optimistic world can never diverge from the canonical one.
            //
            // validate: within height, and a real block (vanilla or a mod
            // block registered from a loaded mods/ dir)
            if (0..256).contains(&y) && lf_voxel::registry::is_known_block(block) {
                let (cx, _lx) = (x.div_euclid(16), x.rem_euclid(16));
                let (cz, _lz) = (z.div_euclid(16), z.rem_euclid(16));
                if world.chunk(cx, cz).is_none() {
                    world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
                }
                world.set_block(x, y, z, BlockState(block));
                edits.push((x, y, z, block));
                let upd = ProtocolCodec::encode_server(&ServerMessage::BlockUpdate { x, y, z, block });
                for p in players.values() {
                    if p.addr == src {
                        continue;
                    }
                    let _ = socket.send_to(&upd, p.addr);
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
        ClientMessage::TradeOffer { to, give, want } => {
            // P37 escrow: register the offer, notify the recipient.
            let from = match players.values().find(|p| p.addr == src) {
                Some(p) => p,
                None => return,
            };
            let from_name = from.name.clone();
            let from_id = players.iter().find(|(_, p)| p.addr == src).map(|(id, _)| *id).unwrap_or(0);
            if players.get(&to).is_none() {
                let reply = ProtocolCodec::encode_server(&ServerMessage::Reject {
                    reason: "trade target is not online".into(),
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
            // Complete the escrow: the accepter receives `give`, the
            // offerer receives `want` (both peers apply the swap —
            // authoritative-lite, same policy as blocks).
            let sender_id = players.iter().find(|(_, p)| p.addr == src).map(|(id, _)| *id).unwrap_or(0);
            let Some(offer) = offers.remove(&offer_id) else { return };
            let accepted = offer.to == sender_id;
            let to_accepter = ProtocolCodec::encode_server(&ServerMessage::TradeResolved {
                offer_id, accepted, items: if accepted { offer.give.clone() } else { vec![] },
            });
            let to_offerer = ProtocolCodec::encode_server(&ServerMessage::TradeResolved {
                offer_id, accepted, items: if accepted { offer.want.clone() } else { vec![] },
            });
            if let Some(t) = players.get(&offer.to) {
                let _ = socket.send_to(&to_accepter, t.addr);
            }
            if let Some(f) = players.get(&offer.from) {
                let _ = socket.send_to(&to_offerer, f.addr);
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
            let leaving = players.iter().find(|(_, p)| p.addr == src).map(|(i, _)| *i);
            if let Some(id) = leaving {
                players.remove(&id);
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

        c2.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock { x: 5, y: 70, z: -3, block: 1 })).unwrap();
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
        // both hellos
        for (sock, name) in [(&c1, "alice"), (&c2, "bob")] {
            sock.send(&ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: name.into(), protocol_version: PROTOCOL_VERSION,
            })).unwrap();
        }
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
        sender.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock { x: 2, y: 70, z: 2, block: probe_id })).unwrap();
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
        sender.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock { x: 3, y: 70, z: 3, block: unknown_id })).unwrap();
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
            a.send(&ProtocolCodec::encode_client(&ClientMessage::SetBlock { x, y, z, block })).unwrap();
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
}
