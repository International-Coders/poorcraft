//! P3D-803: lobbies, invites, and the transport boundary.
//!
//! The session layer under multiplayer: a lobby is created by a host
//! who chooses its member cap (D-029 — the host owns the population
//! limit), grows through deterministic invite/accept flows, and never
//! loses leadership ambiguously — when the host leaves, the lowest
//! remaining member id takes over. All messages move across a
//! `Transport` trait; the in-memory `LoopbackTransport` is the
//! deterministic test implementation and the future Steam SDK socket
//! implements the same trait, so nothing above this layer changes.

use std::collections::{BTreeMap, VecDeque};

/// A peer identity (a Steam id, or a loopback id in tests).
pub type PeerId = u64;

/// The transport boundary: session code sends and receives opaque
/// frames; the implementation owns the wire.
pub trait Transport {
    fn send(&mut self, to: PeerId, payload: &[u8]);
    /// Drain one inbound frame, if any.
    fn recv(&mut self) -> Option<(PeerId, Vec<u8>)>;
}

/// Deterministic in-memory transport: per-peer FIFO outboxes, no loss,
/// no reordering — the test twin of a real socket.
#[derive(Clone, Debug, Default)]
pub struct LoopbackTransport {
    pub queues: BTreeMap<PeerId, VecDeque<(PeerId, Vec<u8>)>>,
}

impl LoopbackTransport {
    pub fn new() -> LoopbackTransport {
        LoopbackTransport::default()
    }
}

impl Transport for LoopbackTransport {
    fn send(&mut self, to: PeerId, payload: &[u8]) {
        // Loopback frames carry sender 0 (the bus); real transports
        // stamp the true sender.
        self.queues.entry(to).or_default().push_back((0, payload.to_vec()));
    }
    fn recv(&mut self) -> Option<(PeerId, Vec<u8>)> {
        // Loopback drains peer 0's queue (the test bus).
        self.queues.get_mut(&0).and_then(|q| q.pop_front())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberState {
    /// Invited; the invite is pending until accepted or declined.
    Invited,
    /// A full member.
    Joined,
}

#[derive(Clone, Debug)]
pub struct Lobby {
    pub id: u64,
    pub host: PeerId,
    /// Host-chosen hard cap (D-029).
    pub cap: usize,
    pub members: BTreeMap<PeerId, MemberState>,
}

impl Lobby {
    /// Full members in id order.
    pub fn joined(&self) -> Vec<PeerId> {
        self.members
            .iter()
            .filter(|(_, s)| **s == MemberState::Joined)
            .map(|(p, _)| *p)
            .collect()
    }
}

/// Why a session op failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    /// D-029: the host's cap is the law.
    LobbyFull,
    AlreadyMember,
    NotMember,
    NoInvite,
    UnknownLobby,
    WrongHost,
}

/// The lobby manager: lifecycle owner for every lobby in the world.
#[derive(Clone, Debug, Default)]
pub struct LobbyManager {
    pub lobbies: BTreeMap<u64, Lobby>,
    next_lobby: u64,
}

impl LobbyManager {
    pub fn new() -> LobbyManager {
        LobbyManager::default()
    }

    /// Create a lobby hosted by `host` with the host's chosen cap.
    pub fn create(&mut self, host: PeerId, cap: usize) -> Result<u64, SessionError> {
        if cap == 0 {
            return Err(SessionError::LobbyFull);
        }
        self.next_lobby += 1;
        let id = self.next_lobby;
        let mut members = BTreeMap::new();
        members.insert(host, MemberState::Joined);
        self.lobbies.insert(id, Lobby { id, host, cap, members });
        Ok(id)
    }

    /// Invite a peer into a lobby (host-only).
    pub fn invite(&mut self, lobby: u64, host: PeerId, peer: PeerId) -> Result<(), SessionError> {
        let l = self.lobbies.get_mut(&lobby).ok_or(SessionError::UnknownLobby)?;
        if l.host != host {
            return Err(SessionError::WrongHost);
        }
        if l.members.contains_key(&peer) {
            return Err(SessionError::AlreadyMember);
        }
        if l.joined().len() >= l.cap {
            return Err(SessionError::LobbyFull);
        }
        l.members.insert(peer, MemberState::Invited);
        Ok(())
    }

