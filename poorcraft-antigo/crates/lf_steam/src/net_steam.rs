//! king-quest Steam pass: ISteamNetworkingSockets transport for the
//! game's protocol-v4 messages, behind the `steam` feature. The host
//! opens a P2P listen socket and a Steam lobby whose `lf_host_steamid`
//! datum points joiners at the host identity; clients join the lobby,
//! read the datum, and `connect_p2p`. Bytes on the wire are exactly
//! `lf_protocol` codec frames, so the same messages ride UDP or Steam.

use steamworks::networking_sockets::{ListenSocket, NetConnection, NetPollGroup};
use steamworks::networking_types::{
    ListenSocketEvent, NetworkingConfigEntry, NetworkingConnectionState, NetworkingIdentity,
    NetworkingMessage, SendFlags,
};
use steamworks::{Client, LobbyId, LobbyType, SteamId};

use lf_protocol::{ClientMessage, ProtocolCodec, ServerMessage};

/// Lobby datum key carrying the host's raw Steam id.
pub const LOBBY_HOST_KEY: &str = "lf_host_steamid";

/// A peer's stable key: the raw Steam id.
pub type PeerId = u64;

/// Inbound event/messages from the transport, already decoded to
/// protocol-v4 where applicable.
pub enum HostEvent {
    PeerConnected(PeerId),
    PeerMessage(PeerId, ClientMessage),
    PeerDisconnected(PeerId),
}

pub struct SteamHost {
    client: Client,
    lobby: LobbyId,
    listen: ListenSocket,
    poll: NetPollGroup,
    /// accepted connections by peer steam id
    peers: Vec<(PeerId, NetConnection)>,
    pub host_id: u64,
}

