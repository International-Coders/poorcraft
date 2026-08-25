//! UDP multiplayer client: connects to a loreforge-server, sends local state,
//! and surfaces remote players / block edits / chat.

use std::collections::HashMap;
use std::net::UdpSocket;

use lf_protocol::{ClientMessage, ProtocolCodec, ServerMessage, PROTOCOL_VERSION};

pub struct NetClient {
    socket: UdpSocket,
    pub player_id: Option<u64>,
    pub remote_players: HashMap<u64, RemotePlayer>,
    pub chat_log: Vec<String>,
    pub connected: bool,
    last_send: std::time::Instant,
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

    pub fn send_block(&self, x: i32, y: i32, z: i32, block: u32) {
        let msg = ProtocolCodec::encode_client(&ClientMessage::SetBlock { x, y, z, block });
        let _ = self.socket.send(&msg);
    }

    pub fn send_chat(&self, text: &str) {
        let msg = ProtocolCodec::encode_client(&ClientMessage::Chat { text: text.to_string() });
        let _ = self.socket.send(&msg);
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