    /// Accept a pending invite.
    pub fn accept(&mut self, lobby: u64, peer: PeerId) -> Result<(), SessionError> {
        let l = self.lobbies.get_mut(&lobby).ok_or(SessionError::UnknownLobby)?;
        match l.members.get(&peer) {
            Some(MemberState::Joined) => Err(SessionError::AlreadyMember),
            Some(MemberState::Invited) => {
                // Cap checked at invite time; re-check at accept time in
                // case the cap shrank (it cannot here, but the law holds).
                if l.joined().len() >= l.cap {
                    let _ = l.members.remove(&peer);
                    return Err(SessionError::LobbyFull);
                }
                l.members.insert(peer, MemberState::Joined);
                Ok(())
            }
            None => Err(SessionError::NoInvite),
        }
    }

    /// Leave: removes the peer; if the HOST left, the lowest remaining
    /// joined member becomes host; an emptied lobby dissolves.
    pub fn leave(&mut self, lobby: u64, peer: PeerId) -> Result<(), SessionError> {
        let l = self.lobbies.get_mut(&lobby).ok_or(SessionError::UnknownLobby)?;
        if l.members.remove(&peer).is_none() {
            return Err(SessionError::NotMember);
        }
        if l.joined().is_empty() {
            self.lobbies.remove(&lobby);
            return Ok(());
        }
        if l.host == peer {
            if let Some(next) = l.joined().first() {
                l.host = *next;
            }
        }
        Ok(())
    }
}

/// A live session: one lobby routed over one transport. Broadcasts
/// fan out to every joined member in id order (deterministic).
pub struct Session<'t> {
    pub lobby: u64,
    pub transport: &'t mut dyn Transport,
}

