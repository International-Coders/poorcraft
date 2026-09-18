use serde::{Deserialize, Serialize};

/// Messages a client sends to the server.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ClientMessage {
    /// `creative` is the v8 join-mode claim: the joiner's honest game mode,
    /// fixed for the session (a world is created in one mode). The server
    /// records it per player: a CREATIVE joiner's placements are ungated
    /// (creative is infinite by its own law), a SURVIVAL joiner's
    /// placements must pay (the place-payment gate). It is a client
    /// claim of prior-session state — the same honest-bootstrap tier as
    /// PackSync, not a server-verified fact.
    Hello { name: String, protocol_version: u32, creative: bool },
    /// Player state, sent ~20/s.
    Position { pos: [f32; 3], yaw: f32, pitch: f32 },
    /// Request to change a block (validated/applied by the server).
    /// `block` is the FULL BlockState u32 (v8): the block id in the low
    /// bits and the shape/fluid state nibbles in the high bits, so a
    /// placed slab arrives as a slab (it used to arrive as a full cube —
    /// peers and the editor's own chunk reload disagreed with the
    /// placement). The server masks the id wherever the law needs ids.
    ///
    /// `mine` is the v5 mine claim: `Some(..)` iff this edit is a player
    /// MINED dig (places and simulation edits claim nothing), carrying the
    /// held item id (`None` inside = a bare hand). The server evaluates
    /// the harvest law against the block it actually had and grants the
    /// canonical yield back to the editor alone
    /// ([`ServerMessage::ItemGrant`]) — a rejected or already-mined dig
    /// never pays, and a bare-handed dig of a no-tool block pays the same
    /// as it does offline.
    ///
    /// `place` is the v8 PLACE-PAYMENT claim: `Some(..)` iff this edit is a
    /// player ITEM PLACEMENT, naming the item whose consumption paid for
    /// the block. The server gates it against the player's canonical
    /// LEDGER — the item must be able to place this block
    /// (`lf_game::items::placement_pays`) and the ledger must hold one,
    /// which is then consumed. A placement the ledger cannot pay is
    /// refused (corrective echo + Reject, editor alone) and moves
    /// nothing. BOTH claims on one edit is a smuggle and refuses. Both
    /// `None` is a simulation edit (fluids, falling blocks, machines,
    /// spell effects) — the client-simmed tier, accepted ungated as
    /// shipped.
    SetBlock { x: i32, y: i32, z: i32, block: u32, mine: Option<MineClaim>, place: Option<PlaceClaim> },
    Chat { text: String },
    /// P37 (protocol v4) player trading: offer items to a player.
    TradeOffer { to: u64, give: Vec<(String, u8)>, want: Vec<(String, u8)> },
    /// Accept a received offer.
    TradeAccept { offer_id: u64 },
    /// Cancel/decline a standing offer (either side).
    TradeCancel { offer_id: u64 },
    /// THE PACK-SYNC LAW (protocol v6): the client's claim of its own
    /// pack — aggregated (item id, count) pairs, counts as u32 because a
    /// full pack can hold far more than 255 of one item — sent on join
    /// and whenever the live pack drifts from the last uploaded
    /// snapshot. The server rebuilds its per-player canonical LEDGER
    /// from it; the ledger is what the trade escrow gates against and
    /// what mined yields pay into. A client-claimed seed is the honest
    /// bootstrap (the server cannot know prior-session history);
    /// server-known deltas (grants, escrow moves) are applied by the
    /// server itself, so an honest client's ledger never overcounts its
    /// real pack.
    PackSync { items: Vec<(String, u32)> },
    /// THE CRAFT-REQUEST LAW (protocol v7): while connected, a workbench
    /// craft is a REQUEST, not a local act — the client names the recipe
    /// spec (the same (ingredients, output, output_count) tuple the
    /// transactional engine executes) and how many batches it wants. The
    /// server gates the spec against its own recipe book (a spec no
    /// recipe names is refused — output cannot be fabricated from
    /// nothing), executes `crafting::execute` against the player's
    /// canonical LEDGER, and answers the crafter ALONE with
    /// [`ServerMessage::CraftVerdict`]; the local pack moves only when
    /// the verdict lands. `req_id` is client-chosen and monotonic; the
    /// server answers a replayed id with a refusal that moves nothing,
    /// so a duplicated datagram pays once (the dig pays-once law's
    /// crafting twin).
    CraftRequest {
        req_id: u64,
        ingredients: Vec<(String, u8)>,
        output: String,
        output_count: u8,
        qty: u32,
    },
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
    /// THE YIELD-GRANT LAW (protocol v5): the canonical yield of the
    /// editor's accepted MINE, delivered to the editor ALONE. The server
    /// — not the client — is the granter of mined rewards: a rejected op,
    /// a place, a simulation edit, or an already-air cell never grants.
    ItemGrant { items: Vec<(String, u8)> },
    /// THE CRAFT-VERDICT LAW (protocol v7): the canonical answer to a
    /// [`ClientMessage::CraftRequest`], delivered to the crafter ALONE —
    /// the server, not the client, decides what a bench makes. THE
    /// VERDICT IS THE DELTA: a grant carries exactly what the ledger
    /// consumed (`consumed`, per-item totals in u32 — a large batch
    /// exceeds u8) and exactly what it produced (`output` =
    /// `output_count × qty`); the client applies both when the verdict
    /// lands. A refusal carries the reason (a spec the book does not
    /// name, a short ledger, no room, a replayed id) and moves nothing.
    /// A lost verdict errs safe: the delta never reaches the pack, and
    /// the next PackSync re-claim removes the ledger-side remainder — a
    /// craft can be lost in flight, never fabricated.
    CraftVerdict {
        req_id: u64,
        granted: bool,
        consumed: Vec<(String, u32)>,
        output: Option<(String, u32)>,
        reason: Option<String>,
    },
}

