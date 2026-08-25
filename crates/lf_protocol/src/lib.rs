use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ServerMessage {
    Handshake { protocol_version: u32 },
    Login { name: String, auth_token: Option<String> },
    ModList { mods: Vec<(String, String)>, hashes: Vec<String> },
    ChunkData { pos: (i32, i32), sections: Vec<u8> },
    Chat { from: String, text: String },
    Disconnect { reason: String },
}

pub struct ProtocolCodec;

impl ProtocolCodec {
    pub fn encode(msg: &ServerMessage) -> Vec<u8> {
        let bytes = bincode::serialize(msg).unwrap_or_default();
        let mut result = Vec::new();
        result.push(match msg {
            ServerMessage::Handshake { .. } => 0x01,
            ServerMessage::Login { .. } => 0x02,
            ServerMessage::ModList { .. } => 0x03,
            ServerMessage::ChunkData { .. } => 0x10,
            ServerMessage::Chat { .. } => 0x30,
            ServerMessage::Disconnect { .. } => 0x40,
        });
        result.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        result.extend_from_slice(&bytes);
        result
    }

    pub fn decode(data: &[u8]) -> Option<ServerMessage> {
        if data.len() < 3 { return None; }
        let payload_len = u16::from_be_bytes([data[1], data[2]]) as usize;
        if data.len() < 3 + payload_len { return None; }
        let payload = &data[3..3 + payload_len];
        bincode::deserialize(payload).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_roundtrip() {
        let msg = ServerMessage::Handshake { protocol_version: 1 };
        let encoded = ProtocolCodec::encode(&msg);
        let decoded = ProtocolCodec::decode(&encoded);
        assert_eq!(decoded, Some(msg));
    }
}