impl SteamHost {
    /// Init, open the P2P listen socket, create a public lobby and stamp
    /// it with this host's Steam id so joiners can find the identity.
    pub fn bind(name: &str) -> Result<Self, String> {
        let client = Client::init().map_err(|e| format!("steam init: {e}"))?;
        let sockets = client.networking_sockets();
        let listen = sockets
            .create_listen_socket_p2p(0, Vec::<NetworkingConfigEntry>::new())
            .map_err(|e| format!("listen socket: {e:?}"))?;
        let poll = sockets.create_poll_group();
        let my_id = client.user().steam_id();
        let (tx, rx) = std::sync::mpsc::channel();
        client.matchmaking().create_lobby(LobbyType::Public, 8, move |r| {
            let _ = tx.send(r);
        });
        let mut lobby: Option<LobbyId> = None;
        for _ in 0..100 {
            client.run_callbacks();
            if let Ok(r) = rx.try_recv() {
                lobby = Some(r.expect("lobby create"));
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let lobby = lobby.ok_or("lobby create timed out")?;
        client.matchmaking().set_lobby_data(lobby, LOBBY_HOST_KEY, &my_id.raw().to_string());
        client.matchmaking().set_lobby_data(lobby, "name", &format!("LOREFORGE — {name}"));
        Ok(Self {
            client,
            lobby,
            listen,
            poll,
            peers: Vec::new(),
            host_id: my_id.raw(),
        })
    }

    pub fn lobby_raw(&self) -> u64 {
        self.lobby.raw()
    }

    /// Pump callbacks + the poll group; accept new connections and decode
    /// inbound protocol-v4 frames. Send replies with `send_to`/`broadcast`.
    pub fn pump(&mut self) -> Vec<HostEvent> {
        self.client.run_callbacks();
        let mut out = Vec::new();
        while let Some(event) = self.listen.try_receive_event() {
            match event {
                ListenSocketEvent::Connecting(req) => {
                    let _ = req.accept();
                }
                ListenSocketEvent::Connected(connected) => {
                    let peer = connected
                        .remote()
                        .steam_id()
                        .map(|s| s.raw())
                        .unwrap_or(0);
                    let conn = connected.take_connection();
                    conn.set_poll_group(&self.poll);
                    out.push(HostEvent::PeerConnected(peer));
                    self.peers.push((peer, conn));
                }
                ListenSocketEvent::Disconnected(d) => {
                    let peer = d.remote().steam_id().map(|s| s.raw()).unwrap_or(0);
                    self.peers.retain(|(p, _)| *p != peer);
                    out.push(HostEvent::PeerDisconnected(peer));
                }
            }
        }
        for (_, conn) in &mut self.peers {
            for msg in conn.receive_messages(32).unwrap_or_default() {
                let peer = msg.identity_peer().steam_id().map(|s| s.raw()).unwrap_or(0);
                if let Some(cm) = ProtocolCodec::decode_client(msg.data()) {
                    out.push(HostEvent::PeerMessage(peer, cm));
                }
            }
        }
        out
    }

    fn connection(&mut self, peer: PeerId) -> Option<&mut NetConnection> {
        self.peers.iter_mut().find(|(p, _)| *p == peer).map(|(_, c)| c)
    }

    pub fn send_to(&mut self, peer: PeerId, msg: &ServerMessage) {
        if let Some(conn) = self.connection(peer) {
            let bytes = ProtocolCodec::encode_server(msg);
            let _ = conn.send_message(&bytes, SendFlags::UNRELIABLE);
        }
    }

    pub fn broadcast(&mut self, msg: &ServerMessage) {
        let peers: Vec<PeerId> = self.peers.iter().map(|(p, _)| *p).collect();
        for peer in peers {
            self.send_to(peer, msg);
        }
    }
}

/// The joining side: join a lobby by raw id, read the host identity from
/// its datum, and connect P2P to it.
pub struct SteamClientNet {
    client: Client,
    conn: NetConnection,
    poll: NetPollGroup,
    lobby: LobbyId,
    pub host_id: u64,
    announced: bool,
}

impl SteamClientNet {
    pub fn join(lobby_raw: u64, _name: &str) -> Result<Self, String> {
        let client = Client::init().map_err(|e| format!("steam init: {e}"))?;
        let lobby = LobbyId::from_raw(lobby_raw);
        let (tx, rx) = std::sync::mpsc::channel();
        client.matchmaking().join_lobby(lobby, move |r| {
            let _ = tx.send(r);
        });
        let mut joined = false;
        for _ in 0..100 {
            client.run_callbacks();
            if let Ok(r) = rx.try_recv() {
                joined = r.is_ok();
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if !joined {
            return Err("lobby join timed out (wrong id, or host offline)".into());
        }
        let host_raw: u64 = client
            .matchmaking()
            .lobby_data(lobby, LOBBY_HOST_KEY)
            .ok_or("lobby has no host datum")?
            .parse()
            .map_err(|e| format!("host datum: {e}"))?;
        let sockets = client.networking_sockets();
        let poll = sockets.create_poll_group();
        let conn = sockets
            .connect_p2p(
                NetworkingIdentity::new_steam_id(SteamId::from_raw(host_raw)),
                0,
                Vec::<NetworkingConfigEntry>::new(),
            )
            .map_err(|e| format!("connect_p2p: {e:?}"))?;
        Ok(Self { client, conn, poll, lobby, host_id: host_raw, announced: false })
    }

    /// Pump; returns decoded protocol-v4 server messages and connection
    /// status. `Connected` fires once the host accepts the session.
    pub fn pump(&mut self) -> Vec<SteamClientEvent> {
        self.client.run_callbacks();
        let mut out = Vec::new();
        // connection-state polling is not exposed on NetConnection in
        // steamworks 0.12; treat the first writable send as Connected and
        // a send failure as Disconnected.
        if !self.announced {
            let bytes = ProtocolCodec::encode_client(&ClientMessage::Hello {
                name: String::new(),
                protocol_version: lf_protocol::PROTOCOL_VERSION,
            });
            let probe = self.conn.send_message(&bytes, SendFlags::UNRELIABLE);
            if probe.is_ok() {
                self.announced = true;
                out.push(SteamClientEvent::Connected);
            }
        } else {
            // keep the connection alive; a send failure means it died
            let probe = self.conn.send_message(&[], SendFlags::UNRELIABLE);
            if probe.is_err() {
                out.push(SteamClientEvent::Disconnected);
            }
        }
        for msg in self.poll.receive_messages(32) {
            if let Some(sm) = ProtocolCodec::decode_server(msg.data()) {
                out.push(SteamClientEvent::Message(sm));
            }
        }
        out
    }

    pub fn send(&mut self, msg: &ClientMessage) {
        let bytes = ProtocolCodec::encode_client(msg);
        let _ = self.conn.send_message(&bytes, SendFlags::UNRELIABLE);
    }

    /// Direct P2P connect to a known host identity (e.g. an anonymous
    /// game-server host on the same machine).
    pub fn connect_direct(host_raw: u64) -> Result<Self, String> {
        let client = Client::init().map_err(|e| format!("steam init: {e}"))?;
        let sockets = client.networking_sockets();
        let poll = sockets.create_poll_group();
        let conn = sockets
            .connect_p2p(
                NetworkingIdentity::new_steam_id(SteamId::from_raw(host_raw)),
                0,
                Vec::<NetworkingConfigEntry>::new(),
            )
            .map_err(|e| format!("connect_p2p: {e:?}"))?;
        Ok(Self { client, conn, poll, lobby: LobbyId::from_raw(0), host_id: host_raw, announced: false })
    }

    /// Leave the lobby on drop of the session (no-op when connecting
    /// directly to a game-server identity).
    pub fn leave_lobby(&self) {
        if self.lobby.raw() != 0 {
            self.client.matchmaking().leave_lobby(self.lobby);
        }
    }
}

/// Client-side transport events.
pub enum SteamClientEvent {
    Connected,
    Message(ServerMessage),
    Disconnected,
}

/// Single-machine / dedicated-server host: the anonymous game-server
/// identity (distinct from the local user) so a same-account client can
/// connect P2P on one machine. No lobby — discovery is the printed id.
pub struct SteamGameServerHost {
    server: steamworks::Server,
    client: Client,
    listen: ListenSocket,
    poll: NetPollGroup,
    peers: Vec<(PeerId, NetConnection)>,
    pub host_id: u64,
}

impl SteamGameServerHost {
    pub fn bind() -> Result<Self, String> {
        use std::net::Ipv4Addr;
        let (mut server, client) = steamworks::Server::init(
            Ipv4Addr::UNSPECIFIED,
            27015,
            27016,
            steamworks::ServerMode::NoAuthentication,
            "0.0.1",
        )
        .map_err(|e| format!("gameserver init: {e:?}"))?;
        // gameserver logon is asynchronous: the networking interfaces only
        // work once Steam assigns the server identity. Wait for it.
        for _ in 0..200 {
            server.run_callbacks();
            if server.steam_id().raw() != 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        if server.steam_id().raw() == 0 {
            return Err("gameserver logon timed out (no identity assigned)".into());
        }
        let sockets = client.networking_sockets();
        let listen = sockets
            .create_listen_socket_p2p(0, Vec::<NetworkingConfigEntry>::new())
            .map_err(|e| format!("listen socket: {e:?}"))?;
        let poll = sockets.create_poll_group();
        let host_id = server.steam_id().raw();
        Ok(Self { server, client, listen, poll, peers: Vec::new(), host_id })
    }

    pub fn pump(&mut self) -> Vec<HostEvent> {
        self.server.run_callbacks();
        self.client.run_callbacks();
        let mut out = Vec::new();
        while let Some(event) = self.listen.try_receive_event() {
            match event {
                ListenSocketEvent::Connecting(req) => {
                    let _ = req.accept();
                }
                ListenSocketEvent::Connected(connected) => {
                    let peer = connected
                        .remote()
                        .steam_id()
                        .map(|s| s.raw())
                        .unwrap_or(0);
                    let mut conn = connected.take_connection();
                    conn.set_poll_group(&self.poll);
                    out.push(HostEvent::PeerConnected(peer));
                    self.peers.push((peer, conn));
                }
                ListenSocketEvent::Disconnected(d) => {
                    let peer = d.remote().steam_id().map(|s| s.raw()).unwrap_or(0);
                    self.peers.retain(|(p, _)| *p != peer);
                    out.push(HostEvent::PeerDisconnected(peer));
                }
            }
        }
        for (_, conn) in &mut self.peers {
            for msg in conn.receive_messages(32).unwrap_or_default() {
                let peer = msg.identity_peer().steam_id().map(|s| s.raw()).unwrap_or(0);
                if let Some(cm) = ProtocolCodec::decode_client(msg.data()) {
                    out.push(HostEvent::PeerMessage(peer, cm));
                }
            }
        }
        out
    }

    pub fn send_to(&mut self, peer: PeerId, msg: &ServerMessage) {
        if let Some(conn) = self.connection(peer) {
            let bytes = ProtocolCodec::encode_server(msg);
            let _ = conn.send_message(&bytes, SendFlags::UNRELIABLE);
        }
    }

    fn connection(&mut self, peer: PeerId) -> Option<&mut NetConnection> {
        self.peers.iter_mut().find(|(p, _)| *p == peer).map(|(_, c)| c)
    }
}

// ===== king-quest Steam pass: local loopback pair (Valve's test API) ====
//
// steamworks-rs 0.12 does not wrap ISteamNetworkingSockets::
// CreateSocketPair, but steamworks-sys does. The pair gives two
// already-connected loopback connections IN PROCESS — Valve built it
// "for testing", and it lets us exercise the full host/client message
// path (protocol-v4 codec, poll, decode, reply) with a single Steam
// identity and no network.

fn raw_sockets() -> *mut steamworks_sys::ISteamNetworkingSockets {
    unsafe { steamworks_sys::SteamAPI_SteamNetworkingSockets_SteamAPI_v012() }
}

/// One end of a local loopback connection pair (raw sys wrapper).
pub struct PairConnection {
    sockets: *mut steamworks_sys::ISteamNetworkingSockets,
    handle: steamworks_sys::HSteamNetConnection,
}
unsafe impl Send for PairConnection {}

impl PairConnection {
    /// UNRELIABLE by default; pass `reliable` for the reliable lane.
    pub fn send_raw(&mut self, data: &[u8], reliable: bool) -> bool {
        let flags: i32 = if reliable { 8 } else { 0 }; // k_nSteamNetworkingSend_Reliable
        let mut out: i64 = 0;
        let (sockets, handle) = (self.sockets, self.handle);
        let r = unsafe {
            steamworks_sys::SteamAPI_ISteamNetworkingSockets_SendMessageToConnection(
                sockets, handle, data.as_ptr() as *const _, data.len() as u32, flags, &mut out,
            )
        };
        r == steamworks_sys::EResult::k_EResultOK
    }

    /// Drain inbound payloads (released messages are copied out).
    pub fn receive_raw(&mut self, max: usize) -> Vec<Vec<u8>> {
        let mut arr: [*mut steamworks_sys::SteamNetworkingMessage_t; 16] =
            [std::ptr::null_mut(); 16];
        let (sockets, handle) = (self.sockets, self.handle);
        let n = unsafe {
            steamworks_sys::SteamAPI_ISteamNetworkingSockets_ReceiveMessagesOnConnection(
                sockets, handle, arr.as_mut_ptr(), max as i32,
            )
        };
        (0..n.max(0) as usize)
            .map(|i| unsafe {
                let m = arr[i];
                let data = std::slice::from_raw_parts(
                    (*m).m_pData as *const u8,
                    (*m).m_cbSize as usize,
                )
                .to_vec();
                if let Some(release) = (*m).m_pfnRelease {
                    release(m);
                }
                data
            })
            .collect()
    }

    pub fn flush(&mut self) {
        let (sockets, handle) = (self.sockets, self.handle);
        unsafe {
            steamworks_sys::SteamAPI_ISteamNetworkingSockets_FlushMessagesOnConnection(
                sockets, handle,
            );
        }
    }
}

/// Create two already-connected loopback connections in-process. The
/// returned `Client` must stay alive while the pair is used — dropping it
/// tears down the SteamAPI session the pair rides on.
pub fn create_local_pair() -> Result<(PairConnection, PairConnection, Client), String> {
    // the loopback pair rides on the SteamAPI session: initialize it
    // (idempotent — the probe/game has usually already done this)
    let client = Client::init().map_err(|e| format!("steam init: {e}"))?;
    client.run_callbacks();
    let sockets = raw_sockets();
    if sockets.is_null() {
        return Err("SteamNetworkingSockets interface not initialized — start the Steam client first".into());
    }
    let (mut a, mut b): (steamworks_sys::HSteamNetConnection, steamworks_sys::HSteamNetConnection) =
        (0, 0);
    let ok = unsafe {
        steamworks_sys::SteamAPI_ISteamNetworkingSockets_CreateSocketPair(
            sockets, &mut a, &mut b, true, std::ptr::null(), std::ptr::null(),
        )
    };
    if !ok || a == 0 || b == 0 {
        return Err("CreateSocketPair failed".into());
    }
    Ok((
        PairConnection { sockets, handle: a },
        PairConnection { sockets, handle: b },
        client,
    ))
}