impl<'t> Session<'t> {
    pub fn new(lobby: u64, transport: &'t mut dyn Transport) -> Session<'t> {
        Session { lobby, transport }
    }

    /// Send to every joined member EXCEPT `from`.
    pub fn broadcast(&mut self, members: &[PeerId], from: PeerId, payload: &[u8]) -> usize {
        let mut sent = 0;
        for p in members {
            if *p == from {
                continue;
            }
            self.transport.send(*p, payload);
            sent += 1;
        }
        sent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The invite lifecycle: host invites, peer accepts; declines and
    /// uninvited accepts fail; only the host may invite; peers cannot
    /// join twice.
    #[test]
    fn p3d803_invite_lifecycle() {
        let mut m = LobbyManager::new();
        let lobby = m.create(100, 8).unwrap();

        // Non-host invites are refused.
        assert_eq!(m.invite(lobby, 999, 1), Err(SessionError::WrongHost));
        // Unknown lobby refused.
        assert_eq!(m.invite(9999, 100, 1), Err(SessionError::UnknownLobby));

        assert!(m.invite(lobby, 100, 1).is_ok());
        assert!(m.invite(lobby, 100, 2).is_ok());
        // Invited peer cannot be re-invited.
        assert_eq!(m.invite(lobby, 100, 1), Err(SessionError::AlreadyMember));

        // Uninvited peers cannot accept.
        assert_eq!(m.accept(lobby, 3), Err(SessionError::NoInvite));
        assert!(m.accept(lobby, 1).is_ok());
        assert_eq!(m.accept(lobby, 1), Err(SessionError::AlreadyMember));
        assert_eq!(m.lobbies[&lobby].joined(), vec![1, 100], "id order");

        // A member leaving frees their slot; invited-but-not-joined
        // members do not count against the joined roster.
        assert!(m.leave(lobby, 1).is_ok());
        assert_eq!(m.lobbies[&lobby].joined(), vec![100]);
        assert!(m.accept(lobby, 2).is_ok());
        assert_eq!(m.lobbies[&lobby].joined(), vec![2, 100], "id order");
    }

    /// The host's cap is the law (D-029): invites are free, but the
    /// cap binds at ACCEPT time (with the failed invite withdrawn), a
    /// freed slot lets the next acceptance through, and cap=0 lobbies
    /// do not exist.
    #[test]
    fn p3d803_host_cap_enforced() {
        let mut m = LobbyManager::new();
        assert_eq!(m.create(1, 0), Err(SessionError::LobbyFull));
        let lobby = m.create(1, 3).unwrap(); // host + 2 slots

        assert!(m.invite(lobby, 1, 2).is_ok());
        assert!(m.invite(lobby, 1, 3).is_ok());
        assert!(m.invite(lobby, 1, 4).is_ok(), "invites do not reserve slots");
        assert!(m.accept(lobby, 2).is_ok());
        assert!(m.accept(lobby, 3).is_ok()); // 3 joined = full
        assert_eq!(m.accept(lobby, 4), Err(SessionError::LobbyFull), "cap binds at accept");
        assert!(!m.lobbies[&lobby].members.contains_key(&4), "failed invite withdrawn");
        assert!(m.leave(lobby, 2).is_ok());
        assert!(m.invite(lobby, 1, 4).is_ok());
        assert!(m.accept(lobby, 4).is_ok(), "freed slot reopens");
        assert_eq!(m.lobbies[&lobby].joined(), vec![1, 3, 4]);
    }

    /// Host migration is deterministic: the lowest remaining joined
    /// member takes over; an emptied lobby dissolves.
    #[test]
    fn p3d803_host_migration_and_dissolution() {
        let mut m = LobbyManager::new();
        let lobby = m.create(50, 8).unwrap();
        for p in [10u64, 30, 20] {
            assert!(m.invite(lobby, 50, p).is_ok());
            assert!(m.accept(lobby, p).is_ok());
        }
        assert_eq!(m.lobbies[&lobby].joined(), vec![10, 20, 30, 50], "id order");

        // A non-host leaves: host unchanged.
        assert!(m.leave(lobby, 30).is_ok());
        assert_eq!(m.lobbies[&lobby].host, 50);

        // The host leaves: lowest remaining joined id (10) leads.
        assert!(m.leave(lobby, 50).is_ok());
        assert_eq!(m.lobbies[&lobby].host, 10);

        // Everyone leaves: the lobby dissolves (the last leave removes it).
        assert!(m.leave(lobby, 10).is_ok());
        assert_eq!(m.lobbies[&lobby].joined(), vec![20]);
        assert!(m.leave(lobby, 20).is_ok());
        assert!(m.lobbies.get(&lobby).is_none(), "empty lobby dissolved");
        // Leaving a dissolved lobby is a clean error.
        assert_eq!(m.leave(lobby, 20), Err(SessionError::UnknownLobby));

        // Determinism: the same flow reaches the same end state.
        let mut n = LobbyManager::new();
        let l2 = n.create(50, 8).unwrap();
        for p in [10u64, 30, 20] {
            n.invite(l2, 50, p).unwrap();
            n.accept(l2, p).unwrap();
        }
        for p in [30u64, 50, 10, 20] {
            n.leave(l2, p).unwrap();
        }
        assert!(n.lobbies.get(&l2).is_none());
    }

    /// The loopback transport delivers FIFO per peer; broadcast fans
    /// out to every joined member except the sender; and a SECOND
    /// transport behind the same trait drives the same session flows —
    /// the boundary holds for the future Steam socket.
    #[test]
    fn p3d803_transport_boundary_and_broadcast() {
        let mut m = LobbyManager::new();
        let lobby = m.create(1, 8).unwrap();
        for p in [2u64, 3] {
            m.invite(lobby, 1, p).unwrap();
            m.accept(lobby, p).unwrap();
        }
        let members = m.lobbies[&lobby].joined();

        let mut bus = LoopbackTransport::new();
        {
            let mut session = Session::new(lobby, &mut bus);
            assert_eq!(session.broadcast(&members, 1, b"hello"), 2, "fan-out to 2 and 3");
            assert_eq!(session.broadcast(&members, 2, b"hi"), 2, "sender excluded");
        }
        // Loopback queues: peer 2 got hello+hi, peer 3 got hello+hi,
        // FIFO order preserved. (We peek by draining fresh clones.)
        let mut probe = bus.clone();
        // Peer 3 is in on both messages; peer 2 sent the second one.
        let q3 = probe.queues.get(&3).expect("peer 3 has mail");
        let frames: Vec<Vec<u8>> = q3.iter().map(|(_, p)| p.clone()).collect();
        assert_eq!(frames.len(), 2, "FIFO: hello then hi");
        assert_eq!(frames[0], b"hello".to_vec());
        assert_eq!(frames[1], b"hi".to_vec());
        let q2 = probe.queues.get(&2).expect("peer 2 has mail");
        assert_eq!(q2.len(), 1, "peer 2 only receives hello (it sent hi)");

        // Trait substitutability: a hand-rolled spy transport drives
        // the identical broadcast path.
        struct Spy {
            sent: BTreeMap<PeerId, usize>,
        }
        impl Transport for Spy {
            fn send(&mut self, to: PeerId, _payload: &[u8]) {
                *self.sent.entry(to).or_insert(0) += 1;
            }
            fn recv(&mut self) -> Option<(PeerId, Vec<u8>)> {
                None
            }
        }
        let mut spy = Spy { sent: BTreeMap::new() };
        {
            let mut session = Session::new(lobby, &mut spy);
            assert_eq!(session.broadcast(&members, 1, b"x"), 2);
        }
        assert_eq!(spy.sent.get(&2), Some(&1));
        assert_eq!(spy.sent.get(&3), Some(&1));
    }
}
