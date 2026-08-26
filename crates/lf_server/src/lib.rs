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
                            &mut next_id, src, msg);
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
            // validate roughly: within height, known block id
            if (0..256).contains(&y) && block <= 18 {
                let (cx, _lx) = (x.div_euclid(16), x.rem_euclid(16));
                let (cz, _lz) = (z.div_euclid(16), z.rem_euclid(16));
                if world.chunk(cx, cz).is_none() {
                    world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
                }
                world.set_block(x, y, z, BlockState(block));
                edits.push((x, y, z, block));
                let upd = ProtocolCodec::encode_server(&ServerMessage::BlockUpdate { x, y, z, block });
                for p in players.values() {
                    let _ = socket.send_to(&upd, p.addr);
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
        pump(200);
        let c1_msgs = drain(&c1);
        assert!(c1_msgs.iter().any(|m| matches!(m, ServerMessage::BlockUpdate { x: 5, y: 70, z: -3, block: 1 })),
            "alice receives block update");

        c1.send(&ProtocolCodec::encode_client(&ClientMessage::Position { pos: [10.0, 80.0, 10.0], yaw: 0.0, pitch: 0.0 })).unwrap();
        pump(200);
        let c2_msgs = drain(&c2);
        assert!(c2_msgs.iter().any(|m| matches!(m, ServerMessage::PlayerStates { states }
            if states.iter().any(|(_, pos, _)| pos == &[10.0, 80.0, 10.0]))),
            "bob sees alice position");

        server.stop();
    }
}
