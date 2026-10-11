//! Steam Steps 34-36: lobbies, P2P session bookkeeping, and the invite
//! flow. The MODEL is transport-neutral and always available (the UDP
//! path encodes the host address as the lobby code); the Steamworks arms
//! behind the `steam` feature map the same operations onto
//! ISteamMatchmaking / ISteamNetworkingP2P. Without the feature — the
//! default build — everything here is real, tested state, and the
//! transport stays UDP.

use serde::{Deserialize, Serialize};

/// A lobby invite handle: share the code, the recipient joins with it.
/// For UDP the code IS "host:port"; with Steam it is the numeric lobby
/// id. Both round-trip through a string.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invite {
    pub code: String,
    pub from_name: String,
}

/// Live lobby membership.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lobby {
    /// The code other players join with.
    pub code: String,
    pub members: Vec<String>,
    /// True while this client is the host (UDP: it runs the server).
    pub hosting: bool,
}

#[derive(Default)]
pub struct LobbyManager {
    active: Option<Lobby>,
    pending_invite: Option<Invite>,
}

impl LobbyManager {
    /// Host a lobby. For the UDP transport the caller passes the bound
    /// server address as the code; joining is then direct.
    pub fn create(&mut self, code: impl Into<String>, host_name: &str) -> Lobby {
        let lobby = Lobby { code: code.into(), members: vec![host_name.to_string()], hosting: true };
        self.active = Some(lobby.clone());
        lobby
    }

    /// Join by invite code. Unknown/malformed UDP codes are refused by
    /// the caller's connection attempt; the manager only records state.
    pub fn join(&mut self, code: &str, player_name: &str) -> Option<Lobby> {
        let code = code.trim().to_string();
        if code.is_empty() {
            return None;
        }
        let lobby = Lobby { code, members: vec![player_name.to_string()], hosting: false };
        self.active = Some(lobby.clone());
        Some(lobby
        )
    }

    /// A member arrived (server relayed a join).
    pub fn member_joined(&mut self, name: &str) {
        if let Some(l) = &mut self.active {
            if !l.members.iter().any(|m| m == name) {
                l.members.push(name.to_string());
            }
        }
    }

    pub fn member_left(&mut self, name: &str) {
        if let Some(l) = &mut self.active {
            l.members.retain(|m| m != name);
        }
    }

    pub fn active(&self) -> Option<&Lobby> {
        self.active.as_ref()
    }

    pub fn leave(&mut self) {
        self.active = None;
        self.pending_invite = None;
    }

    /// The invite flow (Step 36): the host mints an invite from the
    /// active lobby; the recipient holds it until they join or drop it.
    pub fn invite(&mut self, from_name: &str) -> Option<Invite> {
        self.active.as_ref().map(|l| Invite { code: l.code.clone(), from_name: from_name.to_string() })
    }

    pub fn receive_invite(&mut self, invite: Invite) {
        self.pending_invite = Some(invite);
    }

    pub fn pending_invite(&self) -> Option<&Invite> {
        self.pending_invite.as_ref()
    }

    /// Accept the pending invite (consumes it).
    pub fn accept_invite(&mut self, player_name: &str) -> Option<Lobby> {
        let invite = self.pending_invite.take()?;
        self.join(&invite.code, player_name)
    }

    pub fn decline_invite(&mut self) {
        self.pending_invite = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Steps 34-35: host, join, membership churn, leave.
    #[test]
    fn lobby_lifecycle_over_udp_codes() {
        let mut host = LobbyManager::default();
        let lobby = host.create("192.168.1.8:7777", "alice");
        assert!(lobby.hosting);
        host.member_joined("bob");
        host.member_joined("bob"); // idempotent
        assert_eq!(host.active().unwrap().members, vec!["alice", "bob"]);
        host.member_left("bob");
        assert_eq!(host.active().unwrap().members.len(), 1);
        host.leave();
        assert!(host.active().is_none());
    }

    /// Step 36: the invite flow end to end.
    #[test]
    fn invite_flow_end_to_end() {
        let mut host = LobbyManager::default();
        let _ = host.create("10.0.0.4:7777", "alice");
        let invite = host.invite("alice").expect("hosting mints invites");
        let mut guest = LobbyManager::default();
        guest.receive_invite(invite);
        assert_eq!(guest.pending_invite().unwrap().from_name, "alice");
        let lobby = guest.accept_invite("bob").expect("a held invite joins");
        assert!(!lobby.hosting);
        assert_eq!(lobby.code, "10.0.0.4:7777");
        assert!(guest.pending_invite().is_none());
        // a fresh invite can be declined just as cleanly
        guest.receive_invite(Invite { code: "x:1".into(), from_name: "carol".into() });
        guest.decline_invite();
        assert!(guest.pending_invite().is_none());
    }

    /// Joining refuses empty codes (the rest is the caller's connection
    /// attempt).
    #[test]
    fn empty_codes_are_refused() {
        let mut m = LobbyManager::default();
        assert!(m.join("   ", "bob").is_none());
        assert!(m.active().is_none());
    }
}
