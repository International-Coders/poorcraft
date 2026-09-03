# Multiplayer and Steam Authority

## Player promise

Hosting is a button, joining a friend is a reliable flow, and every player sees
the same consequential world. Water, wheels, machines, NPCs, mobs, companions,
inventories, quests, faction knowledge, castle alarms, and edits have one
authoritative outcome. Disconnect and reconnect do not duplicate or erase it.

## Core decision

Build one transport-neutral authoritative host. Singleplayer connects to it
in-process; LAN/direct internet uses UDP; Steam uses
ISteamNetworkingSockets/lobbies. The protocol and simulation do not branch on
the transport beyond identity and delivery capabilities.

## Authority matrix

| State/action | Authority | Client behavior |
|---|---|---|
| movement | host validation with client prediction | predict local motion, reconcile |
| block break/place | host | show pending feedback, apply accepted delta |
| inventory/crafting/trade | host | send transaction, render receipt/rejection |
| survival/combat/drops | host | interpolate/predict cosmetics only |
| fluids/wheels/machines/power | host | interpolate surfaces/animation |
| mobs/NPCs/companions | host | interpolate poses and present intent |
| quests/reputation/knowledge | host | display reasoned events |
| castle/settlement state | host | render replicated summary/deltas |
| UI layout/audio particles | client | derived from authoritative events |

## Protocol shape

Replace one undifferentiated message stream with explicit channels:

- reliable ordered: handshake, commands, inventory, quests, chat, trades,
  entity lifecycle, settlement events, block/fluid baselines;
- unreliable sequenced: player/entity transforms, look, animation parameters;
- reliable unordered or chunked: world sections, fluid-region baselines,
  manifests, large resync payloads;
- local/in-process: same typed messages without serialization when safe, with
  serialization parity tests retained.

Every packet or logical message carries protocol version, session, channel,
sequence/ack as applicable, and bounded length. Commands carry actor and
monotonic command ID. Deltas carry base/version so stale application is
detectable.

## Handshake

Before entering the world, exchange:

```text
protocol version
game/build version and content hash
WorldIdentity and save schema
required mod IDs, versions, deterministic fingerprints, asset availability
player platform identity and display name
transport/session capabilities
server rules and permission role
```

Mismatch is a clear actionable rejection. Never let mismatched worldgen or
mods quietly create divergent terrain.

## Interest management and replication

- Track player interest by chunk plus system-specific halos.
- Send chunk baselines, then ordered block and block-entity deltas.
- Replicate fluid-region baselines only for active/nearby regions and sparse
  versioned deltas afterward.
- Spawn/despawn entities with stable IDs; transforms reference a known spawn
  generation.
- Send NPC intent/activity/pose at a lower rate than player transforms and
  reliable events for speech, damage, inventory, memory-visible outcomes, and
  alarms.
- Settlement far-state replicates as small summaries until the player enters.
- A reconnect can request version gaps or a fresh baseline without replaying
  all historical edits.

## Transactions and validation

- Craft, trade, loot, equip, machine input/output, quest rewards, companion
  hire/wages, and faction changes are atomic host transactions.
- The current peer trade path must reserve and verify both inventories on the
  host before resolving. A client response cannot mint an item.
- Block commands validate reach, tool/action state, collision, permissions,
  protected structure cells, and expected old state.
- Movement validation is tolerant of latency but rejects impossible speed,
  flight, or penetration under current game mode.
- The host rate-limits chat, edits, requests, and expensive path/fluid triggers.

## Steam player flow

### Host

1. Choose world and visibility.
2. Start authoritative host.
3. Create lobby with build/content hash, host identity, capacity, joinability,
   and world display metadata—never sensitive save data.
4. Accept Steam Networking Sockets sessions and run the same handshake.
5. Enable overlay invites and `Join Game` routing when launched through Steam.

### Join

1. Accept invite or select a lobby/friend.
2. Resolve host identity and establish Steam P2P session.
3. Run content/world handshake; show download/mismatch actions.
4. Load initial snapshot with progress and cancellation.
5. Spawn only after authoritative readiness.

### Fallbacks

- Direct-connect UDP remains first-class for non-Steam builds and diagnostics.
- If Steam initialization fails, explain the fallback; never silently show a
  lobby button that cannot work.
- A dedicated server can advertise through Steam only when supported and can
  still accept the configured direct transport.

## External dependencies and honest limits

A real partner App ID, two licensed Steam accounts/machines, store/lobby
configuration, overlay launch, achievements, cloud saves, and public discovery
cannot be proven with Spacewar self-connection alone. Code-level adapters and
loopback tests are necessary but not sufficient. Maintain a dated two-account
manual test matrix and do not label Steam complete until it passes.

Host migration is post-beta unless evidence shows it is essential. Beta must
save safely and notify clients if the host leaves.

## Security and resilience

- Treat all remote inputs as untrusted and length-bound before allocation.
- No path traversal in mod/UGC/save transfer.
- No remote code mods; the current data-only mod boundary remains.
- Avoid panics on malformed packets; log a bounded reason and disconnect.
- Add heartbeat, timeout, graceful leave, reconnect token, and duplicate
  command suppression.
- Persist authoritative snapshots and an optional bounded event journal so a
  crash cannot leave half a transaction.

## Tests

- identical command script through in-process and UDP hosts yields the same
  snapshot hash;
- packet reorder/drop/duplicate simulation converges or requests resync;
- malformed/oversized/version-mismatched frames reject without panic;
- two players share block, inventory, craft, trade, machine, water, mob, NPC,
  quest, reputation, and castle-alarm outcomes;
- reconnect during each transaction class neither loses nor duplicates state;
- interest exit/re-entry restores exact entity/fluid/settlement versions;
- mod/content mismatch is refused before world entry;
- Steam adapter codec parity and callback lifecycle are automated where the
  SDK permits;
- manual two-account matrix covers host, invite, join, play, disconnect,
  reconnect, host exit, overlay, and a 30-minute mixed-system session.

## Beta gate

The beta multiplayer claim requires at least one real two-machine or
two-account session using a non-loopback transport, plus the deterministic
cross-transport suite. "Steam feature compiles" and "lobby created" remain
valid evidence, but they are not the product gate.