/// THE MINE CLAIM (protocol v5): rides a `SetBlock` that is a player
/// MINED dig. `held` is the item id in the digging hand — `None` = a bare
/// hand, which still claims the mine (a bare-handed dig of a no-tool
/// block is a legal yield, online and off).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MineClaim {
    pub held: Option<String>,
}

/// THE PLACE-PAYMENT CLAIM (protocol v8): rides a `SetBlock` that is a
/// player ITEM PLACEMENT. `item` names the item whose consumption paid
/// for the block — the server verifies the item can place that block and
/// that the ledger holds one, then consumes it. Payment is the point:
/// a placed block that cost nothing is world content fabricated from
/// nothing, and since the yield law (v5) pays canonical drops for mined
/// blocks, a free placement would mint items through the server's own
/// grant (place ore, mine it, collect the drop).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlaceClaim {
    pub item: String,
}

/// v8: the place-payment round trip and the join-mode claim. Server and
/// client ship together; the existing version gate rejects mismatched
/// peers.
pub const PROTOCOL_VERSION: u32 = 8;

/// THE PACK-SYNC CADENCE: a drifted pack is uploaded at most this often
/// (the client's PackMirror enforces it), except after a server-side
/// delta lands (a grant, an escrow move), which forces the client's
/// next upload immediately. The stale-upload clobber window (an upload
/// built before the client saw the delta) is then bounded by one round
/// trip and self-heals: an upload can only ever REMOVE server-known
/// deltas from the ledger, never add phantom items, so the window errs
/// safe — a brief undercount mis-refuses a gate, never mis-admits one.
pub const PACK_SYNC_MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);

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
        assert_eq!(PROTOCOL_VERSION, 8);
    }

    /// v7: the craft round trip — the request names the recipe spec and a
    /// client-chosen req id; the verdict carries exactly what the ledger
    /// produced (u32: a large batch exceeds u8) or the refusal's reason.
    #[test]
    fn craft_request_and_verdict_round_trip() {
        let request = ClientMessage::CraftRequest {
            req_id: 42,
            ingredients: vec![("log".into(), 1)],
            output: "planks".into(),
            output_count: 4,
            qty: 64,
        };
        assert_eq!(
            ProtocolCodec::decode_client(&ProtocolCodec::encode_client(&request)),
            Some(request),
            "the craft request survives the wire"
        );
        let granted = ServerMessage::CraftVerdict {
            req_id: 42, granted: true,
            consumed: vec![("log".into(), 2)],
            output: Some(("planks".into(), 256)),
            reason: None,
        };
        assert_eq!(
            ProtocolCodec::decode_server(&ProtocolCodec::encode_server(&granted)),
            Some(granted),
            "a granted verdict carries the ledger's exact delta"
        );
        let refused = ServerMessage::CraftVerdict {
            req_id: 43, granted: false, consumed: vec![],
            output: None, reason: Some("missing log (need 2, have 1)".into()),
        };
        assert_eq!(
            ProtocolCodec::decode_server(&ProtocolCodec::encode_server(&refused)),
            Some(refused),
            "a refusal carries its reason"
        );
        assert_eq!(PROTOCOL_VERSION, 8);
    }

    /// v6/v8: the mine claim, the place-payment claim, and the pack sync round-trip.
    #[test]
    fn mine_claim_item_grant_and_pack_sync_round_trip() {
        let tool_mine = ClientMessage::SetBlock {
            x: 3, y: 70, z: -4, block: 0,
            mine: Some(MineClaim { held: Some("stone_pickaxe".into()) }),
            place: None,
        };
        assert_eq!(
            ProtocolCodec::decode_client(&ProtocolCodec::encode_client(&tool_mine)),
            Some(tool_mine),
            "a tool mine claim survives the wire"
        );
        let bare_mine = ClientMessage::SetBlock { x: 3, y: 70, z: -4, block: 0, mine: Some(MineClaim { held: None }), place: None };
        assert_eq!(
            ProtocolCodec::decode_client(&ProtocolCodec::encode_client(&bare_mine)),
            Some(bare_mine),
            "a bare-handed mine still claims the mine"
        );
        let sim = ClientMessage::SetBlock { x: 3, y: 70, z: -4, block: 2, mine: None, place: None };
        assert_eq!(
            ProtocolCodec::decode_client(&ProtocolCodec::encode_client(&sim)),
            Some(sim),
            "a simulation edit carries no claim"
        );
        let paid_place = ClientMessage::SetBlock {
            x: 3, y: 70, z: -4, block: 2,
            mine: None,
            place: Some(PlaceClaim { item: "stone".into() }),
        };
        assert_eq!(
            ProtocolCodec::decode_client(&ProtocolCodec::encode_client(&paid_place)),
            Some(paid_place),
            "the place-payment claim survives the wire"
        );
        // v8: the join-mode claim rides Hello.
        let hello = ClientMessage::Hello { name: "smith".into(), protocol_version: PROTOCOL_VERSION, creative: true };
        assert_eq!(
            ProtocolCodec::decode_client(&ProtocolCodec::encode_client(&hello)),
            Some(hello),
            "the creative join claim survives the wire"
        );
        let grant = ServerMessage::ItemGrant { items: vec![("stone".into(), 1), ("raw_iron".into(), 2)] };
        assert_eq!(
            ProtocolCodec::decode_server(&ProtocolCodec::encode_server(&grant)),
            Some(grant),
            "the yield grant survives the wire"
        );
        let sync = ClientMessage::PackSync {
            items: vec![("wood".into(), 256), ("stone_pickaxe".into(), 1)],
        };
        assert_eq!(
            ProtocolCodec::decode_client(&ProtocolCodec::encode_client(&sync)),
            Some(sync),
            "the pack claim survives the wire (u32 counts: a pack holds >255 of one item)"
        );
        assert_eq!(PROTOCOL_VERSION, 8);
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
            ClientMessage::Hello { name: "zari".into(), protocol_version: PROTOCOL_VERSION, creative: false },
            ClientMessage::Position { pos: [1.0, 65.0, 2.0], yaw: 0.5, pitch: -0.1 },
            ClientMessage::SetBlock { x: -3, y: 70, z: 12, block: 2, mine: None, place: None },
            ClientMessage::Chat { text: "hello world".into() },
            ClientMessage::PackSync { items: vec![("wood".into(), 3)] },
            ClientMessage::CraftRequest {
                req_id: 1, ingredients: vec![("log".into(), 1)],
                output: "planks".into(), output_count: 4, qty: 2,
            },
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
            ServerMessage::ItemGrant { items: vec![("stone".into(), 1)] },
            ServerMessage::CraftVerdict {
                req_id: 7, granted: false, consumed: vec![],
                output: None, reason: Some("short".into()),
            },
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
