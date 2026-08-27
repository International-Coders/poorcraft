use serde::{Deserialize, Serialize};

/// Messages a client sends to the server.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ClientMessage {
    Hello { name: String, protocol_version: u32 },
    /// Player state, sent ~20/s.
    Position { pos: [f32; 3], yaw: f32, pitch: f32 },
    /// Request to change a block (validated/applied by the server).
    SetBlock { x: i32, y: i32, z: i32, block: u32 },
    Chat { text: String },
    /// P37 (protocol v4) player trading: offer items to a player.
    TradeOffer { to: u64, give: Vec<(String, u8)>, want: Vec<(String, u8)> },
    /// Accept a received offer.
    TradeAccept { offer_id: u64 },
    /// Cancel/decline a standing offer (either side).
    TradeCancel { offer_id: u64 },
    Goodbye,
}

/// Messages the server sends to clients.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ServerMessage {
    /// Acceptance + your id + world seed + current player list.
    Welcome { your_id: u64, seed: u64, players: Vec<(u64, String)> },
    /// Snapshot of all player states (id, position, yaw), ~20/s.
    PlayerStates { states: Vec<(u64, [f32; 3], f32)> },
    BlockUpdate { x: i32, y: i32, z: i32, block: u32 },
    Chat { from: String, text: String },
    PlayerJoined { id: u64, name: String },
    PlayerLeft { id: u64 },
    Reject { reason: String },
    /// P37 (protocol v4): an offer addressed to you.
    TradeOffered { offer_id: u64, from: u64, from_name: String, give: Vec<(String, u8)>, want: Vec<(String, u8)> },
    /// Escrow verdict: accepted swaps deliver items to BOTH sides;
    /// cancelled offers free them. `items` is what THIS client receives.
    TradeResolved { offer_id: u64, accepted: bool, items: Vec<(String, u8)> },
}

pub const PROTOCOL_VERSION: u32 = 4;

/// One escrowed trade offer on the server (P37). The server holds the
/// offer and validates the participants; item swaps apply on the peers
/// (authoritative-lite, same policy as blocks).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TradeOfferRecord {
    pub offer_id: u64,
    pub from: u64,
    pub to: u64,
    pub give: Vec<(String, u8)>,
    pub want: Vec<(String, u8)>,
}

#[cfg(test)]
mod trade_tests {
    use super::*;

    /// v4 messages round-trip through the wire format.
    #[test]
    fn trade_messages_round_trip() {
        let msg = ClientMessage::TradeOffer {
            to: 7,
            give: vec![("iron_ingot".into(), 4)],
            want: vec![("dragon_scale".into(), 1)],
        };
        let bytes = bincode::serialize(&msg).unwrap();
        let back: ClientMessage = bincode::deserialize(&bytes).unwrap();
        assert_eq!(msg, back);
        let resolved = ServerMessage::TradeResolved {
            offer_id: 1,
            accepted: true,
            items: vec![("dragon_scale".into(), 1)],
        };
        let back: ServerMessage = bincode::deserialize(&bincode::serialize(&resolved).unwrap()).unwrap();
        assert_eq!(back, resolved);
        assert_eq!(PROTOCOL_VERSION, 4);
    }
}

pub struct ProtocolCodec;

impl ProtocolCodec {
    /// Frame: 1 byte kind tag + 2 byte length + bincode payload.
    pub fn encode_client(msg: &ClientMessage) -> Vec<u8> {
        encode(0x01, msg)
    }

    pub fn encode_server(msg: &ServerMessage) -> Vec<u8> {
        encode(0x02, msg)
    }

    pub fn decode_client(data: &[u8]) -> Option<ClientMessage> {
        decode(data, 0x01)
    }

    pub fn decode_server(data: &[u8]) -> Option<ServerMessage> {
        decode(data, 0x02)
    }
}

fn encode(kind: u8, msg: &impl serde::Serialize) -> Vec<u8> {
    let payload = bincode::serialize(msg).unwrap_or_default();
    let mut out = Vec::with_capacity(payload.len() + 3);
    out.push(kind);
    out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    out.extend_from_slice(&payload);
    out
}

fn decode<T: serde::de::DeserializeOwned>(data: &[u8], want_kind: u8) -> Option<T> {
    if data.len() < 3 || data[0] != want_kind {
        return None;
    }
    let len = u16::from_be_bytes([data[1], data[2]]) as usize;
    if data.len() < 3 + len {
        return None;
    }
    bincode::deserialize(&data[3..3 + len]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_roundtrip() {
        let msgs = vec![
            ClientMessage::Hello { name: "zari".into(), protocol_version: PROTOCOL_VERSION },
            ClientMessage::Position { pos: [1.0, 65.0, 2.0], yaw: 0.5, pitch: -0.1 },
            ClientMessage::SetBlock { x: -3, y: 70, z: 12, block: 2 },
            ClientMessage::Chat { text: "hello world".into() },
            ClientMessage::Goodbye,
        ];
        for m in msgs {
            let enc = ProtocolCodec::encode_client(&m);
            assert_eq!(ProtocolCodec::decode_client(&enc), Some(m));
        }
    }

    #[test]
    fn server_roundtrip() {
        let msgs = vec![
            ServerMessage::Welcome { your_id: 7, seed: 12345, players: vec![(7, "zari".into())] },
            ServerMessage::PlayerStates { states: vec![(7, [0.0, 64.0, 0.0], 1.5)] },
            ServerMessage::BlockUpdate { x: 1, y: 2, z: 3, block: 0 },
            ServerMessage::Chat { from: "zari".into(), text: "hi".into() },
            ServerMessage::PlayerJoined { id: 8, name: "maya".into() },
            ServerMessage::PlayerLeft { id: 8 },
            ServerMessage::Reject { reason: "version".into() },
        ];
        for m in msgs {
            let enc = ProtocolCodec::encode_server(&m);
            assert_eq!(ProtocolCodec::decode_server(&enc), Some(m));
        }
    }

    #[test]
    fn kinds_do_not_cross() {
        let enc = ProtocolCodec::encode_client(&ClientMessage::Goodbye);
        assert!(ProtocolCodec::decode_server(&enc).is_none());
    }

    #[test]
    fn truncated_frames_rejected() {
        let enc = ProtocolCodec::encode_server(&ServerMessage::Chat { from: "a".into(), text: "b".into() });
        assert!(ProtocolCodec::decode_server(&enc[..enc.len() - 1]).is_none());
        assert!(ProtocolCodec::decode_server(&[]).is_none());
    }
}
