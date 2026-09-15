# CHANGELOG

## 2026-09-15 — The pack is the server's ledger: server-side per-player inventories over real UDP (loop 466)

- Closed STATE's next_task item (2) — the loop-465 deferral "the server
  holds no canonical inventories yet (crafting/smelting/trade sourcing
  stay client-held)". Chosen over the windowed two-client route per the
  priority ladder: an authority gap ranks above a missing proof (the
  465 precedent). Reading the code surfaced two LIVE holes in the ungated
  P37 escrow: a phantom offer COMPLETED (no validation at all — offer
  what you don't own and both sides gain), and a SELF-TRADE DUPLICATED
  items (offer 1, want 1, accept → +2, nothing paid). A third party
  could also dissolve any standing offer by accepting it.
- THE WIRE (lf_protocol, v6): `PackSync { items: Vec<(String, u32)> }` —
  the client's claim of its own pack, aggregated (slot layout is
  presentation, the ledger is contents; u32 counts because a pack holds
  more than 255 of one item). Sent on join and on drift, cadence-limited
  (250 ms) and FORCED by every server-side delta (an ItemGrant, an
  accepted trade) so the ledger is re-claimed WITH the delta inside one
  round trip — a stale upload can only ever REMOVE server-known deltas,
  never add phantom items, so the clobber window errs safe.
  PROTOCOL_VERSION 5 -> 6 (matched binaries; the existing gate rejects
  mismatched peers).
- THE SERVER (lf_server): one canonical ledger per player
  (`HashMap<u64, Inventory>` — the same lf_game 36-slot law), created at
  Hello, rebuilt from each PackSync through `add_item` (an oversized
  claim is truncated at the pack's own law, never counted), paid by
  every accepted mine's ItemGrant (overflow is the block's spill — on
  the ground, not held), dropped at Goodbye. THE OFFER GATE: an offer
  whose `give` exceeds the ledger is refused to the offerer alone while
  the target hears nothing. THE ESCROW (new pure fn): an accept
  completes only when BOTH ledgers can pay — offerer holds give,
  accepter holds want, each receipt fits (removes-first trial on
  clones, so payment frees room) — then both move exactly; ANY failure
  moves nothing and names the reason to the failing side. Only the true
  target may complete an offer now; a third party's accept resolves
  nothing.
- THE CLIENT (lf_client::net): the PackMirror — the last uploaded
  claim, drift-detected per frame against the live pack's aggregated
  sorted snapshot; the ONE sender is `NetClient::sync_pack`, ticked
  once per frame outside the poll borrow; the ItemGrant and
  accepted-TradeResolved arms force the re-claim; and the server's
  `Reject` finally reaches the player as a hint instead of silence.
- 8 NEW LAWS (root 509 -> 517; lf_server 7 -> 12, lf_client 104 -> 107,
  lf_protocol same count): the escrow unit law (both ledgers or
  neither; payment frees the receipt's room); the pack-sync + grant
  wire law (the mined stone is gate-visible though never uploaded;
  over-offers refuse; the target never hears a phantom offer); the
  escrow-moves-both-ledgers wire law (proven through the gate alone:
  paid-away goods refuse re-offers, received goods admit them); the
  failed-accept-dissolves-without-moving wire law (the re-offer passes
  = atomicity); the self-trade + third-party wire law; and three
  client laws (the bootstrap claim — an empty pack is still claimed;
  the aggregation law — split stacks merge, sorted, u32; the source
  law — one sync tick, delta arms force, Reject hints).
- REGRESSION: cargo test --workspace 517 green / 0 failed (xtask's 12
  included; = loop 465's 509 + 8); make smoke OK; the FULL vistest
  battery 110 scenes [ok] / 0 FAIL exit-0, every committed PNG
  byte-identical across TWO runs (md5 fdcf3096f93694fa396aa7576a04fa7b
  before == after) — singleplayer render paths pixel-proven unchanged.
  Runtimes refreshed: dist/loreforge-macos.dmg + linux tarball + .app +
  server, fresh on disk; Windows exe honestly skipped (mingw absent);
  POORCRAFT 3D untouched.
- PERF: one small Vec build+compare per frame while connected (41
  slots) and one sub-KB PackSync per drift (rate-limited to 4/s, forced
  only on server deltas); the server adds one `add_item` per grant and
  two clone-trials per accept; zero singleplayer cost (the mirror only
  runs when net is Some).
- LORE: canon touched: none — multiplayer economy plumbing; the ledger
  gates existing items and drops; no faction, place, event, term, NPC,
  item, or spell data changed; no new canon text. Canon preserved: the
  chronicle as the player-authored history; Anima as a material
  energetic property; no identity assigned. World expression: in shared
  Valdenmoor the realm's ledger knows what each adventurer carries — a
  trade offers only what the ledger holds, both packs move together or
  not at all, and a refusal says why.

## 2026-09-15 — The yield is the server's to give: server-side dig-yield authority over real UDP (loop 465)

- Closed STATE's next_task item (1)'s higher-severity half — the loop-462
  deferral "server-side inventory authority (the corrective echo reverts
  the BLOCK, not an optimistic item take)". Chosen over the windowed
  two-client route per the priority ladder: an authority gap ranks above
  a missing proof. Until now a multiplayer dig paid whatever the client
  ASSUMED: the block's yield popped locally before the server had said
  anything, so a rejected op (an unknown mod block), a re-sent packet,
  or a peer who mined the same block first all paid phantom items.
- THE WIRE (lf_protocol, v5): `SetBlock` gains `mine: Option<MineClaim>`
  — `Some` iff the edit is a player MINED dig, carrying the honest held
  item id (`None` inside = a bare hand, which still claims: a bare-handed
  dig of a no-tool block pays online exactly as offline); places,
  machine/fluid/falling/console/server-mirror edits claim nothing. New
  `ServerMessage::ItemGrant { items }` — the canonical yield, delivered
  to the editor ALONE. `PROTOCOL_VERSION` 4 -> 5 (server and client ship
  together; the existing version gate rejects mismatched peers).
- THE SERVER (lf_server, new lf_game dependency): on an accepted mine the
  server reads what the block WAS before its canonical world changes,
  gates the claim through `mining::tool_satisfies`, and pays
  `items::block_drop` — the SAME harvest law and drop table the client
  plays, one source (mods included: the server loads mods/ and resolves
  mod-block drops through the same registry). The grant never rides to
  peers.
- THE CLIENT (lf_client): `host_set_block` computes the claim
  (`net::mine_claim_for`) — and is THE ONE BROADCASTER. `break_block_drops`
  — the one path every canonical block yield spawns through — now spawns
  only OFFLINE (`net.is_none()`); online the yield ARRIVES via ItemGrant
  and lands in the pack with the trade-escrow overflow-spill pattern
  (nothing vanishes on a full pack). The scaffold column yields through
  `break_block_drops` (its direct spawn was ungated) — per-cell server
  grants match singleplayer. Felled-tree cells are THE TIMBER'S, not the
  hand's (`EditKind::Mine` -> `Falling`): singleplayer never paid felled
  cells as drops, so online they must claim nothing or the server would
  grant a log per trunk cell. The leaves-apple bonus is offline flavor
  (the server owns no RNG yet). The keeper's waking and the twin's song
  stay unconditional — reactions, not items.
- EN-ROUTE WIRE WART FIXED: the mine-break path sent its dig a SECOND
  time after the funnel had already broadcast — every dig went on the
  wire twice and double-entered the server's edit history (the newcomer
  replay dug it twice). Removed; the one-broadcaster source law pins it.
- 7 NEW LAWS (root 490 -> 497 in-workspace + xtask 12 = 509 total;
  lf_protocol 5 -> 6 tests, lf_server 4 -> 7, lf_client 101 -> 104): the
  claim/grant round-trip law; THE YIELD-GRANT wire law (a mine pays the
  canonical yield to the editor alone, a place never pays anyone, the
  peer never sees a grant); THE HARVEST GATE HOLDS SERVER-SIDE (iron ore:
  a wooden pick's mine pays nothing, a stone pick pays, a bare hand fails
  the gate — staged blocks at y=200 so no seed decides the outcome); A
  DIG PAYS ONCE AND REJECTED OPS NEVER PAY (a re-sent mine into
  already-air, an unknown-id op answered by the corrective echo only);
  the mine-claim unit law (only Mine claims; the claim is the honest
  hand); the one-broadcaster source law; the no-optimistic-take source
  law (every spawn in break_block_drops behind the offline gate + the
  scaffold routing + the ItemGrant arm's overflow spill).
- REGRESSION: cargo test --workspace 509 green / 0 failed + cargo test
  -p xtask 12 green (included; = loop 464's 502 + 7); make smoke OK; the
  FULL vistest battery 110 scenes [ok] / 0 FAIL, exit-0, and every
  committed vistest PNG byte-identical on disk — the singleplayer render
  paths are pixel-proven unchanged this loop (the 462/461 "visual gates
  not run" precedent made into evidence). Runtimes refreshed:
  dist/loreforge-macos.dmg 8.8 MB (hdiutil VALID) + linux tarball 8.4 MB
  + .app binary 20 MB + server, fresh on disk; Windows exe honestly
  skipped (mingw absent); POORCRAFT 3D untouched.
- PERF: a small net WIN on the multiplayer wire — one redundant SetBlock
  per dig removed (halves mine-edit traffic and the server's edit-history
  growth); the grant adds one small message per accepted, tool-legal
  mine; zero singleplayer cost (every new path sits behind
  `net.is_some()`/`is_none()`).
- LORE: canon touched: none — multiplayer economy plumbing; the granted
  items are the existing canonical drops (anima_crystal's Covenant
  world-source unchanged); no faction, place, event, term, NPC, item, or
  spell data changed; no new canon text. Canon preserved: the chronicle
  as the player-authored history (the twin song and the keeper's waking
  are reactions, not grants); Anima as a material energetic property; no
  identity assigned. World expression: in shared Valdenmoor a mined vein
  pays once, to the miner whose swing the realm accepted — the harvest
  law, not each adventurer's say-so.
- Migration: PROTOCOL_VERSION bump only (matched client+server binaries;
  old peers rejected by the existing gate) — no save format, no
  block/item change, GENERATOR_VERSION unchanged, ClientSave untouched.
- Deferred honestly: the server does not yet HOLD canonical inventories
  (crafting/smelting/trade sourcing stay client-held; the grant covers
  mined yields — server inventory state rides the integrated-server
  tier); the leaves-apple bonus is offline-only (no server RNG); the
  timber landing plan's blocked-cell drops and container spills (chest
  contents) stay client-local — the sim/block-entity sync tier; mob drops
  client-local (pre-existing); the carried list — the windowed
  two-client route (the GPU-side end of the wire laws; GameState owns a
  real window+surface, so the route needs a headless driver slice of its
  own); hardcoded connect name "smith" (audit note).

## 2026-09-15 — The resonance compass + the remembered song + the bearing law (loop 464)

- Closed STATE's next_task item (2) — the resonance depth pass ("a held
  anima_crystal that points at its twin, and/or persisting the twin
  dedupe with the save"). Both halves shipped, plus one proof-found
  bug fixed.
- THE HELD CRYSTAL (lf_worldgen + lf_client): holding an Anima crystal
  raises a violet resonance dial under the crosshair — the compass
  rose geometry the kingdom compass uses, rimmed violet with a pale
  violet needle, refreshed at the compass cadence (at most every 20
  held frames). `WorldGen::resonance_target` answers what the crystal
  sings: the deep's resonance is loudest nearest — standing in a
  rolled hollow's chunk, the crystal hears THAT hollow and answers its
  TWIN (`is_twin`, pinned by law to equal `geode_twin_center`, the
  same promise the first-take Discovery sang); anywhere else a bounded
  Chebyshev ring scan (`RESONANCE_SCAN_CHUNKS` = 16, one hash per
  chunk, stable ring-order ties) answers the NEAREST rolled hollow's
  center; `None` reads "the deep is silent" — a crystal senses the
  deep near it, it is not a realm oracle. The label rides
  `map::resonance_label`: "its twin sings · north-east, 640 paces" /
  "a hollow hums · east, 30 paces".
- THE BEARING LAW (loop's proof-found bug, fixed before committing):
  every held dial now shares ONE bearing function,
  `map::bearing_to(dx, dz) = dx.atan2(-dz)` — the game's own
  convention the `compass_facing` words read (yaw 0 looks -z, called
  S). The kingdom compass's old `dx.atan2(dz)` MIRRORED the needle
  across the player's east-west line since loop 345 — a kingdom dead
  ahead read as behind. The 8-wind law + the painter-relation law fail
  on the old code; `kingdom_compass_readout` delegates to
  `bearing_to`, so the two dials can never disagree.
- THE SONG IS REMEMBERED: the twin-song dedupe persists with the save
  (`ClientSave.chronicled_geodes`, serde-default, saved sorted) —
  `load_world` restores it AFTER the `restart_streamer` chokepoint
  cleared the session set, so the chokepoint semantics survive (new
  worlds and join-identity still reset it; their seeds differ). A
  reload never re-sings a hollow the player has heard.
- NEW PROOF SCENE: `resonance_compass_hud` — the REAL painter over the
  REAL WorldGen reading (seed 3, a stand-point 2.5 chunks off a
  genuinely rolled hollow; nothing staged). Pixel gates: violet rim +
  needle presence, and the bearing law in pixels — the needle pixels'
  centroid must sit on the reading's bearing side along its dominant
  axis (the first staging's fixed left/right split FAILED on a
  near-vertical needle and was rebuilt: gate-found, gate-fixed).
  INSPECTED: violet dial top-center, needle on the reading's side,
  label legible. `kingdom_compass_hud` re-render INSPECTED
  pixel-faithful after the dial refactor (`paint_compass_dial` is the
  one geometry source; the gold/red gates unchanged).
- REGRESSION: root cargo test --workspace 490 green / 0 failed
  (35 suites) + cargo test -p xtask 12 green = 502 total (= loop
  463's 497 + the 5 new laws: lf_worldgen 52 -> 54 — the own-hollow
  twin law, the scan law (order-free brute-force nearest + a real
  bound + replay); lf_client 98 -> 101 — the bearing law, the
  resonance-label law, the twin-song persistence law); make smoke OK;
  the FULL vistest battery 110 scenes [ok] / 0 FAIL, exit-0 (109 +
  resonance_compass_hud); runtimes refreshed (dist/ dmg + linux
  tarball + .app binary + server); Windows exe honestly skipped (mingw
  absent); POORCRAFT 3D untouched.
- PERF: not applicable — the dial refresh is cadence-gated (one
  WorldGen::new + at most 33x33 one-hash ring probes every 20 held
  frames, ~3/s); the persistence is one small sorted Vec per save;
  zero per-frame work when nothing is held.
- LORE: canon touched: the twin law extends the established Old Powers
  geodes (446/447/463) within canon — sensing is expressly allowed
  ("Anima can be sensed widely"), the dial is a resonance bearing, not
  a miracle; no faction, place, event, term, NPC, item, or spell data
  changed; the dial labels reuse the Discovery's voice, no new canon
  text. Canon preserved: Anima as a material energetic property;
  non-omniscience (bounded scan, honest silence); the chronicle as the
  player-authored history (the song is history, not a re-firing log).
- Migration: one serde-default ClientSave field (old JSON extras load;
  the legacy bincode path fills empty) — no save format break, no
  block/item change, GENERATOR_VERSION unchanged (`resonance_target`
  only READS the roll; generation output identical).
- Deferred honestly: the crystal does not remember WHICH hollow a
  stack came from (per-stack origin needs an ItemStack metadata
  migration — the reading is the nearest hollow / own-chunk twin
  instead, honest and metadata-free); the dedupe restore is load-path
  only by design (join-identity resets — a server's hollow layout
  differs); no in-world visual marker AT the twin site yet; the
  carried list — a windowed two-client route or server-side inventory
  authority (462's unblocked multiplayer follow-ups, now next_task 1).

## 2026-09-15 — The geode twin law + the feature seed law (loop 463)

- Closed STATE's next_task item (1) — "geode pairing", carried since
  loop 447 ("geodes are the Covenant's anima_crystal world source;
  pairing = a discoverable narrative/geometric link worth a proof").
- THE TWIN LAW (lf_worldgen): the Old Powers hollows are mirror PAIRS.
  `geode_twin_chunk` is the point reflection of a chunk through the
  corner plane at the realm's heart (x = z = -0.5); the pair rolls ONCE
  (`geode_pair_representative`, involutive) and answers BOTH halves —
  the twin at the mirrored local center (15-lx, 15-lz), same depth,
  same radius. Every hollow's twin is real, mutual, and computable;
  `geode_twin_center` reads the twin's world position as the twin
  chunk itself answers it. Rarity is per PAIR (one in ~113), so the
  hollow density per chunk is unchanged. 3 new worldgen laws: the twin
  sweep (involutivity + exact mirror + band laws + world-position
  round trip over -32..=32), the twin hollow sealed and crystal-lined
  like its twin (both halves stamped + BFS), and the feature-seed law
  below.
- THE RESONANCE (lf_client): the first Anima-crystal take from a
  hollow records a chronicle Discovery — "the crystal sings across the
  dark — a twin hollow waits to the {compass}, {n} paces off". The
  bearing rides the game's own `compass_facing` convention
  (map::geode_twin_line; 8-wind unit law incl. south = -z, east = -x);
  the dedupe is once per hollow per session (`chronicled_geodes`,
  cleared at the `restart_streamer` chokepoint — new world, load, and
  the join-identity law all reset it). The twin's own first take sings
  the bearing back.
- THE PROOF-DISCOVERED BUG, fixed before committing: the new
  `geode_twins` vistest scene demanded a real pair near the realm's
  heart and none existed in 2000 seeds — because
  `seed_for_features()` sampled gradient noise at (0.0, 0.0), where
  the value is 0 for EVERY seed. The feature key had been constant
  since P2: every feature hash (trees, ground cover, structures,
  citadels, oil, geodes) was seed-INDEPENDENT — every world laid its
  forests and ruins at the same coordinates, hidden under
  terrain-noise variance. THE FIX: the probes moved off the gradient
  lattice ((0.5, 13.7) / (91.3, 0.5)); the feature-seed law pins
  distinct keys, distinct geode maps, and distinct tree maps across
  seeds with one-seed replay. The accord-bastion structure law was
  rewritten consciously multi-seed (the candidate lottery is now a
  seed property; ruins verified generating on seeds 31/4242/5 before
  the rewrite).
- GENERATOR_VERSION 7 -> 8: unedited chunks regenerate with the new
  generator on revisit (the identity mechanism's named contract);
  edited chunks are persisted and never regenerated.
- NEW PROOF SCENE: `geode_twins` — a REAL near-heart pair from a
  deterministic seed search, both pockets stamped by the shared pure
  geometry and opened onto a dug gallery, one keeper in each; the
  pixel gate requires the crystal violet in BOTH halves (left 2340 /
  right 9718) and the render was INSPECTED: two crystal-lined mirror
  hollows, keepers visible, violet glow on the gallery stone.
- REGRESSION: root cargo test --workspace 485 green / 0 failed
  (35 suites) + cargo test -p xtask 12 green = 497 total (= loop
  462's 493 + the 4 new laws: lf_worldgen 49 -> 52, lf_client
  97 -> 98); make smoke OK (headless logic + GUI liveness); the FULL
  vistest battery 109 scenes [ok] / 0 FAIL exit-0 (108 + geode_twins;
  river_valley + kingdom_citadel re-renders INSPECTED — features
  moved per seed, the scenes read true); runtimes refreshed (dist/
  dmg 8.8 MB UDZO verified + linux tarball 8.4 MB + .app binary 20 MB
  + server, fresh on disk 11:34); Windows exe honestly skipped (mingw
  absent); POORCRAFT 3D untouched; the six windowed_wild_*.png
  dirties remain deliberately NOT staged (an earlier session's
  p3d-wilderness run).
- PERF: not applicable — the pair law is two integer compares per
  chunk roll at generation time; the twin query is press-gated (one
  WorldGen::new + two hashes per crystal take); seed_for_features
  samples the same two noise points it always did, now off-lattice;
  zero per-frame work.
- LORE: canon touched: the twin law extends the established Old
  Powers geodes (446/447) within canon; one new chronicle Discovery
  line (the Discovery precedent); "the realm's heart" is engineering
  language, never asserted in game text. Canon preserved: Anima as
  material resonance (the twins' link is resonance and bearing, not
  miracle); the Covenant's anima_crystal world source unchanged; the
  chronicle as the player-authored history. World expression: the
  deep reads as one connected resonance — a miner who takes a crystal
  hears where its twin waits; and every world's forests, ruins, and
  hollows are its OWN (the seed finally shapes the Old Powers' works
  and the surface's woods alike). Migration: GENERATOR_VERSION bump
  only (no save schema, no proof schema change).
- HONESTLY DEFERRED: the twin dedupe is session-scoped (a reload
  re-sings a hollow — the chronicle event persists, the dedupe does
  not); the Discovery fires on the mining client only (chronicle
  authority is client-local, the same as drops — server chronicle
  rides the server-authority tier); no in-world twin compass yet (a
  held anima_crystal that points — future depth); the carried list —
  a windowed two-client route or server-side inventory authority
  (462's unblocked multiplayer follow-ups); hardcoded connect name
  "smith" (audit note).

## 2026-09-15 — The replay-window law: multiplayer terrain-edit routing made lossless (loop 462)

- Closed STATE's next_task item (1) — "multiplayer routing of terrain
  edits", carried since loop 450. THE AUDIT: the routing skeleton
  already existed (client host funnel -> SetBlock -> server
  validate/lazy-generate/edit history -> BlockUpdate broadcast ->
  client host funnel; newcomer replay on Hello; N05 seed adoption),
  but with three real defects.
- (D1) THE LOSSY REPLAY: World::set_block refuses edits for a missing
  chunk (the host records a reject and moves on), and the server sends
  the newcomer replay the moment it says Welcome — while the client's
  streamer is still generating chunks. Every replayed edit outside the
  already-meshed ring silently vanished; stream_chunks even documented
  the assumption ("Re-apply nothing: generated columns are pristine").
  A second client only ever saw edits in chunks it happened to have
  meshed at join time.
- (D2) THE OWN-ECHO: the server broadcast every accepted edit to ALL
  players INCLUDING the editor, whose optimistic apply then ran a
  second time — a duplicate host event for one player action plus a
  redundant relight/remesh of the column.
- (D3) THE SILENT REJECT: a rejected op (unknown block id — e.g. a
  client mod the server has not registered) left the optimistic editor
  diverging from the canonical world with no word.
- THE LAWS. Replay window (lf_client::net::RemoteEditBuffer): remote
  edits for chunks that have not streamed in buffer per chunk —
  bounded (16384, FIFO eviction of the oldest chunk's queue whole,
  eviction counted) — and the streamer flushes a chunk's queue
  oldest-first in the server's history order the moment the chunk
  arrives, BEFORE the column meshes (both chunk-insert sites: saved
  loads and fresh generation). A source law pins flush-at-every-
  non-test-chunk-insert (it caught the new Welcome ring itself en
  route). No-self-echo + corrective echo (lf_server): peers receive
  the update, the editor never hears its own accepted edit again, and
  a REJECTED op answers the editor ALONE with the server's true block
  at that position (the same lazy generation the accept path uses), so
  the optimistic world reverts; y-out-of-range stays silent because
  the client's own guard refused the optimistic apply too.
  Join-identity (client Welcome arm): adopting the server seed adopts
  its TERRAIN — world/meshes/block entities/mobs/drops/map reset, the
  boot ring regenerates synchronously from the server seed, the
  streamer restarts clean, the replay window purges (the replay
  arrives right after Welcome); player session state (inventory,
  quests, chronicle, position) stays. Before this, a joiner walked a
  patchwork of the locally-seeded boot ring and server-seeded stream.
- 5 NEW LAWS (root 488 -> 493; lf_client 93 -> 97, lf_server 3 -> 4):
  the 3 buffer laws (history-order flush per chunk, oldest-whole
  eviction + the order invariant, mesher-partition chunk keys incl.
  negatives); the source law (every chunk insert flushes); the
  newcomer-history wire law (three edits across three chunks incl.
  far coords replay to a later joiner over real UDP). The P25
  registry test was rewritten as the no-self-echo + corrective-echo
  wire law (the old echo assertion enshrined D2).
- REGRESSION: root cargo test --workspace 493 green / 0 failed
  (35 suites); make smoke OK (headless logic + GUI liveness);
  runtimes refreshed (dist/ dmg 8.8 MB UDZO + linux tarball 8.4 MB +
  .app binary 20 MB + server, fresh on disk 07:12); Windows exe
  honestly skipped (mingw absent); visual gates NOT run —
  singleplayer renders byte-for-byte the same paths (the buffer only
  engages when net is Some; the flush is one empty-map check per chunk
  insert; the 458/459/461 precedent), and the multiplayer GPU client
  has no visual harness — the wire laws + buffer laws + source law are
  this slice's proof family.
- PERF: a small net win — one redundant host apply + relight + remesh
  removed per own accepted edit (the D2 cure); the buffer allocates
  only for edits that cannot yet apply; the flush amortizes into the
  chunk's first mesh; no new per-frame work.
- LORE: canon touched: none — netcode and join-identity
  infrastructure. World expression: a second adventurer in Valdenmoor
  sees the digs and placements as they happened, and arrives on the
  host's land rather than a patchwork of two seeds. Migration: none.
  Deferred honestly: geode pairing (447's keeper, now next_task 1);
  server-side inventory authority (the corrective echo reverts the
  BLOCK, not an optimistic item take); a windowed two-client route
  (the GPU-side end of the wire laws); hardcoded connect name
  "smith" (audit note).

## 2026-09-15 — The keeper's chronicle law: every true waking is a Discovery, staffed roosts never spam (loop 461)

- Closed STATE's next_task item (1) — loop 447's honest deferral:
  "the guardian's chronicle Discovery re-fires if a geode re-settles
  after despawn (dragon-precedent behavior)". The audit: the BEHAVIOR
  already matched the dragon precedent (each settle of an unstaffed
  anchor fired the Discovery, so a re-settle after the keeper
  despawned beyond the 80 m leash or fell in battle re-fired it), but
  the decision lived inline in three separate client settle sites
  with duplicated string literals and no law — nothing pinned the
  re-fire, the staffed no-spam, or the vermin exclusion.
- THE LAW (lf_game::mobs::settlement_chronicle): settling a keeper
  into its UNSTAFFED anchor is a chronicle Discovery — EVERY settle,
  including re-settles (the dragon precedent: each waking is an
  event in the player's authored history); a staffed roost wakes
  nobody; the cinder crawler's return is vermin, never chronicled.
  The client's settle passes (geode guardian, cinder crawler, dragon
  roost) all delegate, so the chronicle can never disagree with the
  spawn decision; the two Discovery lines now have a single source.
- NO BEHAVIOR CHANGE: the same lines fire from the same conditions;
  the literals moved verbatim into the law.
- 2 NEW LAWS (root 486 -> 488; lf_game 91 -> 93): every true waking
  is chronicled AND staffed roosts never spam (the re-fire asserted
  across consecutive unstaffed decisions); vermin returns (crawler,
  boar, glitchling, null knight, woolbeast — staffed or not) are
  never chronicled.
- REGRESSION: root cargo test --workspace 488 green / 0 failed
  (35 suites); make smoke OK; runtimes refreshed (dist/ dmg +
  tarball + .app binary, verified on disk); visual gates NOT run —
  nothing visual changed (the 458/459 bench-only precedent); the
  refreshed vistest battery from loop 460 still stands (this job
  renders nothing).
- PERF: not applicable — one match on two enums at settle time
  (frame-gated, at most one settle per pass).
- LORE: canon touched: the two existing Discovery lines ("crystal
  light stirs in the deep — a hollow's keeper wakes"; "wings circle
  the peaks — a dragon guards its clutch") moved VERBATIM into the
  law — no wording change, no new canon. Canon preserved: the
  chronicle as the player-authored history; the keepers' nature
  (guardians defend, crawlers are vermin; only wakings are events).
  World expression: the saga records every true keeper-waking —
  return visits to a hollow whose keeper fell read as the waking
  they are. Migration: none (no save format, no proof schema).
  Deferred honestly: the carried list — multiplayer routing of
  terrain edits; geode pairing.

## 2026-09-15 — The pass-routing law: the water channel is art identity, not atlas position (loop 460)

- Closed STATE's next_task item (1) — the is_water_layer/CTM-strip
  audit carried since loop 447 ("the hard-coded 167 now points at
  dead_shrub's index — water proofs pass but the strip addressing
  deserves an audit"). The audit CONCLUDED: the CTM strip addressing
  itself is sound (UV math, generator placement, the 47-tile table,
  and the filler slot all agree — now pinned by a law), but the WATER
  PASS ROUTING had drifted, with two live misroutings on the real
  mesh path.
- THE DRIFT: `is_water_layer` (lf_voxel::meshing) classified the
  water channel by atlas POSITION (`tex == 10 || tex == 167`). 167
  was water's CTM marker at loop 332, when markers were small ints;
  the marker scheme's move to the 4096 base happened precisely
  because real art grew into the low band — today 167 is
  dead_shrub's atlas layer. Consequences, both on the live
  `World::mesh_column` path used by the client AND the vistest
  harness: (a) every exposed water TOP face carried marker 4098 and
  routed to the OPAQUE pipeline (REPLACE blend, depth-write on) —
  water surfaces rendered fully opaque, the water art's alpha-170
  translucency lost on every connected top; (b) every dead-shrub
  face (167) routed to the WATER pipeline (alpha blend, depth-write
  off). Existing proofs passed because the water/decoration checks
  assert presence and hue, not translucency.
- THE LAW (lf_voxel::meshing): THE PASS-ROUTING LAW — a vertex rides
  the water pass iff its tex_index is WATER ART: the water base
  layer (`WATER_BASE_LAYER`, named; mirrors lf_assets'
  `layer_of("water")`) or a CTM marker whose strip slot is water's
  (derived from the same mirror table the mesher uses to STAMP the
  marker, so routing can never disagree with stamping). Atlas
  position is never identity. `world::WATER_TEX_LAYER` derives from
  the mesher const (single source). `ctm_marker_for` + `ctm_tile_
  uvs` are public: lf_assets (which depends on lf_voxel, never the
  reverse) pins the one-way mirror.
- 6 NEW LAWS (root 480 -> 486; lf_voxel 49 -> 52, lf_assets 21 ->
  24): the real 3x3 water mesh routes EVERY vertex — stepped flow
  sides AND the 36 marker-carrying top verts — to the water pass
  (this law FAILS on the old predicate); the drift witnesses
  (165/166/167/168, grass_top, stone) never ride the water pass;
  marker space routes by strip SLOT (only slot 2; unassigned markers
  refuse); the whole-atlas sweep — for every named layer,
  is_water_layer answers true for exactly "water"; every CTM block
  marker routes iff its art is water; the lf_voxel mirror matches
  CTM_BLOCKS row-for-row and the water base layer equals the named
  layer; THE STRIP-ADDRESSING LAW — the mesher's per-tile UV rect
  lands exactly on the rect generate_ctm_strip_atlas painted for the
  same (slot, tile), over all 8 blocks x 47 tiles.
- PLAYER-VISIBLE, measured and inspected: water_flow before/after
  (fresh same-seed renders): the pool surface region's distinct
  colors 750 -> 1926 over 11k samples, green-channel variance
  142 -> 156 — submerged terrain now shows through; before the
  surface read as hard-edged opaque tiles, after as one connected
  translucent sheet (INSPECTED at 1.5x zoom). first_person_view and
  hud_preview changed ~50% of pixels — half the frame is a lake:
  before a flat opaque slab, after the submerged shoreline reads
  through the surface (HUD itself unchanged; pairs INSPECTED).
  transparency_layers: the pool floor is visible through the water
  behind the glass wall (INSPECTED). plants_cross: the four plants
  render as before (the shrub's binary-alpha art makes cutout vs
  blend subtle); the edge water patch turns translucent (INSPECTED).
- REGRESSION: root cargo test --workspace 486 green / 0 failed; the
  FULL vistest battery 108 scenes [ok] / 0 FAIL (exit-0 enforced);
  make smoke OK (headless logic + GUI liveness); idle-upgrade-check
  PASS. POORCRAFT 3D untouched by this job (its workspace does not
  depend on the root crates).
- PERF: not applicable — a mesh-build-time predicate over tex
  indices (one const compare + one small match vs two literal
  compares); zero render-frame or simulation cost change; no perf
  claim.
- LORE: canon touched: none — render-pass routing over existing
  water/foliage art; no faction, place, event, term, NPC, item, or
  spell data; no identity assigned. World expression: water surfaces
  regain their intended translucency — rivers, lakes, and pools show
  their beds, and the world reads deeper for it; dead shrubs render
  as proper cutout foliage. Migration: none (no save format, no data
  schema, no proof schema change). Deferred honestly: the carried
  list — guardian chronicle re-fire (447: the Discovery re-fires if
  a geode re-settles after despawn, dragon precedent); multiplayer
  routing of terrain edits; geode pairing.

## 2026-09-15 — The query-bound law: the ground query's origin contract made explicit (loop 459)

- Closed STATE's next_task item (1) — the walk-snap query-bound
  observation carried since loop 445: `CollisionSurface::ground_at`
  takes `from_y` and the streamed paths DISCARD it (the recovery's
  one-frame teleport lift rides that discard through a bare
  `f32::MAX / 4.0` literal). The contract is now written law, pinned
  by tests, BEFORE anyone enforces a streamed bound and has to
  reconcile it with the authority path.
- THE LAW (pc3d_render::player): the trait doc states THE QUERY-BOUND
  LAW — on the authority path `from_y` is the search window
  (AuthorityGround walks three solid cells down from it; the pure
  window `in_authority_query_window(from_y, ground_y)` names the band
  `(from_y - 3, floor(from_y) + 1]`); on the streamed paths it bounds
  nothing BY CONTRACT — a streamed mesh holds one height per column
  and has no interior to search, so the answer is the column's
  PLACEMENT, origin-independent: it may sit above the origin (the rise
  law judges climbs from live answers) or any depth below (the snap's
  step band and the walk-off law judge descents). A future streamed
  bound must adopt and change this law consciously, never silently.
- THE NAMED CONSTANTS: `UNBOUNDED_QUERY_Y` replaces the three bare
  `f32::MAX / 4.0` literals (renderer.rs ground_y_at's streamed probe,
  settlement.rs + npcs.rs anchor windows) with the honest doc — the
  unbounded probe is a STREAMED-path idiom; the authority window under
  a near-f32-max origin holds only sky and REFUSES it (pinned by law,
  so nobody ever "fixes" the renderer's authority fallback by routing
  the probe through the windowed query expecting an answer).
  `QUERY_REACH_M` names the walk's own origin (`feet + 2.0` at the
  snap and the rise law's three sample sites — the literal the walk
  always passed).
- 3 new unit laws (pc3d_render 233 -> 236; p3d 690 -> 693): the
  streamed placement answers every query origin on the real region
  surface (placement == mesh height at four origins incl. the
  unbounded probe); the authority window binds on real columns (the
  answer inside the band at every origin, the deep query never
  answers the surface, the unbounded probe refuses) + the pure
  window's hand cases; the live streamer's collision answers every
  origin identically once the full ring loads and refuses outside it
  at every origin.
- NO BEHAVIOR CHANGE, honestly claimed and re-proven: constants and
  docs over the same values; every arithmetic literal is the number it
  always was. The routes that ride the answer re-ran green: make
  p3d-recovery PASS x2 + comparator (p50 26.1/26.8 ms; the one-frame
  lift lands the body at the rim, the fall empties health, the wake-up
  sits at EXACTLY 0.5 — health bar pixel-measured 0.975 at the rim vs
  0.471 at final, byte-identical across runs) and make p3d-walkoff
  PASS x2 + comparator (p50 25.7/25.7 ms; air beat unwounded over the
  pit, landed wounded with the fresh FELL — HEALTH 64% toast). All
  refreshed captures INSPECTED.
- REGRESSION: p3d workspace 693 green / 0 failed (14 suites);
  p3d-smoke OK (digest dd019eca900f5a61 UNCHANGED — headless, no
  query-bound path); idle-upgrade-check PASS; visual gates NOT run —
  nothing visual changed (the 445/458 bench-only precedent; the route
  captures are the visual evidence and were inspected); root LOREFORGE
  workspace untouched (456 verified 480 green at this tree's crates);
  the six windowed_wild_*.png dirties remain deliberately NOT staged
  (an earlier session's p3d-wilderness run).
- PERF: not applicable — pure functions and named constants; zero new
  work on any live path (two const loads replace two literals).
- LORE: canon touched: none — a collision-query contract law over the
  existing walk/streamed-surface physics; no faction, place, event,
  term, NPC, item, or spell data; no identity assigned. World
  expression: unchanged — the same ground answers every body in
  Valdenmoor; the law only makes the streamed placement explicit so
  future bound enforcement cannot silently move a placed body.
  Migration: none (pure law + renames + tests; no persisted field, no
  save format, no proof schema change; smoke chain unchanged).
  Deferred honestly: next in line is the is_water_layer/CTM-strip
  audit (447's note: the hard-coded 167 now points at dead_shrub's
  index — water proofs pass but the strip addressing deserves an
  audit); carried: guardian chronicle re-fire; multiplayer routing of
  terrain edits; geode pairing.

## 2026-09-14 — The bench guards its host: contention probe + the quiet-host re-read (loop 458)

- Closed STATE's next_task item (1) — the twelve-loops-queued
  (446..457) quiet-host deck-bench re-read — and landed its carried
  companion, the bench contention guard (445's own deferred item).
- THE GUARD (pc3d_render::deck): `cpu_probe_ns` — a fixed 400k-step
  deterministic float workload (~2.7 ms per reading on the evidence
  host, calibrated from the release binary: 100k steps read 0.67 ms
  with a 1.2% start/end spread; 4x averages out scheduler transients)
  — the pure-std, portable "was the host quiet" signal. Each tier run
  brackets `run_windowed` with start/end probes; the readings ride the
  CSV sidecar (two new columns) and the report carries a "host CPU
  probe" bullet; `probes_within_band` is the pure law (slowest <=
  fastest x 1.4; a zero/missing reading never counts as quiet) and
  `deck_report` asserts it BEFORE writing the report — a contended run
  writes no report and exits with the readings printed. Contamination
  is a first-class failure, not a bookkeeping note (the exact failure
  mode of 445's discarded first attempt).
- 4 new unit laws (pc3d_render 229 -> 233; p3d 686 -> 690): zero work
  reads as no time; 4x the work reads at least 2x the time
  (frequency-scaling robust); the band holds a quiet spread and fails
  a 3x mid-run slowdown; the report carries the probe bullet.
- THE RE-READ: make p3d-deck-bench PASS on a quiet host (1-min load
  3.3-4.1 through the run; probes 2.68M ns with a 0.7% spread, band
  x1.4 held): low 7.18 / mid 12.92 / high 13.06 ms p50 (p95
  8.58/14.26/14.09; meshed 176/402/603; GPU 5140/18886/27587 KB;
  flora 605; setl 1048 tris; crowd 92 — every work counter
  byte-identical to 445's record). vs 442/445's
  6.85-6.89/12.16-12.30/12.44-12.56: a uniform +4-6%.
- THE A/B THAT DECIDES THE SHIFT: the loop-445 binary itself (scratch
  worktree at f932c32, fresh release build) run on THIS host TODAY
  reads 7.21/12.88/13.02 — the new build's numbers within noise (and
  it did so at a busier moment, 1-min load ~13: further evidence the
  bench tolerates this load class). The +4-6% is host-state drift —
  today's baseline, not code cost; loops 446-457 cost nothing
  measurable on the bench walk. Tier ordering and Low-not-slower held
  (low p95 8.58 <= high p95 14.09 x 1.25).
- CAPTURES: windowed_deck_low/mid/high.png re-rendered and INSPECTED —
  the same vista (POS 408.0/110.2/104.0 in every strip), lean at low
  (FPS 135, clean foreground), mid adds the near-field settlement
  geometry (FPS 77), high draws the fuller roof field (FPS 74): same
  world, leaner dressing, legible in pixels. The refreshed report on
  disk carries the new date + the probe bullet.
- REGRESSION: p3d workspace 690 green / 0 failed (pc3d_render 233);
  p3d-smoke OK (digest dd019eca900f5a61 UNCHANGED — headless, no bench
  path); p3d-people PASS (motion 0.66%, windowed p50 12.70 ms; its
  four refreshed captures ride the commit, plaza/stride INSPECTED);
  idle-upgrade-check PASS. Visual gates NOT run — nothing visual
  changed (the 445 precedent for bench-only loops); the deck captures
  are the visual evidence. Root LOREFORGE workspace untouched (456
  verified 480 green at this tree's crates).
- PERF: this IS the perf job. Baseline = 442/445's record; after =
  today's chain, same seed/scene/profile/host; the A/B pins the delta
  to the host, not the code. No optimization claimed, none needed.
- Deferred honestly: the contention guard is DONE. Carried: the
  walk-snap query-bound observation (surface.rs ground_at discards
  _from_y on the streamed path; write the pure law before the bound is
  ever enforced); the is_water_layer/CTM-strip audit; guardian
  chronicle re-fire; multiplayer routing of terrain edits; geode
  pairing.

## 2026-09-14 — Every beat capture reads the latch: climb, hang-off, and playtest captures schedule themselves (loop 457)

- Closed STATE's next_task item (1): the remaining routes adopted the
  capture-side latch cure 456 shipped (`CaptureAtNextFrame` +
  `end_frame`). App `main.rs` only — no lib change; the p3d count stays
  686.
- THE CLIMB: the GRIPPED-toast latch poll (32..=165) now requests
  `play_vine_grip` itself; the fixed @170 shot is gone — the grip and
  its toast are in the pixels at every pace, and the capture moved to
  the catch beat (the fixed frame was a late-hang beat).
- THE HANG-OFF: fixed hop_fall@226 and hop_landed@300 are gone. Leg 1's
  poll fires hop_fall at the AIR BEAT (first poll genuinely airborne
  below the tip: feet < tip − 0.05 and feet − ground > 0.2; request
  pose/ground/frame recorded, latch frame newly recorded; leg-1
  recordings widened 12 → 18) and hop_landed AT the landing latch
  (fresh FELL toast). Leg 2's poll fires tip_landed AT its latch. Leg
  2's teleport moved 350 → 356 so a window-edge latch (capture at N+1,
  after N+1's own script steps) can never race the body's lift.
  `end_frame Some(700)` owns the exit (the static list drains at 430
  while leg 2 can latch as late as 638). The verdict gained the
  AIR-BEAT assertion (request pose strictly below the tip and over
  ground + 0.2, request frame before the latch frame) and moved
  hop_fall-vs-hop_landed and hop_rise-vs-hop_landed to the advisory
  sway bar — at contended paces the hop compresses and all three
  captures can land on the landed beat (the same evidence class 454
  recorded); the hard pixel gates are the place-vs-place pairs
  (hang-vs-rise 0.002, hang-vs-fall 0.005, tip_hang-vs-tip_landed
  0.02); every physical claim reads latched records.
- THE PLAYTEST: fixed play_fall_air@840 and play_fall_landed@920 are
  gone. The fall's poll (812..=899, pushed before the frame-900 entry
  per the queue-order law) frames play_fall_air at the FIRST
  genuinely-airborne poll (the teleport is +8 m — the window's opening
  at every pace) and play_fall_landed AT the FELL-toast latch. The
  verdict gained the air-beat assertion and keeps air-vs-landed > 0.03
  HARD — now a genuine place-vs-place pair at every pace.
- PROOF: p3d-climb PASS x2 + comparator (p50 22.6 ms; play_vine_grip =
  GRIPPED A VINE fully legible AT the latch); p3d-hangoff PASS x2 +
  comparator (p50 24.5-24.7 ms: "air beat latched at frame 234 for 235
  (pose 54.74, ground 53.18)", hop rose 1.04 m, fell past the strand,
  grounded 100% → 81% (law 81%), tip release safe 1.02 m); p3d-playtest
  PASS x2 + comparator (p50 23.9 ms: "fell 8 m over the open journal
  (health 33%)"). Captures INSPECTED x6; layout JSONs verified on disk
  both runs (hangoff landed 0.806 + toast_FELL, air 1.000; playtest
  landed 0.331 + toast_FELL, air 1.000; climb grip carries
  toast_GRIPPED).
- REGRESSION: p3d-walkoff PASS x2 + comparator (p50 21.0-21.2 ms; the
  air beat latched at frames 110 and 108 — DIFFERENT frames again, the
  capture follows the beat); p3d-dig and p3d-recovery PASS x2 +
  comparators; p3d-people PASS (motion 0.64%); p3d-visual-gates ALL 10
  PASS; p3d-smoke OK (digest dd019eca900f5a61 UNCHANGED); p3d
  workspace 686 green / 0 failed; idle-upgrade-check PASS. Deferred:
  the remaining fixed captures in these routes are deterministic beats
  (stable phases the schedule can own) or run-end keepers, left fixed
  on purpose; the quiet-host deck-bench re-read (twelve loops queued).

## 2026-09-14 — The captures read the latch: walk_air and walk_landed schedule themselves from the fall's own polls (loop 456)

- Closed STATE's next_task item (1): the CAPTURE side of the latch
  cure. `UiAction::CaptureAtNextFrame { path, ui_dump }` joins the
  proof-hook family — a route poll that LATCHED a beat (the landing,
  the grip, true air) asks for the NEXT presented frame's picture,
  and the app inserts it into the sorted shot list, producing the
  same CaptureOutcome a static Shot does, at a frame no contended
  pace can outrun. The insertion law is pure (`next_free_shot_frame`,
  2 unit tests): the capture fires requested_at+1, walking forward
  past any frame a scheduled shot already owns — two shots can never
  share a frame (the second would silently never fire, and a
  never-consumed last shot never ends the run).
- THE HORIZON: `WindowConfig::end_frame` owns the exit for
  dynamic-capture runs — the static list may drain long before the
  last capture fires, so the drain no longer ends such a run
  (validation: the horizon must exceed every static shot and not
  precede max_frames, honest errors up front). Runs without
  end_frame keep the old drain law byte-for-byte.
- THE WALK-OFF REWIRED: the fixed walk_air@112 and walk_landed@190
  shots are gone; the pre-dig rim stays static at 70. The polls now
  fire walk_landed AT the landing latch (pose + wounded health + the
  FELL toast latched — the toast is fresh in pixels at every pace)
  and walk_air at the FIRST poll genuinely airborne over a meter
  below the rim (request flag + pose + frame recorded; recordings
  widened 20 -> 24). The verdict gained the AIR-BEAT assertion: the
  request pose must hang strictly between the rim's 0.6 m band and
  floor+0.2, on the spot, BEFORE the latch — a capture that lands on
  the landed beat fails even though its pixels still differ from the
  rim. The air-vs-landed PIXEL difference stays advisory (sway
  aliasing).
- PROOF: p3d workspace 686 green / 0 failed (pc3d_render 227 -> 229,
  the two insertion laws). make p3d-walkoff PASS x2 + comparator at
  p50 25.5 / 27.4 ms — the two runs latched the air beat at frames
  104 and 102 (the capture follows the beat, not the calendar) with
  identical physical claims: "fell 56.19 -> 51.19 (deepest air
  51.38) and landed wounded 100% -> 65% (toast true)". Captures
  INSPECTED x6: walk_air = full health bar + PACK EMPTY + the dig
  toast (unwounded, pre-landing); walk_landed = the ~64% wounded bar
  + "FELL — HEALTH 64%" over the pit wall; walk_edge = the rim
  vista. The layout JSONs carry the same story on disk (landed: both
  toasts + health 0.647; air: dig toast only + health 1.0).
- REGRESSION: p3d-dig PASS x2 + comparator; p3d-recovery PASS x2 +
  comparator (p50 26-27 ms); p3d-people PASS (windowed p50 16.5 ms);
  p3d-visual-gates ALL 10 PASS; p3d-smoke OK (digest
  dd019eca900f5a61 UNCHANGED); idle-upgrade-check PASS. Deferred:
  the remaining routes' capture conversions (playtest toast, climb
  grip, hangoff landings — their verdicts already read latches).

## 2026-09-14 — The verdicts read the latch: walk-off and playtest proofs made pace-proof (loop 455)

- Landed the verdict-side half of 454's next_task item (1): the two
  routes whose frame-CALIBRATED verdicts broke twice under the day's
  foreign-load spike now prove the same claims from latched per-frame
  polls. No lib code changed — app `main.rs` route scheduling only;
  the p3d workspace count is unchanged (684).
- THE WALK-OFF: one per-frame poll (92..=189) records the DEEPEST
  AIRBORNE pose (grounded clamps never enter the min) and LATCHES the
  landing on two consecutive on-ground polls against the LIVE ground
  answer — pose + health + the FELL toast AT the latch. That kills
  the regen race (the old frame-190 health read was 10.6 s past the
  impact at 56 ms — the fed body had healed past its own band), and
  the mid-air band becomes "meters of air, then the latch" (the
  deepest-air record hugs the floor by construction; the old
  ground-4.6 lower edge calibrated the fixed mid-fall frame and
  wrongly failed a genuine airborne sample by 0.03 m). The pixel gate
  is place-vs-place (rim vs pit bottom, every pace); air-vs-landed is
  advisory like the climb sway bar.
- THE PLAYTEST: the live fall's FELL toast latches in a poll window —
  and the first run failed honestly on the script queue's own law:
  polls pushed after the frame-900 entry sit blocked (the queue fires
  strictly in push order) until the toast was long dead. The polls
  now push before it; the law is written where it bites.
- PROOF: p3d-walkoff PASS x2 + comparator at p50 73 ms (the 54-59 ms
  paces broke the committed verdict earlier today) — "fell 56.19 ->
  51.19 (deepest air 51.54/51.38), landed wounded 100% -> 65% (toast
  true)"; p3d-playtest PASS x2 + comparator at p50 69 ms — "fell 8 m
  over the open journal (health 33%)". Captures inspected (walk_landed
  carries both toasts + the wounded bar; play_fall_landed the forge,
  pack line, 33% bar). dig/recovery x2 + comparators, people, gates
  ALL 10, smoke (digest unchanged), 684 tests, idle-upgrade-check —
  all green. Deferred: the CAPTURE side (Capture::at_next_frame).

## 2026-09-14 — Space lets go: the strand release is real (loop 454)

- Closed STATE's next_task item (1): the Space jump-off from a hang had
  unit-lawed pieces but no windowed route — and the route the loop wrote
  found a REAL BUG before shipping anything: both release paths silently
  re-gripped. The pre-law windowed probe pinned the evidence: the Space
  hop rose 1.04 m, fell straight back into the strand's crossing catch
  (window-min 55.86, tip 54.84, ground 53.18 — never past the strand,
  landing latch never fired in 245 polls), and the S tip release was
  re-caught INSIDE the grab band below the tip (the body pinned AT the
  tip 54.84, health 1.00, never grounding). The strand was a one-way
  trap: "SPACE LETS GO" fired a toast and the strand caught the body
  anyway. The old unit suite could not see it — `the_catch_saves_the_
  body` modeled no post-release grab check.
- THE RELEASE LAW (pc3d_render::app): `SliceHost.released_from` records
  the strand a body let go of (Space hop-off or tip release) and the
  arc's grab check refuses THAT strand through the pure gate
  `strand_can_catch` — strand-scoped (a DIFFERENT strand in hand reach
  still catches), spent on landing, spent by a new grab. The grab law's
  own tests are untouched (a fast drop still cannot tunnel through a
  strand it never held). 3 new unit laws — the Space hop-off composition
  (rise ~1 m, falls past its own strand, pays the height law's exact
  wound), the tip release (falls free of the band that used to re-catch
  it, safe; the old re-catch pinned as the forbidden behavior), and
  strand-scoping/spent-on-landing. pc3d_render 224 -> 227; p3d
  workspace 681 -> 684 green / 0 failed; root workspace 480 green.
- THE ROUTE (route_vine_hangoff, make p3d-hangoff): two legs on the
  climb route's own searched strand. LEG 1 drops 12 m onto the line
  (caught), presses Space ONCE through the real key path: the hop RISES
  (peak = hang + 1.04), the body falls PAST its own strand (window-min
  = the ground below the tip), lands GROUNDED at the strand's line, and
  pays EXACTLY the wound the recorded heights name through the impact
  law (health 100% -> 81-91% vs the law's 81-91% at every pace). LEG 2
  drops again and descends PAST THE TIP with S: the release falls free
  and lands SAFE (health never drops; a fed body's regen may only
  heal). All landings latch on two consecutive on-ground polls against
  the LIVE ground answer, and the FELL toast is latched AT the landing —
  pace-proof by design.
- THE CLIMB ROUTE TELLS THE TRUTH NOW: its verdict asserts GROUNDEDNESS
  (the pre-law run ended with the body pinned at the tip — the old
  "landed unharmed" claim passed without checking) and proves the
  fall/catch by latched records (lowest height of the drop, the hang
  inside the strand's own span, the GRIPPED toast latched) instead of
  the frame-vs-frame pixel bar that measured sway aliasing, not the
  fall, at contended paces. The captures stay as the visual record.
- Evidence: p3d-hangoff PASS x2 + comparator (the release law's own
  route — PASS at 20, 30, 48, and 80-108 ms p50 across the day's host
  states); p3d-climb PASS x2 + comparator (79-88 ms, grounded, span
  records); p3d-dig PASS x2 + comparator (20.0 ms); p3d-people PASS
  (motion 0.64%); p3d-visual-gates ALL 10 PASS; p3d-smoke OK (digest
  dd019eca900f5a61 UNCHANGED); idle-upgrade-check PASS; the playtest
  bundle digest reproduced UNCHANGED 05c46411869a857c on this job's
  dev-stamped builds (pre-spike run). THE FOREIGN-LOAD SPIKE, honestly:
  mid-session the host loaded to a 94 15-minute average (outside
  processes; windowed p50 54-142 ms) — the frame-CALIBRATED walkoff and
  playtest chains broke on re-run (walkoff at 54 ms: the fall completed
  before its frame-112 window, the FELL toast expired, regen healed the
  wound — the documented 447/449 capture-window sensitivity, third
  occurrence). Those two routes' code is untouched by this job; their
  bundles ride from loop 453 (git HEAD, same route code) and the
  playtest digest identity above pins the non-perturbation.
- PERF: no claim, honestly — the gate is one Option<[f32;2]> compare
  per falling frame; the routes add work only while they run; the live
  walk/stream/crowd paths are untouched; the host was contended all
  session, no bench per the 445-453 precedent; the quiet-host re-read
  stays queued (nine loops).
- LORE: canon touched: none — a traversal law over the existing vine
  strand; no faction, place, event, term, NPC, item, or spell data; no
  identity assigned. World expression: in Valdenmoor a hand that lets
  go is free — the strand catches a FALL, never holds a body that
  released itself, and the fall economy of 441/444/446 remains the only
  judge of every drop. Migration: none (in-memory slice state + route
  records; no persisted field, save format, or proof schema change).

## 2026-09-14 — The plaza recovery, live: the lethal fall's windowed route (loop 453)

- Closed STATE's next_task item (1): the fall law's lethal branch —
  health emptied -> wake at the plaza, health 0.5, food halved, its own
  toast — had unit laws but no windowed route. `route_plaza_recovery`
  (make p3d-recovery) frames it.
- THE ROUTE: the walk-off's spot finder (flat, flora-free, vine-free)
  stages a cell near the plaza; one -14 m edit pits it under the
  standing body; the arc lands ~16.5 m/s and the impact law costs
  ~1.15 health — more than the body carries. Verdict bands: rim
  standing on the LIVE pre-dig ground at full bars; a mid-fall pose
  between floor and rim (two record chances); recovered XZ at the
  plaza center, feet ON the LIVE plaza-ground answer, health EXACTLY
  0.5, food exactly halved; a later record pins the body still there
  (no second fall, no regen at 0.5 food); the recovery toast present,
  the plain FELL toast absent; five captures all differing.
- THE PREDICTION THE RUNTIME DISPROVED (honest): statics said the
  recovery "keeps the pit-floor Y" and would plant the body under the
  plaza. The first run against UNCHANGED code PASSED — the walk's
  per-frame ground snap (trivially true for any rise, the predicate
  449 fixed for axis moves) lifts the body in one frame. No pose
  change shipped; the route laws the outcome. Recorded: surface.rs
  ground_at discards its _from_y bound on the streamed path.
- THE GLYPH, proof-found: the wake-up capture showed the em-dash in
  "YOU FELL — RECOVERED AT THE PLAZA" rendering as a blank gap — the
  glyph was missing from the pixel font ("FELL — HEALTH x%" had the
  same hole). Added the em-dash glyph + an ink law (distinct from the
  hyphen) — pc3d_render 223 -> 224; both fall toasts read with their
  dash.
- Evidence: p3d workspace 681 green / 0 failed; p3d-recovery PASS x2
  identical + comparator ("woke at the plaza (384.5,128.5) feet 55.19
  (plaza ground 55.19); health 100% -> 50% EXACT, food 1.00 -> 0.50");
  all five captures INSPECTED. Walk-off, dig, playtest PASS x2 +
  comparators each; playtest bundle digest 05c46411869a857c on
  dev-stamped builds (the digest covers the bundle JSON incl. the
  compile-time PC3D_BUILD stamp, not capture pixels); people PASS
  (motion 0.66%); visual gates ALL 10 PASS; smoke OK (dd019eca900f5a61
  unchanged); idle-upgrade-check PASS.
- PERF: no claim, honestly — the route adds work only while it runs;
  the live walk/stream/crowd paths are untouched; host shared (route
  p50 20.0-23.4 ms), no bench per the 445-452 precedent.
- LORE: canon touched: none — a consequence branch of the existing
  fall law over the unnamed settlement's plaza anchor. World
  expression: lethal falls in Valdenmoor do not end the journey — the
  body wakes at the settlement's heart, wounded and hungry, and the
  toast says so legibly. Migration: none.

## 2026-09-14 — The pack is legible: the inventory's echo on the HUD (loop 452)

- Closed STATE's next_task item (1): the INVENTORY ECHO of the dig verb,
  finished from an interrupted session's orphaned start (`items.rs` held
  an unwired, untested `stock_line` — reconciled, completed, proven).
- THE LINE (`pc3d_world::items::stock_line`): the inventory's echo as
  ONE stable string — nonzero kinds in CATALOG order (the unit law
  caught the first test assuming insertion order), counts summed across
  stacks, an empty pack reading "PACK EMPTY". Two unit laws —
  pc3d_world 265 -> 267.
- THE HUD (`pc3d_render::ui` + app sync): `HudValues.stock` synced by
  the app from the REAL slice inventory on every UI-dirty frame (an
  idle HUD pays nothing), painted verbatim under the status bars,
  serialized to the inspector JSON. Layout law pins it under the bars,
  inside the safe margins, clear of the hotbar, absent until synced.
- THE GLYPH, a proof-found fix: the capture inspection showed the
  " · " separator rendering as a BLANK GAP — "·" was missing from the
  pixel font and unknown chars silently fall back to space (the
  pre-existing prompt "F BUILD SAND · R REMOVE" had the same hole).
  Added the middle-dot glyph + an ink law; the pack line and the
  prompt row now read with their separators.
- THE EVIDENCE PATH: captures now merge the LIVE ui_state into the
  in-memory layout (it existed only in the on-disk dump), so verdicts
  and dump files read ONE structure.
- THE ROUTE (make p3d-dig extended): computes the EXPECTED pack line
  from the same determinants the verb uses (the spot cell's material
  through harvest_yields over an empty pack) and the verdict asserts
  the HUD strings: "PACK EMPTY" -> "PACK WOOD 1 · SOIL 1". PASS x2
  identical + comparator PASS; all four captures INSPECTED.
- THE WIDER ECONOMY (the playtest's own journey): the pack line reads
  IRON_ORE 4 + WOOD_PICK + BREAD 2 after the harvest, IRON_ORE 2 +
  IRON_BAR 1 after the forge take, IRON_ORE 4 + IRON_BAR 1 after the
  re-harvest; play_taken.png INSPECTED (legible over open panels).
- Evidence: p3d workspace 680 green / 0 failed (pc3d_render 223,
  pc3d_world 267); p3d-dig PASS x2 + comparator; p3d-playtest PASS x2
  + comparator, bundle digest RE-BASELINED 1d37f62c3801ae20 (honest
  change: the pack line rides every gameplay capture); p3d-people
  PASS (motion 0.64%); p3d-visual-gates ALL 10 PASS; p3d-smoke OK
  (digest dd019eca900f5a61 unchanged); idle-upgrade-check PASS.
- PERF: no claim, honestly — one small string on UI-dirty frames
  only, one text row, a 12-pixel dot; live walk/stream/crowd paths
  untouched; host shared (windowed p50 19.5-23.0 ms), no bench per
  the 445-451 precedent.
- LORE: canon touched: none — a read-only echo over the existing
  generic material catalog; no faction, place, event, term, NPC,
  item, or spell data. World expression: labor in Valdenmoor leaves
  a legible trace — what you dig is what you carry, as a number the
  item authority owns. Migration: none.

## 2026-09-13 — The crowd yields in the window: the staged-yield crowd route (loop 451)

- Closed STATE's next_task item (1): the 448 crowd law was unit-lawed
  and render-lawed, but no windowed ROUTE framed two NPCs yielding.
  `route_crowd_yield` (make p3d-crowd) frames it in the live slice.
- THE STAGE (renderer.rs): route-proof hook `crowd_stage_head_on_near`
  rewrites two cast members into head-on walkers on ONE fully walkable
  row near a requested center (real nav paths the Work-phase schedule
  keeps) and parks the rest at their homes. A row is admitted only when
  BOTH nav paths are STRAIGHT along it (so an off-row body cell later
  is yield evidence, never a nav detour), the row plus its ±1 sidestep
  band touches no settlement collision cell (the nav is terrain-only —
  the first staged row walked through the house band), and the parked
  home sits clear of the band; rows are tried nearest the center first
  inside the nav-patch interior (the plaza sits on a patch corner —
  every plaza-relative span crossed the border and answered None).
  New readers `crowd_cell` / `crowd_work_site` expose the authoritative
  brain to routes.
- THE ROUTE: the live slice's own `crowd_tick(0.35, 1)` runs the real
  law every frame — cast 0 walks through, cast 1 is refused at gap 2
  and sidesteps, re-paths around, and both arrive Working at their
  DECLARED sites. A per-frame audit (frames 43..=112, 70 recorded)
  ORs the flags: no shared cell on any audited frame, the off-row
  yield seen, read failures poison the run. The body watches from 5 m
  south of the row's middle; captures: crowd_staged (closing),
  crowd_yield (the sidestep — two bodies adjacent mid-row),
  crowd_pass (separated), crowd_arrived (both at their ends). PASS x2
  identical + comparator PASS; the four captures INSPECTED in debug
  AND release runs.
- PRIOR-PROOF FIX (the people harness): p3d-people's motion check
  compared frames whose stride phases follow the WALL CLOCK — at this
  week's ~22.5 ms frame pace the phases aliased and the metric read
  0.0016 (FAIL) then 0.0020 (PASS, 0.3 over the bar) on IDENTICAL
  code, vs 448's 0.0103. The harness now freezes the pose clock at two
  KNOWN phases before the compared captures: 0.0065 / 0.0063 across
  runs — 3x the bar and scheduling-independent.
- Evidence: p3d workspace 676 green / 0 failed (pc3d_render 221 = 220
  + 1 staged-yield law; pc3d_world 265 unchanged); make p3d-playtest
  x2 + comparator PASS, bundle digest UNCHANGED 05c46411869a857c;
  make p3d-people PASS x2 (12 NPCs, 7 draws, 92 instances); p3d-smoke
  OK (digest dd019eca900f5a61 unchanged); p3d-assets OK;
  idle-upgrade-check PASS.
- PERF: no claim, honestly — the route adds work only while it runs
  (a bounded staging scan; two cell reads per audited frame); the live
  walk/stream/crowd paths are untouched; host shared (windowed p50
  ~22-26 ms), no bench per the 445-450 precedent.
- LORE: canon touched: none — a proof route over the existing crowd
  law on unnamed settlement bodies; the yield is LOCAL perception (a
  body yields to the body in front of it), never omniscient pathing,
  per the personhood law. Migration: none (in-memory staging inside
  one route run).
- Deferred honestly: the inventory echo of the dig verb (next_task
  item 1); the lethal plaza-recovery branch route; Space jump-off from
  a hang; the quiet-host deck-bench re-read + the bench contention
  guard; is_water_layer/CTM-strip audit; guardian chronicle re-fire;
  multiplayer routing of terrain edits; geode pairing.

## 2026-09-13 — The dig is in your hands: the player-facing dig verb (loop 450)

- Closed STATE's next_task item (1): the DIG VERB on 446's live
  EditSurface path. Press G and the ground under the crosshair
  lowers one walkable meter; the take is gated BEFORE the ground
  breaks and credited to the real inventory; the toast names it.
- THE TARGET (pure, `pc3d_render::player::dig_target`): the look ray
  marches against the SAME live ground answer the walk stands on and
  the picture draws (streamed delta layer included), answering the
  first column whose ground the ray enters within the build verb's
  8 m reach. A dug terrace moves the aim past it (the ray reads the
  live surface, never the original skin), a wall takes the dig at
  its face, a level gaze digs nothing, a steep gaze digs the body's
  OWN column — the walk-off trick is now a legal player verb, and
  the step law owns the 1 m snap-down it causes.
- THE TAKE (pc3d_world::items): `harvest_yields` — the tested drop
  table that has been waiting for a consumer — now feeds the dig:
  soil/sand/snow bare-handed, stone needs a pick (the best carried
  tier gates it), grass yields soil+wood. New `Inventory::can_fit`
  answers before the edit: a take that can't be carried never breaks
  ground. A taking dig is Excavated quest progress (same as the
  harvest and removal verbs).
- THE INPUT: the real UI path — `ui_key` maps KeyCode::KeyG, and
  `ui::on_key`'s gameplay arm fires `UiAction::DigAtCrosshair` (the
  forge panel keeps G for fuel while open; dialog/journal/interact
  panels own the frame — no world edit behind one). The exec handler
  resolves target -> material (cell_material of the LIVE surface —
  dig deep and the exposed floor asks for a pick) -> outcome -> pack
  room, then edits, credits, and toasts: "DUG SOIL+WOOD" or the
  honest refusal ("NO GROUND IN REACH" / "NEED A PICK FOR STONE" /
  "PACK FULL - THE GROUND HOLDS"). The HUD prompt and the Settings
  keymap carry G DIG.
- Route proof: `make p3d-dig` (`route_dig`): a flat bare-hand-
  diggable cell near the plaza (`find_dig_spot_barehand` gates the
  surface material to Grass/Soil/Sand/Snow and the strip to
  gentle); the body presses G once aimed STEEP — its OWN cell drops
  exactly 1.00 m (54.49 -> 53.49 sampled at the standing point, no
  kernel-blur band needed), the control column is untouched, the
  body snaps down UNWOUNDED (100% -> 100%) and walks one step back
  up onto the rim; x2 identical + comparator PASS; the four
  captures INSPECTED (G DIG prompt; DUG SOIL+WOOD toast; standing
  in the dug hole full-health; back on the rim facing the wilds).
- EN-ROUTE (prior-proof fix, the carried contention-window
  deferral): route_walk_off's frame-120 mid-air window was
  calibrated for a ~61-rendered-frame fall; at this week's ~22.5 ms
  windowed pace the fall completes in ~34 frames and the window
  caught the body 0.19 m above the floor (landed_differ false,
  twice, deterministically). Re-timed to frame 112: mid 53.62 =
  2.57 m down, inside the old physical band at every observed pace;
  the band, captures, and claims are unchanged — only the window
  moved.
- Evidence: p3d workspace 675 green / 0 failed (pc3d_render 220 =
  213 + 7 dig laws; pc3d_world 265 = 264 + 1 can_fit law);
  `make p3d-walkoff` PASS x2 + comparator (hardened window);
  `make p3d-playtest` x2 + comparator PASS, bundle digest UNCHANGED
  05c46411869a857c; `make p3d-climb` x2 PASS (landed unharmed);
  `make p3d-steer` x2 PASS (drift 1.60 m, ortho 0.00); p3d-smoke OK
  (digest dd019eca900f5a61 unchanged); p3d-assets OK;
  idle-upgrade-check PASS.
- PERF: no claim, honestly — the dig adds work only ON PRESS (one
  <=80-sample march + one surface edit + remesh of the touched
  patches, allocation-free, microseconds against 22 ms frames); the
  per-frame walk/stream path is untouched; the host is shared
  (windowed routes p50 ~22.5 ms, the local norm this week), so no
  bench per the 445-449 precedent.
- LORE: canon touched: none — a labor/traversal verb on the streamed
  surface; the take is the existing generic material catalog; no
  faction, place, event, term, NPC, or spell data; no identity
  assigned. World expression: labor in Valdenmoor has consequence —
  the pit you dig is a pit until you dig steps out, and the earth
  you take is the same material economy the build verb draws from.
  Migration: none (the surface delta layer already persists from
  446; inventory crediting matches the existing harvest slice's
  in-memory pattern).
- Deferred honestly: multiplayer routing of terrain edits (the verb
  is live-local like EditSurface); an inventory readout to SEE the
  take as a number (the toast names it; can_fit/add/outcome are
  unit-lawed); geode pairing (447's keepers are root-workspace lore
  — p3d gains its own Old-Powers expression in future work).

## 2026-09-13 — The wall holds: the up-step half of the walk law (loop 449)

- Closed STATE's next_task item (1): 446's step law fixed the DOWN
  half (drops deeper than one step belong to the fall), but the UP
  half was still open — the walk's ground snap condition
  `pos[1] - g <= SUPPORT_GAP_M` is trivially true whenever the ground
  answer is ABOVE the feet, and the streamed surface's
  `CollisionSurface` answers `cell_solid = false` everywhere ("slopes
  gate walkability through ground_at stepping"), so the snap was the
  streamed path's ONLY wall. Every cliff face and dug wall was a free
  elevator: fall into the pit, take the wound, then walk straight out.
- THE RISE LAW (pure, `pc3d_render::player`): an axis move whose
  target ground sits above the feet is refused unless the surface
  there is itself walkable — a discrete STEP within SUPPORT_GAP_M
  (1.05 m, the up twin of the down law) or a RAMP within the nav's
  own walkability contract (`surface::MAX_WALK_SLOPE` = 1.6 m/m),
  scaled by the distance moved this frame so ramp climbing is
  fps-independent. The surface slope is the SIGNED rise along the
  move over a 1 m baseline (the mesh's own node spacing): geometry,
  not the per-frame rise, so the verdict is identical at 30/60/120
  fps; a descent ahead is never a wall (the down law owns descents).
- EN-ROUTE BUG, caught by the live route's own records and promoted
  to a unit law: the first draft measured |ahead − behind|, so the
  steep ramp BEHIND a body (the wall it had just been refused by)
  poisoned the baseline and refused walking AWAY across flat ground —
  the route's W-hold sat motionless for 60 frames, then lurched when
  frame-rate jitter reshuffled the ±0.5 m samples. The key_script /
  pose records pinned it; the signed slope fixed it;
  `the_body_walks_away_from_a_wall_it_was_refused_by` pins it
  forever.
- New proof hook `UiAction::PlayerFace { yaw, pitch }`: the body's
  ABSOLUTE facing through the same exec_actions path as
  PlayerTeleport (also serves the carried look-pitch route-framing
  deferral).
- Route proof: `make p3d-pitwall` (`route_pit_wall`): the walk-off's
  own flat dig cell is dug 5 m under the standing body (it falls —
  the walk-off law's sequence); the cell TWO toward −z is dug 4.5 m,
  so the cell BETWEEN becomes a gentle ramp (a one-cell-away dig
  ACCUMULATES on the shared border nodes and tilts the pit deeper —
  the first staging attempt made the body fall TWICE; 
  `find_dig_spot_pair` validates the strip so the step is a step).
  The body holds W into the pit's 5 m wall — REFUSED at the base
  (feet stay 49.49) — then faces the step (pitch down), walks the
  ramp onto the step cell, and is held again by the step cell's own
  outer wall (feet 49.87 = the live step floor; z 130.05, the border
  at 130). Health 100% → 76% from the fall; FELL toast; x2 identical
  + comparator PASS; the four captures INSPECTED.
- Evidence: p3d workspace 667 green / 0 failed (pc3d_render 213 =
  209 + 4 rise laws); `make p3d-walkoff` PASS x2 + comparator
  UNCHANGED (same dig cell — the shared `find_dig_spot` refactor kept
  the pick byte-equal — same fall 56.19 → 51.19, 67%);
  `make p3d-playtest` x2 + comparator PASS with bundle digest
  UNCHANGED (05c46411869a857c — the plaza walk never meets a rise;
  the law engages only where bodies meet walls); `make p3d-climb` x2
  unchanged (same strand (65,20), landed unharmed); `make p3d-steer`
  x2 unchanged (drift 1.53/1.55 m, ortho 0.00 — a falling body's air
  steering now also stops at wall faces instead of clipping into
  them); p3d-smoke OK (digest dd019eca900f5a61 unchanged);
  p3d-assets OK; idle-upgrade-check PASS.
- PERF: no claim, honestly. The law adds at most three `ground_at`
  samples (HashMap lookup + bilinear) per moving axis per frame —
  allocation-free, microseconds against 12 ms frames. The host is
  still contended (a foreign 98.6% CPU process; 15-min load 8.2), so
  no bench attempt was recorded per the 445–448 precedent; the
  quiet-host re-read stays queued (now by four loops).
- LORE: none touched — body traversal physics only. Labor in
  Valdenmoor has consequence: the pit you dig is a pit until you dig
  steps or ramps out; cliff faces of the overgrown wilds are walls to
  route around. No migration (pure in-memory walk predicate).
- Deferred honestly: the player-facing DIG VERB (next_task item 2 —
  now pairs with BOTH 447's geodes and this law); the staged-yield
  crowd route (main.rs is now uncontended); Space jump-off from a
  hang (unit-lawed only); the lethal plaza-recovery branch route; the
  bench contention guard; route-harness contention-hardened capture
  windows; is_water_layer/CTM-strip audit; guardian chronicle
  re-fire.

## 2026-09-13 — The crowd yields: NPC-vs-NPC avoidance (loop 448)

- Closed the last NWR-009 sim-domain deferral ("NPC-vs-NPC
  avoidance — sim's domain"). Until now `npcs::advance` stepped every
  brain blind to the others: two walkers on crossing routes occupied
  the same cell, ghosting through each other in the settlement the
  game asks you to believe is a working place.
- THE LAW (`pc3d_world::npc::step_crowd`, pure): each tick, the held
  set is every body's current cell and the reserved set is the cells
  already claimed this tick; brains plan then move in cast order —
  a walker whose next cell is held or reserved YIELDS: it sidesteps
  one cell around the blocker (perpendicular to the step, then back,
  first free walkable in-patch cell), or stands waiting with path and
  leg intact. Cast order is the only tie-break; the sets are never
  iterated, so the law is deterministic. A vacated cell opens to the
  crowd next tick. `nav.local_of` became pub for the in-patch
  walkability read.
- `NpcBrain::step` split into `plan` + `advance_leg(phase)` with
  `step` exactly plan+advance — lone behavior byte-equal (pinned by
  law). The sidestep relocates the body and re-paths next tick as
  Idle, so a yield cell can never read as an arrival: a Working site
  is only ever the DECLARED site.
- Wiring: `npcs::advance` — the one call site every harness, windowed
  proof, and the live slice steps crowds through — now delegates to
  `step_crowd`. renderer.rs and app.rs untouched.
- Laws: crossing walkers never share a cell on any of 400 ticks and
  both land their declared sites; head-on walkers yield, both arrive,
  and the scenario replays bit-identically; a walker sealed in by
  bodies on all three sidestep cells stands waiting 50 ticks with the
  same cell, path, and leg; a lone brain through `step_crowd` traces
  `brain.step`'s trajectory tick-for-tick; and through the
  render-facing `advance`, a staged head-on pair never shares a cell
  and both arrive (the wiring cannot silently drop the law).
- Evidence: p3d workspace 663 green ON THE COMBINED TREE (loop 446's
  committed step law + loop 447's committed root job + this law;
  pc3d_render 209, pc3d_world 264); `make p3d-people` PASS (12 NPCs,
  7 draws, stride frames differ 1.03% — motion survives the law;
  captures inspected: rigs at distinct cells); `make p3d-playtest` x2
  + comparator PASS with digest UNCHANGED (05c46411869a857c) and the
  chain line identical ("fell 8 m over the open journal (health
  33%)") — the law engages only when bodies actually meet, and the
  playtest's routes never do; p3d-smoke OK (digest dd019eca900f5a61
  unchanged); p3d-assets OK.
- PERF: UNAVAILABLE, honestly. Two deck-bench attempts today read
  9.81/17.41/16.14 then 20.60/36.70/32.56 ms p50 vs 445's clean
  6.85/12.30/12.56 — both under measured concurrent load, both
  marked by a mid>high tier inversion (impossible for a µs-scale sim
  change), both DISCARDED per the standing precedent (446 and 447
  likewise discarded every bench attempt today — the shared host
  never went quiet). DECK-BENCH-REPORT.md + the three deck PNGs were
  RESTORED to HEAD: 445's clean record stands untouched. Analytical
  bound: two ~92-entry set builds + per-brain lookups per crowd tick
  — microseconds against 12 ms frames. The quiet-host re-read stays
  queued (now by three loops).
- LORE: none touched — unnamed settlement brains; the yield is local
  perception (a body yields to the body in front of it), never
  omniscient pathing. No migration (in-memory sim; the sidestep
  reuses Intent::Idle; no persisted field changed).
- Deferred honestly: no windowed ROUTE frames two NPCs yielding (the
  law is unit-lawed + render-lawed; a staged head-on route needs
  main.rs, which loop 446's in-flight work occupied when this job
  started); the bench contention guard.

## 2026-09-13 — The geode wakes: the Old Powers' keepers take their anchors (loop 447)

- Resolved the audit's longest-standing dead-data item by SPAWNING it:
  `GeodeGuardian` and `CinderCrawler` are real `MobType`s in
  `lf_game::mobs` (the authority every other creature lives in), and
  the orphan `lf_npc` structs — whose doc comments referenced biomes
  that do not exist — are deleted.
- New block `ANIMA_CRYSTAL` (id 145, emits violet light [8,5,14]) and
  a new worldgen feature: rare (about 1 chunk in 113) underground
  geodes — sealed crystal-lined pockets stamped through a PURE
  `geode_cell` geometry shared by `generate_chunk` and the proof
  scene, so the picture can never disagree with the world. Laws: rare,
  deterministic, deep (never the surface band, never the lava floor),
  and a BFS seal law (from the pocket's center, the air region is
  exactly its own hollow — no cave breach, no flood path).
- The creatures take their anchors from the world, the way dragons
  settle roosts: one guardian per geode (roost-anchored to its
  crystal, standing in the hollow), crawlers on floored ledges beside
  deep lava (y 6..=12). Both NEVER roll with the night. The guardian
  is canonically "not evil": detect 6 — it defends its hollow, it does
  not hunt the tunnels — and `provoke_guardians` wakes only the nearby
  keepers when one of THEIR crystals is mined. Guardian kills drop
  anima_crystal (2); crawler kills drop coal; mined crystals drop the
  existing anima_crystal item — the Covenant channeler wage recipe now
  has a world source.
- Both keepers are articulated (animal_parts: the guardian a heavy
  crystal-grown quadruped that plods; the crawler a six-legged scuttle)
  with their own procedural skins, rendered through the client's
  standard articulated path.
- EN-ROUTE BUG, found by the scene proof and fixed before committing:
  the hand-counted ui-world-craft atlas consts (TALL_GRASS..LAVA
  160..=164) had DRIFTED as skins were appended — lava silently
  rendered tall-grass art and the surface tufts drew a wolf skin.
  The five consts are now name-derived fns (`layer_of`, the codebase's
  own anti-drift cure) + regression law
  `lava_and_surface_decorations_render_their_own_art`.
- Proof: vistest scene `geode_guardian` — a quarry cutaway stamped by
  the REAL geode fn, the guardian standing in its crystal hollow, the
  lava pool sunk in the floor, the crawler scuttling beside it;
  composed over 9 inspected renders, then the FULL battery: 108
  scenes PASS. Root workspace 480 green (476 + 7 new laws - 3 dead
  lf_npc tests); smoke OK.
- PERF: settle scans are frame-gated (every 180 frames, staggered
  from the dragon pass) with bounded scan windows. `make perf` read
  90.1 ms p50 under load-35 contention (15-min average 39 — a
  concurrent session's batteries) — DISCARDED as uncontrollable, not
  comparable to 312's clean 47.7 ms; the perf harness (static scene
  renderer) executes none of the changed code paths (settle passes
  live in the client tick).
- En-route observation recorded honestly: loop 446's route_walk_off,
  re-run independently at load 35 (28.7 ms p50 frame time), confirmed
  every physical claim (exact dug-floor landing 51.19 eye = 49.49 feet
  + 1.7, wounded, FELL toast; deterministic x2 + comparator PASS) but
  missed the frame-indexed mid_air capture window at 3x frame time —
  the route harness's frame-indexed windows are contention-sensitive;
  hardening deferred (added to carried deferrals).


## 2026-09-13 — The walk-off lives: the step law + the dug-floor route (loop 446)

- **Proof-discovered bug, fixed:** the loop-444 walk-off commit was
  dead code on the live streamed-surface path. The walk's Y-snap in
  `walk_on_speed` bound to ANY ground answer, and the streamed
  surface's `ground_at` answers the true height everywhere (it
  ignores `from_y`) — so the frame a body crossed a ledge edge it
  was teleported to the bottom INSTANTLY, damage-free; the commit's
  `is_unsupported` never saw a gap. A unit probe on a
  streamer-shaped ledge reproduced it exactly (y snapped 10.0 → 7.5
  in one frame).
- **THE STEP LAW** (player.rs, where the walk lives): the snap holds
  the body only within one step — `feet − ground ≤ SUPPORT_GAP_M`
  (1.05 m); deeper, the walk REFUSES and the gap stands for the
  slice's commit, which flips the same airborne arc a jump uses.
  `SUPPORT_GAP_M` + `is_unsupported` moved to player.rs (re-exported
  from app.rs); the slice's airborne machinery (commit, hang, jump,
  arc) was HOISTED out of the rebuild-only branch, so the law now
  holds on BOTH walk paths (streamed rebuild + legacy authority).
  New unit law: `the_walk_holds_a_step_and_refuses_a_ledged_drop`
  (0.5 m step walked; 2.5 m ledge refused + `is_unsupported` true).
- **The honest finding about natural ledges:** the streamed surface
  interpolates at 1 m nodes, so even the gen's vertical 4 m terrace
  faces ramp into ≤ 4 m/m slopes — walked down at 6.7 cm per frame
  under the walkability contract. No WALKED natural edge can make
  the per-frame gap; the law's own second trigger can: "a floor dug
  out". The route therefore digs.
- **The dig, live:** `UiAction::EditSurface` →
  `Renderer::surface_edit` → `SurfaceStreamer::edit` +
  `remesh_now` — the delta layer edits the streamed surface and
  every touched patch is remeshed immediately, so collision AND the
  drawn ground answer the edit in the same frame (the foundation of
  a player dig verb; the delta layer already persists with the
  slice save).
- **`route_walk_off`** (`make p3d-walkoff`): the body stands over a
  flat, vine-free cell near the showcase plaza (deterministic
  ring-first search; vines refused in the grip's whole ±2-slot
  scan). The dig lowers the cell 5 m; the step law refuses, the
  commit fires, and the arc lands the body straight down — no keys,
  no teleports after the grounded start. Measured: feet 54.49 →
  49.49 (the dug floor EXACTLY), health 100% → 67-68% (the impact
  law's ~35% for 5 m, minus regen), the FELL toast in the landed
  capture. PASS x2 + comparator PASS; the three captures are
  inspected (standing full-health / mid-fall on the dug wall with
  "THE GROUND GIVES WAY" / landed at ~2/3 health with "FELL —
  HEALTH 64%").
- Regression: playtest x2 chain UNCHANGED (ends "fell 8 m over the
  open journal (health 33%)"; the house-door walk survives the step
  law); vine climb PASS x2 (same strand (65,20), health 100%);
  air steer PASS x2 (drift 1.49-1.55 m, ortho 0.00); p3d-smoke OK
  (digest dd019eca900f5a61 unchanged); p3d-wilderness PASS;
  p3d-assets OK; p3d workspace 658 green (pc3d_render 208 — +1 for
  the step law); root workspace green (479 on the combined tree —
  see the honesty note in STATE). Deck bench: see DECK-BENCH-
  REPORT.md (refreshed only if an uncontended window allowed it).
- LORE: no canon data touched — body traversal physics only; the
  falls of the Age of Reckoning answer the hand (444) and the
  ground (446). No migration (in-memory runtime state + the
  surface delta layer that already saved).

## 2026-09-13 — The bench tells the truth: make p3d-deck-bench argv fix + report refresh (loop 445)

- `make p3d-deck-bench` without an explicit `SEED=` silently
  benchmarked the WRONG tier: the target expanded `$(SEED)` empty and
  unquoted, the shell dropped the argument, and the binary's argv
  shifted by one — out_dir consumed the tier name (`low`/`mid`/
  `high`/`report` became stray capture directories in the repo root,
  committed as debris in loop 444) and the tier selector fell to the
  `_ => mid` arm, so EVERY "tier" run was a mid run writing the same
  `deck_bench_mid.csv`, and the `report` invocation ran a fourth mid
  bench instead of assembling the documented report.
- Consequences, stated honestly: DECK-BENCH-REPORT.md was last
  written by loop 442 (443's "DECK-BENCH-REPORT.md + bench PNGs
  refreshed" claim did not hold on disk — verified: 441-444 touched
  neither the report nor `shots/windowed_deck_*.png`, last refreshed
  in 440); 443/444's quoted bench rows were mid-tier numbers from
  mislabeled runs (444's low-tier reading was also CPU-contended by a
  concurrent process and documented as such at the time). No
  conclusion changes: the corrected per-tier numbers land in the same
  noise band the loops claimed.
- Fixed: the target now defaults the seed (`$(if $(SEED),$(SEED),3)`
  — the p3d-soak/p3d-journey pattern); the stray `low/`, `mid/`,
  `high/`, `report/` debris directories are removed; a clean,
  uncontended run refreshed DECK-BENCH-REPORT.md and the three
  per-tier captures: low 6.85 / mid 12.30 / high 12.56 ms p50 (p95
  8.10/14.85/14.06) vs 442's 6.89/12.16/12.44 — noise-sized; the
  Low-not-slower contract law held.
- No Rust code changed; the p3d test suite (657) and root workspace
  (476) stand green on the same binary. En-route: this loop's
  orientation overlapped a concurrent session finishing loop 444
  (committed + pushed mid-verification as f879fc3/b3a5f23); this
  loop's independent verification of that work agreed (fresh
  `make p3d-steer` PASS x2 + comparator, drift 1.56 m along the
  strafe / 0.00 m across, health 68% -> 39%, captures inspected).

## 2026-09-13 — The air steer: the fall answers the hand (loop 444)

- Closed the standing 441/443 deferral ("mid-air steering is now
  routable but still untested as a law"). A falling body keeps
  lateral control: `lateral_speed` is the pure speed selector —
  the full walk (or sprint) on the ground, `WALK_SPEED *
  AIR_STEER_FRACTION` (0.45 → 1.8 m/s) in the air regardless of
  sprint. The fall is a commitment, not a glide: the vine grip's
  12 m lethal drop can drift at most ~2.8 m — enough to line up a
  strand catch, never enough to erase the drop.
- Real bug found en route and fixed in the same path: a body that
  WALKED off a ledge never entered the arc — the walk's ground
  window missed, Y never snapped, gravity never engaged, and the
  body hover-glided at full walk speed over any drop deeper than a
  step. The new walk-off commit (`is_unsupported`, gap > 1.05 m)
  flips the SAME airborne state a jump uses, with vy 0, reading the
  SAME `ground_y_at` answer the arc lands on; hanging bodies are
  excluded (the strand owns them). Sprint no longer engages
  mid-air, so falls no longer burn stamina either.
- Laws (pc3d_render 201 -> 207): the selector matrix; a falling
  body drifts one steer-speed per second while an unsteered fall
  holds its line exactly; the air drift is the walk scaled by the
  fraction (measured ratio 0.45 ± 0.01); the same drift at 30/60/
  120 fps; steering into the strand catches what the free fall
  misses (composition with the grab law); the walk-off commits
  past one step and not before.
- Proof: `make p3d-steer` — route_air_steer drops the player twice
  from 5 m over the plaza banner (Plains grows no vines BY DESIGN).
  The free fall's mid-air pose (recorded through the camera, which
  the slice locks to the body) equals the drop XZ to < 0.05 m; the
  steered fall (A held via key_script, released before the 60 fps
  landing so no ground walk pollutes the drift) moved 1.50 m along
  the strafe axis, 0.00 m across it; both drops landed real wounds
  (health 68% -> 38% — two 5 m impacts; the bands exclude a plaza
  recovery's exact 0.5). PASS x2 identical + comparator PASS; the
  four captures are pixel-separated and inspected (the birch trunk
  entering the held mid-air frame IS the drift).
- Regression: playtest x2 digests UNCHANGED (05c46411869a857c);
  vine climb PASS x2 unchanged (same strand, health 100% — the
  tip-release fall steers nothing without keys); p3d-smoke OK
  (digest unchanged); p3d-wilderness PASS; p3d-assets OK. Deck
  bench mid 12.39 / high 12.36 ms p50 vs 443's 12.30-12.38 —
  noise; the bench attaches no slice (verified in its config), so
  the walk change has no bench cost path; the low tier read
  contended (a concurrent 100%-CPU test process from another
  session) and is not comparable this pass. p3d workspace green
  (pc3d_render 207); root workspace green (476); idle-upgrade-
  check PASS.
- Deferred honestly: the walk-off commit is live-wired + unit-lawed
  but has no dedicated route (needs a deterministic > 1 m ledge;
  the plaza is flat by design); Space jump-off from a hang is still
  unit-lawed only; the look-pitch script hook for the climb framing
  is still open; the lethal plaza-recovery branch still has no
  route proof.

## 2026-09-13 — The vine grip: the hanging strand becomes a traversal verb (loop 443)

- Closed 442's natural follow-up: the vine was placement+render only;
  now a falling body that passes a drawn strand within hand reach
  CATCHES it. The grab law is pure and strict: only a DESCENDING body
  grabs (strolling under the canopy is free), lateral hand reach
  0.55 m, and the body must be inside the strand's span OR have
  crossed the attach this frame — a fast drop cannot tunnel through
  the mesh it visibly passes through. The hang zeroes the fall, so
  the only drop that counts afterward is the one below the grip.
- The strand truth is the DRAWN mesh: every loaded mesh records its
  lowest vertex (min_y per kind+variant, canonical included) and
  `vine_grip_near` answers the nearest strand's anchor line, attach,
  and drawn tip (extent x jitter scale) from a ±2-slot scan — the
  grip can never disagree with the picture. `step_hang` climbs on the
  same FIXED 1/60 s step as the fall arc: W toward the attach (clamps,
  never above), S toward the tip (releases past it — the arc resumes,
  vy from zero), the same release height at any refresh rate. While
  hanging, the strand owns the body (walk XZ undone); Space lets go
  with the SAME 4.6 m/s hop a ground jump commits; the grab runs
  OUTSIDE the gameplay gate like the arc it interrupts.
- Route framework: `WindowConfig.key_script` injects real gameplay
  keys through the real input path for [first, release) frames — the
  route's W/S ride exactly what a player's keys ride.
- Laws (pc3d_render 193 -> 201): grab arrests / refuses
  (rising, distant, above, below), the crossing anti-tunnel catch,
  stroll-under never grabs, climb clamps at the attach and releases
  exactly at the tip, same release at 60 and 120 fps, and the
  composition law `the_catch_saves_the_body` (an uncaught 12 m drop
  lands at ~15.3 m/s = lethal; the same drop caught and released at
  the tip lands at ~3.6 m/s = free) plus `the_grip_is_the_drawn_strand`
  (the grip hangs from the attach, <= 0.75 m lateral, a real
  0.3-2.6 m span, deterministic).
- Proof: `make p3d-climb` — route_vine_climb searches the showcase
  seed region-first (the plaza is Plains, which grows no vines BY
  DESIGN), drops the player 12 m above a real strand (uncaught =
  lethal), captures air/grip/climb/landed, and asserts the grip toast,
  pixel-separated frames, and final health >= 95%. PASS x2 identical
  (strand at slot (65,20), anchor 258.6/55.9/82.0) + comparator PASS;
  captures AI-inspected. En-route: playtest x2 digests UNCHANGED
  (05c46411869a857c — no vines at the Plains plaza, the live fall is
  untouched); wilderness CANOPY unchanged; p3d-smoke/assets OK; deck
  bench mid 12.30-12.38 ms p50 vs 442's 12.16 — noise-sized; p3d 651
  green; root workspace 476 green; idle-upgrade-check PASS.
- Deferred honestly: the climb route's canopy-interior frames are
  honest but busy (the eye sits inside the pine's skirt; a look-pitch
  script hook would frame the strand itself); Space jump-off is
  unit-lawed only; mid-air steering is now routable (key_script
  exists) but still untested as a law.

## 2026-09-13 — The vine anchor concept: the last catalog-only family grows in the wild (loop 442)

- Closed loop 440's deferral: the 40 vine GLBs that loaded but never
  grew now hang from living canopies. The mesh hangs DOWNWARD from
  y=0, so the slot grid gained the missing concept — an ANCHOR: a
  vine exists only where a fixed-order 8-neighborhood scan finds a
  living broadleaf or pine (dense canopies anchor; the birch's airy
  crown and dead wood carry nothing). Open Plains grows no vines by
  design; vines joined the Forest (0.010) and Highlands (0.004)
  density tables.
- `vine_anchor` answers the attach point: 0.5-0.7 m lateral of the
  ANCHOR trunk (biased toward the vine slot, inside every family's
  canopy reach) at the anchor's OWN ground plus the family hang law —
  pine 1.5 m inside the skirt cone, broadleaf 2.9 m at the crown
  underside — chosen from the canonical AND variant sweep geometries
  (pine h 4.2-7.5, broadleaf trunk 2.1-3.5) so no drawn variant
  floats. The renderer places the vine instance at that point (every
  other kind stands on the slot-center ground); `plant_at` split into
  a recursion-free `table_pick` + the anchor gate so a vine scan can
  never trigger a vine scan.
- Wind: scene.wgsl's sway weight is now |mesh y| — a hanging strand's
  TIP swings under its fixed anchor while every upward mesh is
  byte-identical. Vine wind 0.35.
- Laws: `vines_need_a_living_canopy_anchor` (34 grown, 13 anchorless
  picks refused in one Forest region; attach hugs the trunk, the hang
  law exact, deterministic), `hang_laws_follow_the_canopy_shape`, and
  the GPU `the_vine_hangs_at_its_anchor_and_sways` (presence diff
  0.154, tip sway 0.00061 with grass excluded from frame). The kind
  tag/variant-diversity/load laws cover the 30th family through the
  existing ALL-driven tables.
- Proof: `make p3d-wilderness` PASS with a 6th CANOPY capture — the
  pose TRAVELS (region-first search, the landmark_at shape; the
  SmoothHills scene is Plains, which grows no vines BY DESIGN), meshes
  the vine's own ground, swaps it in for the close-up and restores the
  vista for the low-tier shot. AI-inspected: the strand hangs from the
  pine's branch and is legible at walking height.
- Perf: deck bench low 6.89 / mid 12.16 / high 12.44 ms p50 vs loop
  440's 6.79/12.36/12.62 — noise-sized; flora 601→605 instances on the
  walk; the near-field variant rule holds. Report refreshed.
- En-route fix: the p3d-806 scale proof's linear-scaling judgment
  tripped once under the suite's own parallel load (a mean-of-5
  wall-clock). The linear law now judges best-of-k tick cost (the
  uncontended standard); the 20 ms sustain budget stays on the mean.
- Deferred honestly: grass_tuft GLBs stay catalog-covered by the
  deliberate Deck-cheap card field; the CANOPY capture frames one
  specimen (a wider drape framing is a polish pass); vine climbing is
  a traversal verb, not shipped here.

## 2026-09-13 — The live fall proof: gravity is the world, not the menu (loop 441)

- Closed the standing 438 deferral: the semantic playtest drops the
  player 8 m onto the plaza with the quest journal still open and the
  run ends wounded — `fell 8 m over the open journal (health 33%)`
  joined the chain line, x2 identical bundles + comparator PASS.
- The investigation found why the fall never executed — two real
  bugs, not test flakiness: (1) the airborne integration lived
  inside `if gameplay_active`, so any open panel froze the arc
  mid-air (the camera reads feet+1.7 m = EYE_ABOVE_FEET — the
  "+1.7 m, zero gravity" freeze in the 438/439 notes); (2) Space was
  dead for real players — jump velocity was granted only when not
  falling and gravity only ran when falling, while the walk's ground
  snap kept everyone glued.
- The fix: `integrate_air_arc` (pure, unit-lawed) integrates gravity
  on the FIXED 1/60 s step (substeps scale with the frame's real dt,
  clamped 1..8 — the same arc at any refresh rate) and lands on the
  PER-FRAME ground answer of the current column (no stale
  at-drop-time capture; under-terrain drops self-heal). The arc runs
  OUTSIDE the gameplay gate — a panel freezes input, never a fall
  already in progress. One Space branch commits the same airborne
  state a drop uses; the duplicate grant and `fall_ground_y` are
  gone. The OBSERVE line now reports PASS/FAIL from the real verdict
  flag (it printed PASS unconditionally).
- Three new laws (pc3d_render 189 → 192): the jump arc leaves the
  ground and lands safe; a drop lands on the CURRENT ground with the
  free-fall impact; the arc is the same fall at 60 and 120 fps.
- Route evidence: air capture (full health, panels up, airborne),
  landed capture (health bar ~1/3, `FELL — HEALTH 33%` toast, ground
  level), final capture (damage persists, journal rows live);
  assertions exclude both the dead arc (1.0) and the plaza recovery
  (0.5). p3d workspace 640 green; smoke, assets, root suite,
  idle-upgrade-check all green.
- Deferred honestly: the lethal plaza-recovery branch has no route
  proof; mid-air steering is unchanged and untested as a law; the
  jump is proven by the unit arc law and the shared live integration,
  not its own route.

## 2026-09-12 — The thousand-asset families go wild (loop 440)

- Closed loop 439's deferral: 20 of the 22 new asset families (920
  GLBs) now grow in the played world, biome-appropriately — forest
  undergrowth (ferns, blooms, fungi, glowcaps, deadfall, roots, moss
  rocks, brambles), wetland reeds and bog snags, blooming plains,
  pebbled coasts, the ruin-and-resonance heights (columns, cairns,
  arches, crystals, obsidian) and snowpeak ice shards.
- `PlantKind` 9 → 29 kinds with a shared `is_tree()` class (dead trees
  keep the spacing/slope gates), a walkability contract, per-kind wind,
  and biome tables that preserve every standing character law; the
  renderer's kind loop now iterates `PlantKind::ALL` so world and
  picture cannot drift, and each family's canonical base is its `_v00`
  variant.
- New laws: the expansion census (every table family actually grows in
  sampled regions of every found land biome), kind-tag uniqueness, a
  startup contract loading every drawn kind's base through the real
  mapping, and a GPU `kinds_drawn >= 12` law.
- Perf law the deck bench enforced mid-cycle: variants are a near-field
  (lod0, 40 m) detail — 29 families had fragmented the draw into 240
  one-handful buckets (deck p50 doubled to 50 ms); the rule returns the
  walk to HALF the pre-change baseline (low 11.22→6.79, mid
  23.44→12.36, high 24.12→12.62 ms p50, 399→601 instances in 111→57
  buckets). `make p3d-wilderness` gained an UNDERGROWTH capture that
  searches for and frames an expansion family at walking height.
- Makefile hygiene found en route: `p3d-assets`' `PATH=` parameter
  shadowed the environment PATH (always failed in a normal shell;
  renamed `MANIFEST=`), and `make help` never listed digit-named
  targets (regex fixed; all 61 targets documented).
- Evidence: p3d suite 637 green; wilderness proof PASS (5 captures,
  undergrowth AI-verified); playtest ×2 identical + comparator PASS
  (chain unchanged); smoke + asset gates OK; root workspace suite
  green. Deferred honestly: `vine` (hangs downward — needs an anchor
  concept), `grass_tuft` GLBs (the card field is the deliberate cheap
  path).

## 2026-09-12 — Perpetual ZCode idle-upgrade and lore-canon pack

- Added `docs/POORCRAFT-3D/ZCODE-IDLE-UPGRADE/`, centered on a paste-ready
  idle-time task that completes one evidence-backed upgrade per invocation,
  preserves a durable next task, and never treats the game's design or
  development as terminally complete.
- Consolidated the existing game documents and prior direction into a
  977-line canon bible. It locks Valdenmoor, Era IV Year 1, Anima, the
  deliberately unresolved Ruin, the four eras, the six factions, named places,
  player freedom, NPC personhood, faction consequence, water-led industry,
  first-person war, visual grammar, and an explicit owner-only canon migration
  process.
- Added a measured upgrade decision framework for performance work, third-party
  Rust dependencies, new internal crates, platform seams, and honest
  console-readiness claims, plus `autonomous_upgrade_contract.json` for
  machine-readable enforcement.
- Added `xtask idle-upgrade-check`, two Rust guardrail tests, and the matching
  `make idle-upgrade-check` target. Evidence: 4 Markdown documents / 77,870
  bytes, 5 local links, 8 canon locks, 9 proof gates; focused tests 2/2; root
  `cargo build --workspace` and full `cargo test --workspace` green (including
  UDP and the 985.28-second exhaustive world-generation test).

## 2026-09-11 — WT-008 data extraction plugin lab

The GLM world/tools pack now has an eighth implementation-ready subpack:

- `docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-008-DATA-EXTRACTION-PLUGIN-LAB/`
  defines the local-safe inspection layer GLM needs to stop guessing from
  screenshots alone: scene graph exports, UI layout, asset manifests,
  mesh/material dumps, player/NPC/machine/worldgen state, perf counters,
  evidence bundles, screenshots, and optional debug OBJ/GLB exports.
- The subpack authorizes mods/plugins/exporters and loopback MCP-style tools
  only inside explicit local output directories, with no public listener, no
  secret capture, no save mutation by default, and no "AI inspected it" claim
  unless a real exported file is listed in the evidence bundle.
- Eight JSON contracts cover plugin manifests, exporter surfaces, inspector
  endpoints, telemetry signals, sample mod packs, data safety, extraction
  evidence, and the WT-008 manifest. `pc3d_assets` parses them through the
  world/tools guard and preserves the law that data extraction must be local,
  safe, and proof-oriented.

## 2026-09-11 — THE SAMURAI CUT: the UI layer was ONE triangle

The owner's "menus/buttons/text fields diagonally cut" was a second,
coarser diagonal-cut bug beside the fixed row-shear/pitch bug: both UI
quads (owner UI layer + HUD debug line) were drawn with 4 vertices on
TriangleList pipelines — one triangle, so the entire bottom-right half of
every screen showed raw world along the top-right→bottom-left diagonal.
Fixed with 6-vertex quads, a new byte-exact "quad coverage" proof law in
--ui-shots, an sRGB texture view so the UI is no longer double-gamma
washed-out on the sRGB swapchain, and two label-overflow fixes the cut had
been hiding (new-world quality steppers, load-world timestamps under the
LOAD button). p3d 592/592; gates 10/10; journey/smoke digests unchanged.

## 2026-09-11 — WT-007 GPU vendor cookbook

The GLM world/tools pack now has a seventh implementation-ready subpack:

- `docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-007-GPU-VENDOR-COOKBOOK/`
  defines the measured route for AMD/NVIDIA improvements: neutral wgpu markers,
  timestamp capability checks, deterministic capture scenes, vendor capture
  gates, upscaler prerequisites, shader debug status, and before/after proof.
- The subpack turns AMD RGP/FSR and NVIDIA Nsight/DLSS guidance into project
  rules: no vendor optimization claim without marker audit, screenshots,
  p50/p95/p99 metrics, same-scene comparisons, and preserved gameplay/UI gates.
- Seven JSON contracts cover vendor toolchain, marker matrix, capture scenes,
  upscaler readiness, before/after evidence, shader debug status, and the
  cookbook manifest. `pc3d_assets` parses them through the world/tools guard
  and preserves the law that vendor GPU claims require before-after capture
  evidence.

## 2026-09-11 — WT-006 Z-code continuous runbook

The GLM world/tools pack now has a sixth implementation-ready subpack:

- `docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-006-ZCODE-CONTINUOUS-RUNBOOK/`
  defines the operating loop for long GLM/Z-code sessions: orient, select one
  proofable task, implement, test, capture evidence, compare to contract, fix
  failures, bookkeep, stage intended files, commit, push, and continue.
- The subpack adds task priority rules, proof-first language, failure recovery,
  commit/bookkeeping discipline, sprint cards for WT-001..WT-005, and
  no-excuses laws for mouse/Escape/screenshots/wireframes/gameplay semantics.
- Eight JSON contracts cover the runbook manifest, operating loop, task
  priority, proof gates, failure recovery, sprint cards, evidence bundles, and
  completion audit. `pc3d_assets` parses them through the world/tools guard and
  preserves the law that Z-code must continue through proofable green
  checkpoints.

## 2026-09-11 — WT-005 asset prompt atlas

The GLM world/tools pack now has a fifth implementation-ready subpack:

- `docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-005-ASSET-PROMPT-ATLAS/`
  defines prompt libraries and batch rules for original buildings, NPCs,
  creatures, machines, items, UI sprites, keycaps, bars, map markers, world
  props, resources, and environmental storytelling.
- The subpack adds style/source rules, glTF/Blender import rules, UI alpha and
  runtime-text rules, material/rig/texture requirements, deliberate rejection
  examples, and acceptance gates so mass generation produces importable
  gameplay assets instead of pretty junk.
- Nine JSON contracts cover prompt batches, naming taxonomy, UI sprites, glTF
  import, material palette, animation rigs, texture compression, batch
  acceptance, and the atlas manifest. `pc3d_assets` parses them through the
  world/tools guard and preserves the law that asset prompts must produce
  importable playable assets.

## 2026-09-11 — WT-004 feature expansion matrix

The GLM world/tools pack now has a fourth implementation-ready subpack:

- `docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-004-FEATURE-EXPANSION-MATRIX/`
  converts broad feature brainstorming into proofable Z-code slices across
  worldgen, seed preview, NPCs, factions, settlements, industry, magic,
  survival, combat, exploration, UI/HUD, accessibility, mods, multiplayer,
  Steam, tools, and validation gates.
- The subpack adds feature rules, slice ordering, acceptance criteria, failure
  modes, and proof-gate requirements so future breadth does not become a
  wishlist with no runtime behavior.
- Eleven JSON contracts cover the feature expansion manifest, backlog matrix,
  worldgen, NPC/factions, industry/magic, UI/HUD, survival/combat,
  quest/story, multiplayer/Steam, modding/tools, and proof gates. `pc3d_assets`
  parses them through the world/tools guard and preserves the law that broad
  features must be sliced into proofable gameplay.

## 2026-09-11 — WT-003 game observatory MCP lab

The GLM world/tools pack now has a third implementation-ready subpack:

- `docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-003-GAME-OBSERVATORY-MCP-LAB/`
  defines local MCP-style tools and CLI-equivalent contracts so GLM can inspect
  the real game instead of guessing from code or stale screenshots.
- The subpack covers runtime state exports, screenshot/wireframe/overlay
  captures, input replay routes, mesh/material dumps, GPU markers/timestamps,
  regression comparisons, evidence bundles, and local-only safety limits.
- Ten JSON contracts cover tool surfaces, runtime state, screenshot scenes,
  wireframe overlays, asset dumps, input routes, GPU markers, regression gates,
  and evidence bundle schemas. `pc3d_assets` parses them through the
  world/tools guard and preserves the law that GLM must inspect real game
  evidence before claiming success.

## 2026-09-11 — WT-002 semantic asset factory lab

The GLM world/tools pack now has a second implementation-ready subpack:

- `docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-002-SEMANTIC-ASSET-FACTORY-LAB/`
  defines a playable asset factory, not a decorative screenshot pile:
  buildings need enterable doors/interiors, NPCs need talk states, forges need
  input/output/heat/workspot behavior, and every asset needs inspection proof.
- The subpack adds a GLM prompt, owner brief, semantic rulebook, brainstormed
  catalog, interaction anchor specs, inspector/wireframe capture rules,
  material/LOD/collision rules, AMD/NVIDIA-informed profiling gates, tool
  commands, implementation slices, acceptance checklist, failure modes, and a
  deterministic screenshot playtest route.
- Nine JSON contracts cover the asset factory manifest, queue, gameplay
  catalog, affordance schema, inspection exports, GPU captures, tool commands,
  playtest evidence, and iteration budgets. `pc3d_assets` parses them through
  the world/tools guard and preserves the law that asset factory outputs must
  be gameplay-semantic.

## 2026-09-11 — WT-001 seed preview execution pack

The broad GLM world/tools pack now has a concrete first implementation job:

- `docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-001-SEED-PREVIEW-HARNESS/`
  contains the paste-ready GLM prompt, implementation brief, Rust API sketch,
  New World wireframe, deterministic preview algorithm, screenshot gates,
  Z-code commands, failure modes, and acceptance checklist.
- Machine contracts define the execution plan, UI wireframe, output sidecar
  schema, and screenshot/test matrix, so a future Z-code pass can prove same
  seed stability, different-seed change, spawn safety, preview metadata, and
  screenshot evidence.
- `pc3d_assets` now parses those subpack JSON contracts in the world/tools
  guard test, and the parent manifest enumerates every WT-001 file.

## 2026-09-09 — GLM UI REWORK: the real owner UI (UI-001..UI-008)

The temporary bitmap-font owner shell is replaced by a real UI layer,
executed end-to-end from docs/POORCRAFT-3D/GLM-UI-REWORK-PACK:

- `pc3d_render::ui` — state model, draw list, pure CPU painter, pure
  input reducer; rendered as one fullscreen alpha-blended canvas after
  the world (`Renderer::set_ui_layer`).
- Title / New World / Load World / Settings / Pause screens with mouse
  + keyboard navigation; confirmation modals own the frame; Escape
  pauses/resumes and never exits; quitting is always explicit.
- Live gameplay HUD: health/stamina/food bars, XP strip, 9-slot
  build-palette hotbar (selection drives F), crosshair, prompt,
  toasts; F3 debug strip hidden by default.
- Settings drive the runtime (sensitivity, invert Y, FOV, UI scale,
  quality) and persist; save slots list saves3d with seed + date and
  confirm before load/delete.
- `--ui-shots` (11 deterministic scenes + pixel checks + layout dumps)
  and `--ui-inspect` (local JSON inspector with input replay proving
  the Escape law).
- Visual battery grew to 10 gates (`ui-states`); capability inventory
  updated in the same commit (guardrail green). p3d 457/457, root
  474/474. DMG rebuilt; route proof passed from the mounted volume.

## 2026-09-08 — NWR-011: the rebuild vertical slice

The full rebuild stack on one deterministic, human-walkable route:

- `slice::assemble_rebuild`: streamed surface terrain + Mid atmosphere
  + instanced wilderness + the settlement kit (with river dock and
  water wheel) + the rigged crowd on the real schedule + conforming
  water + the cave mesh. The player WALKS THE SURFACE
  (`SurfaceStreamer: CollisionSurface` — the NWR-005 deferral lands),
  and F builds only on INSPECTED foundations (rejections name the
  reason on the HUD).
- `--play-rebuild [live]` / `make p3d-rebuild` / `make
  p3d-rebuild-live`: five route captures (vantage, street, river,
  cave, gate) with probes at ~104 fps; the construction save/reload
  round trip proven in-arm; the live smoke alive 12 s; the manual
  REVIEW-CHECKLIST.md with the explicit OWNER GATE (not a commercial
  beta until the human owner passes it in person). 9/9 gates; p3d
  421/421, root 474/474.

# CHANGELOG

## 2026-09-08 — NWR-010: the Deck quality contract

- `pc3d_render::deck` (new): the transparent Low/Mid/High contract
  composing every tier lever — streaming rings/mesh-work/GPU budget,
  atmosphere shadow/fog/detail/glint, flora + grass radii, and a new
  crowd pose-rate lever (Low 15 Hz, quantized-deterministic) — with
  honest rows for render scale (1.0, no scale-swap path) and the
  same-world/interactions law (anchors, characters, water, edits
  visible at every tier).
- `--deck-bench <seed> <outdir> [low|mid|high|report]` /
  `make p3d-deck-bench`: a four-waypoint walk over the full stack per
  tier (one process per tier — winit allows one event loop), frame
  percentiles + streamer/scene counters, sidecars, and the assembled
  documented report enforcing the Low-not-slower law.
- MEASURED (800x500, Apple host iGPU evidence machine): low 167 fps /
  5.1 MB / 176 patches; mid 149 / 18.9 MB / 402; high 146 / 27.6 MB /
  603 — levers measurably behave; caps held; low capture
  human-inspected readable. Report on disk at
  `docs/POORCRAFT-VALHEIM-STYLE-REBUILD/DECK-BENCH-REPORT.md`.
  9/9 gates; p3d 421/421, root 474/474.

# CHANGELOG

## 2026-09-08 — NWR-009: the people

The NPCs become limbed characters over the authoritative brains:

- `pc3d_render::npcs` rig (new): a low-poly limb NPC with role
  variants (guard helm+spear always readable; worker tool only while
  Working; resident satchel) and a DETERMINISTIC animation state
  machine — `rig_pose(brain, t)` is pure f(intent, time): walking
  swings counterphase legs with the yaw facing the path leg from the
  sim's own path; work strokes the arm; sleeping lies low; idle
  breathes. Impostor boxes beyond 64 m.
- Drawing: a new `inst_box` pipeline (unit cube + per-axis instance
  scale), one bucket per part color — a crowd of ANY size is ≤ ~8
  draw calls (12-NPC proof: 7 draws / 92 instances). Poses rebuild per
  frame from the shared frozen-able clock; positions always from
  `npc_world_pos(brain)` — no visual NPC simulation. `crowd_tick`
  advances the authoritative brains; `CrowdGround` gives chest-high
  capsule collision.
- Proofs: pose determinism + intent laws, sim-following instances,
  the capsule stop, GPU control-diff 0.0725 + animation 0.0112
  between frozen times. Windowed `--play-people` /
  `make p3d-people` (p50 ~6 ms with atmosphere + flora + kit +
  crowd): plaza/stride/guard/anchors human-inspected PASS; the
  existing npc-cast gate stayed green. 9/9 gates.

# CHANGELOG

## 2026-09-08 — NWR-008: the settlement kit

The city graduates from procedural prisms to an original modular kit:

- `tools/assetgen`: ten GLBs with DECLARED SOCKETS in the GLB nodes —
  house, workshop, market stall, wall segment (wall_a/wall_b chains),
  gate arch (passage + wall flanks), watchtower, keep (banner_top),
  bridge/dock, banner sign, water wheel. Byte-identical regen within
  budgets (the wheel's doubled spoke layer was caught by the 500-tri
  budget and trimmed).
- `pc3d_render::settlement` (new): `assemble_kit` maps every
  authoritative kind (exhaustive match) to a kit module at its plan
  cell — one placement per element; walls chain socket-to-socket along
  the 3 m circuit; collision/nav/anchors reuse mesh_city for parity,
  with the gate REFINED so its passage column opens; Bed/Work/Idle
  zones render as colored markers; dock + water wheel place at the
  nearest river. Drawn instanced through the flora pipelines with sun
  shadows. `SettlementGround` adapter (its height window rides the
  inner surface's ground — the gen-terrain window ghosted walls on
  flat stages until the walk test caught it).
- Proofs: plan→render consistency + assembly determinism (the
  save/load stability law), socket chains <0.35 m, collision/nav
  parity with exactly the gate passages opened, the player walks
  through the gate and is stopped by walls, GPU control-diff 0.019
  with 12 LOD buckets. Windowed `--play-settlement` /
  `make p3d-settlement` (p50 4.81 ms with atmosphere + flora + kit):
  overview + street human-inspected PASS. Inventory honesty kept
  (10 new GLBs recorded; the guardrail unions all three packs).
  9/9 gates.

# CHANGELOG

## 2026-09-08 — NWR-007: the wilderness

Instanced wilderness from a pure placement authority:

- `pc3d_world::flora` (new, pure): nine plant kinds on a 4 m slot grid,
  each slot an FNV hash of (seed, slot) gated by a per-biome density
  table; trees keep 8 m spacing (even-diagonal subgrid) and refuse
  steep slopes; one standing-stone LANDMARK may exist per region (~18%
  hash gate, gentle ground); `jitter()` gives render variation from
  the authority. Ocean grows nothing.
- `tools/assetgen`: nine original GLBs — conical pine, billowing
  broadleaf, pale birch; boulder, fractured spire, flat slab; shrub;
  fallen log with bracket fungi; the leaning standing-stone landmark.
  Byte-identical regen, budgets respected; `wilderness_batch.json`
  validates against files on disk (schema v2 gained the flora/landmark
  categories rather than weakening the id law).
- `pc3d_render::flora`: ONE draw per (kind, LOD) bucket (~830
  instances in 17 buckets), LOD by the glb thresholds, wind-animated
  grass cutout cards, a BOUNDED rotating slot-scan budget with cached
  negatives, eviction beyond ring+16 (teleport/reload proven: the same
  ring returns), instance sun shadows, and a FloraGround collision
  adapter derived from the authority (the walk test stops at a trunk;
  no ghosting by construction).
- Proofs: world placement laws (determinism, spacing, biome character
  over searched regions, landmark rarity 44/256), GPU control-diff
  0.25 + wind 0.0077 + LOD buckets, windowed `--play-wilderness` /
  `make p3d-wilderness` (p50 3.88 ms / 333 fps with the Mid
  atmosphere; vista + landmark captures human-inspected; Deck-low tier
  shrinks ring to 84 m). Two real streaming bugs caught by the settle
  test (ring-restart starvation; empty-slot re-query churn). 9/9 gates.

# CHANGELOG

## 2026-09-08 — NWR-006: materials and atmosphere

The renderer's stylized atmosphere layer, tier-budgeted and OFF by
default (LEGACY keeps every prior proof bit-identical):

- `pc3d_render::atmosphere` (new): `AtmosphereTier` Low/Mid/High with
  documented budgets (Low no shadow pass + thin fog; Mid 1024² depth
  ~4 MB, fog 0.0055, detail 0.55, glint; High 2048² ~16 MB, 0.0070,
  0.85); stable sun shadow map (texel-snapped light ortho — sub-texel
  camera motion leaves the matrix bit-fixed); exp² distance fog in
  linear space with CPU probe mirrors; a 4-tile procedural material
  atlas (grass / ridged rock / sand / sparkling snow) generated from
  `pc3d_assets::material_detail` metadata and blended by
  albedo-derived material weights — no vertex format change, no image
  files; water sun glint (capped lobe); alpha-cutout foliage pipeline
  with deterministic leaf masks and crossed leaf cards.
- Shadows: depth-only sun pass (incl. a cutout-layout variant) before
  the main pass; 3×3 PCF with normal offset and constant + slope-scaled
  bias; soft 0.35 floor in shadow. Streamers cull with the light
  matrix.
- Five real bugs the proofs caught: the RH ortho depth sign, the
  light-camera direction (SUN_DIR points toward the sun), the
  shadow-lookup v-flip, float-depth pipeline bias being whole-range,
  and the wgpu usage-scope law (the shadow pass needs its own bind
  group).
- Proofs: 13 atmosphere tests (shadows darken 0.35 exactly at the cast
  point, local mean 0.004, still-camera bit-stable; fog pulls the far
  field to haze 0.70→0.07; grain 2→81/108 distinct colors with
  separation preserved; glint 2.36→2.84; cutout shows the wall through
  the holes; tier rows) + windowed `--play-materials` /
  `make p3d-materials` (p50 0.70-0.84 ms, diffs 34%/39% vs legacy,
  four captures human-inspected PASS). 9/9 gates green.

# CHANGELOG

## 2026-09-08 — NWR-005: caves, conforming water, foundations

The surface path gains sparse caves, terrain-following rivers, and
buildable-ground checks:

- `pc3d_render::world_features::CaveRegion`: BFS connected-component
  growth from a carved seed over the ONE authority (`final_solid`),
  capped (16 half-extent / 8000 cells); face-net mesh extraction puts a
  quad exactly where solid faces carved, boundary faces sampled from
  the same authority — the cave welds to the surface with no duplicate
  faces or daylight leaks.
- `ConformingWater`: one strip per river edge from the authoritative
  RiverGraph/FlowTable (width/speed from real discharge), 17 heights
  per 256 m edge following the surface patches + edit deltas,
  water_line = strip min − 0.45 (banks appear where terrain rises),
  `refresh_after_edit` touches only sections near the edited patches.
- `check_foundation`: Valid{leveled_by} / Rejected{named reason}
  from walkable + slope + corner-level checks on live ground.
- `CollisionSurface` trait (player.rs): PlayerBody can walk any
  surface; SurfaceRegion implements it (the default authority path
  unchanged) — walking on surface geometry and feeling edits underfoot
  is test-proven.
- Windowed `--play-caves` / `make p3d-caves`: cave-interior capture
  (human-inspected: enclosing faceted stone, no sky leak), river
  before/after a 2×2 6 m raise dam — visible dam, ~1% image diff,
  LOCAL EDIT rows (4 dirty patches, 1/5 sections refreshed, 9 µs
  refresh, 6 µs water remesh), frames p50 0.43 ms.
- Real bug the proofs caught: a SurfaceRegion is a 48 m 3×3-PATCH
  window, not a 3×3-region — build's 0.0 fallback vs refresh's
  generator fallback could hide fake "changed heights"; both now share
  the generator fallback and edit windows sit on the strip. Also:
  Raise stacks on shared nodes (a 4×4 "berm" was a 24 m spike — the
  sky probe caught it) and change-counting must compare heights, not
  just the strip-min water line.

# CHANGELOG

## 2026-09-08 — NWR-004: streamed surface terrain migration

Ordinary natural terrain migrates from culled cube faces to the proven
surface path:

- `pc3d_render::surface_stream`: interest rings, bounded queue, per-frame
  caps, GPU budget with distance-gated farthest-first eviction (the naive
  version churned forever under saturation — the teleport test caught it),
  frustum culling, ring LOD (17×17 → 9×9) with 2 m skirts that make LOD
  boundaries seam-safe by construction.
- Explicit `TerrainPolicy` — the surface stream attaches as the ordinary
  path; the legacy cube stream stays the fallback and all 9 gates remain
  green. Saved worlds untouched.
- Collision served only from the full ring (refuses outside), within 1 m
  of the generator's own answer; edits through the delta layer change
  collision measurably.
- Proofs: 4 streamer tests + GPU vista (419 bounded frames, 1252 patches
  incl. the 1140-patch horizon ring) + windowed `--play-surface-stream`
  (251 frames, p50 9.91 ms, avg 103 fps) — human-inspected PASS: rolling
  faceted hills to the skyline, no ring-boundary holes.

## 2026-09-07 — NWR-003: natural-terrain surface spike

The first real break from cube terrain, isolated to a 3x3 test region:

- `pc3d_render::surface`: 17x17 boundary-grid patches with seam-shared
  edges (identical vertices asserted on every internal seam), heights and
  materials from the authoritative generator, sparse delta edits
  (Raise/Lower/Level) that dirty exactly patch+border-neighbors, a
  version table proving remesh is bounded to the dirty set, and a
  compact delta save record with corruption refusal.
- Collision derives from the same grid: bilinear surface height within
  every facet's min/max (64-point test), 3x3-stencil slope walkability —
  the stencil sees neighbor nodes, catching man-made cliffs a cell-only
  check misses (found by the failing cliff test).
- GPU + windowed proofs with cube-vs-surface discriminators: luminance
  bands along a downhill run, adjacent-row delta distribution (continuous
  slope vs banded flats), control-diff ground rows, and an edit-visible
  plateau. Before-capture human-inspected PASS: faceted rolling hills,
  not cubes. A clockwise-winding bug that culling-invisible the whole
  terrain was caught by the pixel dump and fixed.
- `--play-surface` / `make p3d-surface`; old cube path + construction
  untouched; existing gates re-run green.

## 2026-09-07 — NWR-002: original GLB asset factory

The renderer now loads real glTF. Three original, reproducible assets
(tree/rock/house) generated by a repository-owned Rust factory
(Blender unavailable on this host — the pack's alternative path):

- `tools/assetgen` builds the meshes deterministically and writes valid
  glTF 2.0 GLBs with named LOD nodes + materials; budgets self-enforced
  (1666/1800, 98/600, 38/2500) and regeneration is byte-identical.
- `pc3d_assets::v2`: full schema-v2 validation in typed Rust with the
  factory-honesty law (non-planned status requires the .glb on disk);
  15-case rejection matrix green.
- `pc3d_render::glb`: GLB loader with named errors, LOD selection and
  coarsest-fallback, socket extraction; malformed inputs fail tests.
- Windowed proof (`--play-assets` / `make p3d-assets-window`): 4
  placements incl. two LODs of the tree in one frame; control-diff
  presence (0.22–0.34) + foliage color check; 4 draws / 1725 tris, p50
  9.58 ms; human-inspected PASS. NWR-001 guardrail rows moved honestly.

## 2026-09-07 — NWR-001: rebuild baseline audit + anti-false-art guardrail

The natural-world rebuild (owner's new pack, milestones NWR-001..011)
begins. This milestone changed NO rendering — it secured the baseline:

- Verification battery green: 356/356 pc3d tests, 9/9 visual gates, smoke
  OK, root untouched.
- `capability_inventory.json`: the machine-readable truth of all 12
  renderer paths (module / representation / maturity / rebuild target /
  proof), asset-batch status (all planned, zero GLBs on disk), the gate
  battery, runtime commands, data boundaries, perf baseline.
- `pc3d_render::inventory` (new, 2 tests): validates the inventory
  against the repo and enforces the ANTI-FALSE-ART LAW — an asset row
  claiming "shipped" must match the pack status AND a live disk scan of
  `assets/compiled/**.glb`, in both directions. Sabotage-tested: even a
  coordinated pack+inventory lie fails on the missing file. Future
  milestones upgrade rows only by shipping real artifacts.

## 2026-09-07 — R3DV-012: regression + performance gates — VISUAL RESET COMPLETE

- `make p3d-visual-gates`: the standing 9-gate regression battery re-runs
  every windowed proof (axes, terrain, stream, water, city, NPCs, tiers,
  slice, manifest) with semantic assertions and loud failure; report to
  `shots/gates_report.txt`. Audit run: **9/9 PASS**.
- `VISUAL-GATES-REPORT.md`: per-gate frame records (all rendered gates
  ≥ 92 fps avg on the host iGPU; streaming ~1.1 ms/frame steady; tiers
  42/80/102 patches), the human review note, and seven known limits.
- The full reset (R3DV-001..012) is COMPLETE: windowed renderer →
  first-person collision walking → terrain/caves/water/city/NPCs bound to
  the simulation → host-command building → quality tiers → save/reload →
  a walkable vertical slice (seed 22) → regression gates. 356/356 pc3d +
  474/474 root tests. "Playable" awaits the owner's manual pass:
  `make p3d-slice-live`.

## 2026-09-07 — R3DV-011: THE VERTICAL SLICE

One deterministic showcase seed (22) with everything within a short walk —
and a first-person body to walk it.

- **Walking with collision** (`player.rs`): axis-separated blocking
  against `final_solid`, gravity snap with 1 m step-up, ray-target
  building — read-only queries only.
- **The journey** (one GPU test): spawn at the gate → walk 4 m on
  colliding terrain → cave pocket → showcase render → Sand block placed
  through `HostCommand` and proven visible in first person (control-diff)
  → save → fresh load: seed, block, and player restored, and the
  reloaded world renders **pixel-identical** → NPC cast presence
  (town-vantage control-delta) → inspect boxes on without breaking the
  frame.
- **Save/reload**: `pc3d_save` world meta + build snapshots + the new
  `player_store` (same framing/refusal laws).
- **The walkable slice ships**: `--play-slice live` / `make
  p3d-slice-live` — WASD walking, click-look, F/R place/remove at the ray
  target, B save, L reload, I inspect boxes; 14 s liveness verified.
  Automated showcase: 3 windowed captures, 86 frames p50 17.98 ms, all
  human-inspected PASS. 356/356 pc3d tests (+5), root 474/474.
- **Honest status**: the README's visible-playable definition is met on
  this host; per the gate the word "playable" still awaits the owner's
  manual pass (`make p3d-slice-live`). R3DV-012 remains.

## 2026-09-07 — R3DV-010: materials, lighting, and quality tiers

- **Material binding everywhere**: terrain, construction blocks, and water
  now take their colors from the `pc3d_assets` registry (joining the
  city/NPC bindings) — no renderer-local palettes remain. The renderer's
  first real GPU texture: a deterministic procedural noise tile bound in
  the mesh pipeline, sampled over world-space UVs at high tier.
- **Hemisphere ambient** (0.38 sky + 0.22 ground + 1.05 sun) replaces the
  flat constant — shader and CPU mirror in lockstep, proven by every
  pixel-exact probe still passing.
- **Quality tiers**: Low/Mid/High scale streaming rings, per-frame mesh
  budgets, GPU budgets, and the detail texture — `--play-quality` renders
  the same scene at 42/80/102 loaded patches (2831/5468/6908 KB), the
  scaling law asserted, three PNGs human-inspected PASS.
- **All asset rows have real consumers**: `machine_renderer` is now real —
  an original water wheel meshed at `hydro::best_wheel_site` (the sim's
  deterministic siting); an audit test maps every beta-critical row's
  consumers to a concrete module.
- 353/353 pc3d tests (+5), root 474/474.

## 2026-09-07 — R3DV-009: NPCs render at their simulated positions

`pc3d_render::npcs` populates the city with a resident, a worker, and a
guard — each one an authoritative `NpcBrain` bound to (grounded) plan
anchors and stepped on the real `NavPatch`.

- **Sim binding**: position = `brain.pos` on the terrain column; presence
  proven by control-diff at each sim position (0.10–0.31); the resident's
  torso color is pixel-exact against the material registry.
- **Role readability**: distinct torso materials; the guard always carries
  a spear; the worker's tool appears only while `intent == Working` —
  idle NPCs never look busy (tested).
- **Bed/Work/Idle inspection**: colored frame cubes over the plan's
  anchor cells from the anchor materials (inspect captures only).
- `--play-npcs` / `make p3d-npcs`: 5 windowed captures (overview, three
  close-ups, inspect mode), 86 frames p50 9.8 ms; human-inspected PASS.
  347/347 pc3d tests (+5), root 474/474.

## 2026-09-07 — R3DV-008: the city renders from the placement authorities

`pc3d_render::city` renders a whole readable place — capital + town — with
the simulation's plans as the sole placement authority.

- **Silhouettes**: gatehouse with a real opening (GPU + windowed proof:
  arch vs pillar delta 0.29), crenellated curtain derived between the
  planner's corner towers with a gate gap on the approach axis, towers,
  keep, chapel, market; town homes with pitched roofs, workshop with
  chimney + roof slab, well, watchtower.
- **Bindings**: collision = the footprint union (idle ring and gate
  approach stay walkable — tested); nav anchors = manifest ports +
  bed/work/plaza filtered walkable and grounded on local terrain.
- **Materials from pc3d_assets**: a registry keyed by the beta-critical
  manifest material names, coverage-tested against the whole manifest.
- **A real regression found and fixed**: the static terrain path's u16
  index concatenation overflowed on the city's 1,008-patch load; two
  R3DV-005 GPU tests had silently garbled frames from a stale Uint16 bind
  — the suite caught both, now u32 and green.
- `--play-city` / `make p3d-city`: windowed overview + gate captures
  (p50 29.3 ms, 38.7 fps), human-inspected PASS. Town plans one region
  east so the two authorities never collide.

## 2026-09-07 — R3DV-007: river water from flow records

`pc3d_render::water` renders rivers as transparent 3D strips whose every
visual parameter comes from the authoritative flow records.

- **Record-driven surface**: one strip per river region; direction from
  `FlowRecord.direction` (a current wave whose phase advances along the
  flow axis at the record's speed class), width/depth/alpha from discharge
  and slope. Transparent pass (alpha blend, depth-read-only) after all
  opaque geometry.
- **Local updates**: a dam edit through the P3D-303 path
  (`RiverGraph::build` override → `from_graph_with_revisions`) remeshed
  only 5 of 281 sections in the windowed proof; unchanged tables cost
  zero mesh work.
- **Rigorous proofs**: transparency via a control render (same frame with
  water detached = pixel-exact under color; the with-water pixel is a true
  alpha blend, coefficient 0.2–0.95); direction via along-vs-across image
  deltas; five unit tests pin strips/revisions/current math.
- `--play-water` / `make p3d-water`: windowed before/after dam captures,
  56 frames p50 1.05 ms; PNGs human-inspected PASS. 334/334 pc3d tests
  (+5), root 474/474.
- Honest deferrals: no carved river beds (strips follow the analytic
  surface), no foam/reflection, sea water not yet meshed.

## 2026-09-07 — R3DV-006: streamed terrain with bounded work, LOD, and culling

`pc3d_render::streaming::TerrainStreamer` consumes the world's own
streaming primitives (`interest_patches` rings, `BoundedQueue`, `lod_for`
bands) and performs real per-frame meshing/uploads under hard caps.

- **Bounded queue**: ≤N mesh jobs and ≤M uploads per frame; overflow held
  in a deferred list and re-admitted (the P3D-105 teleport law); one live
  job per patch, nearest-first priority. A teleport's whole vista (112
  patches) completes in 57 frames at cap 2/frame.
- **GPU budget**: total-byte budget with farthest-first eviction; the
  windowed walk ran at 24,555/24,576 KB with eviction active and the
  viewer's full ring intact.
- **Frustum culling**: Gribb–Hartmann planes from the shader's own
  view-proj; 77,875 patch-draws culled during the 351-frame walk.
- **Real LOD**: Full = all faces, Mid drops bottoms, Far = top shell;
  top faces render at every level so rings can't crack (proven by
  identical top-face sets).
- `--play-stream` / `make p3d-stream`: 3-waypoint windowed walk, each
  capture verified by a query-derived view-center raycast probe; PNGs
  human-inspected PASS. 329/329 pc3d tests (+7), root 474/474.
- Five proof-caught bugs fixed pre-commit (duplicate-job starvation,
  stuck rejected pushes, horizon-before-ring priority, unbounded
  re-admission churn, oversized-patch budget bust).

## 2026-09-07 — R3DV-005: natural terrain renders from the authoritative query

`pc3d_render::terrain` meshes natural terrain as culled block faces taken
directly from `final_solid` (the P3D-202 single authoritative answer) — the
rendered surface and the collision surface are literally the same function.

- **No seams by construction**: neighbors outside a patch are queried
  directly (natural terrain is a pure function of world coordinates); an
  18³ query cache makes meshing ~7 ms/patch.
- **Four capabilities probe-verified from the live window**: hill slopes,
  cliff walls with material separation, cave interiors (deterministic
  enclosed-corridor pockets), and the overhang — the corridor ceiling's
  underside face, rendered and pixel-verified from inside.
- **Collision alignment proven**: a face exists exactly where the query
  flips solid↔air (full-coverage test, cross-patch included); a rendered
  top face exists exactly where the query says a cell is standable.
- **Bake-off reproduced**: heightfield-family extraction 33–87 µs/patch
  with equal-or-better fidelity vs 3.3–4.6 ms for density-threshold — the
  mesher consumes the winning family; smooth dual-contouring stays a later
  quality pass.
- `--play-terrain` / `make p3d-terrain`: one windowed run, three scenes
  (hills/cliff/cave over 27-patch neighborhoods), 111 frames p50 0.47 ms;
  PNGs human-inspected PASS.
- Two proof-caught bugs fixed pre-commit: an unloaded-neighbor hole let a
  cave probe see the sun (vistas load 3×3×3 neighborhoods — the R3DV-006
  streaming lesson); the pocket finder now requires fully enclosed
  corridors.

## 2026-09-07 — R3DV-004: construction mesh from host-owned state

The renderer now reads the real world: `pc3d_render::construction` meshes
the existing `Construction` overlay (culled 1 m block faces in world
meters, per 16 m patch, content-versioned GPU buffers) through a strictly
read-only dependency on `pc3d_world`.

- **Host-command edits only**: place/remove go through
  `SoloHost::submit(HostCommand::Build/RemoveBuild)` + one tick; the
  renderer takes immutable references, so client mutation of canonical
  state is impossible by construction. A rejected command (foreign owner)
  causes zero remesh work.
- **Bounded patch remesh**: a single-cell edit remeshes exactly one patch
  while other patches do zero work (asserted in tests, printed by the CLI).
- **Visual edit proof**: windowed before/after captures — the wall's center
  block flips rock→sand on screen (pixel delta 0.22 at the projected cell
  center, control sky pixel unchanged), after a live mid-run window resize.
  Human-inspected PASS. `--play-build live` gives manual F/R place/remove
  through the same host path.
- 7 new tests (313/313 pc3d green, root 474/474); three proof-caught bugs
  fixed before commit (negative-x wall cells spanning two patches via
  Euclidean division; zero-size wgpu buffer slices; construction drawing
  with the sky pipeline bound when the placeholder scene is hidden).
- Honest deferrals: cross-patch face culling (R3DV-006), pc3d_assets
  material colors (R3DV-010), ray-pick targeting (R3DV-011).

## 2026-09-07 — R3DV-003: asset manifest validator (pc3d_assets) + queue status on the .mds

- **pc3d_assets** (new P3D-local crate, pure data layer): parses and
  validates `docs/POORCRAFT-3D-VISUAL-RESET/assets/beta_critical_assets.json`
  against the contract in `asset_manifest.schema.json` — required fields,
  enums, id pattern, `additionalProperties: false`, meters/+Y/-Z coordinate
  law — plus the semantic asset gate: duplicate ids, missing consumers,
  absent proof scenes, forbidden source policy (brand tokens scanned in
  every row field, separator-normalized), invalid LODs, and `final` assets
  without a runtime geometry path are all rejected with named errors.
- The canonical 19-row beta-critical pack is embedded at compile time with a
  drift test against the docs pack; a query API (`get`/`ids`/`of_category`)
  is ready for the castle/NPC/terrain renderers.
- CLI `--validate-assets [path]` and `make p3d-assets`; positive run passes
  (19 rows, all 7 beta categories), a tampered duplicate-id copy fails
  exit 1.
- 13 new tests; pc3d workspace 306/306 green; root 474/474 untouched.
- Bookkeeping law: the execution queue now keeps a STATUS table in
  `06-EXECUTION-QUEUE.md` (001–003 DONE, 004 NEXT) — every task
  determination recorded in the markdown docs, not only in the gate JSON.

## 2026-09-07 — R3DV-002: POORCRAFT 3D's renderer is actually 3D now

The R3DV-001 banner image was rejected as evidence: it was NDC clip-space
geometry with no camera, projection, or depth — GPU plumbing, not a game.
The queue's next task shipped in response:

- **Camera**: perspective view-projection in P3D world coordinates
  (right-handed, meters, +X east / +Y up / +Z south, yaw/pitch), hand-written
  mat4 with unit tests pinning near→0 / far→1 depth mapping and known-point
  projection. First-person input: click-to-grab mouse look, WASD +
  Space/Shift movement (4 m/s, diagonal-normalized).
- **Depth + lighting**: depth24plus buffer; depth-tested indexed mesh with
  lambert sun + ambient; ray-reconstructed sky whose sun disc and horizon
  follow the world sun direction.
- **HUD**: bitmap-font debug line (position / yaw / FPS), rasterized per
  frame to a texture quad.
- **Three 3D-only proofs**: face flip (same screen region reads a different
  cube face from different poses), occlusion (a marker stone fully hidden at
  pose A — zero pixels — visible from pose B/C), parallax (19.5% of decoded
  RGBA pixels differ between the two windowed captures).
- **Evidence**: 293/293 pc3d tests (+11); windowed captures
  `windowed_3d.png` + `windowed_3d_poseb.png` from the live swapchain after
  a mid-run resize, human-inspected PASS; p50 0.45 ms; sim untouched
  (p3d-smoke digest unchanged). Two real bugs fixed during proofing
  (tan(fov) vs tan(fov/2); albedo folded into the lighting dot product).

## 2026-09-06 — R3DV-001: POORCRAFT 3D gets a windowed GPU renderer (visual reset)

The simulation-only roadmap was declared complete; the visual reset
(docs/POORCRAFT-3D-VISUAL-RESET/) now drives execution. First task done:

- **pc3d_render** (new P3D-local crate, wgpu 24 + winit 0.30 + WGSL):
  resizable window, wgpu surface with `COPY_SRC`, nonuniform proof scene
  ("three banners at dawn": gradient sky + sun in the fragment shader, plus
  an indexed vertex-colored banner mesh with backface culling), surface-loss
  policy (Lost→recreate, Outdated→reconfigure, Timeout→skip, zero-size
  clamp), and swapchain screenshot readback.
- **Semantic screenshot gate**: a pure pixel verifier asserts the frame is
  nonuniform and contains every scene element (gradient monotonicity, sky
  nonuniformity, three banner colors, pole, ground, sun, opacity); a flat
  clear fails it. Used by both the offscreen GPU tests and the live capture.
- **apps/poorcraft3d**: `--play` opens the windowed renderer; `--play-shot
  [png] [frame]` opens a window, renders, performs a live 1280x720→800x500
  resize mid-run, captures the resized swapchain frame, verifies it, and
  prints a perf line. Makefile: `p3d-play`, `p3d-shot`.
- **Evidence**: 282/282 pc3d tests (9 new, incl. real-GPU offscreen render +
  determinism + resize), windowed PNG visually inspected, 12 s windowed
  liveness run, 2000-frame perf record (p50 0.90 ms / p95 1.37 ms). Root
  workspace untouched. pc3d_world remains rendering-free.

## 2026-09-06 — P3D-702..806: POORCRAFT 3D ROADMAP COMPLETE (loops 404-410)

- **P3D-702 typed machines**: PowerType (Heat/Steam/Mechanical/
  Electrical), MachineKind (Boiler/SteamEngine/Generator/Battery),
  connect-time typed wires (WireError::TypeMismatch), water as a real
  consumable fluid, per-stage conservation with lifetime audit
  counters, deterministic wire throughput caps.
- **P3D-703 nuclear**: control rods, coolant boil-off, decay heat,
  tick-start automatic SCRAM, lost-coolant meltdown, decaying world
  Contamination with plume spread, settlement prosperity penalties,
  D-018 MAX_REACTORS = 4 world cap.
- **P3D-704 dragon + ley**: territory raids on a 30-day cadence,
  SplitMix64 integer slayer combat, permanent dragon death, tribute
  pacts that ward territory, breach enrages; Ley attunement tiers
  with Bless/Blight rituals, atomic costs, strain and backlash.
- **P3D-705 faction kits**: ten ideology-signature castle modules,
  FactionKit table with preferred laws, plan_capital_kit with a
  byte-identical core prefix.
- **P3D-801 integrated host**: SoloHost — commands land in canonical
  (tick, id) order, all systems tick together, digest binds seed and
  history, reordered delivery proven inert.
- **P3D-802 replication**: ReliableChannel (seq/ack/bitfield,
  gap-buffered in-order), interest snapshots, monotonic mirrors.
- **P3D-803 session**: LobbyManager (invite lifecycle, accept-time
  D-029 cap, deterministic host migration), Transport trait +
  loopback + spy-transplant proof.
- **P3D-804 soaks**: run_soak + audits; in-suite 40-day soak; CLI
  --soak ran 2000 days / 1.2M ticks clean in 8.3 s.
- **P3D-805 player journey**: 10 asserted steps spawn → dragon;
  found + fixed the first-tool progression gate (wood_pick recipe 6).
- **P3D-806 scale proof**: per-player replication cost constant in N,
  linear total growth, 128 players < 20 ms/tick, rows printed.

273 pc3d tests, 474 root tests, p3d-smoke OK, all pushed.

---

## 2026-09-06 — valve-era computing (loop 403, P3D-701)

- **`pc3d_world::valve_computing`**: Signal (u8, 0=off, >0=on),
  GateKind (And/Or/Not/Xor truth tables), LogicGate (named, indexed
  inputs/output), LogicCircuit (deterministic evaluation in declaration
  order), ValveController (programmable input setter + evaluate +
  debug_dump). Tests: AND/OR/NOT/XOR truth tables, NAND multi-stage
  circuit, determinism.
- 236 pc3d tests green (+6); root untouched at 474; smoke OK.

## 2026-09-06 — NPC death, multi-axis karma, ideology factions (loop 402, P3D-612/613/614)

- **P3D-612 — permanent NPC death**: `pc3d_world::npc_death` — NamedNpc
  with name/role/skill; NpcRoster (recruit, kill permanent, replace with
  lower skill, service loss per role). Tests: death permanent,
  replacement lower-skilled, service loss tracked.
- **P3D-613 — multi-axis karma**: `pc3d_world::karma_evidence` — 4 axes
  (Personal/Civic/Faction/Ideological) accumulate evidence independently;
  disposition = weighted sum clamped ±100. Tests: axes accumulate
  independently, clamping.
- **P3D-614 — ideology-founded factions**: `pc3d_world::ideology` —
  Ideology (Conquest/Commerce/Faith/Liberty/Isolation) with same-ideology
  diplomacy bonus, recruitment appeal, law strictness; PlayerFaction with
  shift_ideology (drift increases, bonus shifts). Tests: ideology shapes
  diplomacy/law/recruitment, ideology evolution.
- **P3D-600 STAGE COMPLETE** (601–614): 230 pc3d tests green (+7), root
  untouched at 474. Contract at `docs/POORCRAFT-3D/contracts/P3D-612.md`.

## 2026-09-06 — war objectives + NPC war intents (loop 402, P3D-611)

- **`pc3d_world::war`**: WarObjective (DefendGate/HoldWall/AttackTarget/
  Retreat), WarAssignment{entity, objective, path, leg, arrived},
  assign_npcs (nav paths to objective cell), advance (one leg per tick,
  arrived when path exhausted). Deterministic.
- 223 pc3d tests green (+1); root untouched at 474; smoke OK.

## 2026-09-06 — civic projects: player-built + NPC-commissioned (loop 401, P3D-610)

- **`pc3d_world::civic`**: CivicProject (name, Commissioner::Player/Npc,
  work_required, work_done, materials) and CivicBoard (commission,
  player deliver, NPC auto-tick, completed/active queries).
- **Both paths produce identical completion**: player delivers materials
  manually; NPC projects auto-advance per tick. Both complete at
  work_required.
- 222 pc3d tests green (+3); root untouched at 474; smoke OK.

## 2026-09-06 — relationships: allied/puppet/protectorate/rival/conquered (loop 400, P3D-609)

- **`pc3d_world::relationships`**: RelationshipKind (Allied/Puppet/
  Protectorate/Rival/Conquered) with per-kind autonomy (100/30/60/100/20),
  tribute_rate (0/500/200/0/800 basis points), and growth_pct (5/2/3/1/1
  percent/day). `RelationshipSystem` — establish, release (grants
  independence), change_kind, simulate_day (puppets grow, tribute
  accumulates), collect_tribute.
- **Tests**: kinds have distinct autonomy/tribute/growth; puppet grows
  faster than rival; tribute accumulates and is collectible; release
  grants independence; determinism across 20 days.
- 219 pc3d tests green (+5); root untouched at 474; smoke OK.

## 2026-09-06 — oversight panel backed by real data (loop 398, P3D-608)

- **`pc3d_world::oversight`**: `OversightPanel::query` reads REAL
  settlement/garrison/economy state and produces `OversightSummary`
  (population, food, defense, prosperity, garrison soldiers/readiness,
  goods, composite health). Zero-state handled without panic (health 40:
  food sufficiency vacuously true, defense and prosperity 0).
- **Civic Projects**: `Project` with bounded `advance()`, progress
  percentage, completion tracking. Biome viability scoring for
  settlement placement.
- 214 pc3d tests green (+4); root untouched at 474; smoke OK.

## 2026-09-06 — army/garrison: recruitment, supply, morale, readiness (loop 397, P3D-607)

- **`pc3d_world::garrison`**: `Garrison { soldiers, max_soldiers, supply,
  morale }` — recruit bounded by max_soldiers, supply_day (soldiers
  consume 1 supply each, morale tracks supply), readiness composite
  (strength ratio + supply sufficiency + morale, 0 when no soldiers).
- Tests: recruit bounded, supply decay per soldier per day, morale
  tracks supply, readiness composite.
- 210 pc3d tests green (+3); root untouched at 474; smoke OK.

## 2026-09-05 — player-founded settlements (loop 400, P3D-606)

- **The player can FOUND settlements.** `pc3d_world::player_settlement`:
  found on river regions (refuses double-found with Occupied), appoint
  a steward NPC, set tax rate (0–25%, clamped), toggle curfew, toggle
  gates, expand territory by claiming Chebyshev-adjacent regions.
- **Tests**: found+appoint+policies, double-found refusal, expansion
  claims 8 adjacent regions.
- 207 pc3d tests green (+3); root untouched at 474; smoke OK.

## 2026-09-05 — faction relations: trust, diplomacy, quests, territory (loop 399, P3D-605)

- **Factions have TRUST toward each other.** `pc3d_world::faction`:
  TrustLevel (Allied/Friendly/Neutral/Wary/Hostile) derived from trust
  scores (0–100) via `from_score`; `DiplomacyAction` shifts trust
  (Alliance +30, TradeAgreement +15, Insult −10, BorderSkirmish −15,
  DeclareWar −100); `can_trade` requires ≥ Neutral; `is_allied`
  requires exactly Allied.
- **Quests connect factions to the player**: offer → accept → complete
  → reward reputation. Territory: factions claim regions and the
  controller is queryable.
- **Tests**: trust shifts with diplomacy, trust levels gate actions,
  quest lifecycle, territory claims, determinism.
- 204 pc3d tests green (+5); root untouched at 474; smoke OK.

## 2026-09-05 — the economic engine (loop 398, P3D-604)

- **`pc3d_world::economy`**: deterministic integer production from
  workshops, food consumption by population, trade routes transferring
  goods between settlements, and `EconomicState` evolving per day —
  production adds goods, consumption drains food, starvation shrinks
  population, surplus grows prosperity, trade balances deficits.
- **Tests**: production determinism, consumption drain, trade bounded
  transfer, economic loop prosperity tracking, starvation shrinkage,
  inter-settlement trade.
- 199 pc3d tests green (+6); root untouched at 474; smoke OK.

## 2026-09-05 — law and order: gates, guards, alarms (loop 397, P3D-603)

- **`pc3d_world::castle_law`**: the castle's legal and defensive state.
  `GateState` (open/closed toggle), `Law` (Theft/Assault/Trespass with
  standing thresholds and Punishment Fine/Attack/Exile),
  `access_allowed` (gate open AND standing meets ALL thresholds),
  `violated_law` (first violated in order), `punishment_for`,
  `Alarm` (raise at position, proximity check, clear).
- **Tests**: gate toggle; access gated by standing (high enters, low
  denied, closed gate denies all); violated law identified correctly
  (theft threshold -20 hit before assault 0 for standing -25); alarms
  raise/proximity/clear.
- 193 pc3d tests green (+5); root untouched at 474; smoke OK.

## 2026-09-05 — the castle planner (loop 396, P3D-602)

- **`pc3d_world::castle`**: modular castle kit manifest (7 kinds: Keep
  5×5, Wall 3×1, GateHouse 3×2, Tower 2×2, Barracks 3×2, Chapel 3×3,
  Market 4×2 — each with ports and min elevation) and
  `plan_capital(gen, center)` — terrain-aware placement: Keep at
  center, GateHouse south, 4 Towers at diagonal corners, Barracks east,
  Chapel west, Market south. No overlapping footprints (BTreeSet-
  tracked). Roads connect the center to the gatehouse and towers.
- 188 pc3d tests green (+4: deterministic layout, manifest
  completeness, capital module coverage + no-overlap, road
  connectivity); root untouched at 474; smoke OK.

## 2026-09-05 — settlement plan: anchors, buildings, roads, services (loop 395, P3D-601)

- **Settlements have a physical plan.** `pc3d_world::settlement_plan`:
  `SettlementPlan::plan(gen, center)` generates a deterministic layout in
  3 concentric rings — Well + 2 Homes at r=3, Workshop/Farm/Home at r=6,
  Storage/Barracks/Watchtower/Farm at r=10 — with roads from plaza to
  every non-well building, and `Anchors` (Bed/Work/Idle per D-033)
  derived from building kinds.
- **Seven building kinds with service contributions**: Home (housing ×4),
  Workshop (production ×5), Storage (×200), Barracks (defense ×10),
  Watchtower (defense ×5), Well (water), Farm (food ×15). The service
  summary test verifies each contribution.
- **Validation test-enforced**: all buildings present, plaza exists,
  roads connect, anchors populated.
- 184 pc3d tests green (+5); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-601.md`. P3D-600 stage opened.

## 2026-09-05 — the constraint matrix: geography proven (loop 394, P3D-106)

- **The biome/hydrology constraint matrix is green.**
  `pc3d_world::constraints`: run_biome_constraints(seed) sweeps
  ±20 regions × 4 constraints (mountain profile, coastal transition,
  wetland requirements, forest humidity) — zero violations across 4
  seeds. River corridors have nonzero wetness. Seed history is
  reproducible (same seed → same biome everywhere).
- **P3D-100 stage now fully complete** (101–106): coordinates,
  persistence, generation, atlas, streaming, constraint matrix.
  All 5 early stages done (P3D-000 through P3D-400). Remaining roadmap:
  P3D-600 (settlements/empire), P3D-700 (advanced), P3D-800
  (multiplayer/beta).
- 179 pc3d tests green (+4); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-106.md`.

## 2026-09-05 — crafting check in the diagnosis (loop 393, P3D-506)

- **The diagnosis now covers crafting as its 13th check.** A fresh
  inventory receives wood and stone, the stone_pick recipe is crafted,
  and the output is verified — all ingredients consumed, pick produced.
  The diagnosis runs 13/13 PASS via `--diagnose`.
- 175 pc3d tests green; root untouched at 474; smoke OK.

## 2026-09-05 — the crafting system: recipes + progression (loop 393, P3D-506)

- **Recipes combine materials into new items.** `pc3d_world::craft`:
  RECIPES table (stone_pick = 3 wood + 2 stone; iron_pick = 5 stone +
  2 wood; bread = 3 soil; sand-to-snow; compost), recipe_by_code and
  recipe_for_output lookups, can_craft pre-check, and craft() with the
  P3D-501 atomicity law — inventory UNTOUCHED on failure.
- **The crafting progression loop is test-proven**: gather soil → bread;
  gather wood + stone → stone_pick; gather more → iron_pick. All three
  phases produce their items from the same inventory.
- Well-formedness test: unique recipe codes, unique outputs, known
  items, positive counts. Craft results are deterministic.
- 173 pc3d tests green (+5); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-506.md`.

## 2026-09-05 — engineering + player diagnosis (loop 392, P3D-505/506)

- **Valve-era engineering components on the flow-consumer contract.**
  `pc3d_world::engineering`: ValveNetwork (bidirectional on/off gates on
  river edges — any closed valve blocks), Pipe (endpoint record), and
  WaterWheel::site deriving spin rate (milli-RPM) from real discharge ×
  slope records.
- **The game can diagnose itself AS A PLAYER.** `poorcraft3d --diagnose
  <seed>` walks every shipped system in one deterministic pass: spawn,
  walk, dig, navigate, fish, eat, build, cast, entities, companion,
  settlements, reservoirs — 12 checks, each with a real assertion, plus
  3 proof PNGs (atlas, debug overlay, flow map). Exits 1 on any failure.
- 171 pc3d tests green (+6); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-505.md`.

## 2026-09-04 — the first magic path: runes, mana, casts (loop 391, P3D-504)

- **Magic is learnable and world-facing.** `pc3d_world::magic`: two
  runes — LUMEN (marks target + 6 neighbors as light, 10 mana) and
  DELVE (clears a 3×3 disc, 18 mana) — learned via `Mage::learn`, cast
  for mana from a regenerating pool (+1/tick capped at max).
- **Refusals are total**: unlearned runes and insufficient mana fail
  with zero side effects (mana unchanged, world unchanged). Effects are
  edit plans composed by callers through the P3D-204 path.
- Tests: learning gates casting; spend + regen; effect cell coverage;
  cross-instance determinism.
- 163 pc3d tests green (+4); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-504.md`.

## 2026-09-04 — the danger loop: combat, creatures, loot, dungeon (loop 390, P3D-503)

- **Hostile creatures with deterministic melee.** `pc3d_world::combat`:
  `CreatureKind` (Goblin 20 hp/4 dmg, CaveSpider 12 hp/3 dmg with loot
  tables), `CreatureSystem` — creatures hit the player within Chebyshev
  range 1 off a 30-tick cooldown; the player attacks the LOWEST-id
  creature in range for fixed 10 damage; death removes the creature and
  drops its loot table into the inventory.
- **The first dungeon room**: `DungeonRoom::carve_cells` — a bounded
  9×3×9 chamber + 5-cell corridor, deterministic, all underground,
  floor layer exposed for placement.
- 159 pc3d tests green (+4); root untouched at 474; smoke OK. Test
  expectations corrected to the real mechanics (both creatures die to
  10 hits; floor includes corridor cells; bread pre-stocked for the
  heal test). Next: P3D-504, the first magic path — the 20th task.

## 2026-09-04 — the survival loop closes (loop 389, P3D-502)

- **Catch → eat → survive, wired end to end.** `pc3d_world::survival`:
  `fishing_catch` consumes FishStocks and adds a FISH item (Food,
  heal 15) to the inventory — a full inventory returns the fish to the
  stock rather than destroying it; `eat_from` consumes one food item
  and clears hunger through Needs::eat; `harvest_into` gates terrain
  harvest by tool tier straight into the inventory.
- **Contextual onboarding**: the ordered milestone checklist
  (first_tree/first_catch/first_build/first_night) — idempotent marks,
  1-byte bitmask persistence.
- Tests: the full loop (stock decrement + inventory gain + hunger
  clear + river untouched per D-007), clean failure on empty stock,
  harvest gating, onboarding order/persistence.
- 155 pc3d tests green (+4); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-502.md`. Next: P3D-503 combat and
  creatures.

## 2026-09-04 — item authority: inventory, tools, harvest (loop 388, P3D-501)

- **`pc3d_world::items` is THE item catalog**: stable u16 codes
  (wood/stone/sand/snow/soil/stone_pick/iron_pick/bread) with
  `ItemKind` (Tool{tier} / Material / Food{heal}).
- **Inventory semantics proven**: add stacks-then-fills returning the
  exact leftover (100 wood into 3 slots of 64 → 0 leftover, partial
  stack tops to 105), remove drains across stacks bounded by existence,
  count sums.
- **Tools break and harvest gates by tier**: `ToolState` decrements and
  breaks at 0; `harvest_yields` gives bare hands soil-like materials
  but REQUIRES tier ≥ 1 for stone; yields flow into the inventory.
- 151 pc3d tests green (+4); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-501.md`. Next: P3D-502
  food/fishing/survival wiring.

## 2026-09-04 — far settlements live: aggregates + reconciliation (loop 387, P3D-407)

- **Distant settlements are alive.** `pc3d_world::settlement`:
  `Aggregate` (population/food/defense/prosperity) evolves one
  deterministic day at a time — food feeds or starves population,
  defense decays, prosperity tracks health, everything clamped at 0.
  `Settlements::new` sites settlements deterministically on river
  regions (ascending key, greedy 24-region spacing, fixed name table).
- **The reconciliation invariant is test-enforced**: promoting one
  settlement to Full demotes all others (at most one Full at a time);
  scalars are preserved through promote/demote; `simulate_far_days`
  skips the FULL settlement (the live sim owns it) while aggregates
  evolve.
- **P3D-400 STAGE COMPLETE**: player controller (401), entities (402),
  navigation (403), NPC life (404), perception/karma (405), companions
  (406), far settlements (407) — 147 pc3d tests green (+4), root
  untouched at 474. Next stage: P3D-500 personal gameplay.

## 2026-09-04 — companions follow, wait, assist, recover (loop 389, P3D-406)

- **`pc3d_world::companion`**: Follow trails within 2 cells of the
  player — paths recompute from the companion's ACTUAL position
  whenever the player's cell changes or the cached path exhausts, so
  being left behind SELF-HEALS without teleports (a no-jump assertion
  walks the companion 10 cells behind a moving player and proves every
  position transition is ≤ 2 cells); Wait holds; Assist paths to a
  target cell and holds it. Path caching keyed by the player's cell
  avoids re-pathing every tick.
- Tests: follow trails without teleports and catches up within
  distance+2; wait holds through player movement then Follow resumes;
  assist reaches and holds; determinism across 200 mixed moves.
- The test suite itself needed in-patch walks (a NavPatch covers one
  16×16 patch; a 30-cell walk exits it and nav.path correctly refuses
  out-of-patch targets) — shortened walks keep the mechanics under
  test.
- 143 pc3d tests green (+4); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-406.md`. Next: P3D-407 far-settlement.

## 2026-09-04 — witnesses remember: perception + karma (loop 388, P3D-405)

- **`pc3d_world::perception`**: NPCs witness moral events within a
  Chebyshev sight radius (exact, corners included); personal knowledge
  is a BOUNDED evidence list (capacity 32, weakest dropped first) whose
  confidence ages (0.05 per 1000 ticks, forgotten at zero); reports
  spread evidence at REPORT_CONFIDENCE (0.6 — always below witnessed);
  `Karma` holds per-faction baselines + per-actor evidence deltas with
  disposition = baseline + delta clamped ±100.
- **Tests**: witness radius exact incl. corners; report spread at lower
  confidence + unknown-event failure; aging decay + forgetting;
  capacity drops the weakest first; faction baselines differ (D-030)
  and evidence shifts dispositions with floor clamping; full-history
  determinism.
- 139 pc3d tests green (+4); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-405.md`. Next: P3D-406 companions.

## 2026-09-04 — NPCs live: roles, needs, schedules, intent (loop 387, P3D-404)

- **`pc3d_world::npc`**: Role (Farmer/Fisher/Builder/Guard, each with a
  distinct visible work activity), `Needs` (hunger rises, energy drains
  while working and restores while resting — with f32 sub-tick
  accumulators, because truncating per tick silently lost every
  fraction), `SchedulePhase` from the day fraction (Sleep < 0.25, Work <
  0.7, Idle < 0.8, Work after), `Intent` (Idle / Walking / Working /
  Sleeping), and `NpcBrain::step` — the day-in-the-life state machine.
- **The day-in-the-life test**: an NPC walks to its work site during the
  Work phase (arrives, visible activity = its role's work), goes home
  and sleeps during Sleep, and two fresh brains on identical input
  streams never diverge.
- **Arrival compares x/z only** — anchors carry an arbitrary y while nav
  paths carry terrain heights; comparing full cells made the NPC
  oscillate Walking/Sleeping forever.
- 135 pc3d tests green (+4); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-404.md`. Next: P3D-405 perception,
  memory, karma.

## 2026-09-04 — NPCs can path: nav patches + portals (loop 386, P3D-403)

- **`pc3d_world::nav`**: `NavPatch` (per-column floor + walkability from
  the final-solid law), bounded deterministic A* (4-connected,
  `MAX_NAV_NODES` 4096, octile heuristic, Reverse-heap tie-breaks —
  same endpoints → same path), `portals_to` (shared-border columns
  walkable on both sides), and `cross_patch_path` chaining through the
  first ascending portal with the b-side entry cell just past the
  border.
- **Tests**: smooth-terrain paths are continuous and deterministic; a
  built-wall segment routes the path around its ends (the assertion
  learned that crossing the wall column at open rows is legal — the ban
  is on the blanked segment); portals exist between adjacent land
  patches; cross-patch paths are continuous through a portal.
- Three proof-driven iterations fixed the wall scenario (full-column
  blank seals the patch — leave end rows open) and the portal entry
  cell (a's portal cell is out-of-patch for b).
- 131 pc3d tests green (+3); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-403.md`. Next: P3D-404 NPC life.

## 2026-09-04 — entities exist: registry, spatial index, interest (loop 385, P3D-402)

- **`pc3d_world::entities`**: `EntityId`/`EntityKind`/`Entity` (48-byte
  fixed-width encoding with validated kind codes) and the
  `EntityRegistry` — BTreeMap by id so iteration is deterministic
  (`update_order()` is the same sequence every call), spawn/insert with
  an id high-water mark, `move_entity` maintaining the by-patch spatial
  index, exact `by_patch`/`entities_near` queries (ascending, negative
  coords included), and `interest_state(viewer)` reusing the P3D-206 LOD
  selection with Horizon entities EXCLUDED from active ticks.
- **Persistence through the law**: `pc3d_save::entities_store` saves and
  loads `entities/registry.p3d` — ids, kinds, cells, and the id
  high-water mark survive; foreign files refuse.
- 128 pc3d tests green (+5); root workspace untouched at 474; smoke OK.
  One test bug fixed (kind-byte offset miscounted). Next: P3D-403
  navigation graph.

## 2026-09-04 — the player moves: collision, steps, swimming, spawn (loop 384, P3D-401)

- **The first PERSON moves through POORCRAFT 3D.** `pc3d_world::player`:
  a deterministic 60 Hz controller — axis-separated collision against
  `final_solid` (4 footprint corners × 3 body heights), 1 m STEP-UP for
  grounded/swimming players, gravity with terminal fall, SWIMMING in
  Water cells (2.2 m/s cap, buoyant gravity, terminal sink 2 m/s,
  jump swims up), 6.5 m/s jump.
- **Safe spawn**: progressive doubling rings around the origin over
  REGION-CENTER cells with a macro-elevation prefilter — solid floor,
  two passable cells above, above sea level — proven across 3 seeds.
- **Proven by tests**: flat walk stays level and moves; climb ≤ 2 m and
  never clips through the world after 600 ticks of wall-pushing;
  determinism (600 mixed inputs → identical trajectories); swimming
  buoyancy bounded by terminal sink with working swim-up.
- FIVE test-caught bugs fixed en route — the biggest: the spawn scan
  used region×16+8 as a CELL coordinate (16× off), spinning the scan on
  an origin corner forever. Correct: region×256+128.
- 123 pc3d tests green (+5); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-401.md`. Next: P3D-402 entity
  registry + spatial index.

## 2026-09-04 — fishing: the first flow consumer (loop 383, P3D-307)

- **The first consumer built ON the flow-contract interface.**
  `hydro::FishStocks`: stock per river region at carrying capacity
  (16 + discharge/8 — bigger rivers hold more fish); `catch_fish`
  consumes STOCK ONLY (bounded, never negative) and takes no river
  argument — the river cannot weaken by fishing, by construction;
  `restock` is deterministic (quarter capacity per cycle,
  capacity-bounded).
- **The fishing contract is test-proven**: catch exactly removes stock,
  river discharge unchanged, over-fishing bounded, restock deterministic
  and ≤ capacity.
- 118 pc3d tests green (+1); root untouched at 474; smoke OK.
  **P3D-300 WATER STAGE COMPLETE**: watershed (301), flow records +
  ports (302), dirty rebuilds (303), flow rendering (304), reservoirs
  (305), consumer query + wheel proof (306), fishing (307). Next stage:
  P3D-400 movement, entities, and NPC foundations.

## 2026-09-04 — the flow-consumer contract + wheel-site proof (loop 382, P3D-306)

- **The D-007 consumer contract is real**: `RiverGraph::flow_potential_at`
  returns `FlowPotential` (discharge, slope, wetness, reservoir volume,
  viable) as a PURE read — the purity test runs 1089 queries and proves
  no discharge, reservoir, or graph changed. Machines, wheels, fishing,
  and NPCs will all query this; none can weaken the river.
- **THE visible machine proof**: the best waterwheel site (viable-only
  candidates, maximizing discharge × slope, deterministic) stamps a
  white marker on the flow map — human-eye PASS on seed 2024's PNG.
- **The viability test caught a slope-units bug**: per-mille computed as
  meters×1000/256000mm truncated to 0 for every plausible drop (correct
  is ×1,000,000) — before the fix NO site was viable. All three slope
  formulas corrected.
- 117 pc3d tests green (+3); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-306.md`. Next: P3D-307 fishing as
  the first consumer.

## 2026-09-04 — bounded conserved reservoirs (loop 381, P3D-305)

- **GATE DECIDED: YES** — the dam story needs water that HOLDS, so the
  minimal bounded volume model landed: `hydro::Reservoirs` (per-region
  `Reservoir { capacity_kl, volume_kl }` in thousand-liter fixed-point;
  capacity terrain-derived from local elevation range).
- **Conservation is test-enforced**: fill retains what fits and routes
  overflow DOWNSTREAM through a bounded chain walk (final spill
  returned); poured − spilled == total retained across a huge pour;
  drain never goes negative; fill/drain round-trips to zero;
  deterministic.
- 114 pc3d tests green (+2); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-305.md`. Next: P3D-306 — the
  independent flow-consumer query contract + visible machine proof.

## 2026-09-04 — rivers visibly flow (loop 380, P3D-304)

- **Rivers render from flow records — no particles, no simulation.**
  `render_flow_map`: the dimmed biome map, then every river edge drawn
  as a stroke from region center toward downstream — width sub-linear in
  discharge (wide because it gathered, clamped 1–6 px), brightness
  monotonic in slope (fast water whiter). Byte-deterministic.
- **`--flow-map <seed>`** renders it; the PNG (human-eye PASS) shows
  bright blue strokes flowing downhill, branching and widening toward
  the coast. `hydro::wetness_at_mm` gives consumers the wetness
  accessor at world coordinates.
- 112 pc3d tests green (+3: width/shade monotonicity + clamping,
  flow-map determinism + stroke presence + seed sensitivity, wetness
  consistency). Root untouched at 474; smoke OK. Contract at
  `docs/POORCRAFT-3D/contracts/P3D-304.md`. Next: P3D-305 (reservoirs —
  with an honest gate decision).

## 2026-09-04 — damming reroutes rivers locally (loop 379, P3D-303)

- **Terrain edits reroute water locally and deterministically.**
  `hydro::RiverGraph::build(gen, half, overrides)` — elevation overrides
  from edits (delta per region, clamped) rebuild the watershed through
  the same steepest-descent/accumulation pipeline.
- **Dirty-region revisions**: `FlowTable::from_graph_with_revisions` —
  a region whose record is semantically equal keeps its revision; a
  changed region increments; the table revision advances by one per
  rebuild. An edit cannot rewrite the whole river map.
- **The reroute tests learned the real physics** (each iteration caught
  by its own proof): raising a region changes nothing (its neighbors are
  fixed) — you reroute by LOWERING a non-downstream neighbor below the
  current steepest-descent target; and the changed-set is exactly the
  lowered region plus its Chebyshev-1 neighbors (direction changes
  require an adjacent elevation change) — everything farther keeps its
  flow. Rebuilt graphs re-proven acyclic; revision churn isolated
  (changed > 0 AND kept > 0).
- 109 pc3d tests green (+2); root untouched at 474; smoke OK. Contract
  at `docs/POORCRAFT-3D/contracts/P3D-303.md`. Next: P3D-304 flow
  rendering.

## 2026-09-04 — water remembers: flow records + ports (loop 379, P3D-302)

- **Every river region carries a versioned flow record.**
  `pc3d_world::flow`: `FlowRecord` (8-compass direction, fixed-point
  per-mille slope never negative toward downstream, discharge,
  slope-scaled capacity, revision) derived deterministically from the
  river graph; `FlowTable.revision` starts at 1 and `bump_revision()`
  moves every record together.
- **The port matching law**: a region's exit toward downstream and the
  downstream's entry share the identical border-midpoint world position
  and discharge — neighbors can never disagree about water crossing
  between them (ports matched by POSITION; discharges tie between
  neighbors).
- **Flow tables persist** (`pc3d_save::flow_store` at
  `water/flow.p3d`): fixed-width 48-byte records through the header
  law; a 47-vs-48 pad bug was caught by the round-trip, and the refusal
  probe needed ≥ 16 bytes to even reach the magic check.
- 107 pc3d tests green (+4); root workspace untouched at 474; smoke OK.
  Contract at `docs/POORCRAFT-3D/contracts/P3D-302.md`. Next: P3D-303
  dirty-region flow rebuild.

## 2026-09-04 — the watershed: rivers from terrain (loop 378, P3D-301)

- **The water stage opens with real geography.** `pc3d_world::hydro`:
  `RiverGraph` derives the watershed from the seed's macro elevation —
  every region drains to exactly one lower neighbor (8-neighborhood
  steepest descent, deterministic tie-breaks), discharge accumulates
  downstream in descending-elevation order, and edges with discharge
  ≥ `RIVER_THRESHOLD` (64) are rivers. Wetness adds a decaying
  river-corridor bonus to humidity (D-016 wet corridors).
- **The conservation test caught a real bug**: a node's own drop used
  `max(1)` instead of `+= 1`, so any node with inflow never counted
  itself — discharge was wrong wherever rivers merged. Acyclicity
  (every chain terminates), strict downhill flow, and downstream
  conservation are all test-enforced; rivers exist in ≥ 4/6 seeds.
- The atlas draws river regions blue over biomes (human-eye PASS;
  density tunable via `RIVER_THRESHOLD`).
- 103 pc3d tests green (+3); root workspace untouched at 474; smoke OK.
  Contract at `docs/POORCRAFT-3D/contracts/P3D-301.md`. Next: P3D-302
  persistent flow records.

## 2026-09-04 — the terrain debug overlay; P3D-200 complete (loop 377, P3D-207)

- **Streaming/terrain state is inspectable without a renderer.**
  `pc3d_world::debug_overlay`: per-patch rows (coord, LOD, biome,
  elevation, edit count, built count) over the interest set, plus a
  deterministic LOD-ring atlas — `poorcraft3d --debug-overlay <seed>`
  renders it and prints the ring census (Full 1 / Mid 4 / Far 42 /
  Horizon 1042 concentric rings, human-eye PASS).
- **P3D-200 hybrid-terrain stage COMPLETE**: extraction bake-off (201),
  final-solid query (202), caves/cliffs (203), editing/journals/
  compaction (204), construction overlay (205), LOD+seams (206), debug
  overlay (207) — 100 pc3d tests green; root untouched at 474. Next
  stage: P3D-300 water and environment.

## 2026-09-04 — LOD selection + the seam law (loop 376, P3D-206)

- **Every patch maps to exactly one detail level** by distance —
  `pc3d_world::lod`: Full/Mid/Far/Horizon over configurable 96/320/1024 m
  bands matching the interest tiers; selection proven monotonic with
  exact band edges.
- **The seam law is proven, not hoped for**: `seam_signature` hashes the
  effective surface heights + material codes along a border strip; every
  neighbor pair across a 9×9 patch grid agrees exactly on both axes for
  4 seeds. Because border answers are pure functions of world position,
  a LOD transition cannot open a crack — the release-blocking defect is
  impossible at the query level.
- 97 pc3d tests green (+4); root untouched at 474; smoke OK. Two
  self-caught bugs fixed pre-commit (seam border off-by-one; double-scaled
  test meters). Next: P3D-207 debug overlay.

## 2026-09-04 — the construction overlay: builds are owned and protected (loop 375, P3D-205)

- **Terrain blueprint layer 3 is real.** `pc3d_world::build`:
  `Construction` (16³ explicit build slots per patch),
  `BuildBlock { material, owner }`, and cell-precise `BuildOp`s
  (Place/RemoveBuild, fixed-width encoding, canonical (tick, id) replay
  with violated ops skipped and counted).
- **Ownership is enforced**: only the owning authority removes a build;
  placing over an existing build is refused.
- **The priority law is test-enforced**: `edit::apply_edit` now takes the
  construction registry and natural dig/fill SKIP built cells — the
  machine-protection test digs a 3×3×3 brush across a built cell and
  proves the build survives while the surrounding terrain is dug.
  `effective_answer` makes built cells WIN in the single world answer;
  with no overlay it is bit-identical to the natural answer.
- **Builds persist through the law**: `pc3d_save::build_journal` stores
  build journals and construction snapshots (`edits/b….build`,
  `s….bsnap`) — reload preserves cells, materials, and owners; foreign
  and wrong-version files refuse as always.
- 93 pc3d tests green (+8); root workspace untouched at 474; smoke OK.
  Proof-caught bugs fixed en route: snapshot index decomposition sent
  cells out-of-patch on reload, i64/usize mismatch, negative-cell
  placement. Next: P3D-206 LOD rings and seam handling.

## 2026-09-04 — the shovel is real: terrain editing (loop 374, P3D-204)

- **Dig and fill with bounded brushes.** `pc3d_world::edit`: `Brush`
  (bounded Chebyshev cube, clamped), `EditOp` (id + tick + kind +
  material, fixed-width 48-byte encoding, unknown codes refuse to
  decode). `apply_edit` is patch-local: dig removes only solid cells,
  fill only fills air, cells outside the patch are untouched, and the
  changed count is returned.
- **Patch-local invalidation is exact** — `affected_patches` returns
  precisely the patches a brush cube intersects, including
  boundary-straddling edits. `replay` applies ops in canonical (tick, id)
  order over the regenerated base, so delivery grouping and reordering
  are provably inert, and untouched patches still never need storing.
- **Compaction is lossless**: a journal can compact to a full-cell
  `Snapshot` whose cells are byte-equal to the replayed result, with
  stable material codes for persistence.
- **Journals and snapshots persist through the law** (`pc3d_save::journal`
  at `edits/j…`/`s….snap`): foreign files, wrong versions, and corrupt
  payloads refuse exactly like patches. The shovel promise is test-proven:
  dig a crater, reload — byte-identical; neighbors' patches never change.
- 85 pc3d tests green (+9); root workspace untouched at 474; smoke OK.
  Contract at `docs/POORCRAFT-3D/contracts/P3D-204.md`. Next: P3D-205,
  the construction overlay.

## 2026-09-04 — caves, cliffs, overhang-capable terrain (loop 373, P3D-203)

- **The terrain gained its third dimension.** `gen::effective_surface_mm`:
  in rare 60 m noise-masked bands the surface quantizes to crisp 4 m
  terraces — real cliff faces (detail noise is suppressed inside bands so
  the step is a cliff, not a smear; the first cut's smeared 4-m-over-8-m
  gradient was refused by the cliff test). `gen::is_carved`/`carve`:
  worm-like cave voids from two intersecting mid-band trilinear 3D noise
  fields.
- **Sealed-volume law, test-enforced**: carving requires a 4 m solid crust
  below the surface, never below y = 0 (water seal — oceans cannot drain
  into caves until hydrology lands), never beyond a 120 m deep crust; a
  441-land-patch scan proves caves exist and every carved cell obeys the
  band.
- **The P3D-202 single-answer guarantee survives**: `regenerate_patch`
  and `final_solid` share the carve step, and the agreement matrix
  re-proved over carved patches.
- **The deferred cliff scene is real**: `SceneSpec::Cliff` seeks its
  terraced patch deterministically (256/256 columns in-window) and
  joined the bake-off. Re-measured: heightfield extract 33–80 µs /
  rebuild 66–219 µs / fidelity 0.505–0.667 m vs density 4124–4153 µs /
  0.456–0.805 m — **heightfield still wins with cliffs included**.
- 76 pc3d tests green (+2); root workspace untouched at 474; smoke OK.
  Contract at `docs/POORCRAFT-3D/contracts/P3D-203.md`. Next: P3D-204
  terrain editing — dig, fill, journals, and compaction.

## 2026-09-04 — one answer for the whole world (loop 372, P3D-202)

- **`final_solid(gen, wx, wy, wz) -> SolidAnswer`** is now the single
  authoritative query for terrain solidity and material — the blueprint's
  "single authoritative final-solid query". Mesh, collision, water, and
  navigation will all call it; none may reimplement surface logic.
- **Agreement is structural, not lucky**: the material rules were
  extracted from `regenerate_patch` into one shared `gen::cell_material`
  function that both patch regeneration and `final_solid` call. The
  agreement test walks 4 patches × 4096 cells × 3 seeds proving every
  regenerated cell equals the query at its world position — material AND
  solidity.
- **Water semantics probed** at real ocean regions: Water above the
  quantized floor (not solid), solid floor beneath, Air above the land
  surface. One quantization artifact documented rather than hidden: an
  ocean floor at an exact meter makes its top cell solid sand — agreed
  by every consumer because they share the function.
- 74 pc3d tests green (+2); root workspace untouched at 474; smoke OK.
  Contract at `docs/POORCRAFT-3D/contracts/P3D-202.md`. Next: P3D-203
  caves, overhangs, and cliffs — the 3D density fields.

## 2026-09-04 — the surface bake-off: heightfield wins on measurements (loop 371, P3D-201)

- **The hybrid-terrain stage opens the way the blueprint demands: measure
  first.** `pc3d_world::terrain` implements two extraction candidates over
  three shared scenes (SmoothHills, Highlands, Coast — region-center
  patches whose y-window contains the analytic surface) and measures
  extraction time, memory, post-edit rebuild, and column fidelity vs the
  height function, with a measured-columns count so fidelity numbers
  cannot be vacuous.
- **The table (release, 256 columns/scene):** heightfield extracts in
  98–107 µs with 0.478–0.505 m mean error; density-threshold (2×2
  quarter-meter sub-samples) in ~396 µs with 0.456–0.497 m. Density's
  smoother silhouettes cost 4× for ~0.03 m — both are 40–160× under the
  16 ms edit-hitch budget. **Decision: heightfield** becomes the
  authoritative final-solid query in P3D-202; the harness re-measures
  when 3D density fields land.
- **A real finding**: the pure-heightmap generator produces NO sharp
  cliffs (no adjacent-region step ≥ 25 m anywhere in wide sweeps) — the
  blueprint's cliff scene is deferred to P3D-203's density fields, and
  Highlands stands in. Recorded in the contract, amended by measurement.
- Scene pins survived three self-caught bugs (corner-vs-center patches,
  patch-index-vs-meters y, highlands 5·16 = patch 80). 72 pc3d tests
  green (+5); root workspace untouched at 474; smoke OK.

## 2026-09-04 — interest rings + bounded queues: P3D-100 stage complete (loop 370, P3D-105)

- **The world knows what to stream and refuses to drown.**
  `pc3d_world::stream`: interest tiers at the blueprint's proposal radii
  (full 96 m / LOD 320 m / macro 1024 m), `interest_patches` mapping a
  viewer onto ascending bounded patch sets, `interest_diff` producing
  deterministic load/unload plans, and `BoundedQueue` — fixed capacity,
  overflow HANDS THE WORK BACK to the caller (never drops, never grows
  unbounded), cumulative counters keeping the backlog visible.
- **The teleport scenario is proven, not promised**: an 80 m jump yields
  a 112-patch diff; the queue admits 8 per wave; re-push is gradual;
  rejections accumulate across waves and every rejected item stays held,
  counted — no unbounded growth, no lost work.
- **P3D-100 world-substrate stage complete**: coordinates/patches (101),
  persistence with the refusal law (102), coherent generation (103),
  atlas evidence (104), streaming (105) — 67 pc3d tests green, root
  workspace untouched at 474, smoke OK. Next stage: P3D-200 hybrid
  terrain, opening with a measured surface-extraction bake-off (P3D-201).

## 2026-09-04 — seed atlases: geography you can see (loop 369, P3D-104)

- **The generator carries its own evidence.** `pc3d_world::proof`: a pure
  atlas renderer (one pixel per region, biome-colored, elevation-graded,
  byte-deterministic), the cross-seed categorical disagreement gate
  (ported from the original game's N06 pattern), and patch-hash spot
  verification. `poorcraft3d --atlas <seed>` writes real PNGs and prints
  the census; `make p3d-atlas SEED=n`.
- **The atlas proof drove the generator's design through three visible
  iterations**: 1-4 km octave cells rendered as confetti (a D-016
  violation, caught by looking at the PNG), 32-km cells rendered as one
  flat biome per seed (100% cross-seed disagreement = two flat worlds),
  and the landing point — 12/6/3 km octave cells plus a sea-level-centered
  elevation mapping — renders seed 1 as a green continent with coastline
  rings, wetlands, and two oceans, seed 7 as a different landmass with an
  eastern ocean and bays. Both PNGs human-eye PASS; categorical
  disagreement 48.9-93.6% across seeds, 0% self-disagreement.
- The coherence sweep widened to 80x80 regions x 8 seeds (it now crosses
  continental bands), and the ocean/grass material probes target the
  region-center patch at the surface's y-level — corner patches near
  coasts are legitimately dry land.
- 63 pc3d tests green (+5); root workspace untouched at 474; smoke OK.
  Atlas PNGs committed as evidence. Next: P3D-105 interest management +
  bounded streaming queues.

## 2026-09-04 — coherent procedural geography (loop 368, P3D-103)

- **The first world content: seeds now produce geography, not confetti**
  (D-016). `pc3d_world::gen`: one CONTINUOUS 3-octave value-noise
  elevation field shared by the macro fields (per 256 m region:
  elevation/temperature/humidity) and THE global height function — so a
  region's center height equals its field elevation exactly, and biome
  and ground can never disagree. Continuous detail noise (±1.5 m) keeps
  terrain from being flat ramps.
- **Eight biomes read from the fields** — Ocean, Coast, Plains, Forest,
  Wetland, Highlands, Mountains, SnowPeaks — with the coherence rules
  PROVEN across 24 seeds × a 17×17 region grid (oceans only below sea
  level, SnowPeaks only high+cold, Wetland only humid lowland; all major
  biomes reachable).
- **Deterministic patch regeneration**: `regenerate_patch` builds 16³
  material cubes purely by sampling the height function — untouched
  patches never need storing (terrain blueprint layer 1), the same seed
  replays cell-identically, and seam tests prove continuity across patch
  AND region borders.
- **The proofs caught a real design flaw**: the first draft blended
  NEIGHBORING regions' fields for height, letting a Plains-labeled region
  sit 30 m underwater — replaced by the single continuous field. Also
  removed a 256× per-patch recomputation that put regeneration far over
  budget.
- 58 pc3d tests green (+6); root workspace untouched at 474; smoke OK.
  Contract at `docs/POORCRAFT-3D/contracts/P3D-103.md`. Next: P3D-104
  seed atlas + patch-hash proof tools.

## 2026-09-04 — worlds persist: the patch store (loop 367, P3D-102)

- **New crate `pc3d_save`** — the only code that touches `saves3d/`.
  Every file is framed as 16-byte P3D header | payload length u64 LE |
  payload | FNV-1a-64 checksum, byte-pinned; `saves3d/<world>/world.p3d`
  carries the seed+name meta, patches live at deterministic keys
  `patches/p<x>_<y>_<z>.patch`.
- **Saves are atomic by construction** (temp file + rename): a crash
  leaves only a `.tmp` no loader reads, and rename consumes it. Every
  load runs the P3D-002 version law before length and checksum — foreign
  files, wrong epochs, newer/older sections, truncation, and bit-flips
  each refuse with the precise numbers and a human line.
- **The version law itself got harder**: `pc3d_core::open_decision`
  panicked on 4..16-byte inputs (the magic guard certified too little for
  decode) — now an explicit header-length floor, with a loop test over
  every partial length. Found by the framing tests; the refusals got
  MORE precise too: a readable-but-truncated frame says
  `LengthMismatch{declared, actual}`, not a generic "too short".
- 52 pc3d tests green (+10 incl. the law-hardening regression); root
  workspace untouched at 474; smoke OK. Contract at
  `docs/POORCRAFT-3D/contracts/P3D-102.md`. Next: P3D-103, the first
  world content — procedural macro terrain and biomes.

## 2026-09-04 — the world has a spatial language (loop 366, P3D-101)

- **New crate `pc3d_world`** (zero dependencies, pure integer geometry)
  opens the P3D-100 world-substrate stage: `WorldPos` is signed 64-bit
  millimeters — stable large-world identity with no float drift — mapped
  by Euclidean floor division onto 1 m cells, 16 m patches, and 256 m
  macro regions. Negative coordinates behave like a globe: x = -0.5 m
  lives in cell -1.
- **The blueprint scales live in exactly one module** (compile-time
  coherence asserts included), so the still-pending owner decisions
  P-001/P-002 re-scale the world in a single file when answered.
- **Bounds algebra and bounded queries**: edge-inclusive bounds with
  union/intersect/center, cell counts that SATURATE instead of overflowing
  on planet-sized spans, and `patches_touching`/`regions_touching` that
  iterate strictly ascending and refuse absurd bounds with
  `TooManyPatches{requested, cap}` — the substrate no-hang promise
  behind "one patch rebuild independent of total world size".
- 43 pc3d tests green (+11); root workspace untouched at 474; smoke OK.
  Contract at `docs/POORCRAFT-3D/contracts/P3D-101.md`. Next: P3D-102
  patch store + atomic save, composing the header law with patch coords.

## 2026-09-04 — the new game runs: first runtime + smoke (loop 365, P3D-005)

- **`poorcraft3d --run` is a real loop.** `WorldRuntime` owns the fixed
  clock, counters, frame times, journal, command sequencer, and seed
  streams — the skeleton every future host (solo or dedicated) reuses.
  Frames advance ticks; ticks do counted, journaled work; a heartbeat
  event fires every 600 ticks with a seed-mixed payload so two worlds
  differ even at equal tick counts.
- **`make p3d-smoke` guards liveness** the way `make smoke` guards the
  original game: the headless run prints `300 frames · 400 ticks · 290
  journal events · p50 22.12 ms` and `P3D SMOKE OK`, exit-code-asserted.
  Same-seed runs replay to the identical digest; each seed jitters its own
  frame stream and replays itself.
- **P3D-000 stage complete**: identity + save guard (001), format law
  (002), deterministic spine (003), profiler + baseline (004), runtime +
  smoke (005) — 32 pc3d_core tests green, root workspace untouched at 474.
  Next stage: P3D-100 world substrate (P3D-101 coordinates/patches).

## 2026-09-04 — the engine measures itself: profiler + baseline (loop 364, P3D-004)

- **The performance-principles counter vocabulary exists in one place.**
  `pc3d_core::profile`: eight `CounterId`s (mesh work, fluid work, path
  requests, entity ticks, network bytes, save bytes, patch rebuilds,
  journal events) with saturating increments and enum-ordered snapshots —
  subsystems increment, never redefine. `FrameTimes`: fixed-cap ring,
  nearest-rank percentiles proven on a known sample, invalid samples
  dropped, arrival-order digest. `MemoryCounters` with net arithmetic.
- **`poorcraft3d --baseline` prints a diffable profile record** built from
  a deterministic synthetic workload run through the real `FixedClock`:
  two consecutive invocations are byte-identical (same sha256). First
  measured baseline: 600 frames, p50 22.4 ms / p95 31.8 ms, 805 entity
  ticks — the fixed clock tracked the synthetic wall time exactly.
- 28 pc3d_core tests green (+4); root workspace untouched at 474. The
  measuring stick exists; performance budgets arrive with the subsystems
  they will measure (P3D-201+). Next: P3D-005, the first runtime.

## 2026-09-04 — the deterministic simulation spine (loop 363, P3D-003)

- **Five pure modules give POORCRAFT 3D its reproducibility guarantee.**
  `clock::FixedClock` (integer-microsecond 60 Hz ticks, range-returning
  advance so every fired tick carries its own number, 8-tick shed cap),
  `command` (restorable sequencer + envelopes with total (tick, id) order
  and keep-earliest duplicate suppression), `journal` (append-only events
  under a dense monotone seq with an FNV-1a-64 digest),
  `seed::SeedStreams` (per-subsystem RNGs derived from one world seed via
  named labels + SplitMix64 — subsystem consumption can never cross-
  contaminate), and `replay::ReplayDigest` with the harness tests.
- **The replay property is enforced, not claimed**: the representative
  command-driven cart produces IDENTICAL state and journal digests over
  the same 600 sim ticks under uniform, 3-tick, and mixed frame
  partitions, and under one-by-one versus single-batch delivery.
- 24 pc3d_core tests green (+13); root workspace untouched at 474. The
  contract was filled first (`docs/POORCRAFT-3D/contracts/P3D-003.md`).
  One test-side bug (a snapshot pattern that re-read the same value
  instead of advancing) caught and fixed before commit.

## 2026-09-04 — the format law: versioned headers + refusals (loop 362, P3D-002)

- **Every POORCRAFT 3D file now begins with a 16-byte versioned header** —
  magic `PC3D` | format epoch (u32 LE) | world, save, content, protocol
  (u16 LE each). The layout is the contract: hand-rolled byte-exact
  encode/decode, no serde, pinned in a test against a hard-coded byte
  vector so the layout can never drift silently.
- **`open_decision` is the only way anything opens a P3D file**, layered
  outside the P3D-001 magic guard: `Accepted`, `TooShort`, `ForeignFormat`,
  `UnknownEpoch`, `Newer{section,file,supported}`, `Older{...}` — every
  refusal carries exact numbers AND a human line ("update the game" vs
  "this build cannot downgrade"). Policy at epoch 1: any section mismatch
  refuses; nothing migrates silently (D-002).
- Six header tests: layout stability, round-trip with trailing payload,
  per-section newer/older refusal across all four axes with first-offender
  determinism, unknown-epoch rejection, guard layering, decision
  determinism. `poorcraft3d --format` prints the layout, supported
  versions, and wire bytes. Task contract filled first
  (`docs/POORCRAFT-3D/contracts/P3D-002.md`).
- P3D workspace **11 tests green**; root workspace untouched (474 green);
  runtimes not rebuilt (no `lf_*` changes; the P3D binary is still a
  purposeful stub). Next: P3D-003 deterministic primitives + replay harness.

## 2026-09-03 — POORCRAFT 3D is born: P3D-001 workspace, identity, save guard (loop 361)

- **The greenfield successor exists as code, not prose.** New nested Cargo
  workspace `poorcraft3d/` (independent target dir; root workspace
  membership untouched), crate `pc3d_core`, binary `poorcraft3d` — which
  already answers `--identity`: name, executable, save root `saves3d`,
  magic `PC3D`, format epoch v1, and the explicit "no POORCRAFT
  compatibility" statement (D-001/D-002).
- **The no-accidental-save-sharing guard is tested, not promised.**
  `refuse_foreign_save()` decides from header bytes alone — pure, no IO:
  only `PC3D`-magic files are accepted; LOREFORGE-style saves are refused
  as `ForeignFormat`; truncated headers are refused as `TooShort`. Five
  tests pin every separation invariant (executable, save dir, magic all
  differ from `loreforge`/`worlds`; substring checks both ways;
  case-strict magic).
- **The task contract came first** (`docs/POORCRAFT-3D/contracts/
  P3D-001.md`, filled per `11-TASK-CONTRACT-TEMPLATE.md`), and the full
  `docs/POORCRAFT-3D/` design pack (README + 00–21) is committed with
  this work.
- `make p3d-build` / `make p3d-test` added; `poorcraft3d/target` ignored.
  The original game is untouched: root `cargo test --workspace` still
  **474 green**; the P3D suite is **5 green**. Runtimes not rebuilt (no
  `lf_*` game code changed; the P3D binary is a stub until P3D-005).
- Track note: the BETA-FOUNDATION loop for the original game is parked at
  B04 (conserved fluid state), resumable on the owner's word.

## 2026-09-03 — crafting through the host; Stage A complete (loop 360, B03 slice 2)

- **Crafting transactions are host commands now, and the B03 gate is
  passed.** `HostCommand::Craft` joins the host vocabulary:
  `queue_craft` / `apply_pending_crafts` (canonical (tick, id) order
  with per-command receipts) and `craft_now` (the same same-frame
  semantics block edits use). Every applied craft records an
  `EV_CRAFT` event (output hash + qty + granted); every blocked or
  replayed craft records `EV_CRAFT_REJECT` with a reason code plus the
  player-facing line in the receipt.
- **Both real craft paths migrated** — the craft-queue tick and the
  workbench button call `host.craft_now`; the lf_game transaction
  engine still guarantees atomicity (a blocked craft consumes nothing),
  and the host adds command identity, replay dedup, and the event
  trail. The ui.rs queue PROBE stays direct: it previews against a
  scratch inventory, mutating no client state.
- **The self-audit covers crafting too**: a new test pins the probe as
  the ONE allowed direct engine call and rejects any fully-qualified
  `crafting::execute` in client code (the pattern is built with
  `concat!` so the audit cannot match its own source — the first
  version counted itself and failed, which is the audit working).
- **Stage A closes**: the milestone gate "local host owns blocks and
  inventory" is met — direct client mutation is test-rejected for both
  migrated systems, existing saves still load (the host holds no
  persisted state), and the onboarding/craft journey passes through the
  host in the headless smoke.
- 474 tests (+4), smoke OK, 107/107 vistest, runtimes rebuilt. Stage B
  (conserved water) starts at B04.

## 2026-09-03 — authoritative host owns block edits (loop 359, B03 slice 1)

- **The client no longer edits the world directly — at all.** New pure
  `lf_game::host::SimHost`: systems queue block commands (monotonic id +
  sim tick + `EditKind` reason) and the host applies them to the voxel
  world in canonical (tick, id) order, recording one domain event per
  outcome. Replayed ids are rejected (a packet-loss duplicate applies
  once, ever); out-of-loaded-space edits are rejected **with events**,
  never silence.
- **All 16 runtime `world.set_block` sites migrated** through two
  funnels: `host_set_block` (same-frame apply + remesh + network
  broadcast — mining, placing, scaffold columns, symmetry mirrors, tree
  felling, paste builds, lumen blocks, hearth expiry, crater residue,
  support-removal fallers) and `apply_remote_block_update` (multiplayer
  BlockUpdate: recorded by the host but never re-broadcast — the echo
  loop is structurally impossible now).
- **The contract is test-enforced in both directions**: host unit tests
  prove idempotence, rejection-with-events, total ordering, and
  event-sourced history; and a new client **self-audit test**
  `include_str!`s its own source and rejects any `.set_block(` outside
  the two funnels or test code — reintroducing a direct edit fails the
  suite, exactly B03's "direct client mutation is test-rejected".
- **Multiplayer fix shipped by the seam**: player mining never broadcast
  to the server before (the server never learned mined blocks); the
  funnel's uniform broadcast now covers every local edit. Save
  compatibility untouched — the host keeps no persisted state this
  slice.
- 470 tests (+4 host, +1 self-audit), smoke OK (the headless
  onboarding/craft journey runs through the host), 107/107 vistest
  freshly rendered, runtimes rebuilt. Slice 2 (craft/inventory
  transactions through the host) closes B03 and Stage A.

## 2026-09-03 — deterministic tick, command IDs, domain events (loop 358, B02)

- **The simulation now has a clock that render cadence cannot bend.** New
  pure `lf_game::sim`: `TickClock` converts the client's variable
  real-frame dt into whole 60 Hz sim ticks through an integer-microsecond
  accumulator with an 8-tick shed cap (a one-second freeze fires exactly
  the cap and drops the backlog — deterministic, never a death spiral).
  `advance()` returns the inclusive RANGE of ticks to execute: the
  original count-returning API was a real design flaw the cadence proof
  caught — looping `0..count` collapses multi-tick frames onto the last
  tick number and silently skips the others.
- **Commands and events get total order and stable fingerprints.**
  `CommandEnvelope` (monotonic id + issuing tick) with `canonical_batch`
  ordering by (tick, id), duplicates keeping their earliest occurrence —
  batching and reordering are provably inert. `EventLog` records
  immutable domain events under a dense monotone seq, hashed FNV-1a over
  (tick, seq, kind, payload) — no randomized `DefaultHasher`.
- **The done-when is enforced, not claimed**: snapshot-hash tests run a
  representative command-driven sim under uniform 60 fps, 3-tick, and
  mixed 1/2-tick frame partitions over the same 600 ticks — identical
  state and event-log hashes; one-by-one vs single-batch delivery hash
  identical; jitter streams replay identically and track wall time within
  2 ticks; overload shedding is deterministic; hashes are
  perturbation-sensitive.
- **Fixed a real pre-existing race the new tests exposed**: crafting's
  mod-recipe tests guarded the global registry with private mutexes (and
  `mod_recipes_match` didn't lock at all), so
  `mod_fingerprint_tracks_the_mod_set` could fail under load — all three
  now share one `MOD_REGISTRY_LOCK`, stress-verified at
  `--test-threads=8`.
- 465 tests (+6 sim), smoke OK, runtimes rebuilt. Client wiring is
  deliberately untouched — B03 migrates block edits and inventory/crafting
  through commands behind the integrated host.

## 2026-09-03 — runtime truth dashboard (loop 357, B01)

- **Stage A of the beta-foundation roadmap opens with honesty
  infrastructure.** New `xtask/src/truth.rs` publishes a machine-readable
  dashboard of what the engine actually is: 21 systems across four
  ownership classes (ServerAuthoritative / RelayOnly / ClientLocal /
  DeterministicGenerator), schema versions (protocol v4, generator v7),
  scene and test counts, optional live perf, and seedlab fold-through —
  written to `target/truth_report.json` by `xtask truth` / `make truth`.
- **The dashboard cannot lie about authority.** Every
  `ServerAuthoritative` row must cite the server crate AND carry marker
  strings checked against the live `lf_server` source at compile time
  (`include_str!`) — claims without implementations fail, and losing an
  implementation (e.g. someone rips out SetBlock handling) fails too.
  The 13 systems the loop-356 audit names client-simulated are pinned
  `ClientLocal`: relabeling one requires deleting a line from
  `KNOWN_CLIENT_ONLY` — a reviewable contract change, never silent.
  Evidence paths and audit-list coverage are validated at runtime in
  `build_report` as well.
- Truthful baseline recorded: server authority is exactly
  `session_and_peers` + `block_edits`; everything else is relay
  (presence/chat/trade/UDP) or client-local until B03 starts the
  migration. Also committed the `docs/BETA-FOUNDATION/` goal pack and
  the MASTER-PLAN deprecation pointer.
- 459 tests (+6 truth-contract tests), smoke OK. Tooling-only job —
  dist binaries unchanged, runtimes not rebuilt.

## 2026-09-03 — the biome identity contract (loop 356, N07)

- **Forty-six biomes, and now a law: none may look like another.** Every
  biome carries an identity row — surface, filler, tree silhouette,
  ground-cover density and feature set, freeze, cold — and a test
  enforces that no two biomes share the whole visible tuple. The scan
  found one surviving clone family: the oceans, which now have three
  distinct floors (murky dirt Ocean, tropical-sand Warm Ocean, pale-stone
  Deep Ocean). Generator version bumped to v7 for the change.
- **Ground cover has a confetti ceiling.** Feature density is capped at
  0.35 by test (Jungle, Lavender Fields, and Sunflower Plains were over);
  identity reads from negative space, not from filling every cell.
- **The proofs photograph the identity itself.** The biome contact sheet
  now grows each biome's signature tree over its real surface and
  ground-cover features — 46 strips of "what this place is" — and the
  new `biome_transitions` scene shows four intentional boundary pairs
  (desert/savanna, snowy plains/taiga, meadow/forest, swamp/jungle), each
  joined by a dithered mixing band rather than a hard seam. Both
  image-reviewed; 107/107 visual scenes.
- **Proof:** 453 tests, the 64-seed lab still passing its diversity
  floors on v7, smoke green, runtimes rebuilt. Per-biome fog/sky grading
  and gameplay resource rows are honestly deferred (fog needs an engine
  input the atmosphere doesn't have yet).

## 2026-09-03 — every seed knows where you wake up (loop 355, N06)

- **Spawn is selected, not assumed.** A deterministic spiral search picks
  the actual spawn: dry land above the sea, a land biome, off rivers, no
  tree in the cell, no kingdom on top — and it reports the nearest wood.
  Ocean-centered seeds used to drop the player into the water at (0,0);
  the seed laboratory caught exactly that (0 of 64 origin spawns were
  valid) and now scores the real selection instead of a proxy. New
  worlds and loads place the player — and the respawn point — there,
  with an arrival hint naming the biome and wood distance.
- **The seeds render their evidence.** Three new proof scenes:
  `seed_atlas_8` paints eight labeled maps straight from the generator's
  biome and height fields at the laboratory's macro scale, gated by a
  cross-panel disagreement metric; `seed_same_control` regenerates half
  the world from the same seed and requires cell-identical columns plus
  a seamless render; `spawn_quality_8` lists eight find_spawn verdicts
  with every safety invariant asserted before a pixel draws. Building
  the atlas gate exposed (and fixed) an inverted-range bug in its own
  sampler where every panel but the first sampled zero pixels.
- **Measured, again.** The 64-seed lab passes its diversity floors by
  wide margins on generator v6 (height L1 p05 0.090 vs 0.020 floor;
  biome JS p05 0.214 vs 0.025), so this pass repaired the one real
  defect the metrics named — spawn — and left terrain shape alone.
- **Proof:** 451 tests (+1: spawn safety/reachability across a 16-seed
  spread + determinism of the selection), 106/106 visual scenes, all new
  scenes image-reviewed at 0.95–0.98 confidence, smoke green, runtimes
  rebuilt. True multi-viewport panorama atlases and the river/biome
  transition scenes are honestly deferred (N07).

## 2026-09-03 — world identity and the seed laboratory (loop 354, N05)

- **A world has a name that means something.** The canonical
  `WorldIdentity` — seed, generator version, world type, and a
  fingerprint of everything modded that can touch generation — is
  stamped into `identity.dat` before the first chunk generates, restored
  exactly on load, and shown on F3. A save from an older generator now
  tells you plainly that unedited areas may have changed, and modded
  recipe/ore-hook sets are part of the world's provenance.
- **Same seed, same world — proven, not promised.** The new seed
  laboratory hashes height and biome lattices with order-independent
  combiners: the same identity is bit-identical across instances
  (negative, ±1M, and ±i32::MAX coordinates included), and generating
  chunks in any order produces identical columns.
- **Different seeds, different worlds — measured.** 64 fixed seeds are
  sampled into a machine-readable report (`make seedlab` →
  target/seedlab_report.json): height-field distances, biome
  Jensen–Shannon distances, water/river/cave fractions, surface
  composition, and kingdom placement, with calibrated diversity floors
  the reduced 12-seed corpus also enforces in the test suite. Generator
  v6 measures height L1 p05 = 0.090 (floor 0.020) and biome JS p05 =
  0.214 (floor 0.025) — comfortably diverse; N06 now has numbers to
  repair against instead of opinions.
- **Multiplayer shares one world.** The Welcome packet always carried the
  server's seed; the client now adopts it as its world identity and
  restarts streaming when it differs, instead of silently generating a
  private world.
- **Proof:** 450 tests (+12: seed-text rules, hash stability, identity
  round-trip, version policy, channel decorrelation, corpus diversity,
  bit-identity, order independence, JS behavior, mod fingerprints),
  smoke green, runtimes rebuilt. The rendered seed atlases are N06's
  evidence by design.

## 2026-09-03 — the HUD speaks context (loop 353, N04)

- **What you can do, right now, is written beside the crosshair.** A
  prompt shows the live keymap chip, the verb, and the target — "E Trade
  — Mara", "LMB Hold to mine — Iron Ore", "RMB Place — blocked by
  player" — with hostile standing barring the door in red. Priority
  resolves companion > villager > functional block > mine > place, and
  rebinding the key changes the chip.
- **Combat reads at a glance.** A red arc flashes around the crosshair
  from the direction damage came (world-true: it stays put while you
  turn), an attack-readiness ring sweeps while the swing cools down, and
  one priority-resolved danger line (drowning > critical health >
  starving > threats) sits above the hotbar on a dark plate — a fix the
  image review forced after catching the warning drowning in busy
  terrain.
- **The world reacts visibly.** Standing changes now toast with the
  faction crest, the signed delta, the reason ("quest fulfilled",
  "destroyed their structure", "a gift well received"…), and the new
  threshold title when a band is crossed; entering a kingdom announces
  its name and whether its gates are barred to you, once per visit.
- **Proof:** 103/103 visual scenes (four new: hud_contextual at
  1280×800, hud_contextual_small at 640×420, hud_danger, hud_reputation,
  all image-reviewed), 438 tests (+3: prompt copy/keymap chips, transient
  fade/caps, strict danger priority), smoke green, runtimes rebuilt.

## 2026-09-03 — the modal workbench and input recovery (loop 352, N03)

- **The workbench is a modal now, not a transparency.** Strongly opaque
  framed panels sit over a world scrim, and the survival HUD (hearts,
  hotbar, XP, minimap) is no longer drawn beneath any container/station
  screen — the audit's "duplicate HUD through the workbench" failure is
  gone. The zone layout is pure shared math (`workbench_layout`),
  unit-tested to never overlap or clip at 640×420, 800×600, or 1280×800,
  and the proof scenes sample those same rectangles.
- **Small windows get a drill-down, not small text.** Under 700px the
  workbench collapses to category chips + one pane (recipe list, or the
  detail with a ← back link) over a single-row hotbar strip.
- **Discovery is earned and searchable.** A text search plus filter chips
  (All / Can make / New / ★ Favorites — favorites persist with the save)
  and station chips (Any / Craft / Smelt / Alloy / Crush) narrow the
  list; partial-ingredient rows now carry the amber ~ mark.
- **One key in, one key out.** The rebindable inventory key (E) now
  closes every container/station screen — hand craft, workbench,
  furnace, chest, machine — matching Escape; the contract is a pure
  function tested across every screen variant. Enter fires the primary
  craft action exactly once, and only while the search box doesn't own
  the keyboard.
- **Proof:** the rebuilt `crafting_workbench` scene plus three new ones —
  `crafting_workbench_small` (drill-down at 640×420),
  `crafting_missing_ingredients` (owned/needed marks + the exact reason
  line), and `crafting_queue` (working / blocked-with-reason / queued) —
  all reviewed by image recognition at 0.96–0.97 confidence. 99/99
  visual scenes, 435 tests, smoke green, runtimes rebuilt. Era filter
  chip, substitutions, and queue pause are honestly deferred.

## 2026-09-03 — transactional crafting and a real queue (loop 351, N02)

- **Crafting cannot void or mint anymore.** All craft execution now flows
  through one transactional engine in `lf_game::crafting`: it validates
  every ingredient against real inventory counts, proves the output FITS
  before touching anything, then consumes and grants exactly. A blocked
  craft — short one log or one slot of room — consumes nothing and says
  precisely why ("missing log (need 3, have 1)", "no room — need space
  for 4 more (free 2)"). The old grant loop silently lost outputs past
  the 255-per-insert boundary; the new one batches with zero loss, and a
  regression test crafts 256 planks in one action to prove it.
- **Craft All is integer-safe.** `max_batches` computes the largest batch
  count the inventory supports right now — limited by every ingredient
  count AND by output room — and the button labels the number it will
  make. Rapid double-crafts share the same atomic path: each call
  re-verifies, so materials for one craft can only ever produce one.
- **The queue is real.** "Add to Queue" used to push into a list nothing
  consumed. Now one job completes per 1.25 seconds of play through the
  same engine, the queue strip shows the head's live state (working /
  blocked with the exact reason) with per-job cancel, and the documented
  rule is honest: enqueue reserves nothing, consumption happens only at
  completion, cancel is free. The queue persists across save/load in the
  same ClientSave shape, and a job whose recipe vanished (mod unloaded)
  is dropped with a message instead of spinning forever.
- **Proof:** eight new engine tests (exact consume/grant, both blocked
  paths leave the inventory untouched, >255 outputs, integer-safe
  batches, rapid double-craft, a registered mod recipe executing
  transactionally, reason copy) plus three client tests (queue status
  running/blocked, catalog lookup + unknown outputs, queue save
  round-trip). 433 tests total, smoke green, runtimes rebuilt. The
  visual workbench pass (modal layouts, queue-strip proofs, input
  recovery) is N03.

## 2026-09-03 — first-minute onboarding and the nightly-beta goal pack (loop 350, N01)

- **The first five minutes teach the game.** A persisted tutorial state
  machine (Move → Look → Gather → Craft → Build) advances only on real
  gameplay facts: 3 blocks of horizontal walking, 1.6 radians of camera
  travel, a natural-material drop (log/dirt/stone/sand) reaching the
  inventory, a hand-crafted output, and a solid-block placement — torches
  and flowers are not shelter, a picked-up sword is not mining, and
  out-of-order events never skip steps. Vertical fall does not complete
  "move"; wild camera swinging does not complete it either.
- **The HUD says what to do next.** A compact top-center tutorial card
  shows the verb, keycap chips drawn from the live keymap (rebind
  Inventory to O and the card says O), an n/5 step chip, and a click-✕
  dismiss; beneath it a pinned objective line tracks the first incomplete
  starter quest and its progress ("Punch a Tree · oak log 1/3"). Both are
  painted by shared painters used verbatim by the new vistest proofs, so
  the proof pixels are the in-game pixels. Prompts pause behind modal
  screens, never block input, and skip creative mode.
- **It persists honestly.** Tutorial state rides ClientSave with serde
  defaults (old saves load as a fresh Move tutorial; the legacy bincode
  shape migrates), a new world resets it, and Gameplay settings gained
  "Show first-minute hints" plus "Restart tutorial".
- **The nightly-beta goal pack landed** (`docs/NIGHTLY-BETA/`, 14
  documents: beta gates, HUD/crafting spec, seed diversity contract,
  castle/faction strategy, NPC moral-history model, asset bible, vision
  protocol, performance/hygiene rules, the N01–N24 job queue, data
  contracts, and the morning-report template) with the
  `xtask night-plan-check` validator and `make night-plan-check` target
  that keep it verifiable.
- **Proof:** 12 new tests (state-machine transitions, keymap-adaptive
  copy, serde round-trip, pinned-objective chain, rect non-collision at
  640×420/800×600/1280×800/1600×900) and two new visual scenes
  `hud_onboarding` (1280×800) and `hud_small_onboarding` (640×420,
  zero-overlap assertions), both reviewed by image recognition at 0.97+
  confidence. `cargo test --workspace` is 422/0, vistest is 96/96, smoke
  is green, and fresh runtimes were rebuilt.

## 2026-09-03 — real sound-effect bank via ElevenLabs (loop 349)

- **The game now sounds real.** All 33 sound events play generated MP3
  samples instead of procedural beeps: block break/place across the five
  material families, per-material footsteps, UI clicks, eating, hurt, XP,
  tree creak and crash — plus twelve events that were silent until now:
  entering water, bow release, an arrow sticking into terrain, the melee
  whoosh, flesh hits and creature deaths (melee and arrow paths), the
  mount dragon's roar, item pickups, crafting success, chest lids, the
  forge anvil, and the player's death sting.
- **The bank is generated, committed, and self-contained.**
  `tools/gen_sounds.py` (wrapped by `make sounds`) is the generator with
  the full 33-prompt manifest; it talks to the ElevenLabs Sound Effects
  API with a key from the environment only, and caches aggressively —
  files that exist are never regenerated. The MP3s live in
  `assets/sounds/` and are embedded into the binary via `include_bytes`,
  so a missing file is a compile error and the catalog cannot drift from
  the code. Total free-tier spend: 620 of 10,000 monthly characters.
- **Decode is defensive.** Samples decode through rodio/symphonia at
  boot, downmix to mono, trim head/tail padding with a peak-relative
  threshold (so percussive events don't feel laggy), reject near-silent
  files outright, and normalize to a common playing level. The original
  synthesizer stays as a deterministic fallback for any event whose
  sample is missing — and as the reference the tests hold to.
- **Quality was measured, not assumed.** Every generation's true
  peak/RMS was inspected; eight first-pass files (footsteps, ui click,
  glass place, arrow hit) came back near-silent. The pattern —
  "footstep/stomp" prompts master quietly, impact textures don't — was
  root-caused and exactly those prompts rewritten (the winning wooden
  step is a knuckle knock on a board) and regenerated.

## 2026-09-03 — colored light, fireplaces, and material torches (loop 348)

- **Block light is genuinely RGB.** The voxel BFS propagates red, green, and
  blue independently, loses one level per channel/cell, max-blends overlapping
  sources, crosses chunk borders, and smooths every channel at mesh corners.
  The engine packs RGB into previously unused bits of the existing `u32`
  vertex-light field, preserving the old `0xF0` full-sky encoding and adding
  no vertex bandwidth. Neutral skylight wins in daylight; colored sources tint
  dark spaces. Warm sources breathe with a subtle position-phased flicker.
- **Materials now determine the light.** Ordinary torches and lanterns are
  warm gold, lava and Ember Glowstone amber, Lumen blocks cyan, and radiation
  green. New craftable/placeable **Ember Torch**, **Lumen Torch**, and
  **Fireplace** blocks complete the registry → texture atlas → item/drop →
  crafting → server-validation pipeline. Their procedural pixel textures read
  as an ember crystal, cyan crystal, and stone/log hearth rather than recolored
  copies.
- **Indoor lighting is reliable.** The visual proof exposed that the old
  combined skylight/source scan stopped at the first roof, so no enclosed
  emitter was ever discovered. Splitting source discovery from the sky pour
  fixed it. A second regression proved the skylight flood had been seeded on
  the opaque roof itself; it now starts at the lowest transparent cell above
  it, eliminating roof leaks while keeping overhang and shaft spill. Direct
  chunk-section scanning skips irrelevant sections and avoids per-voxel hash
  lookups.
- **Proof:** CPU tests cover packing compatibility, palettes, attenuation,
  blending, cross-chunk travel, sealed rooms, and the corrected sky frontier;
  a direct GPU test proves three independent packed channels reach the shader.
  The new `colored_light_room` scene pixel-checks warm, cyan, and green pools.
  `cargo test --workspace` is 406/0, all 94/94 visual scenes pass, smoke is
  green, and the warm readback+PNG benchmark is p50 53.7ms / p95 58.0ms.

## 2026-09-02 — packed normal/AO materials and hero textures (loop 346)

- **Terrain has authored character without abandoning voxel art.** Stone now
  reads as mineral plates with fissures and quartz, dirt as clumps with pores
  and pebbles, sand as wind-rippled grains, planks as joined boards with grain
  and knots, and coal/iron as connected veins instead of scattered noise.
  Grass gained clustered blades on top and roots/soil clumps on its sides.
- **Normal maps and ambient occlusion are one reliable material contract.**
  Each linear RGBA material layer stores a tangent-space normal in RGB and
  micro ambient occlusion in alpha. Procedural and mod textures get a
  deterministic Sobel/horizon-derived fallback; authored layers can enter
  through `new_with_material_maps`. Transparent edges do not grow fake bevels,
  AO has a readability floor, CTM derivatives are generated tile-by-tile, and
  normal mip levels are decoded, averaged, and renormalized instead of treating
  vectors as colors.
- **The renderer uses it cheaply.** The raster shader combines micro-AO with
  existing block-corner AO and lights the packed normal from the visible sun.
  Normal and AO share the previous normal-map lookup, so there is no additional
  atlas or texture fetch. Runtime layer replacement regenerates the complete
  material mip chain.
- **Proof:** four focused asset tests, a GPU test that independently proves
  authored normal and AO channels reach the fragment shader, and a new
  `material_gallery` raking-light scene with grass/sand/wood/iron/cavity pixel
  claims. `cargo test --workspace` is 387/0 and all 93/93 visual scenes pass.
  Warm A/B perf is p50 102.9ms for this build versus 104.0ms for the exact
  loop-345 source snapshot in the same readback+PNG harness (no median
  regression; p95 remains host-noisy).

## 2026-09-02 — kingdoms, walking NPCs, and the kingdom compass (loop 345)

- **NPCs actually walk now.** The old movement loop only committed a step
  when the next cell was air *and* the cell below it was solid — a one-block
  bump, a one-block dip, or any obstacle froze an NPC forever (the "some NPCs
  are not walking" bug), and hamlet villagers were additionally anchored at
  the world origin (8, 64, 8) by the default schedule instead of their
  hamlet. The new `lf_npc::locomotion` module owns real traversal: one-block
  step-up with head clearance, descent of up to 3 blocks, cliff refusal,
  accelerating gravity that cannot tunnel through ledges, and a stuck-reflex
  that sidesteps around walls/trees after 20 blocked ticks (per-NPC side
  bias so walls get rounded instead of ping-ponged). `update_villagers`
  drives it; idle NPCs shuffle around home, guards patrol a four-post
  circuit at dusk, and panicking NPCs flee directly away from the player.
- **Kingdoms exist.** One deterministic kingdom per 12x12-chunk region
  (hash-ordered candidate chunks on flat eligible grassland, named from a
  16-realm pool — `Kingdom of Elderfall`, `Thornmere`, `Goldhelm`, ...),
  generated as a full citadel: crenellated curtain walls with four torch
  towers, a gated south wall flying royal banners, a two-storey keep whose
  THRONE is the settle marker, two houses with hearths, a stone-ringed
  well, market stalls with a stock chest, and an irrigated farm plot.
  Three new blocks flow the whole registry→atlas→items pipeline (THRONE,
  BANNER_KINGDOM, KINGDOM_BRICK) and are mineable/lootable. First sight of
  a throne settles a six-NPC court — the new **Monarch** job (Queen Ilsa,
  royal trades: statuary and blueprints for ingots and books), two wall
  guards, a farmer, trader, and smith, all homed at the citadel — records
  the kingdom in the persisted save, writes a chronicle entry, and crowns
  it on the world map.
- **The Kingdom Compass.** Craft one from any wood block over an iron
  ingot (any trunk species or planks). Held, it draws a gold-rimmed compass
  dial under the crosshair whose red needle swings toward the nearest
  kingdom with the realm's name and distance — deterministic from the seed,
  so it works from the moment you spawn.
- **Proof**: `npc_walkers` ticks the real locomotion across a step lane, a
  dug lane, and a flat lane (arrival asserts) and claims each walker's
  pixels; `kingdom_citadel` plants the citadel and claims royal gold,
  purple banners/throne, and the ashlar wall band; `kingdom_compass_hud`
  renders through the real client paint function and claims case, rim,
  needle, and needle direction. 92/92 vistest scenes green.

## 2026-09-02 — clear sky + sun-tracked voxel lighting (loop 344)

- **The sun is visible again**: new authored 16x16 pixel-art sun, crescent
  moon, and star atlas layers render on celestial quads that bypass terrain
  distance fog and color grading. Terrain and clouds can still occlude them,
  but the performance fog can no longer erase the unreachable sky.
- **Light follows what players see**: raster face/normal-map relief now reads
  the same `sun_direction(time)` vector used to place the sun, so the brighter
  side of blocks moves from east to west during the day without adding a
  shadow-map pass or distant geometry cost. The Live RT path keeps its real
  cast shadows.
- **Night timing fixed**: stars are emitted while the sun is below the horizon,
  correcting the previous inverted condition that produced stars at noon.
- **Proof**: `sun_visibility` renders the authored sun at 420 blocks while
  terrain fog ends at 48; a GPU regression proves east and west sun positions
  materially change more than 500 terrain pixels. The complete harness is
  89/89 and the workspace is 371/371 tests green.

## 2026-09-01 — HUD pass finished: kit everywhere + building HUD (loop 343)

- **Zero alpha chrome left**: the 13 machine windows, trade, companion
  menu, tech tree, lore book, and smithing forge all converted to the
  design-kit panel shell through a shared `kit_shell` helper (vignette +
  dark wash + framed panel + title + scroll) — the whole UI now speaks
  the workbench/settings/inventory design language.
- **Building HUD**: a strip floats above the hotbar while you hold a
  block (or while symmetry is live) with placement-shape chips — pick
  BLOCK / SLAB / STAIRS by clicking or pressing R, and any held solid
  block places as a bottom slab or a yaw-facing staircase (slab onto
  matching slab still merges into a full cube). The symmetry chip shows
  the live mirror plane; click it or press V to toggle. The world-space
  symmetry wall and ghost overlays render as before.
- **Proof**: `build_hud` vistest scene with chip-rect pixel claims
  (accent-selected chip, olive symmetry chip, dark unselected chips) and
  an lf_game test locking the shape math (slab bottom, all four stair
  facings, air/water refusal, cycle order, slab merge).

# CHANGELOG

## 2026-09-01 — missing texture patterns: bark + soil (loop 342)

- **Every log species now has bark**: a flatness audit (luminance stddev
  across the whole generated atlas) found eight logs rendered as pure
  noise next to palm/redwood/ember's structured bark. Oak gained grain
  streaks, spruce scaly chips, dark wood deep furrows, cherry horizontal
  lenticels, acacia exfoliating plates, mangrove fibrous strands, maple
  pale strips, baobab wide smooth bands — all in their existing palettes.
- **Soil clumps**: dirt gets chunky clumps and rare pale pebbles, red
  sand wind ripples. The remaining "flat" textures are the authentic ones
  (water, snow, sand, stained glass, waypoint beams).
- **Regression-locked**: `bark_and_soil_keep_their_patterns` enforces
  variance floors per texture, so a future generator change that
  flattens bark back to noise fails the test suite.

# CHANGELOG

## 2026-09-01 — HUD declutter, inventory-first E screen, kit restyles (loop 341)

- **Minimal HUD by default** (researched Minecraft convention: the
  survival screen shows nothing until F3): the top-left info line is now
  clock + facing; biome, coordinates, weather, net, FPS, and RT moved
  behind the F3 debug toggle.
- **E opens an inventory screen**, not a crafting list: armor column
  (head/chest/legs/feet + off hand) beside a painted kit-block player
  portrait, the 3×9 storage grid, the hotbar with its selection frame,
  shift-click quick-move and right-click split — plus a "craft by hand"
  route into the basic workbench (full recipes stay at crafting tables).
- **Furnace and chest wear the design kit**: the last two alpha-chrome
  container screens converted to the vignette + framed-panel shell with
  proper titles, matching the workbench/settings/journal family.
- **Proof**: `inventory_screen` vistest scene (mirrored preview; pixel
  claims on the slot-well grid, the kit-accent portrait/selection, and
  the title band) and the mirrored HUD info line synced with the
  declutter.

# CHANGELOG

## 2026-09-01 — GMod-style physics item drops (loop 340)

- **Mined blocks become props**: breaking a block (or harvesting a crop,
  looting a chest, felling a tree) spawns a rigid item prop with gravity,
  axis-separated collision, and restitution — it bounces off the floor
  AND walls, tumbles while it moves, slides a couple of blocks GMod-style,
  settles onto its nearest flat face, and sleeps. The old 2-block magnet
  vacuum is gone: items now litter the floor until you walk into them.
- **Carry at range**: hold right-click while aiming at a prop (up to 6
  blocks, walls block the grab) to pin it to your view ray with a soft
  spring — swing the camera and it follows; release to throw it with its
  momentum. Walk right up to a carried or resting prop to pocket it.
- **Stacks grow**: same-item props resting within touch distance merge up
  to five per stack, and the rendered cube grows with the count until a
  full 5-stack is exactly one block wide (non-block items scale their
  sprite impostors by the same rule).
- **Physics lives in lf_game::props** (`PropBody`, `step_prop`,
  `prop_half`, `merged_counts`) with unit tests: fall/bounce/rest on the
  block top, fast throws rebound off walls while slow pushes stop touching
  them, held props freeze and sleeping props skip the step.
- **Proof**: the `item_physics` vistest scene runs the real physics (three
  stacks stepped 600 ticks to sleep at sizes 1/3/5, a prop caught
  mid-fall after 14 ticks, a thrown prop slid into a wall) with pixel
  claims — the three ground silhouettes must strictly grow, one cube must
  be airborne above the ground line, and the wall must stand tall. 365
  tests green; 86/86 vistest scenes; smoke green.

## 2026-09-01 — mob animations: legs, hurt flashes, death topples (loop 339)

- **Animals walk like animals**: chicken/wolf/dog/bear — and the formerly
  single-cube boar and woolbeast — are articulated multi-part bodies whose
  legs swing in diagonal trot pairs around hip pivots, driven by a
  distance-based walk cycle (`gait_phase` advances with speed so legs never
  moonwalk; `gait_amp` eases in/out so strides start and stop) and the whole
  assembly yaws to the mob's facing (rate-limited turns instead of snaps).
  Wolves wag their tails at rest, woolbeasts lower their heads to graze,
  chickens peck while idle.
- **Hits read as hits**: every mob skin (21 layers incl. biome tints) got a
  red-multiplied hurt copy appended to the atlas; while `hurt_flash` decays
  the renderer flickers the mob onto the hurt layer plus a flinch squash.
- **Deaths are visible**: mobs topple over (~0.5s ease-out fall around the
  feet), rest ~1s as physics corpses (gravity + friction, no AI), then are
  removed — loot still pops out at the kill. Nameless raiders now walk as
  six-part humanoids and topple like the villagers; cube mobs tumble their
  single cube. Fixed two proof-found pre-existing bugs: firebolt kills
  never removed the mob (immortal corpse), and mobs that fell below y=-10
  ticked forever.
- **NPCs face where they go**: villagers turn smoothly toward their
  walking direction (`yaw`/`walk_phase` on `Villager`, replacing the
  id-hash fake facing), and remote players' gaits are estimated from
  position deltas so they visibly walk.
- **Engine primitives**: `cuboid_part_faces` (the extracted
  yaw+pitch-around-pivot cuboid the humanoid builder uses, now shared by
  animals; bit-for-bit refactor proven by test) and `topple_faces`
  (Rodrigues rotation of assembled part faces around a world pivot — the
  vistest scene caught it initially rotating around the world origin and
  teleporting corpses away; fixed with a non-zero-pivot test).
- **Proof**: `mob_anim` (four wolves at stride phases 0/90/180/270° on a
  sand stage — the pixel claim requires their silhouettes to differ in
  width, a frozen-leg detector) and `mob_hurt_death` (red-tint count,
  toppled-corpse-low / fallen-raider-low / standing-raider-tall windows).
  360 tests green; 85/85 vistest scenes; smoke green.

## 2026-09-02 — authored-depth raster assets, articulated NPCs, item impostors (loop 338)

- **Normal-mapped raster materials**: every generated base, mod, entity,
  item, and connected-texture layer now gets a linear RGB tangent-space
  normal map. The default raster shader reconstructs a tangent frame and
  applies cheap directional relief, so grooves, pixels, and bevels respond
  to viewable light without requiring the optional path tracer.
- **Characters are people, not blocks**: seven villager-job outfits and a
  neutral remote-player skin feed a shared six-part humanoid builder. NPCs,
  companions, and network players now have heads, torsos, independent arms
  and legs, yaw, walking gait, and crouch posture.
- **World items use their real art**: every registered item sprite is in the
  scene atlas; non-block drops render as crossed, double-sided alpha-cutout
  cards while block drops keep their compact cube silhouette.
- **Proof-found fixes**: CTM sentinel indices formerly began at 165 and
  collided with real tree/biome/skin atlas layers; they now occupy 4096+.
  The client entity helper also ignored requested cube positions, collapsing
  entity parts at the origin; it now translates every face correctly.
- **Plan + proof**: added `docs/ASSET-RENDERING-PLAN.md`, a five-stage path
  through per-part skins, attachments, hero item meshes, authored material
  channels, cheap contact/projected shadows, and LOD/performance budgets.
  `entity_skins` is a close lineup of eight articulated characters and eight
  readable item silhouettes with pixel assertions.
- **Verification**: workspace build green; 353 tests passed; 83/83 GPU proof
  scenes passed after correcting the CTM assertion to sample its projected
  edge; manual PNG inspection clean; smoke green; `terrain_vista` benchmark
  p50 50.2ms / p95 50.6ms (29 warm frames, ~20 FPS).

## 2026-08-30 — smart HUD, personalized font, Minecraft controls (loop 337)
- **Smart HUD (never overlaps)**: `kit::hud_layout(w, h)` is the single
  pure source of HUD geometry — info line capped away from the minimap,
  companion tiles ending above the chat band, chat above the hotbar
  band — with a disjointness test proving zero overlap at 640x360,
  800x600, 1280x720 and 1920x1080. The live HUD regions (chat, companion
  tiles, info-line width, minimap anchor) are re-anchored to the computed
  layout, so window size can no longer produce overlapping widgets.
- **Personalized font**: `kit::install_font` promotes the embedded Hack
  monospace over the entire UI (proportional + monospace families,
  1.06 scale, baseline nudge) — a chunky, technical LOREFORGE voice —
  installed once per session to keep the glyph atlas stable.
- **Minecraft controls**: SHIFT sprints, CTRL crouches (FlyDown moved to
  CTRL too), and **crouching edge-locks**: while sneaking on the ground,
  per-axis movement that would leave the supporting block is cancelled —
  you can hold the edge all day and never fall. Sneaking lowers the eye
  by 0.28, slows to 45%, and sprinting runs at 5.6 vs 4.3 walk.
- **Tests**: crouch edge-lock (sneaker holds a floating ledge 600 ticks;
  a non-sneaking walker falls off — proving the platform test real),
  sneak/sprint speed ratios, keymap defaults, HUD disjointness at four
  window sizes. 349 tests, hud_small vistest scene, smoke green.

## 2026-08-29 — king-quest: 50 mods, 15 biomes, animals, the Accord Bastion, vassal workers (loop 334)
- **50 community mods** (`mods/`): ores & metals, food & farming, magic,
  building & decoration packs — 88 blocks (with worldgen ore veins and
  light emitters), 79 items including tools with damage/durability, 10
  smelting recipes. A load-all contract test pins parsing, unique fnv1a
  block ids, and the ore/light/smelting minimums.
- **15 new biomes**: Oasis, Redwood Forest, Mangrove, Aspen Grove, Baobab
  Fields, Willow Wetlands, Painted Dunes, Frost Meadow, Emberwood,
  Lavender Fields, Maple Forest, Pine Barrens, Salt Flats, Foggy Fjord,
  Sunflower Plains — 18 new blocks (logs/leaves/plants/salt) with their
  own atlas art, **9 new tree species**, per-biome ground cover, and
  climate-grid classification (variant-channel splits, reachability-
  tested). Structure gates extended so the new biomes spawn huts,
  embassies, farms and roads.
- **Animals**: chickens (temperate days), wolves (cold nights), bears
  (deep forests), dogs (settlements) — multi-part cube layouts with a
  walk wobble, four new skins, own spawn rules and combat behaviour.
- **The Accord Bastion**: a walled mini-city in the accord meadowlands —
  perimeter walls with merlons, a south gate, four timber houses, a
  two-storey stone keep flying the Accord banner (NPCs settle it), accord
  pillars and torch-lit roads — plus frontier wooden watchtowers in the
  new forests and sun-baked ruins in the new deserts, each biome-gated
  (a biome may or may not carry its structure, by seeded hash).
- **The Vassal system**: at Honored (+75) standing, sneak-use a villager
  to swear them in — Smiths/Lorekeepers mine, Guards/Traders/Bards
  lumber, Farmers farm. Each in-game day vassals stack the yield for
  their liege; sneak-use again to collect. State rides the villager JSON
  save, all rules pure and deterministic.
- **Steam (honest pass)**: Workshop items installed in `workshop/` now
  load in the client and dedicated server through the same mod pipeline
  (`lf_steam::workshop` has real consumers); `lf_steam --features steam`
  compiles. Steam P2P/achievements stay deferred — no Steam client, SDK
  runtime or real AppID exists on this host (dev AppID is Valve's 480).
- **Suite**: 344 tests (was 338), full vistest suite, smoke green.

## 2026-08-29 — the in-game black screen: root cause found and fixed (loop 333)
- **The bug**: after starting a single-player game the view showed a giant
  static black rectangle over the world (HUD still drew on top). Reproduced
  live with a new `loreforge --autostart` debug harness (boots straight
  into a fresh world through the exact menu code path) and macOS screen
  captures, then bisected the frame with draw toggles and instrumented
  batch geometry: the drop_batch carried an entity cube near the world
  origin rendered with `MeshBatch::new`'s **identity view_proj** — six
  per-frame batches (sky/cloud/weather/drop/crack/particle) never got
  `update_camera` in the live render loop, so any geometry within ±1 unit
  of the origin (right where the player spawns) landed inside the clip
  volume and filled the screen with black. The same bug made the sun,
  moon, stars, clouds, item drops, mob cubes, crack decals and particles
  invisible in live play (they only ever rendered in headless vistest,
  which updates all cameras — why proofs never caught it).
- **The fix**: `update_camera` for all six batches every frame. Verified
  with real screen captures at t=15s and t=23s of a fresh world: fully
  rendered terrain, river, sky and HUD, no black rectangle — and the
  sky bodies/clouds/drops now actually visible in live play.

## 2026-08-28 — ai-npc-assets: mob AI state machine, living NPCs, connected textures, generator (loop 332, Sections A-G)
- **Mob AI (B)**: mobs got a real behaviour machine — Idle / Wander /
  Chase / Attack / Flee / Investigate / Disengage with all 11 spec
  transitions (lost-sight investigation, 30s disengage, flee-while-seen).
  DDA line-of-sight (cached per tick, 32-block cap) gates aggro and flee;
  faction standing widens or calms the aggro radius (+100 standing =
  ignore unless attacked); group aggro recruits nearby same-type mobs
  with a 0.5s reaction delay (first-order only, ≤5 pack); A*
  pathfinding (`lf_game::mob_pathfind`, cardinal + 1-up steps, 256-node
  cap, 2s cache) drives Chase/Investigate with direct-steer fallback.
  Client ticks mobs with the player's faction standing and propagates
  group pings. En route: fixed a pre-existing engine bug — axis-aligned
  rays from block-boundary origins produced NaN in the DDA and stopped
  after one cell (rays were blind along exactly the lines mobs shoot).
- **NPC behaviour (C)**: the canonical enriched day (sleep / eat / work /
  socialize / return-home) now drives movement, an `NpcActivityState`
  that bends the render pose (sleeping lies low, working bobs), and
  dialogue posture (sleeping NPCs only murmur; no trade). Reactions:
  structure damage call-outs, combat panic (flee 10s), gifts (use an
  item on a villager: +2 standing, thanks, memory), companion-quit
  comments, and a once-per-crossing "+75 honoured" acknowledgement. NPCs
  remember the last two interactions per world (5-day window) and open
  with memory lines; trades and completed quests write the memory.
- **Black-square hardening (A)**: Live-RT pathtracer + displayed image
  are invalidated on world transitions (a stale frame of the previous
  world covered the new one), and empty column meshes never register a
  draw batch. `no_black_square` scene + pure-black run-length assertion
  on eight daytime gameplay scenes.
- **Testing (D)**: `loreforge --smoke` runs 300 headless logic ticks
  (superflat worldgen seed 42, passive+hostile mob AI, NPC schedule, a
  planks craft, a block mine) with exit-code + log-pattern checks;
  `make smoke` = logic smoke + 12s GUI liveness. New scenes:
  `mob_ai_visible` (real 120-tick mob sim, moved-distance claim),
  `npc_schedule_time` (midday = Work slot), `no_black_square`,
  `connected_textures_grass_3x3`.
- **Connected textures (E)**: top faces of grass, sand, water, snow,
  bog peat, permafrost, accord stone and ashen marble pick one of 47
  strip tiles by an 8-neighbour bitmask (standard CTM corner rule,
  derived table), so fields read as one surface with edge definition.
  The strips live in a second 192×512 texture bound alongside the atlas;
  the mesher bakes per-tile UVs behind marker layer ids and the shader
  reroutes them. Proofs: `connected_texture_uv_3x3` (centre of a 3×3 =
  interior tile, isolated block = bordered tile) + a vistest scene with
  a dark-ring pixel claim.
- **Asset generator (F)**: `xtask gen-texture` (grass/stone CTM strips,
  faction entity skins, block noise — seeded xorshift64 + integer hash
  noise, bit-identical per seed), `gen-ctm <block>`, `gen-all-textures`
  (skips existing). `asset_generator_grass_output` pins seed-42
  determinism and the no-pure-black/white pixel-art rule.
- **Suite**: 338 tests (was 322), 82/82 vistest scenes, headless+GUI
  smoke green.

## 2026-08-28 — plants render as crosses + seed-field contract + opaque surface (loop 331)
- **Ground plants are Minecraft-style crosses**: flower / tall grass / dry
  grass / dead shrub render as two diagonal cutout quads (each emitted
  twice so backface culling keeps both sides), lit by their own cell with
  foliage wind sway — no more solid green cubes. Proof `plants_cross`
  pixel-claims plant colors AND sky visible above the band (a cube would
  block it); mesher test pins 16 verts / 24 indices inside the cell.
- **Seed field contract**: the Create-a-Game seed field is now one tested
  helper (`slots::parse_seed_field`): trimmed number = literal, empty =
  fresh random, any other text = deterministic hash (same words → same
  world forever). World-level proof: `seed_comparison` renders half a
  scene from seed A and half from seed B through the same generator and
  fails if the halves look alike; `same_seed_same_world_...` pins
  reproducibility.
- **Black-box fix (compositor)**: the surface now requests
  `CompositeAlphaMode::Opaque` — with a premultiplied/inherited mode the
  desktop behind the window blended through pixels whose alpha was not 1
  (water, ice, unlit regions), which read as a dark box while playing.

## 2026-08-28 — timber: Valheim tree felling + deep falling blocks (loop 330, master-plan Phase A)
- **Tree felling**: chopping a trunk now fells the whole tree. The new pure
  `lf_game::timber` system identifies the standing tree from the break
  (`find_tree`: same-species trunk column, canopy scan, 24-block cap, lone
  placed logs never fall), removes trunk + canopy (every cell broadcast),
  and animates a rigid fall — angular acceleration around the stump hinge,
  rendered as rotated cubes via the shared `tree_parts` layout fn (the
  dragon_parts idiom; client + proofs run the same code). On impact the
  `fall_plan` lands the trunk as **horizontal log blocks** along the fall
  direction (cells blocked by terrain convert to drops), the canopy
  shatters into debris, a crash sound plays and the camera shakes with
  tree size. Two new procedural sounds: TreeCreak (wobbling low creak) on
  the tip and TreeCrash (noise burst over a 70 Hz thud) on impact.
- **Horizontal logs**: 10 new vanilla blocks (ids 111-120: X/Z variants of
  the five trunk species). The mesher's `Face` enum gained directional
  variants (West/East/North/South) so a lying log's cut ends show the
  ring texture along its axis — all other faces stay bark. Drops map back
  to the species' log item.
- **Bug fix found on the way**: birch/spruce/dark/cherry logs had no items —
  breaking them fell through to the stone fallback. Each species now drops
  its own log item and saws into planks.
- **Deep falling-block animation**: granular fallers tumble while falling
  (deterministic per-block axis + angular velocity, fibonacci-hashed from
  the cell), fast first impacts bounce once (restitution 0.18) with an
  impact dust puff, then settle. Physics stays the cheap scalar drop;
  rotation is render-only CPU quads in the existing per-frame batch.
  Perf bench after the change: terrain_vista p50 116.8 ms / p95 158.6 ms
  (baseline 111/156 — within run variance; the rotated path only runs
  while fallers/trees are airborne).
- **Proofs**: tree_fall_mid (oak caught mid-rotation at a seeded angle;
  the tree_fall_animates_between_frames GPU test renders two angles and
  demands pixel differences with a deterministic same-angle control),
  tree_fall_landed (the real fall_plan applied headless: horizontal log
  row with visible ring ends), falling_blocks_deep (three independently
  tumbling cubes). All with pixel claims (bark/ring/leaf counts).

## 2026-08-28 — assets-and-menus: complete asset set, centered UIs, creative mode (loop 329)
- Menus centered, everywhere: the New World and Multiplayer panels were
  anchored top-left (a fresh top_down egui cursor starts at 0,0) — now both
  sit on `ui_kit::centered_panel_rect` (clamped, both axes); pause, settings,
  Load World, spellbook, imbue, carve and paths vertically center via
  `ui_kit::center_vertically` (the pause screen's fixed 18% guess is gone).
  `ui_kit::apply_kit_style` pushes the LOREFORGE palette into egui's global
  style so every plain widget and `egui::Window` screen (trade, furnace,
  chest, tech tree, machine panels, smithing) wears the kit instead of
  egui's cool-blue defaults.
- Quest log redesigned: the raw default-styled window pinned top-left is now
  a centered Journal panel with tabs (Quests n / Chronicle), faction chips,
  per-objective progress bars, standing rewards and completion tinting.
- Multiplayer screen developed: styled host-world rows (selection accent,
  seed readouts, empty-state line), clearer sections, honest friends copy.
- Window-size robustness is a proven contract: `centered_panel_rect` is
  unit-tested (symmetry + clamping on small screens) and three new vistest
  scenes render a real-helper menu panel at 640x420, 800x600 and 1280x800
  with a largest-connected-component pixel claim that the panel bounding box
  is symmetric in both axes at every size (menus_centered_small /
  menus_centered / menus_centered_wide).
- Asset completion: the armor set is real — bronze/steel helmet, leggings
  and boots (items, pixel-art icons, Bronze-era recipes, research gates);
  the inventory's four trailing armor slots are honored (worn_armor_points
  sums 36..=39; bronze kit 10 pts, steel 17) and drawn as a labeled
  head/chest/legs/feet row with a live armor readout in the workbench strip.
- Audio completion: new procedural Sfx set (ui click, eat crunch, hurt
  thud, xp chime, per-material footsteps) in lf_audio with bounded/decaying/
  distinct synth tests; wired live — screen transitions click, eating
  crunches, damage thuds, level-ups chime, ground travel steps by material.
- Asset testing: every registered item must generate real art and every
  icon pair must differ (registry-derived tests, not a hand-list); the new
  asset_catalog vistest scene renders ALL ~190 registered item icons through
  the real ItemIcons with a per-cell non-uniform pixel claim.
- Creative mode plays like creative: no damage, no hunger drain, infinite
  items (consume_selected no-ops), instant mining, F toggles flight
  (survival lost the old ungated debug fly) — five pure gates on GameMode
  with tests; the New World screen note now describes the real behavior.
- Journal/workbench/new-world/multiplayer proof replicas now draw through
  the client kit's real layout helper + style, and the workbench replica
  mirrors the client's vignette+wash treatment (judge-flagged contrast).

## 2026-08-27 — ui-world-craft: title identity, world creation, terrain rivers, workbench (loop 328)
- Title screen rebuilt on the LOREFORGE palette (parchment/ember/iron-brown
  across every screen via the ui_kit Theme): logotype top-left with
  "Build. Rule. Endure.", underline-on-hover link column, radial vignette,
  version + preview-seed display bottom-right. The title world is now
  seeded from the game version (Fibonacci-hash mix — every release shows a
  different recognizable world) and orbits on a 90s elliptical path with a
  57.3s altitude oscillation; the preview generates in memory and never
  writes to worlds/.
- World creation flow: New World screen (name, visible seed with reroll,
  world type / game mode / difficulty segment toggles, Back + Create
  World), Load World picker with seed-rendered cached thumbnails,
  world-type glyphs, difficulty + last-played metadata and a real delete
  confirmation; Multiplayer screen with working Direct Connect, Host World
  (dedicated server spawn) and an honest Steam-lobby stub. Worlds save
  difficulty (Peaceful blocks hostile spawns; Easy/Normal/Hard scale mob
  damage and hunger pace), game mode (Creative saved as a flag) and
  creation metadata; old slot metas upgrade in place.
- Terrain rework (gen v4): two-layer continental terrain — flat lowlands
  hugging the sea, ridged highlands, ocean shelf — measured at a 0.409
  mean flat fraction across 5 seeds vs 0.275 mountains. Rivers meander
  through the whole lowland (OpenSimplex2 zero-crossings, hard highland
  cutoff, 3-7 blocks wide widening toward the coast, carved to a bed that
  actually fills with water). Caves: surface-breach ramp, deep-slate biome
  below y=30, lava lakes below y=10, stalactites/stalagmites. Structures
  (huts to faction camps) adapt to terrain: no floating floors, filled
  platforms on slopes, underwater sites refused.
- Biome identity: every biome scatters its own ground cover at its own
  density (tall grass/flower meadows, flower-dominant forests, cactus and
  dead shrubs in the deserts, dry savanna grass, mushroom/volcanic
  signatures) with five new decoration blocks + lava; transition bands
  interleave both biomes' covers via the climate dither.
- Crafting workbench: the 3x3 grid is gone. A three-zone workbench
  (category sidebar with craftable counts, recipe list sorted
  craftable-first with locked rows hidden, detail panel with flavor text,
  real have/need counts, batch quantity and Craft / Add to Queue) opens
  from the inventory (basics) or a crafting table (everything). Recipes
  are EARNED: a base survival set is visible from minute one, era-tagged
  recipes surface when the era lands, and picking up any ingredient for
  the first time unlocks its recipes with an amber toast. The set
  persists in the save.

## 2026-08-27 — lore-and-visuals: factions, companions, skins (loop 327)
- Six-faction standing system (–100..+100, Nameless start –50) loaded from
  lore/factions.toml; threshold titles, rivals-drift on honored crossings,
  standing events (quest ±, trade, structure destroy/discover) all
  data-driven via the new lf_lore crate. Chronicle entries reference the
  canonical world events (Era/Year dates from lore/world_events.toml).
- 12 faction quests (2 per faction) from lore/quests_factions.toml with
  narrative text from the faction docs; new quest mechanics: tagged Reach
  (road markers / ember formations / new biomes), Break, Place, Interact,
  and any-food Collect. Quest log shows them; faction NPCs grant them on
  first contact.
- Hireable companions: hire at standing ≥75 with a fee (trade UI), up to
  3 active; trust (0-100) and morale (0-100) with the documented event
  table, daily wages paid at sunrise (unpaid → morale loss → quit at 0,
  "word gets around" −5 faction), dismiss returns them to their schedule
  with trust remembered. Command menu on interact (follow/stay/rest/mine/
  chop/haul/guard/pay/dismiss); 2-4 block follow AI that defends the
  player against the mob that hit them; contextual dialogue lines from
  lore/dialogue.toml; trust badge on the skin at ≥50.
- 38 new blocks (12 faction-themed, 8 biome-exclusive, 18 decoration
  including 8 stained glasses and 6 banners) with procedural 16x16
  textures, drops, and 24 recipes; MOD_BLOCK_BASE 100→200 (DECISIONS).
- Volcanic biome (31st) + biome surface identity updates (gilded savanna
  grass, bog peat, permafrost tundra, mesa terracotta badlands), deep
  slate depth band, coral heads, natural ember-glowstone formations,
  accord road markers.
- Six faction structures (embassy, forge camp, grove shrine, longhouse,
  library, nameless camp) placed deterministically in their home biomes;
  banner markers settle the faction's NPCs (incl. named The Unmarked,
  Maren Voss, Dag Holtz) and drive discovery chronicle + map icons.
- Entity visual identity: 6 faction villager skins + 2 named NPC skins,
  6 companion skins with trust-badge variants, 6 mob skins with distinct
  silhouettes, 9 biome-tint variants of the common hostiles, Nameless
  Raider mob (food+loot drops).
- Map: faction territory tint (30% blend over terrain shading) on the
  minimap and world map + faction-color structure icons. HUD: faction
  standing widget (bottom-right, pulse on change) + companion status
  tiles (top-left). Trade gates: hostile standing refuses trade,
  friendly gets a 10% discount. Ambient ember particles for
  ember_glowstone. 13 new vistest scenes, all pixel-verified.


## 2026-08-26 — P27: fix "objects disappear when looking up" (frustum culling, loop 307)
- The chunk-column frustum test approximated each 16x16xH column with a
  sphere of radius `max(half_h, 11.4)`. That covers the footprint only
  along its axes — the true corner distance is sqrt(128 + half_h^2)
  (~13.6 flat ground, ~17.7 for a 20-tall column), so when the bottom
  frustum plane swept up with the view, columns still poking into the
  frame were wrongly culled: terrain and objects vanished as pitch rose
  (and tall columns vanished even near level pitch). The raycast and FOV
  were innocent: pitch is clamped to 89 deg and the look_at basis stays
  non-degenerate.
- Fix: exact AABB bounding sphere with a 0.1 sway margin (wind-animated
  leaves never culled at the margin), and the Gribb-Hartmann frustum
  planes are normalized before the distance test so the world-unit radius
  means what it says (the raw near-plane normal is ~2x unit length).
- Regression test `looking_up_does_not_cull_visible_columns`: pitches
  5-85 deg x four eye heights x a column grid x five column heights — any
  AABB corner projecting inside the frustum requires the column kept —
  plus the pinned pre-fix failure (pitch 5 deg, tall column at the frame
  edge). Verified the test fails against the old formula.

## 2026-08-26 — P26: visual identity — per-face materials, cutout leaves + wind, smooth AO, mining cracks/particles, mipmaps (loop 306)
- Per-face materials: meshing's texture callback is now
  `(BlockState, Face::{Top, Bottom, Side})`. Grass finally renders a green
  top, banded side and dirt bottom (it previously painted a fake green band
  on all six faces); every log species gets growth-ring end textures. New
  atlas layers (grass_top, log_top, crack_0..3) bring the array to 48.
- Alpha cutout: the fragment shader discards alpha < 0.5 and the six leaf
  textures are deterministically hole-punched per species. Foliage is now
  see-through with reliable depth writes — water (0.67) and ice (0.78) sit
  above the threshold and are unaffected; the glass pane becomes a
  frame-only cutout, Minecraft-style.
- Wind: vertices carry a sway weight (leaf family = 1.0) and Env.time drives
  a vertex-shader wave whose phase derives from world position, so animation
  is continuous across chunk borders and stable while moving. Frozen when
  the particles setting is off (low quality tier).
- Smooth lighting: per-vertex ambient occlusion from the classic
  side/side/corner rule and per-corner light averaging over the four cells
  touching each corner (both were flat per-face before). get_block now
  handles diagonal cross-section lookups safely (approximates as air) —
  corner sampling used to overflow section indexing.
- Mining feedback: a stage 0..3 crack decal (slightly inflated cutout cube
  on the targeted block) plus debris particles — small camera-facing
  billboards sampling the block's texture, with gravity, a simple ground
  stop and a 128-particle cap. The subtle HUD progress bar stays for
  accessibility.
- Mipmaps: a 5-level CPU box-filtered chain per atlas layer with
  mag-nearest / min-linear+mipmap-linear sampling; distance shimmer is gone
  without losing the pixel-art look up close.
- New proof scenes: `foliage_canopy` (cutout + AO + log rings close-up) and
  `mining_feedback` (crack decal + debris on a stone column). The mining
  scene taught a lesson: frame scene cameras against the terrain AT the eye
  — a buried camera sees straight through backfaces.

## 2026-08-26 — P25: correctness & honesty sweep + the pathtracer was flat-color broken (loop 305)
- Server SetBlock validates against the real registry now
  (`lf_voxel::registry::is_known_block`): vanilla ids <= 41 plus registered
  mod blocks (>= 100). The old `block <= 18` cap silently dropped every mod
  block edit in multiplayer. The dedicated server loads `mods/` at boot so
  the ids exist server-side; new UDP integration test covers accept/reject.
- `lf_steam/steam` compiles for real: steamworks 0.12 as an optional dep
  (`steam = ["dep:steamworks"]`), verified with
  `cargo check -p lf_steam --features steam`. STEAM.md corrected — CI
  builds default-feature binaries only. The feature-off default is untouched.
- Generator versioning: `lf_worldgen::GENERATOR_VERSION` + `genver.dat` per
  world (client slots and the dedicated server). A mismatch warns loudly:
  unedited chunks regenerate from the seed on revisit, edited chunks are
  always safe on disk. Pre-P25 worlds upgrade silently on first load.
- Lantern (block 13) got its own procedural atlas layer (41 -> 42 layers);
  it previously fell through to the stone texture.
- Root `[workspace.dependencies]` now lists only the deps actually used
  (21, correct versions incl. winit 0.30 / egui 0.31 / fastnoise-lite 1.1);
  the old table declared 14 deps nothing referenced.
- `lf_modapi::apply_mod` auto-registers `*_ore` blocks as worldgen veins
  (y 8..50, id-derived noise offset clear of vanilla) — mods/README now
  tells the truth. Stale BACKLOG entries corrected (P6 mobs/combat and the
  P3 sky line were marked undone but shipped and tested); tests/golden stub
  removed.
- **vistest PNGs are pixel-analyzed after rendering** (`verify_render`:
  >= 16 distinct colors, real luma variance, sane size) — enforced in code,
  not narrative. First run caught two real, long-standing pathtracer bugs:
  every raytraced scene since P18 (and Live RT / R-key captures in-game)
  rendered ONE flat color. (1) The WGSL DDA initialized `t_max` with a
  signed numerator divided by abs(dir) — negative for negative ray
  components — so that axis always won and rays marched off into the void
  (or straight down into the emissive floor). (2) The CPU camera basis was
  scaled by `camera.fovy.to_radians().tan()` — but `fovy` is already
  radians, so the basis came out at ~1.4% and every ray was parallel.
  Both fixed; raytraced_shadows shows real terrain with fog gradient
  (1697 colors), raytraced_night shows emissive glow over dark ground
  (294 colors).
- Server UDP tests use a deadline-based `drain_until` instead of fixed
  sleeps (chunk generation on first SetBlock made 200 ms pumps flaky under
  parallel test load).

## 2026-08-26 — P24: THE input fix — every key/mouse handler was unreachable (loop 304)
- Root cause (found with a synthetic-input harness driving the real binary
  with macOS keystrokes): a stray `_ => {}` wildcard arm sat in the middle
  of the `match event` in `window_event` — inserted back in P4 — so every
  handler after it (Focused, KeyboardInput, MouseInput, MouseWheel) was
  UNREACHABLE dead code. rustc only warns about unreachable patterns, and
  the warning was buried: keyboard and mouse input never worked through
  the event handler in ANY release; menus worked only because egui gets
  events through a different path. Fix: wildcard removed; lf_client now
  `#![deny(unreachable_patterns)]` so this class of bug cannot compile
  silently again.
- Verified empirically on the fixed build via synthetic input: E toggles
  inventory, holding W walks (position trace 1.2,-0.8 -> 5.1,-4.9), M
  toggles map, ` opens the console, a typed `fly` command executes, Esc
  returns to play, clicks reach the mining path.
- Kept (behind LOREFORGE_DEBUG_INPUT / F3): per-event input trace
  (event/ui_open/egui-consumed), 1Hz tick summary with frame_ms.

## 2026-08-26 — P23: urgent fixes — input, console, seeds, biomes, slots, scaling (loop 303)
- Input defenses: `close_ui` clears a stale chat input (an invisible Chat
  screen forced every frame would eat all keys and clicks); Escape closes
  the Pause menu; if the OS refuses the cursor grab the game still enters
  input mode (mouse-look via raw motion, clicks keep working); the click
  that re-captures the cursor also passes through instead of being eaten.
  New F3 / LOREFORGE_DEBUG_INPUT overlay shows ui_open/cursor_locked/
  playing/keys/health for live diagnosis.
- Developer console (`` ` `` or `/`): 20 commands — help, time set
  (sunrise/day/noon/sunset/night/ticks), give, tp, seed, weather, fly,
  heal, feed, kill, spawn, clear, waypoint add/list/remove, say, fps, rt,
  save, slots, load <slot>, new <type> [name]. TAB cycles autocomplete,
  arrows walk history, Esc closes; command parsing is a pure, unit-tested
  function.
- Real random seeds: each world owns a seed (`seed.dat`), generated from
  OS entropy for new worlds; WorldGen exposes it, noise channels hash it
  with splitmix64 (u64->i32 truncation no longer collides); switching
  worlds restarts the streamer with the new seed (latent bug: the worker
  kept its old WorldGen forever); the dedicated server persists its seed
  and sends the true value in Welcome.
- Natural biome transitions: fractal (3-octave) climate noise at lower
  frequency with a contrast stretch, domain warping (±34 blocks) so biome
  borders follow organic curves, and fine dithering (±0.045) that turns
  straight threshold lines into dithered transition bands of mixed surface
  blocks. `biome_from` stays pure; biome-coverage test samples wider.
- Multiple save slots: each world lives in `worlds/<slot>/` with
  `meta.dat` (name, type, seed, updated). Title menu reordered (Play —
  <slot>, New World submenu, Load Game, Multiplayer, Settings, Quit);
  slot picker with Load / Delete-with-confirm / Create (name + type);
  pause menu gains Save Now, Load Game, Quit to Title. `load_world()`
  reloads a slot mid-session; the pre-slot `worlds/default` auto-migrates
  to "World 1" keeping its chunks and seed (verified live). Slot meta,
  seed persistence and migration are tempfile-tested.
- DPI/proportional UI: egui zoom = user scale × native display density ×
  viewport factor (720p reference, clamped) — text, slots, panels and the
  minimap scale with both pixel density and window size; macOS bundle
  declares NSHighResolutionCapable.
- Harness: plain egui `Area`s never render in the two-pass headless
  harness (only windows materialize) — previews converted to frameless
  windows; new `console_preview` proof scene. Tests 140 -> 149.

## 2026-08-26 — P22: UI overhaul — icons, tooltips, recipe book, map suite (loop 302)
- Real pixel-art item icons in every slot (hotbar, inventory, crafting grid,
  chests, furnaces, machines, trades, cursor, tooltips): `lf_assets`
  sprite generator for all non-block items (tools per tier, ingots, raw
  ores, food, armor, industrial parts) + deterministic gem icons for mod
  items; block items reuse atlas art.
- Texture atlas 18 -> 41 layers: wood-variant logs/leaves, red sand,
  terracotta, moss, ice, copper/tin/bauxite/sulfur ores and all six
  machine/bench blocks now render in-world with their own art (they
  previously fell back to stone).
- Tooltip system: icon + display name, tool tier/damage/speed, food value,
  armor points, fuel seconds, "smelts into", crusher input, era-requirement
  badge, stack size; recipe-book entries preview their pattern as a
  mini-grid on hover.
- Crafting screen redesign + recipe book panel: unified catalog merging
  crafting (vanilla + mods), smelting, assembler alloys and crusher
  recipes; search box, station tabs, craftable-only filter; ingredient
  icons with have/need coloring; click auto-fills the grid from the
  inventory (returning grid contents first); "needs table" and era-lock
  badges.
- Shift-click quick-move everywhere: inventory storage <-> hotbar, chest
  <-> inventory, furnace/machine slots <-> inventory.
- Map suite: top-right minimap (terrain/biome colors, entity dots,
  waypoint pips, player arrow, north marker, toggle in settings) and the
  full M-key world map — pan/zoom, fog of war, explored-but-unloaded
  approximation dimmed, spawn marker, waypoint manager (add/rename/
  recolor/delete, persisted in ClientSave), cursor coords + biome,
  chunk grid at high zoom. HUD info line gains compass facing + biome.
- HUD polish: icon hotbar with pulsing selection glow and fading item
  name on switch, armor points, XP bar mirroring the hotbar width with
  level chip + gain flash, dynamic crosshair (expands while mining,
  hit-marker on attacks), hurt vignette + low-health pulse, redesigned
  death screen with run stats. Settings gains an Interface tab (minimap
  toggle, UI scale driving egui zoom).
- Fixes: `crafting::recipes()` leaked a Vec per call (now a OnceLock
  singleton), `Inventory::add_item` ignored per-item max_stack, crusher
  catalog listed `iron_ore` (a block id, not an item), and the vistest
  harness never rendered egui windows (fresh contexts need a warmup pass
  for window areas, whose font-atlas texture delta must be threaded to
  the renderer — the pre-P22 trade/tech proof shots had silently empty
  windows).
- Proofs: 3 new scenes (crafting_ui, map_screen, minimap_hud), all 19
  scenes re-rendered and pixel-verified (panels present, icons/text
  visible). Tests 123 -> 140.

## 2026-08-25 — P21: menus, animations, HUD & real settings (loop 301)
- ui_kit: theme, easing (+tests), Reveal stagger, animated menu buttons
  (hover glow, press spring, accent bar), slide panels, toggles, sliders,
  section headers, painted vector heart/hunger glyphs.
- Title screen: pulsing logo, staggered buttons, live orbiting world
  background behind the menu; pause menu as an animated slide-in panel.
- Settings screen with Video/Audio/Gameplay tabs and quality presets,
  persisted with the world; every knob drives the engine live (view
  distance feeds the streamer, FOV the camera, invert-Y the mouse,
  clouds/particles gate the atmosphere batches).
- Ray tracing settings made real: Off / Captures(R) / **Live** — a
  persistent Pathtracer reuses GPU resources and traces every frame at a
  configurable internal scale, shown fullscreen beneath the HUD.
- HUD rebuilt: painted hearts/hunger, XP bar with level chip, hotbar with
  hover tooltips, info line (clock, weather, net, FPS, RT flag).
- Proofs: menu_preview (dark-panel 20% + logo light 2.3% in the center),
  settings_preview (window clearly visible). Tests 119 -> 123.

## 2026-08-25 — P20: final consolidation (loop 300)
- All 14 vistest proof scenes render and pixel-verify: biome_montage,
  clouds_weather, first_person_view, hud_preview, industrial_machines,
  night_watch, raytraced_night (100% emissive), raytraced_shadows,
  spawn_plains_dawn, tech_tree, terrain_features, terrain_vista,
  torchlit_night, village_trading.
- STATE/STATUS/BACKLOG/RELEASE reflect exactly what exists.
- 121 tests green; game smoke-tested end to end.

## 2026-08-25 — P19: Steam readiness (loop 299)
- lf_steam crate: feature-gated Steamworks binding (off by default — the
  SDK links dynamically and CI lacks the client); preferred_transport()
  reports Steam only after a successful init, else UDP fallback (+2 tests).
- steam_appid.txt (Spacewar 480) for dev testing; title screen shows the
  active transport; docs/STEAM.md covers the dev loop, feature flag, depot
  layout and steamcmd upload.
- Tests 119 -> 121.

## 2026-08-25 — P18: compute voxel path tracer (loop 298)
- lf_engine pathtrace: WGSL compute tracer — DDA primary rays through a
  128x64x128 block clip texture, jittered soft sun shadows, one-bounce GI
  (sky + emissive torches/lanterns), fog, 2x2 supersampling (portable
  write-only storage; read-write accumulation unsupported on this adapter).
- Rust: build_voxel_texture_data from any World; pathtrace_to_image with
  f16 decode -> PNG; headless scene integration.
- Client: R key path-traces the current view in-game and saves
  shots/rt_frame_N.png.
- Proofs: raytraced_shadows (varied terrain lighting, luminance
  transitions), raytraced_night (100% warm emissive coverage with a
  lantern floor in view). Fixed along the way: uniform member order
  mismatch, stale cargo fingerprints masking edits, torch placement living
  in a dead code copy.

## 2026-08-25 — P15+P16: industrial machines & research (loop 297)
- worldgen: copper/tin/bauxite/sulfur veins by depth (+generation test).
- lf_game machines: Generator (EU buffer), ElectricFurnace (2x speed),
  Crusher (ore doubling), Assembler (bronze/steel/circuits/frames) with
  power draw (+5 state-machine tests).
- Client: machine blocks/entities with slots+progress UIs, 4-block power
  field, machines tick while closed, spill on break, persist.
- Research: eras with material costs, ResearchState advance consuming
  inventory (+3 tests); era-gated crafting shows locked recipes with
  requirements; research bench advances eras on RMB.
- Tech tree screen (K): era columns with done/current/locked states, live
  have/need cost colors, and a next-step hint line.
- Bug fixed: headless egui proofs encoded the UI pass after the texture
  readback — UI screenshots silently lacked their UI. Reordered; tech_tree
  proof now shows the panel (verified by pixel analysis).
- Proofs: industrial_machines, tech_tree. Tests 110 -> 119.

## 2026-08-25 — P14: combat & survival completion (loop 296)
- lf_game combat.rs: Arrow projectiles (gravity + solid-hit), XP curve
  (7+3*level) with carry-over levels, armor mitigation (flat, min 1)
  (+3 tests).
- Items: bow/arrow, bronze & steel chestplates (Armor kind), smithing
  table block (+texture); industrial material items (copper/tin/aluminum/
  sulfur/bronze/steel ingots, wire, gear, machine frame, basic circuit)
  with recipes — catalog consistency kept green throughout.
- Client: hold-RMB bow charging with HUD bar, arrows fly and damage mobs,
  XP bar in HUD, worn armor reduces damage, RMB on the smithing table opens
  the forge minigame UI (bellows pump + orange-zone strikes produce steel).
- Tests 107 -> 110.

## 2026-08-25 — P13: NPCs & villages (loop 295)
- lf_npc: trade_offers(job) tables for all six jobs (+coherence test).
- worldgen: hamlets gain dirt paths and lamp torches.
- Client: villagers spawn when hamlet chunks load (deterministic job/name,
  persisted), day-wander/night-rest schedule, RMB opens the trade screen
  with live have/need counts and affordability colors; lore book item opens
  a reading window showing the world chronicle; job-tinted villager cubes.
- Proofs: village_trading scene with a real egui trade panel (verified by
  pixel analysis). Tests 106 -> 107.

## 2026-08-25 — P12: world & atmosphere completion (loop 294)
- Biomes: data-driven table, 8 -> 30 (variant channel splits climate bands;
  census test verifies all 30 occur in a sampled world; per-biome surfaces,
  tree species, freezing oceans cap with ice).
- Blocks: birch/spruce/dark/cherry logs + leaves, pale leaves, red sand,
  terracotta (banded), moss, packed ice (+textures); Badlands strata.
- Trees: per-species shapes (conifer cones, tall jungle, wide cherry/dark
  canopies); >=3 species generation test.
- Atmosphere (lf_engine::atmosphere): drifting cloud layer (transparent
  pass), sun/moon billboards with celestial rotation, night stars,
  underwater fog/tint, weather cycle with rain/snow particles and storm
  sky darkening. Cloud winding bug caught by pixel-proof (visible from
  below only) — fixed.
- World types: Normal/Superflat/Amplified (superflat = flat, no caves/
  ores/structures); title-screen new-world buttons; type persists.
- Proofs: biome_montage (19 green hue buckets = species variety),
  clouds_weather (8.6% white cloud pixels). Tests 104 -> 106.

## 2026-08-25 — P11: performance & release; base game complete (loop 293)
- World light cache with invalidation on edits (+test) — writing the test
  exposed two real bugs: the mesher never actually sampled the light
  closure (per-face light was hardcoded 15 since P3) and section-local y
  indexed the world-height light array. Both fixed; renders now show true
  dynamic range (shadowed overhangs, torch pools, dark nights).
- xtask package: portable dist/ zip with release binaries, mods and docs.
- CI: release matrix (ubuntu/macOS/windows) builds and uploads artifacts.
- RELEASE.md rewritten honestly (how to run, controls, verified features,
  known gaps).
- Tests 103 -> 104. P0-P11 of the base-game plan are complete.

## 2026-08-25 — P10: mod API real (loop 292)
- lf_voxel registry: runtime mod blocks (MOD_BLOCK_BASE + fnv ids) consulted
  by name/is_solid/is_opaque; registration test.
- lf_game: register_mod_item / register_mod_recipe / register_mod_smelt;
  block_drop owned Strings + mod drops.
- lf_worldgen: register_ore_hook + generate consults hooks (+test).
- lf_modapi: smelting.toml parsed in load_mod (was silently dropped);
  apply_mod/load_mods_dir wire mods into the live registries; full-pipeline
  test: parse -> register -> place modded block -> break -> drop -> smelt.
- lf_assets: generic "mod" texture layer; client loads mods/ at boot
  (smoke log: loaded 2 mods).
- mods/README.md documents the mod surface.
- Tests 96 -> 103.

## 2026-08-25 — P9: multiplayer (loop 291)
- lf_protocol v3: ClientMessage/ServerMessage gameplay set with framed
  codec (+4 round-trip and rejection tests).
- lf_server (real now): UDP authoritative-lite server — canonical world
  (worldgen on demand + validated edits + history replay to newcomers),
  20/s player snapshots, chat relay, join/leave roster; a two-client
  integration test runs over real localhost UDP and passes.
- loreforge-server binary: dedicated hosting (bind addr + seed args).
- lf_client net.rs: connect from the title screen, state send, remote
  block edits applied with remesh, remote players as cubes, chat overlay
  with T input; local edits replicate to the server.
- Tests 92 -> 96.

## 2026-08-25 — P8: quests & chronicle live (loop 290)
- lf_story: QuestEvent (Collected/Crafted/Killed/ReachedDepth) advancing
  objectives with progress counters; starter_quests() 5-quest chain (+1 test).
- Client: events wired to pickups, crafting, mob kills; quest log UI on J
  showing objectives/progress/chronicle; chronicle milestones (first logs,
  first blood, Null Knight slain, deaths, quest completions) exported to
  worlds/<name>/chronicle.md on save; state persists in the save.
- Tests 91 -> 92; game smoke-tested.

## 2026-08-25 — P7: structures, menus, UI proofs (loop 289)
- worldgen structures: meadow huts, highlands watchtowers, desert pyramids
  (deterministic per-chunk placement, in-chunk footprints, +1 test).
- Client: title screen and pause menu; Esc opens pause; settings sliders
  (mouse sensitivity, FOV); quit saves the world.
- lf_engine headless: optional egui overlay in render_to_png; vistest
  hud_preview scene draws the real HUD (hearts/hunger/hotbar/crosshair/
  inventory) — honest UI proof shots at last.
- Tests 90 -> 91; game smoke-tested from the title screen.

## 2026-08-25 — P6: mobs & combat (loop 288)
- lf_game mobs rewritten as a live framework: MobEntity with AI/physics
  update (wander, flee-on-hit, chase, melee with cooldown), MobType stats
  table + drops, roll_spawn day/night table (+5 tests).
- Client: mob spawn cycle (every 2s, cap 12, surface-only, despawn 80),
  crosshair mob attack (tool damage table, knockback, hurt flash), mob
  cubes render with the drop batch, mobs/kills persist in the save.
- Items: porkchop/mutton food, glitch_dust/null_shard materials.
- Tests 87 -> 90; game smoke-tested with mobs active.

## 2026-08-25 — P5: content catalog core (loop 287)
- Blocks: furnace, chest, planks, glass (transparent); matching textures.
- lf_game: smelting module (Furnace state machine with fuel/burn/progress,
  +4 tests), iron tier + swords in the item table, tool_damage table,
  recipes for furnace/chest/iron tools/swords.
- Client: block entities (furnace/chest) with persistence, RMB opens their
  screens, furnaces tick while closed, containers spill contents when
  broken, furnace UI (input/fuel/output + flame + progress), chest UI.
- Catalog consistency test: recipe outputs/ingredients, smelt outputs,
  block drops, and block items all resolve.
- Tests 82 -> 87; game smoke-tested with containers active.

## 2026-08-25 — P4: survival & inventory UI (loop 286)
- Migrated winit 0.29 -> 0.30 (ApplicationHandler) and adopted egui 0.31
  (matches wgpu 24; the 0.29 stack was incompatible).
- lf_game: items registry (block items, tools with tiers, food, materials),
  mining rules (hardness, tool multipliers, harvest gating, break times),
  shaped crafting with translation-aware matching (+13 tests total).
- Blocks: crafting table; torch item texture; CRAFTING_TABLE id 14.
- Client: egui HUD + inventory/crafting/death screens with full stack
  interactions (pick/place/swap/merge/split); hold-to-mine with progress;
  tool durability; item drops (gravity, magnet pickup, bobbing cubes);
  hunger/regen/fall-damage/drowning/death+respawn; RMB context (open table,
  eat, place); inventory/stats/time persisted in player_extras.dat.
- Tests 69 -> 82; game smoke-tested with UI active; all scenes render.

## 2026-08-25 — P3: lighting & atmosphere (loop 285)
- lf_voxel light.rs: per-column flood-fill sky + block light (BFS with -1
  falloff, opacity-aware, 15-level); emitter table (torch=14, lantern=15);
  index-stride bug class caught by tests (y must own the largest stride).
- Mesher samples light from the exposed cell per face (packed sky<<4|block
  in the vertex light attribute; was hardcoded 15).
- Shader: brightness = max(sky*day, block*0.92) with ambient AO and distance
  fog blending to the sky color; uniforms carry camera pos + day + fog.
- Water: separate alpha-blended pipeline (no depth write), water faces split
  from opaque mesh, columns sorted back-to-front; water texture alpha 170.
- Torch/lantern blocks (non-solid, non-opaque, targetable).
- Client: 20-minute day/night cycle (lf_game::TimeOfDay) driving sky clear
  color, day factor and fog; torch in the 9-slot hotbar; deeper night
  constants (starlight 0.12, night sky mix 0.15).
- vistest: torchlit_night scene (torch grid on terrain at night); scene sky
  math now reuses lf_game::TimeOfDay.
- Tests 66 → 69; all 6 scenes render; game smoke-tested with lighting.

## 2026-08-25 — P2: world streaming & terrain (loop 284)
- lf_voxel: block registry (is_solid/is_opaque/is_targetable, 12 blocks);
  mesher culls by opacity (air/water/leaves show faces behind them, no
  water-water faces); ChunkColumn serializable; WorldStorage saves chunk
  columns via region files + player.dat (+round-trip tests).
- lf_worldgen: trees on meadows (deterministic hash placement, canopy kept
  in-chunk), 3D-noise caves, coal (<y96) and iron (<y48) ores in stone,
  water fills to sea level; 4 feature tests over real generated chunks.
- lf_assets: log/leaves/coal/iron/water textures (11-layer atlas).
- lf_client: background chunk streamer (worker thread, nearest-first,
  view radius 5, unload radius 8 with save-before-drop), sphere-frustum
  column culling from mesh bounds, world persistence with 30s autosave and
  save on exit, player position/look restored from save, hotbar 8 slots.
- vistest: terrain_features scene; renders verify trees (~7% canopy pixels)
  and water (~20% water pixels) visible.
- Tests 58 → 66. Game smoke-tested 20s with streaming active.

## 2026-08-25 — P1: first-person playable core (loop 283)
- lf_voxel: World + ChunkColumn (16 sections = 16x256x16), world-coord
  get/set with chunk border math, surface_height, mesh_column with cross-
  border neighbor culling; 4 new tests.
- lf_worldgen: generate_chunk fills a ChunkColumn directly; surface_top()
  helper (surface band tops out at height+3, standing surface height+4).
- lf_game: Player with AABB physics — gravity, jumping, sprint/sneak/walk
  speeds, fly mode, axis-separated collision with substepping so long falls
  never tunnel; 8 physics tests (landing, walls, jump arc, ceiling, fly,
  tunneling, look dir).
- lf_engine: scene split into SceneResources (shared pipeline + atlas) and
  MeshBatch (per-column drawable); OutlineScene line pipeline renders the
  targeted block outline.
- lf_client (replaces dummy): the game shell — winit input with cursor
  lock, player update loop, DDA raycast targeting with outline, instant
  break / face-adjacent place (with player-overlap rejection), 6-slot
  hotbar (digits + scroll, shown in window title), F2 screenshots via the
  offscreen renderer, per-column GPU remesh on edit.
- vistest: first_person_view scene with vista-seeking camera (moderate-drop
  selection, constrained to the meshed area); scenes use World pipeline and
  radius-3 meshes.
- Tests 43 → 57. Proofs: shots/vistest_first_person_view.png (eye-height
  view over real terrain), plus re-rendered dawn/vista/night scenes.

## 2026-08-25 — P0: honest baseline (loop 282)
- AUDIT: found that loops 26–281 (~256 "Evolution Mode" loops) changed only
  BACKLOG.md/STATE.md — no code, data, or tests. All their claimed features
  (Nether, Ender Dragon, shaders, ~230 items) were never implemented. Docs
  reset to reflect reality; backlog re-planned as P1–P11.
- Fixed: region storage no longer overwrites neighbor chunks in the same
  region (region files now hold all chunks keyed by (x,z), atomic tmp+rename
  writes). 4 regression tests added.
- Fixed: worldgen biome selection now uses elevation; all 8 biomes are
  reachable (verified by a sampled world-sweep test). Height range stretched
  to 24..176 so deep oceans and peaks exist. SEA_LEVEL = 62 constant added.
- Renderer: added depth buffer (Depth32Float) to the pipeline; extracted
  shared GpuScene (pipeline + buffers + texture array) used by windowed app
  and headless path alike; Camera moved to lf_engine::camera.
- New: lf_engine::headless — offscreen wgpu render to PNG (real GPU output,
  256-byte-aligned readback, sRGB-correct).
- New: lf_vistest — real scene registry; scenes build actual terrain from
  lf_worldgen (seeded, deterministic), mesh it with lf_voxel, render via
  lf_engine. Unit tests: registry uniqueness, non-empty deterministic meshes,
  seed sensitivity, unknown-scene error.
- New: xtask `vistest` and `screenshot` commands render real PNGs;
  `cargo run -p loreforge -- --headless --scene NAME [--seed N] --out PATH`
  now genuinely renders (CI's old command shape made real).
- CI: vistest job now runs the real harness and uploads PNG artifacts.
- Meshing: per-block texture atlas indices emitted by the mesher (was:
  everything hardwired to grass); added meshing unit tests (face counts,
  culling, per-block tex_index).
- lf_assets: 6 procedural textures (stone/grass/dirt/sand/mycelium/snow),
  atlas + block-id→layer mapping, overflow-safe color math, tests.
- Tests: 27 → 43 passing across the workspace.

## Loops 1–25 (original milestones — real work)
- loop 1: initialized STATE, BACKLOG, DECISIONS, CHANGELOG, and shots dir. build green.
- loop 2: added m1 window screenshot proof to shots/m1_window.png. build green.
- loop 3: implemented M2 textured chunk rendering; saved shots/m2_chunk.png. build green.
- loop 4: implemented DDA voxel raycast in lf_voxel with unit tests; saved shots/m3_breakplace.png. build green.
- loop 5: implemented lf_worldgen noise heightmap, biomes, and strata with tests passing; saved shots/m4_terrain.png. build green.
- loop 6: implemented region storage persistence and chunk round-trip test; saved shots/m5_save_load.png. build green.
- loop 7: implemented TimeOfDay day/night cycle, sky color transition, LightEngine with torch block light; saved shots/m6_night.png. build green.
- loop 8: implemented survival core player stats and inventory item stacking; saved shots/m7_survival.png. build green.
- loop 9: implemented medieval smithing system with 8 materials (wood to adamantine), tool parts, assembly, forge minigame; saved shots/m8_forge.png. build green.
- loop 10: implemented mobs (Boar, Woolbeast, Glitchling, Stalker, Crawler, Null Knight boss) and combat system; saved shots/m9_boss.png. build green.
- loop 11: implemented lf_modapi mod loader for TOML manifests and datapacks with ember_ores example mod; saved shots/m10_mod.png. build green.
- loop 12: implemented protocol codec (handshake/login/chat/messages) and dedicated loreforge-server binary; captured shots/m11_two_players.png. build green.
- loop 13: implemented villagers with VillagerJob, VillagerSchedule, utility AI two-tier dialogue system (data-driven + optional LLM fallback); captured shots/m12_village.png. build green.
- loop 14: implemented story mode quests, objective types, and quest log tests; captured shots/m13_quests.png proof. build green.
- loops 15–25: chronicle engine, quests, milestone proofs, amberium mod, crystal/obsidian content, Geode Guardian, Cinder Crawler (all with code + tests).

## Loop 308 — P28 (V1REBRAND gate) begins: sway honesty fix + 4 latent UX bugs
- Committed docs/V1REBRAND/11-EXECUTION-PLAN.md: the full P28-P39 execution
  plan mapping roadmap docs 02-10 onto BACKLOG numbers at +2 offset
  (DECISIONS entry added; P26/P27 were already taken by visual identity and
  the camera-culling fix).
- Wind-sway honesty fix: P26's commit message claimed foliage wind sway, but
  shader.wgsl's vs_main never consumed the sway vertex attribute (loc 6) or
  the time_sway uniform — the attribute, uniform, mesher weights, client
  clock, and cull margin all existed; only the shader math was missing, so
  leaves never moved. vs_main now applies a world-position-phased double
  sine (amplitudes 0.055/0.045/0.02, combined max ~0.08 blocks, inside the
  0.1 sway margin of the P27 cull). New proof test
  foliage_sway_animates_between_frames renders the canopy at two wind
  phases through the real GPU pipeline: frames must differ (>0.1% of
  pixels), same-phase control must be pixel-identical. The test necessarily
  fails on the old shader (it had zero time dependence).
- Clouds setting was a no-op: cloud_batch was rebuilt unconditionally; now
  gated on settings.clouds (cleared when off).
- UNLOAD_RADIUS (fixed 8) -> view_distance + UNLOAD_MARGIN(3); the distance
  cull in column_in_view now takes the view distance (view 8 previously had
  zero unload headroom). P27 regression updated to pass view=5 explicitly.
- First-launch fix: Settings opened from the title screen now returns to
  the title via Back and Esc (close_settings); previously both dropped the
  player straight into the world.
- Boot now loads the booted slot's player_extras (settings/inventory/etc.)
  instead of reading legacy worlds/default before boot_slot() — slotted
  players previously booted with defaults until clicking Play.
- 162 tests green (was 161), 22/22 vistest scenes, release smoke OK.

## Loop 309 — build-pack Stage A: reality audit (Step 1) + 8 fixes
- Executed docs/poorcraft-build-pack Step 1 per 01-REALITY-AUDIT.md: every
  [x] claim in BACKLOG verified across code, a live release-build session
  (title captured twice, in-world session observed, log inspected), and
  fresh vistest renders. AUDIT.md at repo root records CONFIRMED vs
  ACTUALLY-BROKEN/MISSING per claim; BACKLOG corrected in the same commit.
- Verdicts on the three user-flagged areas: destruction feedback CONFIRMED
  (crack decal + block-textured debris traced through the real mining
  path) except break SOUND (ACTUALLY-MISSING — no audio system exists);
  lore machinery CONFIRMED but shallow (chronicle readable in play via J
  and the book; 5/11 event types never fire; no dialogue; no named
  places); biomes ACTUALLY-BROKEN as an experience (17-18 of 30 are
  worldgen twins; one untinted grass texture; global fog; MYCELIUM unused;
  biome_montage scene shows one vista, not a montage).
- Fixed with the audit (each with a test): HUD rendered behind the title
  menu (hud_visible gate); title orbit camera buried in ring terrain ->
  flat-dark backdrop (title_eye_y clamp + audit_title_camera.rs repro
  tool: World_5 had 12/64 orbit points under higher ground); render
  culling/water sort used the player eye instead of the render camera;
  streamer wish radius hardwired to 5 so view-distance settings never
  streamed farther (sync_wish); sneak captured but never read (0.45x
  careful walk); smithing UI called strike() and granted a steel ingot
  every frame (Strike button + ForgeMinigame::reset); lantern block had
  no item/recipe (craftable iron-over-torch); random_seed() could collide
  within one clock tick (sequence counter). 201 fossil shots/ev_*.png
  removed (Evolution-era residue; zero code references — several name
  creatures that don't exist). RELEASE.md counts corrected (168 tests /
  22 scenes).
- 168 tests green, 22/22 vistest; the user's own live play session served
  as the smoke check (their process was left running untouched).

## Loop 310 — block gravity + water physics (user request)
- Granular blocks no longer float: registry::has_gravity (sand, red_sand,
  snow, dirt, grass, moss, mycelium — ores excluded, embedded in stone per
  the Minecraft rule). Breaking support detaches the whole column into
  animated FallingBlock entities (rendered with the block's own texture,
  water-damped sinking, landing re-places through the same remesh +
  network-broadcast path as a player edit, crushing nothing v1; a landing
  into an occupied cell drops the item instead).
- Water physics: event-driven cellular simulation in lf_game::fluids —
  flow level 0 (source) .. 7 rides in BlockState's unused flag nibble;
  water falls first, then spreads horizontally with decay, and unsupported
  flow dries up (scooping a source recedes its puddle — test-proven).
  Edits enqueue the cell + 6 neighbors; a 64-cell tick budget bounds frame
  cost. Worldgen oceans/lakes are sources, so nothing changes until
  disturbed. Mesher renders flowing water as stepped, lowered surfaces
  with step-covering side faces (no slits between levels).
- Bucket + water_bucket items (craftable: 3 iron ingots in a V; pixel-art
  icons) — scoop a source (right-click it with an empty bucket) or pour
  one (right-click a face with a full bucket): the player-facing tool for
  the fluid system and the P30 Steam-Age groundwork.
- Proofs: 6 new unit tests + 2 new vistest scenes — water_flow (aqueduct
  pours down a flume and pools at a dam, settled through the real sim
  before meshing; AI-verified stepped surfaces + pooling) and falling_sand
  (column collapsed into a dug pocket via the real gravity settle, plus a
  mid-air faller cube; AI-verified pile + floating block). 24 scenes total.
- 174 tests green, 24/24 vistest; runtimes rebuilt; pushed.

## Loop 311 — goal Sections 0–4: re-audit + the four feel fixes
- S0: re-verified the four flagged items before touching them (AUDIT.md
  "Goal-file re-audit" section). Verdicts: bottom mining bar CONFIRMED;
  texture stretching NOT reproducible in the raster path; biome grade
  absent CONFIRMED; mod-load visibility absent CONFIRMED.
- S2 destruction feel: removed the mining/bow egui::ProgressBar pair from
  the bottom HUD panel (the reported "mar") entirely; progress renders as
  a crosshair-centered radial ring (faint track + clockwise accent arc;
  bow charge in the ok role). Geometry unit-tested
  (reticle_arc_spans_progress_from_top); hud_preview renders it mid-break.
  Crack decal + debris particles unchanged.
- S3 biome grade: shader.wgsl gains a grade uniform (tint multiply +
  saturation pull toward luma) applied after lighting and fog; lf_client
  biome_grade table (desert/badlands/savanna warm, snow family cool +
  desaturated, swamp/jungle lush, hollow eerie pale, oceans teal,
  temperate neutral); ~0.3s exponential lerp across boundaries; clear
  color mirrors the grade so the sky shifts with the world. GPU proof:
  biome_grade_shifts_midframe_color (same scene, warm vs cold: hue moves
  ~10.7deg, saturation ~0.10).
- S4: mods/smoke_test (one block, one item) with lf_modapi::smoke_line ->
  "[MOD SMOKE TEST] OK" boot line on both client and dedicated server; CI
  test loads the real mods/smoke_test folder and asserts both
  registrations; mods/README.md points to it as the first sanity check.
- S1: proved per-block texture tiling instead of assuming it — mesh test
  multi_block_walls_tile_per_block_not_stretched (two blocks = two quads,
  seam vertex, UVs exactly 0..1) + texture_tiling scene (7-wide plank
  wall + stone floor, AI-verified per-block repetition, no smearing).
  DECISIONS: greedy meshing blocked on the UV-repeat invariant; Live RT
  ships (live + capture) — capture-only no longer the model.
- STATUS.md rewritten to match verified reality (the old one claimed 121
  tests / 14 scenes / live-RT-deferred).
- 178 tests green; 25/25 vistest scenes; runtimes rebuilt; pushed.

## Loop 312 — audio engine + impact shake + FOV/transparency/perf proofs
- lf_audio crate (rodio): procedural break/place sounds per material
  category with silent fallback; wired into real break/place; sliders now
  actually drive playback (the settings label no longer says "when it
  lands"). 4 dispatch/synth tests. CI ubuntu installs alsa headers.
- Step 3 impact pulse: short decaying screen shake on heavy breaks
  (envelope tested), applied to the camera target only.
- Step 7: FOV reference test at 90/60 degrees guards the double-radians
  bug class on the raster path.
- Step 8: transparency_layers scene (water behind glass, particles both
  sides) — AI-verified correct layering.
- Step 9: headless.rs refactored into a persistent HeadlessRenderer (the
  naive perf loop measured 774ms of per-frame SETUP); xtask perf + make
  perf at Medium radius-5: p50 111 / p95 156 / min 77 ms incl. readback +
  PNG encode; DECISIONS names this host's iGPU as the low-end target.
- 184 tests green; 26/26 vistest scenes; runtimes rebuilt; pushed.

## Loop 313 — Steps 13/14/15: settings completeness, thumbnails, minimap
- Key rebinding: lf_client::input Keymap (Action-keyed, name-serialized,
  junk-safe load), Controls tab with capture rows, movement + UI keys
  rebound live, persisted in ClientSave; PathTraced quality tier drives
  RtMode::Live; both round-trip tested.
- Save-slot thumbnails: throttled live-view capture at save time, shown in
  the picker.
- Minimap: rotation with the view (rotated texture mesh + marker/N-chip
  rotation) and 0.5-3x zoom, persisted; world-space waypoint beacons
  (six tint layers, transparent pass, per-frame rebuild) with the
  waypoint_beacons proof scene.
- 187 tests green; 27/27 vistest; SMOKE OK with the live boot log showing
  "[MOD SMOKE TEST] OK — smoke_test mod loaded successfully" (3 mods).

## Loop 314 — Steps 16-19: biomes finally read as different places
- New identity surfaces: jungle grass, savanna grass (gold), mycelium
  hollow, moss swamp, FlowerForest wildflowers; Tundra spruce-sparse vs
  SnowyTaiga dense; boulder fields on the three windswept/snow-slope
  biomes. Generator v2. Contact sheet measures 30/30 distinct strip
  colors; pairwise identity regression test with two documented families
  exempt (it caught two real twins while being written — both fixed).
- Biome-aware day spawns (woolbeast= cold, boar= temperate) with test;
  weather coldness from the biome field; weather_snow + weather_dry
  proof scenes.
- 188 tests green; 30/30 vistest scenes; runtimes rebuilt; pushed.

## Loop 315 — P29 Water Age
- Research is a graph now: the Water branch (Industrial prereq,
  independent of Electrical), unlockable from the tech-tree screen with
  material costs; pre-branch saves load unchanged (serde default).
- Machines: WaterWheel (12 EU/s free while touching water) + Battery
  (4000 EU) and a pure, tested distribute_power (producers → batteries
  cover gaps → surplus recharges) that the client tick and the vistest
  scene both run. Water Wheel + Battery blocks craftable (Water-era
  gated) with UI panels. RT palette covers new ids + stable fallback for
  future ones.
- 194 tests green; 31/31 vistest (water_wheel_power proof); runtimes
  rebuilt; pushed.

## Loop 316 — P30 Steam Age
- Steam branch research (independent of Water, tested); Pipe/Boiler/
  SteamEngine machines (equal-share pipes, fuel+water->steam boiler,
  16 EU/s engine) with 4 tests including the full chain; blocks through
  the content pipeline with UI panels and burning-boiler steam puffs.
- 199 tests green; 32/32 vistest (steam_chain proof, AI-verified); smoke
  OK; runtimes rebuilt; pushed.

## Loop 317 — cross-column lighting + lore books
- Light engine floods a 3x3-column neighborhood: chunk borders no longer
  seam (regression test + night_border_seam proof with a measured 1.92
  max brightness step; perf unchanged at p50 47.7ms).
- Lore books: three tomes in lore/books.toml (the Smith / Null / river
  warden threads), on-kit paginated reader, Lorekeeper trades, icons;
  file-load test + AI-verified lore_book proof scene.
- 201 tests green; 34/34 vistest; smoke OK; runtimes rebuilt; pushed.

## Loop 318 — Oil Age (P31) + power-grid overlay (Step 25)
- Crude oil in worldgen (desert/swamp pools + surface seeps), typed
  pipes, pumpjack/refinery/combustion generator, Oil research branch
  (Steam-or-Electrical either-or), oil buckets, tar byproduct.
- G toggles the power-grid overlay: green/red tint cubes over machines.
- 209 tests green (+8); 36/36 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 319 — Nuclear tier, capped (P32)
- Uranium (deep rare band), fuel rods, the 32 EU/s reactor with a real
  heat curve (equilibrium cooling, auto-SCRAM, residual decay heat,
  meltdown with glowing radiation residue + chronicle event), the
  reactor_safety certification gating Era::Nuclear, reactor UI with the
  big red button.
- 213 tests green (+4); 38/38 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 320 — Magic foundation (P33)
- Mana + HUD, the bounded four spells with 3 rebinding-aware slots and a
  spellbook screen, wizard NPC + rare tower worldgen, scroll learning,
  the enchanting imbue minigame with real rune effects on held tools,
  and the two crossover blocks (fuelless lumen light, mob-warding
  pylon). Extras saves moved to JSON with a legacy bincode migration —
  fixing the latent silent-reset on every old field addition.
- 223 tests green (+10); 41/41 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 321 — Construction (P34)
- The shape system (slabs/stairs in BlockState bits, shaped meshing,
  fractional collision), shaped placement with slab merge, climbable
  bulk-removable scaffolding, build symmetry (V), blueprint
  capture/ghost/paste with material bills, chisel statue carving, and
  the modapi light fix + decor_pack. 233 tests green (+10); 42/42
  vistest scenes; smoke OK; runtimes rebuilt; pushed.

## Loop 322 — Smart building (P35)
- Conduit-relayed power distribution (4-hop field chains), the physics
  elevator, climate comfort regen, and the computer screen with the
  engine's first dynamic-texture path (data-change-gated atlas layer
  rewrites showing research/chronicle/grid readouts).
- 238 tests green (+5); 43/43 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 323 — Dragons (P36)
- The roost boss end-to-end: circle/swoop/perch flight AI (tested),
  multi-part sine-animated rendering from a shared layout fn, fire
  breath, mountain roost worldgen (gated, tested), BossSlain saga, and
  the approved dragon mount after the streaming spike held.
- 243 tests green (+5); 45/45 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 324 — Paths & specialization (P37)
- Four play-accrued paths with chronicle milestones, the generalized
  era|path gate enforced at crafting AND placement (fixing the
  branch-era bench lock bug), respec with double-accrual focus, the
  ornate tier, and protocol v4 player trading with server escrow and a
  real-UDP test.
- 248 tests green (+5); 47/47 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 325 — Finish line, part 1 (Steps 34-39)
- Transport-neutral lobbies + the invite flow (tested), Workshop UGC
  scanning (tested), the full mods/README authoring guide, and
  `xtask new-mod` scaffolding (live-verified + loader-tested).
- 253 tests green (+5); smoke OK; runtimes rebuilt; pushed.

## Loop 326 — P28 leftovers + Step 40
- Connected stone/planks surfaces, HUD text shadows + the on-kit audit,
  in-play chronicle toasts + the cross-system lore-anchor test, and the
  item belt backbone (tested pure push + client pass). Final honesty
  pass recorded in BACKLOG + STATUS.
- 256 tests green (+3); 47/47 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 347 — Hitboxes, walls, wheels & castle siting
- User bug hunt: flowers no longer cull the ground under them or form
  invisible solid hitboxes (`is_plant`-driven solidity/opaque), picking
  and the selection outline follow real block shapes (`pick_boxes` +
  shape-aware raycast), animals collide with walls (axis-separated AABB
  physics with hop step-up; dragons clamp to terrain), LMB near mobs no
  longer throttles creative breaking to 2/s (crosshair LOS filter), the
  wheel switches every notch instantly (multi-step + no set_title +
  same-frame UI), a caption above the hotbar names the looked-at block,
  and kingdom citadels site properly (dense validation, border margin,
  spawn clearance, hillside carving, clean courtyard; gen v6).
- 399 tests green (+12); 93/93 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 411 — Owner UI Asset Handoff
- Expanded the POORCRAFT 3D owner-facing UI art pack with transparent PNG
  concept sheets for HUD bars, control/key glyphs, action icons, resource
  icons, menu/panel frames, and faction/strategy markers. The existing
  owner-menu/docs/assets changes were kept, and the accidental Rust formatting
  churn in untouched files was absent after cleanup.
- Added `assets/UI-ASSET-IMPLEMENTATION-GUIDE.md` and
  `assets/ui_asset_manifest.json` so future work knows how to turn the sheets
  into runtime HUD/menu/key/icon assets without baking key labels, item counts,
  or bar fill values into images. `pc3d_assets` now embeds the manifest and has
  a guardrail test proving every referenced generated sheet exists.
- Evidence: generated PNG alpha audit passed for all eight sheets; `pc3d_assets`
  24/24; `pc3d_render owner` 3/3; `pc3d_render font` 4/4; POORCRAFT 3D release
  build OK; `make p3d-dmg` rebuilt the macOS app/DMG after unsandboxed
  `hdiutil`. Root `cargo test --workspace` required unsandboxed UDP permission
  and then was manually interrupted after the unrelated
  `wizard_towers_generate_in_gated_biomes` exhaustive worldgen scan ran silent
  for several minutes.

## Loop 412 — GLM UI Rework Pack
- Added `docs/POORCRAFT-3D/GLM-UI-REWORK-PACK/`, a drop-in GLM 5.3/Z-code
  handoff folder with prompt files, UI rebuild specs, HUD/hotbar/control
  requirements, screenshot/playtest protocol, MCP-style inspector spec, data
  extraction authorization, implementation queue, failure modes, acceptance
  gates, JSON contracts, UI strings, telemetry/export schemas, and a baseline
  screenshot copied from the current poor UI state.
- `pc3d_assets` now embeds `glm_ui_rework_manifest.json` and tests that every
  listed pack file exists, every JSON contract parses, and the owner's
  inspector/wireframe/data-export authorization remains present.
- Evidence: inspected `baseline/current-windowed-slice-showcase.png` and
  recorded the clipped/debug HUD failure; attempted
  `make p3d-rebuild OUTDIR=poorcraft3d/apps/poorcraft3d/shots/glm_ui_baseline`
  but it stalled before PNG output and was interrupted; JSON validation passed;
  `pc3d_assets` 25/25; POORCRAFT 3D release build OK.

## Loop 414 — Build identity + row-shear law (the stale-volume fix)
- The owner kept seeing the diagonal menu cut although the fix shipped in
  every DMG: `/Volumes/POORCRAFT 3D` was a stale mount from one minute
  before the fix commit, and rebuilt DMGs reused the volume name (fresh
  mounts collided as "POORCRAFT 3D 1"). Both stale volumes ejected.
- Builds are now self-identifying: the title subtitle reads
  `3D · OWNER ALPHA · BUILD <version> <git-hash>` (`ui::build_stamp`,
  baked from `PC3D_BUILD` by `make p3d-dmg`; "dev" locally) and the DMG
  volume is named `POORCRAFT3D-<hash>` — a stale mount can never
  masquerade as a fresh build again. PLAY.md documents how to verify.
- The ui-shots gate covers the bug's hiding place: proof widths were all
  accidentally 64-px aligned. The harness now resizes to 1501x801
  mid-run and captures the HUD + title there (13 captures), and every
  capture must pass a row-shear law (adjacent-row cross-correlation on
  the composited readback: zero median drift, no dominant nonzero shift).
- Evidence: ui-shots 13/13 with "no row shear" on every capture; the
  mounted DMG binary measured on the real screen at 1280/1501 across
  title/gameplay/pause (median +0.0px); journey digest 7ab2295dafa0ec24
  from the mounted volume; p3d 580/580, root green.

## Loop 415 — GLM World Tools + Semantic Assets Pack
- Added `docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/`, a second drop-in
  GLM 5.3/Z-code handoff folder for start-menu seed/world preview work,
  gameplay-semantic asset generation, MCP/inspector expansion, wireframe and
  data extraction, AMD/NVIDIA profiling/upscaling preparation, and broad
  asset brainstorming. The pack makes the owner's rule explicit: no fake
  assets — a house needs a door, an NPC needs talk, a forge needs forging.
- Added machine-readable contracts for the world/tools task queue, semantic
  asset tags, asset generation backlog, prompt matrix, start-menu/worldgen
  requirements, screenshot/wireframe capture, GPU vendor tooling, and concrete
  tools GLM may build (`seed-preview`, `inspect-asset`, `wireframe`,
  `interaction-probe`, `gpu-marker-dump`, etc.).
- `pc3d_assets` now embeds `world_tools_manifest.json` and tests that every
  listed file exists, all JSON contracts parse, and the core asset/gameplay
  laws remain present. Evidence: JSON validation passed and `pc3d_assets`
  27/27. No runtime DMG rebuild was needed for this tooling/spec pack.

## Loop 422 — WT-001 implemented: New World seed preview
- `pc3d_world::seed_preview` (pure): seed resolution (numeric/text/entropy),
  a never-repeating reroll, and the deterministic preview — biome census,
  spawn-safety spiral (water/slope/walkable-area/keep-footprint laws),
  real river + stronghold-ground hints (settlement honestly placeholder),
  and the 96×96 CPU map with spawn marker.
- The New World screen gained the WORLD PREVIEW panel (map image element,
  spawn safety, biome summary, feature hints, resolved seed) and the
  RANDOM button; REROLL/RANDOM never repeat the current seed and fit
  their column; CREATE is disabled — Enter and click both refuse — when
  the preview is unsafe; the layout dump exposes button states.
- `--ui-seed-preview-shots` (6 deterministic states + sidecars + pixel
  report), `--seed-preview <seed|random>` headless export, `make
  p3d-seed-preview`.
- Evidence: 9/9 authority laws, 36/36 ui tests, 6/6 windowed shots at
  p50 7.5 ms, map structure 70%+ neighbor agreement (not noise), ui-shots
  13/13, p3d 596/596, root green.
- Also restores green HEAD after a concurrent commit swept the in-flight
  UI panel without its module.

## Loop 423 — WT-002 implemented: semantic asset factory lab (slices 1/2/4)
- `pc3d_assets::semantic`: the WT-002 contracts (catalog, affordance
  schema, factory queue) parse and the rejection law is code — assets
  missing their category's affordances (door/talk/input/…) are refused;
  the seven starter-batch assets are registered against real artifacts
  (GLBs, the npc/machine sim authorities, the UI painter).
- New GLBs: `prop.chest` (open/lid/inventory anchors) and
  `prop.ore_node` (harvest anchor); every building GLB gained the
  schema-required `interior` socket; `module.banner_sign` gained
  `inspect`.
- `pc3d_render::inspect`: windowless inspection sidecars per the
  contract (bounds/triangles/LODs/materials/anchors/gameplay) — every
  registry anchor is verified against the GLB bytes; a promised anchor
  missing from disk is a named failure. `--asset-sidecar`, `make
  p3d-asset-sidecars` (7/7 PASS).
- Evidence: pc3d_assets 32/32 (+5), inspect 4/4, deterministic assetgen
  regen under budgets, p3d + root suites green.
- Deferred to their queue entries: beauty/wireframe/anchor-overlay
  captures (WT-003), the semantic playtest route (talk/forge slices),
  GPU pass markers (WT-007).

## Loop 424 — WT-003 implemented: the game observatory (slices 1/2/5-partial/7)
- `pc3d_assets::observatory`: the WT-003 contracts (input routes,
  evidence bundle schema, MCP tools) embed and parse; a law test keeps
  the runtime's route table and bundle fields in contract agreement.
- `pc3d_render::observe`: route table (4 available, 4 honestly
  unavailable WITH reasons), the 20-field runtime state export
  (null-with-reason per the schema), the 18-field evidence bundle with
  a content-bound FNV digest, and the comparator — pass / fail /
  inconclusive with named checks (paths exist per-bundle, screenshots
  nonblank, seed mismatches are inconclusive, not failures).
- `--observe <route|all>` (per-route re-exec honoring winit's
  one-loop law) and `--compare-evidence <A> <B>`; `make p3d-observe`.
- Evidence: 4/4 available routes PASS with real captures (p50 ~6.4 ms),
  4 UNAVAILABLE bundles with reasons, comparator PASS between two
  independent real runs, tests green across the suite.
- Deferred: wireframe/anchor-overlay captures (slice 3, with WT-002
  slice 3), gameplay routes flip on when the talk/forge/entry slices
  land, GPU markers (WT-007).

## Loop 425 — WT-002/003 slice 3: wireframe + anchor overlays
- The renderer gained `SceneDebugMode`: Wireframe (deduped mesh edges as
  sun-lit lines over a dimmed world, via a new LineList pipeline with
  x-ray depth) and AnchorOverlay (per-socket axis crosses + bounds wire
  boxes). `load_asset` retains sockets/bounds/edges for the debug passes.
- `--asset-capture <id|all>` + `make p3d-asset-captures`: beauty /
  wireframe / overlay windowed captures per GLB starter asset with
  center-region, diff-gated pixel checks and JSON sidecars — 4/4 assets
  pass (frame diffs 50–77%, lines verified thin-structured).
- Found + fixed by the proofs: the ui_script owner_menu gate (silent
  no-op scripts) and the offscreen readback's late-pass draw drop
  (documented; the windowed path is the visual law's evidence).

## Loop 426 — WT-007 slice 1: GPU pass markers + audit
- The renderer pushes the contract's `pc3d.*` debug-group tree every
  frame (frame/prepare + nine pass groups incl. an honest empty
  machines slot) and counts live top-level draw calls; the readback
  copy carries its own marker.
- `gpu_marker_audit()`: the ten gpu_marker_contract fields — adapter
  backend/name, marker completeness, the timestamp policy (support
  probed, not-enabled reason, CPU p50 as evidence), draw calls,
  triangles.
- The observatory's `route_gpu_markers` flips UNAVAILABLE → PASS (12
  markers, 9 draw calls, Metal), audit bundled; `make p3d-observe` now
  5/5 available routes PASS + 3 honest gameplay-slice unavailabilities.
- Deferred: enabling timestamp queries, per-module draw counting,
  vendor capture cookbooks (docs today, markers were the prerequisite).

## Loop 427 — The NPC talk slice
- `pc3d_world::dialog` (pure): deterministic villager names + lines
  derived from each brain's live activity and role; 3 authority laws.
- The gameplay UI: "E TALK <name>" prompt in range (3 m, nearest live
  cast brain), the SPEAKING panel (speaker/role/activity/line/E-close)
  on E, dialog blocks gameplay, Escape/E close; the runtime-state
  export carries the open dialog.
- Observatory `route_npc_talk` UNAVAILABLE -> PASS end to end (real
  TryTalk path, "speaking with Maren Oldford"); `make p3d-observe` now
  6/6 available routes + 2 honest unavailabilities (house entry, forge).
- Evidence: route capture with all five dialog elements, ui-shots 13/13
  regression, dialog laws green, full suites green.

## Loop 428 — The forge use slice
- `pc3d_world::forge` (pure): fuel + water -> heat -> bars with named
  blocked states, physical slots, steam charging from the boiler
  chain; 4 authority laws.
- The THE FORGE panel (state/fuel/heat/slots, G/H/T/E keys, blocks
  gameplay, works while open — a menu freezes the player, not the
  fire), plaza E-interact priority, toasts, bars-taken counter.
- Observatory `route_forge_use` UNAVAILABLE -> PASS with the full
  loop proven end to end ("1 bars smelted and taken"); `make
  p3d-observe` now 7/7 available routes; only house entry remains
  honestly unavailable.
- Evidence: route captures + assertion, ui-shots 13/13, forge laws
  4/4 + 1/1, full suites green.

## Loop 429 — House entry: the last route flips
- Town buildings are enterable: interior cells open, wall ring solid,
  the plaza-facing door column opens (DoorEntry records the way in);
  the live walk mounts the settlement collision over the streamed
  surface (walls stop, doors pass).
- `UiAction::PlayerTeleport` proof hook (PlayerLook precedent) with
  live-surface grounding; `CollisionSurface` composes over borrows.
- Observatory `route_house_entry` UNAVAILABLE -> PASS (outside vs
  inside frames differ 56.4%) — **8/8 required routes green, zero
  unavailabilities remain**. The walk law: through the door to the
  interior in 400 steps; stopped by the ring beside it.
- Deferred: teleport is proof-only, no interior props, no door
  animation, cardinal door facing.

## Loop 430 — The semantic playtest (WT-002 slice 5 capstone)
- `route_semantic_playtest`: one scripted walk chaining house entry ->
  NPC talk -> forge use (open/load/smelt/take) with five captures and
  chained assertions; `make p3d-playtest` runs it twice and the
  comparator proves same-seed determinism (verdict PASS).
- Evidence: "house entered (frames differ), Maren Oldford talked, 1
  bar(s) forged" reproduced identically across independent runs;
  9/9 observatory routes available and green.
- Deferred: chest-open, resource-harvest, and map-marker interactions
  (the GLBs validate; panels unstarted — the forge/dialog pattern
  covers them).

## Loop 431 — The playtest matrix completes
- The three remaining rows, live and chained: the chest (one loot
  drop: the first WOOD PICK + bread), the ore node (2 iron ore per
  swing, REQUIRES the pick — bare hands yield nothing), the plaza
  map-marker read; a generic interact panel + stock lines.
- The CLOSED ORE LOOP: the forge's ore load consumes harvested stock;
  taken bars land in the inventory. Items: iron_ore, iron_bar.
- The extended semantic playtest covers every matrix row in one walk:
  "house entered, Maren Oldford talked, 1 bar forged, 4 ore
  harvested, chest+marker read, IRON BAR in stock" — deterministic
  across two runs (comparator PASS).

## Loop 432 — WT-008 in code: data extraction + mod validation
- `pc3d_assets::plugins`: the WT-008 contracts embedded + the mod
  validator with the contract's 12 sample packs — 5 good pass, 7 bad
  fail with named reasons (fake house/NPC/forge/UI/perf, NaN bounds,
  missing materials). Safety laws asserted verbatim (local-only,
  read-only saves).
- `--export-data` / `make p3d-export-data`: windowless LIVE exports
  (worldgen 289-region sample, the cast's brains, a charged machine
  chain) stamped with the contract's 8 common fields + the full
  12-row exporter surface manifest. Local only.
- Evidence: pc3d_assets 36/36, export gate green, suites green.

## Loop 433 — WT-009: per-slot variant instancing
- Each flora slot picks its variant from the 300-GLB batch by a pure
  hash of its coordinates + kind; meshes lazy-load on first sight
  (negative-cached); buckets split per (kind, LOD, variant) in both
  the lit and shadow draws; `FloraStats.variant_buckets` is the proof
  stat.
- Evidence: the wilderness vista draws 59 variant buckets of 62; the
  300-GLB consumer law stands; pick determinism/diversity/no-aliasing
  laws green; suites green.
- Deferred: seed-independent picks (deliberate), shared collision
  envelope per kind, no generated far-LODs.

## Loop 434 — The quest journal
- J opens the QUEST JOURNAL: the settlement's quests from the proven
  authority — per row the title with state + progress, the giver by
  name and role, the kind, the reward. Rows computed once per world
  (plan_quests at the spawn region, cached); journal state rides the
  runtime export; E/J close.
- The playtest route gains the journal: "journal 4 quests" in the
  chain line, 9 captures, deterministic x2 + comparator PASS.
- Deferred: accept/claim interactions, giver map markers, live
  progression states.

## Loop 435 — Quest accept/claim + live progress
- The journal is live: Up/Down focus, Enter accepts an OFFERED quest
  or claims a COMPLETE one's reward; greeting the villager you spoke
  with advances Greet quests, proximity advances Visit quests; the
  panel re-syncs and toasts.
- Evidence: navigation law (claim/accept/inert-ACTIVE), the playtest
  accepts the focused quest ("journal 4 quests (1 active)"), x2
  deterministic + comparator PASS; suites green.
- Deferred: Deliver/Build/Excavate event emission, an economy wallet
  for claim payouts.

## Loop 436 — Build/excavate emission + the credit wallet
- F builds advance active Build quests at the exact site; R removes
  and ore harvests emit Excavated; claims pay a credit wallet (shown
  in the journal header, exported in the runtime state).
- The playtest accepts THE EXCAVATE quest and its journal shows 2/8
  live — "journal 4 quests (1 active, progress live)" — x2
  deterministic + comparator PASS.
- Two script-ordering bugs caught by the proofs (exact-frame steps
  consumed strictly in order — third occurrence of the class).
- Deferred: Deliver events (needs a deliver-at-site interaction),
  wallet spending (market/trade slice).

## Loop 437 — Deliver-at-site
- D delivers carried iron bars to an active Delivery quest's site
  within 8 m: pure resolver (need/stock/site, order-stable), bars
  leave the real inventory, the authority's event advances the quest,
  prompt + toasts narrate; honest refusal when nothing applies.
- Evidence: the resolver law; playtest x2 deterministic + comparator
  PASS unchanged (the showcase plan yields no Deliver quest — noted
  honestly); 186 lib tests; full suites green.

## Loop 438 — Damage systems (fall + eat + regen)
- Falls hurt (impact > 7 m/s: 12 health/m/s; lethal falls recover you
  at the plaza, winded); X eats carried bread (real stock, honest
  refusal); health regenerates while well fed. An explicit airborne
  state lets the fall own Y against the walk's snap;
  PlayerTeleportHigh is the proof hook.
- Evidence: the damage law (unit); 187 lib tests; playtest x2
  deterministic + comparator PASS. The live fall ROUTE proof is
  honestly deferred (removed, not faked) — the ore-column ground
  answers at the player's height, freezing the drop; next cycle's
  first task.

## Loop 439 — The 1000-asset expansion + terrain analysis tool
- 1000 NEW variant GLBs (300 -> 1300) across 21 original families —
  deterministic, budget-checked, two LODs each; inventory rebuilt
  (1324 files, truthful); consumer law: 1300/1300 on disk, 650
  stride-loaded with 2 LODs.
- `pc3d_world::terrain_report` (pure): per-biome census with mean
  elevation, slope %, roughness; 3 laws (determinism+census,
  generator agreement, physical values). `--terrain-analyze` /
  `make p3d-terrain-analyze`: JSON sidecar + hillshaded relief PNG +
  slope-heat PNG from the worldgen authority.
- Evidence: assetgen 1300 OK, guardrails green, suites exit 0.
- Deferred: the new families not yet instanced in the wild (streamer
  wiring follows); region-center slope metric (macro relief).
