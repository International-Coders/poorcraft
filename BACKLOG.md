# BACKLOG — LOREFORGE

Honest status after the P0 audit (see CHANGELOG) and the build-pack Step 1
reality audit (see AUDIT.md, 2026-08-26 — every checked item below was
re-verified against code + a live session; broken claims were fixed or
re-opened). Everything checked below exists in code and is covered by tests
and/or real rendered proofs (`shots/vistest_*.png`).
The former "Evolution Mode" list (~230 items claimed in loops 26–281) contained no
implementations; the genuinely built items are checked here, the rest are planned
below by phase. Its fossil `shots/ev_*.png` "proofs" were removed by the audit.

## Done (verified)

- [x] The furnace is the server's fire: server-side smelting over real
      UDP (2026-09-18, loop 469, root workspace): closed STATE's
      next_task item (2) — the last player-facing consumption tier that
      fabricates its own output. Wire v9: SmeltRequest (Deposit /
      Withdraw / SmeltDone, per furnace pos + slot) and a delta-less
      SmeltVerdict to the smelter alone — the client applied nothing
      before it, so deposits and withdrawals WAIT for the verdict and
      the PackSync claim stays exact. THE SERVER: one commitment
      account per (player, furnace) — the pure furnace-op law
      (deposits fund; withdrawals pay out, a fuel item only while its
      seconds remain banked; a smelt-done transforms one input +
      SMELT_TIME burn into one backed bar of the realm's own smelt
      table), the deterministic burn reconciliation, ledger
      pre-checks, the smelting replay window, Goodbye burn-out. THE
      CLIENT: the furnace tick is the one SmeltDone site (refused
      reports revert their furnace's delta — at-most-once), the slot
      gate reads each click as at most one directional move and undoes
      it until granted (one move in flight; the storage rows freeze
      with a hint), and the pack claim now includes the held cursor
      (goods in hand are carried — the trace-proved fix that keeps
      every furnace op claim-invisible and ledger-exact). En-route
      proof-traced bug fixed before the battery: a refused deposit
      whose UI closed would have duplicated the goods close_ui already
      returned. Evidence: 11 new laws (codec round-trip; the
      furnace-account + intent unit laws; the funded-deposit,
      backed-bar, pay-once, two-fires, burn-out, and carried-hand wire
      laws over real UDP; the furnace source law); workspace 545/0
      (xtask 12 included); smoke OK twice; battery 110 scenes exit-0
      twice with PNGs byte-identical (singleplayer pixel-proven
      unchanged); steam feature compiles clean with v9; runtimes
      fresh. Deferred honestly: eating (the last client-side
      consumption tier of the same shape); the lying-PackSync bootstrap
      tier (a modified client's claim can re-seed the ledger — named at
      466, unchanged); client-local block entities (chest contents,
      furnace-slot persistence across peers — the sim/block-entity sync
      tier); the windowed two-client route.

- [x] The placed block is paid for: server-side place-item payment
      over real UDP (2026-09-17, loop 468, root workspace): closed
      STATE's next_task item (2)'s wider half — chosen over smelting
      by next_task's own rule: a free placement fabricates ANY world
      content and, through loop 465's server-paid mining, was a full
      item mint. Wire v8: PlaceClaim (the paying item) on SetBlock +
      creative on Hello + the FULL BlockState rides the wire (the
      proof-discovered wart: an online slab arrived a full cube and
      the placer's own chunk reload reverted it — fixed and lawed).
      THE SERVER: the smuggle guard (mine+place on one edit refuses)
      and the place-payment gate (placement_pays — the shared pure
      law, every door a closed loop under block_drop — plus the
      ledger holds one, consumed); refusals move nothing and answer
      the editor alone (echo + reasoned Reject); creative joins place
      ungated; claim-free edits stay the client-simmed tier as
      shipped. THE CLIENT: place_claim_for (Place + witness +
      survival only) computed in one funnel; all four placement sites
      name their payment (held item x3, the paste bill per cell);
      the mirror consumes one more online (offline keeps the
      mirror-free law); the hearthlight spell light re-kinded
      Machine (a spell effect claims nothing). Evidence: 10 new laws
      (the placement-payment + place-claim unit laws; the
      paid-moves-the-ledger, short-ledger-refusal, mismatched-claim,
      smuggle-guard, creative, sim-tier, and wide-state wire laws
      over real UDP; the every-placement-names-its-payment source
      law); workspace 534/0 (xtask 12 included); smoke OK; battery
      110 scenes exit-0 twice with PNGs byte-identical (singleplayer
      pixel-proven unchanged); steam feature build repaired and
      re-verified; runtimes fresh. Deferred honestly: the forged
      sim-claim residual (the client-simmed tier's trust); smelting
      and eating; a lying creative Hello (the bootstrap tier); the
      windowed two-client route.

- [x] The bench asks the realm: server-side crafting over real UDP
      (2026-09-15, loop 467, root workspace): closed STATE's next_task
      item (2) — loop 466's deferral "the ledger is client-CLAIMED
      until consumption routes (crafting/smelting/eating/placing stay
      client-side)"; chosen over the windowed two-client route per the
      priority ladder. Wire v7: CraftRequest (recipe spec + client-
      chosen req id) and CraftVerdict (THE VERDICT IS THE DELTA —
      exactly what the ledger consumed and produced, u32 counts, to the
      crafter alone). THE SERVER gates twice: the craft-spec gate
      (crafting::spec_matches_book — only a recipe the realm's book
      names executes, so a connected client cannot fabricate output
      from nothing) and the transactional engine itself
      (crafting::execute against the LEDGER — blocked crafts move
      nothing and say why); a replayed req_id (duplicated datagram) is
      answered with a no-op refusal (bounded 512-entry window) — the
      crafting twin of the dig pays-once law. THE CLIENT: while
      connected both craft paths (workbench click, queue tick) send the
      request and leave the pack to the verdict (offline the integrated
      host is the same authority, byte-equal behavior); the verdict arm
      applies the delta exactly (u8-batched adds, overflow spills at
      the feet), completes the queue head only on its own req id and
      output, and releases the one-in-flight wait (a retry is a fresh
      request — a queued job never double-crafts). Evidence: 7 new laws
      (craft round-trip; the spec-gate unit law; granted-moves-the-
      ledger, blocked-moves-nothing, fabrication-gate, and pays-once
      replay wire laws over real UDP; the online-routes-through-the-
      wire source law); workspace 524/0 (xtask 12 included); smoke OK;
      battery 110 scenes exit-0 with PNGs byte-identical across two
      runs (singleplayer pixel-proven unchanged); runtimes fresh.
      Deferred honestly: smelting/eating/placing consumption stay
      client-side (the remaining consumption tiers); a lost verdict
      errs safe (at-most-once, never fabricated); the windowed
      two-client route.

- [x] The pack is the server's ledger: server-side per-player
      inventories over real UDP (2026-09-15, loop 466, root workspace):
      closed STATE's next_task item (2) — the 465 deferral "the server
      holds no canonical inventories yet"; chosen over the windowed
      two-client route per the priority ladder. Wire v6: PackSync (the
      client's aggregated pack claim, cadence-limited, forced by every
      server-side delta — a stale upload only ever removes server-known
      deltas, so the window errs safe). THE SERVER: one canonical
      ledger per player, seeded by the claim (oversized claims
      truncate at the pack's own law), paid by mined-yield grants,
      dropped at Goodbye; the offer gate refuses goods the ledger does
      not hold (to the offerer alone); the escrow completes only when
      BOTH ledgers can pay (holds + room, removes-first trial) and
      moves both exactly or neither. TWO LIVE HOLES CLOSED: the
      ungated accept completed phantom offers, and a self-trade
      duplicated items (offer 1, want 1 → +2); a third party could
      dissolve any offer by accepting it. THE CLIENT: PackMirror
      (bootstrap claim, drift detection, one sync tick outside the
      poll borrow) + the Reject hint arm (the gate is legible).
      Evidence: 8 new laws (escrow unit law; pack-sync+grant, escrow
      moves-both, failed-accept-atomicity, self-trade/third-party wire
      laws over real UDP; mirror bootstrap + aggregation + source
      laws); workspace 517/0 (xtask 12 included); smoke OK; battery
      110 scenes exit-0 with PNGs byte-identical across two runs
      (singleplayer pixel-proven unchanged); runtimes fresh. Deferred
      honestly: the ledger is client-CLAIMED until consumption routes
      (crafting/smelting/eating/placing stay client-side — a lying
      client can mis-claim its pack; the next tier, server-side
      crafting, makes the ledger server-computed); PackSync overflow
      is truncated (the spill is the client's ground items); the
      windowed two-client route.

- [x] The yield is the server's to give: server-side dig-yield
      authority over real UDP (2026-09-15, loop 465, root workspace):
      closed STATE's next_task item (1)'s higher-severity half — the 462
      deferral "server-side inventory authority (the corrective echo
      reverts the BLOCK, not an optimistic item take)"; chosen over the
      windowed two-client route per the priority ladder (an authority
      gap above a missing proof). THE WIRE (protocol v5): SetBlock
      carries `mine: Option<MineClaim>` — Some iff a player MINED dig,
      with the honest held item id (None = bare hand, still claiming);
      places and simulation edits claim nothing; new ItemGrant message —
      the canonical yield, editor alone. THE SERVER gates the claim
      through mining::tool_satisfies and pays items::block_drop against
      what the block WAS — the same lf_game law the client plays, one
      source, mods included. THE CLIENT: the host funnel computes the
      claim and is the one broadcaster; break_block_drops spawns only
      offline — online the yield arrives via ItemGrant with the
      trade-escrow overflow spill; the scaffold column routes through
      break_block_drops; felled-tree cells claim nothing (EditKind
      Mine -> Falling — singleplayer never paid them as drops); the
      apple bonus is offline flavor. EN-ROUTE: the mine-break path's
      redundant second send_block removed (every dig went on the wire
      twice + double-entered the server's edit history). Evidence: 3 new
      real-UDP wire laws (editor-alone grant + place-never-pays;
      server-side harvest gate incl. bare hand; pays-once +
      rejected-never-pays) + the claim round-trip + the mine-claim unit
      law + 2 source laws (one-broadcaster, no-optimistic-take) = 7 new
      laws; workspace 509/0 + xtask 12; smoke OK; battery 110 scenes
      exit-0 with every committed PNG byte-identical (singleplayer
      pixel-proven unchanged); runtimes fresh. Deferred honestly: the
      server holds no canonical inventories yet (crafting/smelting/trade
      sourcing stay client-held); apple bonus offline-only (no server
      RNG); timber blocked-cell drops + container spills client-local
      (sim/block-entity sync tier); the windowed two-client route (needs
      a headless GameState driver).

- [x] The resonance compass + the remembered song + the bearing law
      (2026-09-15, loop 464, root workspace): closed STATE's next_task
      item (2) — the resonance depth pass. THE HELD CRYSTAL: holding
      an anima_crystal raises a violet resonance dial under the
      crosshair; WorldGen::resonance_target answers the twin of the
      hollow you stand in (== geode_twin_center, the Discovery's
      promise, pinned by law) or, elsewhere, the NEAREST rolled hollow
      via a bounded 16-chunk ring scan (None = "the deep is silent" —
      a sensor, not an oracle); label via map::resonance_label, readout
      at the compass cadence. THE BEARING LAW (proof-found bug fixed):
      the kingdom compass needle had been MIRRORED across the player's
      east-west line since loop 345 (dx.atan2(dz) vs the yaw-0-looks-z
      convention) — map::bearing_to is now the one bearing source for
      both dials, pinned by the 8-wind + painter-relation laws that
      fail on the old code. THE SONG IS REMEMBERED: the twin-song
      dedupe persists in ClientSave (serde-default, saved sorted),
      restored after the restart_streamer chokepoint (new worlds and
      join-identity still reset it). Evidence: resonance_compass_hud
      scene (real painter over a real WorldGen reading; the first
      staging's fixed left/right needle gate FAILED on a near-vertical
      needle and was rebuilt as a dominant-axis centroid gate) INSPECTED;
      kingdom_compass_hud re-render INSPECTED pixel-faithful; battery
      110 [ok] / 0 FAIL exit-0; workspace 490 + xtask 12 = 502 green;
      smoke OK; runtimes fresh. Deferred honestly: per-stack crystal
      origin (needs ItemStack metadata migration); no marker AT the
      twin site; a windowed two-client route or server-side inventory
      authority.

- [x] The geode twin law + the feature seed law (2026-09-15, loop
      463, root workspace): closed STATE's next_task item (1) —
      "geode pairing", carried since loop 447. THE TWIN LAW: geodes
      roll as mirror PAIRS through the realm's heart
      (geode_twin_chunk = point reflection through the x=z=-0.5
      corner plane; geode_pair_representative is involutive; a roll
      answers BOTH halves — the twin at the mirrored center 15-lx/
      15-lz, same depth, same radius; rarity is per pair, density
      unchanged; geode_twin_center reads the twin's world position
      as the twin chunk itself answers it). THE RESONANCE: the
      first Anima-crystal take from a hollow records a chronicle
      Discovery naming the twin's compass bearing and distance
      (map::geode_twin_line rides the game's own compass_facing
      convention; deduped once per hollow per session, cleared at
      the restart_streamer chokepoint). THE PROOF-DISCOVERED BUG
      (fixed before committing): the geode_twins vistest scene
      exposed seed_for_features() sampling gradient noise at the
      origin — 0 for EVERY seed — so since P2 every feature hash
      (trees, structures, geodes, citadels) was seed-independent;
      fixed off-lattice + the feature-seed law (distinct keys/geode
      maps/tree maps across seeds, one-seed replay); accord-bastion
      structure law consciously rewritten multi-seed.
      GENERATOR_VERSION 7 -> 8. Evidence: geode_twins scene (real
      near-heart pair, violet gate both halves) INSPECTED; battery
      109 [ok] / 0 FAIL; workspace 485 + xtask 12 = 497 green;
      smoke OK; runtimes fresh. Deferred honestly: the twin dedupe
      is session-scoped; the Discovery is client-local (chronicle
      server authority rides the server-authority tier); no in-world
      twin compass yet; a windowed two-client route or server-side
      inventory authority.

- [x] The replay-window law: multiplayer terrain-edit routing made
      lossless (2026-09-15, loop 462, root workspace): closed STATE's
      next_task item (1) — "route player edits through the
      authoritative server path ... so a second client sees
      digs/placements", carried since loop 450. The audit found the
      skeleton existed with three real defects: (D1) the newcomer
      replay was lossy — World::set_block refuses edits for a missing
      chunk, so every replayed edit outside the already-meshed ring
      silently vanished; (D2) the server echoed the editor's own
      accepted edit back, double-applying (duplicate host event +
      redundant relight/remesh per player action); (D3) a rejected op
      (unknown block id) left the optimistic editor diverging
      silently. THE LAWS: the replay window (lf_client::net::
      RemoteEditBuffer — remote edits for unstreamed chunks buffer per
      chunk, bounded with FIFO oldest-whole eviction, and the streamer
      flushes a chunk's queue in the server's history order the moment
      the chunk arrives, before meshing; a source law pins flush at
      every non-test chunk insert); the no-self-echo + corrective echo
      (lf_server — peers get updates, the editor never hears its own
      accepted edit, and a reject answers the editor alone with the
      server's true block); the join-identity law (a Welcome seed
      adoption regenerates the boot ring and world-derived state so
      the joiner lands on the server's terrain, not a two-seed
      patchwork). 5 new laws (root 488 -> 493; lf_client 93 -> 97,
      lf_server 3 -> 4) incl. the newcomer-history wire law over real
      UDP (three edits across three chunks replay to a later joiner).
      Evidence: workspace 493/0 (35 suites); smoke OK; runtimes fresh
      on disk; visual gates not run — singleplayer renders the same
      paths (the 458/459/461 precedent) and the multiplayer GPU client
      has no visual harness (honest limit). Deferred honestly: geode
      pairing; server-side inventory authority (the corrective echo
      reverts the block, not an optimistic item take); a windowed
      two-client route.

- [x] The keeper's chronicle law: every true waking is a Discovery,
      staffed roosts never spam (2026-09-15, loop 461, root
      workspace): closed STATE's next_task item (1) — 447's deferral
      ("the guardian's chronicle Discovery re-fires if a geode
      re-settles after despawn, dragon-precedent behavior"). The
      behavior already matched the precedent but lived inline in
      three client settle sites with duplicated literals and no law.
      THE LAW (lf_game::mobs::settlement_chronicle): settling a
      keeper into an UNSTAFFED anchor is a Discovery — every settle
      incl. re-settles (each waking is an event in the player's
      authored history); a staffed roost wakes nobody; the crawler's
      return is vermin, never chronicled. The guardian, crawler, and
      dragon settle passes all delegate; the two Discovery lines
      have a single source. NO behavior change (lines moved
      verbatim). 2 new laws (root 486 -> 488): the re-fire across
      consecutive unstaffed decisions + the staffed no-spam; vermin
      (staffed or not) never chronicled. Evidence: workspace 488/0
      (35 suites); smoke OK; runtimes refreshed on disk; visual
      gates not run — nothing visual changed (the 458/459
      precedent). Deferred honestly: multiplayer routing of terrain
      edits; geode pairing.

- [x] The pass-routing law: the water channel is art identity, not
      atlas position (2026-09-15, loop 460, root workspace): closed
      STATE's next_task item (1) — the is_water_layer/CTM-strip
      audit carried since 447. The audit concluded: the strip
      ADDRESSING is sound (UVs, generator placement, 47-tile table,
      filler slot all agree), but the water-pass ROUTING had
      drifted — `is_water_layer` still classified by the loop-332
      literals `10 || 167`, and 167 is dead_shrub's layer today
      while water's real marker is 4098. Two live misroutings:
      every water TOP rendered opaque (marker 4098 routed to the
      REPLACE-blend pipeline; the art's alpha-170 translucency lost
      on all connected surfaces) and every dead-shrub face rode the
      blended, depth-write-off water pipeline. THE LAW: a vertex
      rides the water pass iff its tex is WATER ART — the named
      WATER_BASE_LAYER or water's own strip slot marker, derived
      from the same mirror table that stamps it; atlas position is
      never identity; lf_assets pins the one-way mirror cross-crate.
      6 new laws (root 480 -> 486): the real water mesh routes all
      verts (fails on the old predicate); the drift witnesses and
      grass/stone refuse; marker space routes by slot; the
      whole-atlas sweep; the mirror pin; the strip-addressing law
      (UV rect == generator rect over all 376 slot-tile pairs).
      Evidence: fresh same-seed before/after renders — pool surface
      distinct colors 750 -> 1926, submerged terrain reads through
      (INSPECTED incl. 1.5x zoom pairs; first_person_view/hud_
      preview ~50% pixel change = the lake going translucent, HUD
      itself unchanged); battery 108 scenes [ok]; workspace 486/0;
      smoke OK; idle-upgrade-check PASS. Deferred honestly: the
      carried list — guardian chronicle re-fire (DONE 461);
      multiplayer routing of terrain edits; geode pairing.

- [x] The query-bound law: the ground query's origin contract made
      explicit (2026-09-15, loop 459, POORCRAFT 3D): closed STATE's
      next_task item (1) — the walk-snap query-bound observation
      carried since 445. CollisionSurface::ground_at's from_y is now
      THE QUERY-BOUND LAW in the trait doc: on the authority path the
      three-cell search window (pure predicate
      in_authority_query_window: answers in (from_y - 3,
      floor(from_y) + 1]); on the streamed paths bounds nothing BY
      CONTRACT — the answer is the column's PLACEMENT,
      origin-independent; a future streamed bound must adopt and
      change the law consciously. UNBOUNDED_QUERY_Y names the
      f32::MAX/4.0 probe (STREAMED-path idiom: the authority window
      under it holds only sky and refuses — pinned by law);
      QUERY_REACH_M names the walk's feet+2.0 origin. 3 new unit
      laws: region-surface placement answers every origin (==
      mesh height, incl. the unbounded probe); the authority window
      binds on real columns + refuses the unbounded probe + hand
      cases; the live streamer answers every origin in the loaded
      full ring, refuses outside it at every origin. NO behavior
      change: recovery PASS x2 + comparator (p50 26.1/26.8 ms; wake-up
      EXACTLY 0.5 — bar pixel-measured 0.975 rim vs 0.471 final,
      byte-identical a/b); walkoff PASS x2 + comparator (p50
      25.7/25.7 ms); captures INSPECTED. Evidence: p3d 693 green
      (pc3d_render 236); smoke digest unchanged (dd019eca900f5a61);
      idle-upgrade-check PASS; visual gates not run (nothing visual
      changed). Deferred honestly: the is_water_layer/CTM-strip audit
      is next in line; carried: guardian chronicle re-fire,
      multiplayer routing of terrain edits, geode pairing.

- [x] The bench guards its host: contention probe + the quiet-host
      re-read (2026-09-14, loop 458, POORCRAFT 3D): closed STATE's
      next_task item (1) — the deck-bench re-read queued since loop
      446 — plus 445's own deferred bench contention guard. THE GUARD
      (pc3d_render::deck): cpu_probe_ns, a fixed 400k-step
      deterministic float workload (~2.7 ms/reading, calibrated), read
      at each tier run's start and end; readings ride the sidecar CSV
      and the report's new "host CPU probe" bullet; the pure
      probes_within_band law (slowest <= fastest x 1.4, zero never
      quiet) is asserted BEFORE the report is written — a contended
      run writes no report. 4 new unit laws; pc3d_render 229 -> 233.
      THE RE-READ: low 7.18 / mid 12.92 / high 13.06 ms p50 on a quiet
      host (probes 0.7% spread; every work counter byte-identical to
      445's record) vs 442/445's 6.85-6.89/12.16-12.30/12.44-12.56 —
      a uniform +4-6% that the A/B DECIDES: the loop-445 binary on
      this host today reads 7.21/12.88/13.02, so the shift is
      host-state drift, not code cost; 446-457 cost nothing measurable
      on the bench walk. Tier captures re-rendered + INSPECTED (same
      vista, leaner dressing per tier). Evidence: p3d 690 green
      (pc3d_render 233); smoke digest unchanged (dd019eca900f5a61);
      people PASS (0.66%); idle-upgrade-check PASS; visual gates not
      run (nothing visual changed — the 445 bench-only precedent).
      Deferred honestly: the contention guard is DONE; carried: the
      walk-snap query-bound law, the is_water_layer/CTM-strip audit,
      guardian chronicle re-fire, multiplayer routing of terrain
      edits, geode pairing.

- [x] Every beat capture reads the latch: climb, hang-off, playtest
      (2026-09-14, loop 457, POORCRAFT 3D): closed STATE's next_task
      item (1) — the remaining routes adopted 456's capture-side latch
      cure (app main.rs only; p3d count unchanged 686). The climb's
      GRIPPED latch poll frames play_vine_grip (fixed @170 gone — the
      toast is IN the pixels at the catch beat); the hang-off's poll
      fires hop_fall at the AIR BEAT (first poll genuinely airborne
      below the tip; request pose+ground+frame recorded, latch frame
      newly recorded, leg-1 cells 12 -> 18) and hop_landed AT the
      landing latch, leg 2 fires tip_landed AT its latch (leg-2
      teleport 350 -> 356 so an edge latch can't race the lift;
      end_frame Some(700) owns the exit past the 430 drain); the
      playtest's poll frames play_fall_air at the first
      genuinely-airborne poll (the +8 m teleport makes it the window's
      opening) and play_fall_landed AT the FELL-toast latch. Both
      verdicts gained the AIR-BEAT assertion (request pose on the spot,
      request frame before the latch); the hang-off's
      fall/rise-vs-landed pixel pairs moved to the advisory sway bar
      (at contended paces the hop compresses — 454's own evidence),
      the playtest's air-vs-landed stays HARD and is now genuinely
      place-vs-place. Evidence: climb PASS x2 + comparator (p50
      22.6 ms; GRIPPED toast legible AT the latch); hangoff PASS x2 +
      comparator (p50 24.5-24.7 ms; air beat latched frame 234->235,
      pose 54.74 = 0.10 below the tip, 1.56 over the ground; hop rose
      1.04, grounded 100% -> 81% = the law's number; tip release safe
      1.02 m); playtest PASS x2 + comparator (p50 23.9 ms; fell 8 m
      over the open journal, health 33%); six captures INSPECTED +
      layout JSONs verified both runs; walkoff/dig/recovery x2 +
      comparators (walkoff air beat 110->111 / 108->109 — different
      frames, the capture follows the beat); people PASS (0.64%);
      gates ALL 10; smoke digest unchanged (dd019eca900f5a61);
      idle-upgrade-check PASS. Deferred honestly: the remaining fixed
      captures in these routes are deterministic beats or run-end
      keepers, left fixed on purpose.

- [x] The captures read the latch: walk_air and walk_landed schedule
      themselves from the fall's own polls (2026-09-14, loop 456,
      POORCRAFT 3D): closed STATE's next_task item (1) — the CAPTURE
      side of the latch cure. UiAction::CaptureAtNextFrame joins the
      proof-hook family: a route poll that LATCHED a beat asks for
      the NEXT presented frame's picture and the app inserts it into
      the sorted shot list (pure insertion law next_free_shot_frame +
      2 unit tests: fires requested_at+1, walking past any owned
      frame — two shots can never share a frame, the second would
      silently never fire). WindowConfig::end_frame owns the exit for
      dynamic-capture runs (the static list may drain early; runs
      without it keep the old drain law; validation errors up front
      if the horizon misorders). THE WALK-OFF REWIRED: walk_air@112
      and walk_landed@190 are gone; the polls fire walk_landed AT the
      landing latch (wounded bar + fresh FELL toast in pixels at
      every pace) and walk_air at the FIRST poll truly airborne over
      a meter below the rim (recorded flag+pose+frame; verdict gains
      the air-beat assertion — request pose strictly between the rim
      band and the floor, before the latch). Evidence: p3d 686 green
      (pc3d_render 229); walkoff PASS x2 + comparator at p50 25.5/
      27.4 ms — the runs latched the air beat at frames 104 and 102
      (the capture follows the beat, not the calendar) with identical
      physical claims (fell 56.19 -> 51.19, deepest air 51.38, 100%
      -> 65%, toast true); six captures INSPECTED (air = full bar +
      dig toast, landed = 64% bar + FELL over the pit wall, edge =
      the rim; layout JSONs carry the same story); dig/recovery x2 +
      comparators; people PASS; gates ALL 10; smoke digest unchanged
      (dd019eca900f5a61); idle-upgrade-check PASS. Deferred honestly:
      the remaining routes' capture conversions (playtest toast,
      climb grip, hangoff landings — verdicts already read latches).

- [x] The verdicts read the latch: walk-off and playtest proofs made
      pace-proof (2026-09-14, loop 455, POORCRAFT 3D): landed the
      verdict-side half of 454's next_task item (1). The two routes
      whose frame-CALIBRATED verdicts broke twice under the day's
      foreign-load spike now prove the same claims from latched
      per-frame polls — walkoff: one poll (92..=189) records the
      DEEPEST AIRBORNE pose (grounded clamps never enter the min) and
      LATCHES the landing on two consecutive on-ground polls vs the
      LIVE ground answer (pose + health + FELL toast AT the latch —
      the regen race gone; the mid-air band now "meters of air, then
      the latch"; the pixel gate place-vs-place, air-vs-landed
      advisory); playtest: the FELL toast latches in a poll window
      pushed BEFORE the frame-900 entry (the script queue fires
      strictly in push order — the first run failed on exactly that
      and the law is now written where it bites). No lib code changed
      (app main.rs only; p3d count unchanged 684). Evidence: walkoff
      PASS x2 + comparator at p50 73 ms (the 54-59 ms paces broke the
      committed verdict today), playtest PASS x2 + comparator at
      69 ms; dig/recovery x2 + comparators; people; gates ALL 10;
      smoke digest unchanged; captures inspected. Deferred honestly:
      the CAPTURE side (Capture::at_next_frame — next_task item 1);
      the carried 454 list.

- [x] Space lets go: the strand release is real (2026-09-14, loop 454,
      POORCRAFT 3D): closed STATE's next_task item (1) — and the route
      found a real bug first: BOTH release paths silently re-gripped.
      The pre-law windowed probe pinned it (the Space hop rose 1.04 m
      and fell back into the crossing catch, window-min never below the
      strand; the S tip release was re-caught inside the band below the
      tip and the body stayed pinned AT the tip, health 1.00, never
      grounding — the strand was a one-way trap under its own "SPACE
      LETS GO" toast; the old unit suite modeled no post-release grab
      check). THE LAW (pc3d_render::app): SliceHost.released_from + the
      pure strand_can_catch gate at the arc's grab — strand-scoped
      (another strand still catches), spent on landing, spent by a new
      grab; the grab law's tunnel tests untouched. 3 new unit laws;
      pc3d_render 224 -> 227. THE ROUTE (make p3d-hangoff): two legs on
      the climb's own strand — the Space hop-off (rises ~1 m, falls
      PAST its own strand, lands grounded at the strand's line with the
      impact law's EXACT wound) and the S tip release (falls free,
      lands safe, health never drops); landings latch on two
      consecutive on-ground polls vs the LIVE ground answer, the FELL
      toast latches AT the landing — pace-proof. THE CLIMB ROUTE'S
      VERDICT NOW ASSERTS GROUNDEDNESS (the pre-law body was pinned at
      the tip; the old claim passed unchecked) and proves the fall by
      latched records instead of the sway-aliased pixel bar. Evidence:
      p3d 684 green / 0 failed; root 480 green; hangoff PASS x2 +
      comparator (PASS at 20/30/48/80-108 ms across the day's host
      states); climb PASS x2 + comparator (79-88 ms); dig PASS x2 +
      comparator; people PASS (0.64%); gates ALL 10; smoke OK (digest
      dd019eca900f5a61 unchanged); playtest digest reproduced UNCHANGED
      05c46411869a857c pre-spike. The foreign-load spike (15-min load
      94; windowed p50 54-142 ms) broke the frame-CALIBRATED walkoff/
      playtest re-runs (the documented 447/449 sensitivity, third
      occurrence) — their code is untouched by this job; their bundles
      ride from loop 453 and the digest pins non-perturbation. PERF: no
      claim — one Option compare per falling frame; host contended, no
      bench per the 445-453 precedent. Deferred honestly: the windowed
      harness's frame-scheduled captures stay pace-sensitive (latch-
      scheduled captures are the systemic cure — next_task item 1); the
      quiet-host deck-bench re-read + the bench contention guard;
      is_water_layer/CTM-strip audit; guardian chronicle re-fire;
      multiplayer routing of terrain edits; geode pairing; the walk-
      snap query-bound observation.

- [x] The plaza recovery, live: the lethal fall's windowed route
      (2026-09-14, loop 453, POORCRAFT 3D): closed STATE's next_task
      item (1) — the fall law's 0.5-health plaza recovery branch had
      unit laws but no windowed route. THE ROUTE (make p3d-recovery,
      route_plaza_recovery): the walk-off's own spot finder stages a
      flat, flora-free, vine-free cell near the plaza; one -14 m edit
      pits it under the standing body; the arc lands ~16.5 m/s and the
      impact law costs ~1.15 health — more than the body carries — so
      the recovery fires. Verdict: rim on the LIVE pre-dig ground at
      full bars; mid-fall between floor and rim; recovered XZ at the
      plaza center, feet ON the LIVE plaza-ground answer, health
      EXACTLY 0.5, food exactly halved; still there later (no second
      fall, no regen at 0.5 food); the recovery toast present, the
      plain FELL toast absent; five captures all differing. THE
      PREDICTION THE RUNTIME DISPROVED (honest): statics said the
      keep-Y teleport would plant the body under the plaza — the first
      run against UNCHANGED code PASSED (the walk's any-rise ground
      snap lifts the body in one frame); no pose change shipped; the
      route laws the outcome; surface.rs ground_at discards its
      _from_y bound (recorded). THE GLYPH (proof-found): the em-dash
      in both fall toasts rendered as a blank gap — glyph added + ink
      law (distinct from the hyphen); pc3d_render 223 -> 224.
      Evidence: p3d 681 green; recovery PASS x2 + comparator + all
      five captures INSPECTED; walkoff/dig/playtest PASS x2 +
      comparators; people PASS (motion 0.66%); gates ALL 10 PASS;
      smoke OK (digest unchanged); idle-upgrade-check PASS. PERF: no
      claim — route-only work; live paths untouched; host shared, no
      bench per the 445-452 precedent. Deferred honestly: Space
      jump-off from a hang; the walk-snap query-bound observation;
      the quiet-host deck-bench re-read + the bench contention guard;
      is_water_layer/CTM-strip audit; guardian chronicle re-fire;
      multiplayer routing of terrain edits; geode pairing.

- [x] The pack is legible: the inventory's echo on the HUD
      (2026-09-14, loop 452, POORCRAFT 3D): closed STATE's next_task
      item (1) — finished from an interrupted session's orphaned
      start (items.rs held an unwired, untested stock_line).
      THE LINE (pc3d_world::items::stock_line): the inventory's
      echo as ONE stable string — nonzero kinds in CATALOG order,
      counts summed across stacks, "PACK EMPTY" when empty; two
      unit laws. THE HUD: HudValues.stock synced from the REAL
      slice inventory on UI-dirty frames, painted verbatim under
      the status bars, serialized to the inspector JSON; layout
      law pins placement and absence-until-synced. THE GLYPH
      (proof-found): the capture inspection showed " · " as a
      BLANK GAP — "·" was missing from the pixel font (the
      pre-existing prompt row had the same hole); added the
      middle-dot glyph + an ink law. THE ROUTE (make p3d-dig
      extended): the expected pack line computed from the same
      determinants the verb uses; the verdict asserts the HUD
      strings "PACK EMPTY" -> "PACK WOOD 1 · SOIL 1"; PASS x2 +
      comparator; captures INSPECTED. The playtest journey
      carries the economy: IRON_ORE 4 -> forge take IRON_BAR 1 ->
      re-harvest, legible over open panels. Evidence: p3d 680
      green (pc3d_render 223, pc3d_world 267); dig PASS x2 +
      comparator; playtest PASS x2 + comparator, bundle digest
      RE-BASELINED 1d37f62c3801ae20 (honest change — the pack
      line rides every gameplay capture); people PASS (motion
      0.64%); visual gates ALL 10 PASS; smoke OK (dd019eca900f5a61
      unchanged); idle-upgrade-check PASS. PERF: no claim — one
      small string on UI-dirty frames, one text row, a 12-pixel
      dot. Deferred honestly: the lethal plaza-recovery branch
      route; Space jump-off from a hang; the quiet-host deck-bench
      re-read + the bench contention guard; is_water_layer/
      CTM-strip audit; guardian chronicle re-fire; multiplayer
      routing of terrain edits; geode pairing.

- [x] The crowd yields in the window: the staged-yield crowd route
      (2026-09-13, loop 451, POORCRAFT 3D): closed STATE's next_task
      item (1) — the 448 crowd law was unit-lawed + render-lawed but
      no windowed route framed two NPCs yielding. THE STAGE
      (`crowd_stage_head_on_near` + readers `crowd_cell`/
      `crowd_work_site` in pc3d_render): two cast members rewritten
      into head-on walkers on ONE fully walkable row near a requested
      center (real nav paths the Work-phase schedule keeps), the rest
      parked at their homes; a row is admitted only when BOTH nav
      paths are STRAIGHT along it, the row + ±1 sidestep band touches
      no settlement collision cell (the nav is terrain-only — the
      first staged row walked through the house band), and the parked
      home sits clear; rows tried nearest-center-first inside the
      nav-patch interior (the plaza sits on a patch corner — every
      plaza-relative span crossed the border). THE ROUTE
      (make p3d-crowd, route_crowd_yield): the live slice's own
      crowd_tick runs the real law every frame — one walks through,
      the other is refused at gap 2 and sidesteps, both arrive Working
      at their DECLARED sites; a per-frame audit (70 frames) ORs the
      flags: no shared cell, the off-row yield seen, read failures
      poison the run; captures staged/yield/pass/arrived INSPECTED
      (debug + release runs). PASS x2 identical + comparator PASS.
      EN-ROUTE: capture frames re-timed behind the vantage teleport;
      the row-end vantage swapped for south-of-row at 5 m (a kit stall
      filled the lens). PRIOR-PROOF FIX: p3d-people's motion check
      aliased wall-clock stride phases below its bar at this week's
      frame pace (0.0016 FAIL then 0.0020 PASS on identical code vs
      448's 0.0103) — the harness now freezes the pose clock at two
      KNOWN phases: 0.0065/0.0063, 3x the bar, scheduling-independent.
      Evidence: p3d 676 green (pc3d_render 221, pc3d_world 265);
      playtest x2 digest UNCHANGED 05c46411869a857c; people x2 PASS
      (12 NPCs, 7 draws, 92 instances); smoke OK (dd019eca900f5a61);
      assets OK; idle-upgrade-check PASS. PERF: no claim — the route
      adds work only while it runs; host shared, no bench per the
      445-450 precedent. Deferred honestly: the inventory echo of the
      dig verb (next_task item 1); the lethal plaza-recovery branch
      route; Space jump-off from a hang; the quiet-host deck-bench
      re-read + the bench contention guard; is_water_layer/CTM-strip
      audit; guardian chronicle re-fire; multiplayer routing of
      terrain edits; geode pairing.

- [x] The dig is in your hands: the player-facing dig verb (2026-09-13,
      loop 450, POORCRAFT 3D): closed STATE's next_task item (1) — G
      digs the ground under the crosshair through 446's live
      EditSurface path. THE TARGET (pure, `player::dig_target`): the
      look ray marches the SAME live ground answer the walk stands on
      and the picture draws (streamed delta layer included) — first
      column met within the build verb's 8 m reach; dug terraces move
      the aim past them, walls take the dig at the face, a steep gaze
      digs underfoot (the walk-off trick is now a legal verb). THE
      TAKE: `harvest_yields` (the existing tested drop table) gains
      its player-facing consumer — soil/sand/snow bare-handed, stone
      needs a pick, grass yields soil+wood; new `Inventory::can_fit`
      answers BEFORE the edit (a take that can't be carried never
      breaks ground); a taking dig is Excavated quest progress. THE
      INPUT: the real UI path — `ui_key` KeyG -> `ui::on_key` ->
      `UiAction::DigAtCrosshair` (the forge keeps G while open;
      panels own the frame); refusals are honest toasts
      ("NEED A PICK FOR STONE" / "PACK FULL - THE GROUND HOLDS" /
      "NO GROUND IN REACH"), takes toast their names ("DUG
      SOIL+WOOD"). HUD prompt + Settings keymap gained G DIG. Route
      proof: `make p3d-dig` (route_dig: the body G-digs its own cell
      aimed steep — exactly 1.00 m down at the standing point,
      control untouched, the step law snaps it down UNWOUNDED, one
      step walks back up) x2 identical + comparator PASS, captures
      inspected. EN-ROUTE (prior-proof fix, the carried
      contention-window deferral): route_walk_off's frame-120
      mid-air window predated this week's ~22.5 ms windowed pace and
      caught the body 0.19 m above the floor twice — re-timed to
      frame 112 (mid 53.62 = 2.57 m down; the physical band is
      unchanged). Evidence: p3d 675 green (pc3d_render 220,
      pc3d_world 265); walkoff x2 PASS + comparator; playtest x2
      digest UNCHANGED 05c46411869a857c; climb/steer x2 unchanged;
      smoke OK (dd019eca900f5a61); assets OK. PERF: no claim — the
      dig works ON PRESS only (one <=80-sample march + edit + remesh,
      allocation-free); host shared, no bench per the 445-449
      precedent. Deferred honestly: multiplayer routing of terrain
      edits; an inventory readout to SEE the take as a number; geode
      pairing (447's keepers are root-workspace lore — p3d gets its
      own Old-Powers expression later).

- [x] The wall holds: the up-step half of the walk law (2026-09-13,
      loop 449, POORCRAFT 3D): the streamed surface's ground snap
      accepted ANY rise (`pos[1] - g <= SUPPORT_GAP_M` is trivially
      true when g is above the feet) and the streamed path's
      `cell_solid` answers false everywhere, so every cliff face and
      dug wall was a free elevator — fall into a pit, walk straight
      out. THE RISE LAW (pure, `player::rise_accepted` +
      `rise_refused_on`): an axis move whose target ground sits above
      the feet is refused unless the surface there is itself walkable
      — a discrete step within SUPPORT_GAP_M (the up twin of the down
      law) or a ramp within the nav's own MAX_WALK_SLOPE, measured as
      a SIGNED rise along the move over a 1 m baseline (the mesh's
      node spacing) — geometry, not the frame, so the verdict is the
      same at 30/60/120 fps; a descent ahead is never a wall.
      EN-ROUTE BUG caught by the live route and promoted to a law:
      the first draft's |ahead-behind| baseline let the ramp BEHIND a
      body glue it to the wall it had just been refused by (and fps
      jitter unstuck it — the route's W-hold sat motionless 60 frames
      then lurched); fixed signed + pinned by
      the_body_walks_away_from_a_wall_it_was_refused_by. New proof
      hook UiAction::PlayerFace {yaw, pitch} (the body's absolute
      facing; also serves the carried look-pitch deferral). Route
      proof: `make p3d-pitwall` (route_pit_wall: the walk-off's dig
      cell dug 5 m — the body falls — the cell TWO toward -z dug
      4.5 m so the between cell is a gentle ramp [a one-cell-away dig
      ACCUMULATES on shared border nodes and tilts the pit deeper —
      the first staging attempt fell twice, hence
      find_dig_spot_pair's strip validation]; W into the 5 m wall is
      REFUSED at the base; the ramp+step admit the walk; the step
      cell's own outer wall refuses again; feet 49.49 -> 49.49 ->
      49.87, health 100% -> 76%, FELL toast) x2 identical +
      comparator PASS, captures inspected. Evidence: p3d 667 green
      (pc3d_render 213); walkoff x2 UNCHANGED (same cell, same fall);
      playtest x2 digest UNCHANGED 05c46411869a857c; climb x2 and
      steer x2 unchanged; smoke OK (dd019eca900f5a61); assets OK.
      PERF: no claim — at most three ground samples per moving axis
      per frame, allocation-free; host still contended (no bench
      recorded; the quiet-host re-read is queued by four loops).
      Deferred honestly: the player-facing dig verb (now pairs with
      this law: a dug pit HOLDS, so stairs/ramps out are a real
      skill); the staged-yield crowd route (main.rs now free).

- [x] The crowd yields: NPC-vs-NPC avoidance (2026-09-13, loop 448,
      POORCRAFT 3D): the last NWR-009 sim-domain deferral closed.
      `pc3d_world::npc::step_crowd` is the pure crowd law: no body
      enters a cell another body stands on or has claimed this tick;
      a blocked walker yields — one sidestep around the blocker
      (perpendicular, then back, first free walkable in-patch cell) or
      stands waiting keeping path and leg; cast order is the only
      tie-break; `step` split into plan + advance_leg with lone
      behavior byte-equal (lawed); the sidestep re-paths as Idle so a
      yield cell can never read as an arrival. `npcs::advance` (the
      one call site every harness and the live slice uses) delegates
      to it. Laws x5 (crossing never shares a cell over 400 ticks +
      declared-site arrivals; head-on yields, arrives, replays
      bit-identically; the sealed crowd holds the walker; lone
      trajectory == brain.step; the render-facing advance enforces
      the law). Evidence: p3d 663 green on the combined tree
      (446+447+448); p3d-people PASS (motion 1.03%, captures
      inspected); p3d-playtest x2 + comparator PASS with digest
      UNCHANGED 05c46411869a857c; smoke/assets OK (digests
      unchanged). PERF UNAVAILABLE honestly: two deck-bench attempts
      discarded under measured concurrent load (mid>high inversion;
      446/447/448 all discarded benches today); report + PNGs
      restored to HEAD (445's clean record stands); the quiet-host
      re-read stays queued. Deferred honestly: no windowed ROUTE
      frames two NPCs yielding (needs main.rs, contended by 446's
      in-flight work when this job started).

- [x] The geode wakes: the Old Powers' keepers take their anchors
      (2026-09-13, loop 447, root loreforge workspace): the audit's
      spawn-or-cut item resolved by SPAWNING — GeodeGuardian and
      CinderCrawler are real MobTypes now, and the orphan lf_npc
      structs are deleted. New ANIMA_CRYSTAL block (145, violet
      [8,5,14] light) + rare (1/113 chunks) SEALED underground geodes
      stamped through a pure geode_cell geometry shared by generation
      and the proof scene (BFS law: the pocket never leaks); one
      guardian per geode (settle-on-load, roost-anchored, detect 6 —
      it defends the hollow, it does not hunt the tunnels; it is not
      evil), crawlers settle on floored ledges beside deep lava;
      mining a crystal provokes the local guardians (provoke_guardians
      law); kills and mined crystals yield the existing anima_crystal
      item — the Covenant craft recipe gains a world source. Laws:
      geode rarity/determinism/depth + seal (lf_worldgen x2),
      never-roll + tables + articulated parts + anchors + provoke
      (lf_game x4), texture regression (lf_assets). EN-ROUTE BUG
      FOUND BY THE PROOF: the hand-counted ui-world-craft layer
      consts had drifted — LAVA silently rendered TALL-GRASS art and
      the surface tufts drew a wolf skin; fixed name-derived (the
      layer_of cure) + the regression law
      lava_and_surface_decorations_render_their_own_art. Proof:
      vistest geode_guardian scene (real stamp fn + real
      animal_parts render), INSPECTED — crystal hollow + keeper, lava
      pool + crawler; FULL battery 108 scenes PASS; workspace 480
      green (476 + 7 new laws - 3 dead tests); smoke OK. Perf: the
      settle scans are frame-gated (every 180 frames, staggered from
      the dragon pass) with bounded windows; make perf read 90.1 ms
      p50 under load-35 contention (15-min avg 39) — DISCARDED as
      uncontrollable, not comparable; the static-scene perf harness
      runs none of the changed code paths. En-route observation:
      loop 446's route_walk_off re-run independently at load 35
      (p50 28.7 ms/frame) confirmed every physical claim (exact
      dug-floor landing, wounded, FELL toast, deterministic x2 +
      comparator) while the FRAME-INDEXED mid_air capture window
      missed at 3x frame time — the route harness's capture windows
      are contention-sensitive; hardening deferred. Deferred honestly:
      the guardian's chronicle Discovery re-fires if a geode re-settles
      after despawn (dragon-precedent behavior); is_water_layer's
      hard-coded 167 now points at dead_shrub's index — water proofs
      pass, but the CTM strip addressing deserves an audit.

- [x] The walk-off lives: the step law + the dug-floor route
      (2026-09-13, loop 446): proof-discovered bug — the loop-444
      walk-off commit was DEAD on the live streamed path (the walk's
      unbounded Y-snap swallowed every drop the surface answered, so
      walked-off bodies landed soft and damage-free; probe-reproduced,
      then fixed). THE STEP LAW: the walk holds within one step
      (1.05 m); deeper drops are refused to the walk-off commit + arc,
      now SHARED by both walk paths. The dig is live
      (`UiAction::EditSurface` → `surface_edit` + `remesh_now`: delta
      edit + same-frame remesh — the player-dig-verb foundation).
      Route proof: `make p3d-walkoff` — the floor dug out under the
      standing body (natural ledges ramp to walkable slopes at 1 m
      node interpolation, BY CONTRACT; the law's "a floor dug out"
      trigger is the honest live one), feet 54.49 → 49.49 EXACT,
      100% → 67-68% wounded, FELL toast, x2 + comparator PASS,
      captures inspected. Playtest/climb/steer/smoke/wilderness/
      assets regressions green; p3d 658 (pc3d_render 208); root
      workspace green. Deferred honestly: the UP-step half of the
      streamed walk still snaps any rise (cliffs climbable — pre-
      existing, needs its own wall/step law + route); the player-facing
      dig verb (key/bindings/targeting) is future work on top of the
      live edit path.

- [x] The bench tells the truth: p3d-deck-bench argv fix + report
      refresh (2026-09-13, loop 445): the target's unquoted empty
      `$(SEED)` shifted the binary's argv, so every default run
      benchmarked the MID tier three times (the tier name became the
      output dir — stray `low/`/`mid/`/`high/`/`report/` debris dirs,
      committed in 444), and the "report" invocation ran a fourth mid
      bench instead of assembling DECK-BENCH-REPORT.md. The report was
      last written in 442; 443's "report + bench PNGs refreshed" claim
      did not hold on disk. Fixed (defaulted seed, debris removed,
      clean uncontended run): low 6.85 / mid 12.30 / high 12.56 ms p50
      vs 442's 6.89/12.16/12.44 — noise-sized, no conclusion changes.
      En-route: independent verification of the concurrent session's
      loop-444 air steer (fresh p3d-steer x2 + comparator PASS, drift
      1.56 m / ortho 0.00 m, captures inspected). No Rust code changed.

- [x] The air steer: the fall answers the hand (2026-09-13,
      loop 444): a falling body keeps lateral control — a fixed
      0.45 fraction of the walk speed (1.8 m/s), sprint never
      applies mid-air, the same drift at any refresh rate. The
      walk-off hole closed in the same path: a body whose support
      vanishes (ledge, dug floor) now commits the fall instead of
      hover-gliding at full walk speed. Route proof:
      `make p3d-steer` (the same 5 m drop twice — the free fall
      holds its line exactly, A held drifts it 1.50 m along the
      strafe axis and 0.00 m across it, both drops land the same
      wound) x2 identical + comparator PASS, captures
      AI-inspected. Composition law: steering into a strand
      catches what the free fall misses. En-route: playtest
      digests UNCHANGED; vine climb unchanged; deck bench
      noise-sized (the bench attaches no slice — the walk change
      has no bench cost path; the low tier read contended by a
      concurrent foreign process, documented). Deferred honestly:
      the walk-off commit has no dedicated live route yet (needs
      a deterministic > 1 m ledge); Space jump-off from a hang is
      still unit-lawed only.

- [x] The vine grip: the hanging strand becomes a traversal verb
      (2026-09-13, loop 443): a falling body that passes a drawn vine
      strand within hand reach CATCHES it — the grab law refuses
      rising, distant, and out-of-span bodies, and a fast drop cannot
      tunnel through the strand (the crossing catch). The hang zeroes
      the fall; the strand owns the body (XZ pinned, Y under climb);
      W climbs to the attach and clamps, S descends and releases past
      the tip so only the drop below the strand counts. The strand
      truth is the DRAWN mesh (per-variant lowest vertex x jitter —
      the grip can never disagree with the picture). Route proof:
      `make p3d-climb` (route_vine_climb: a 12 m lethal drop caught at
      a real strand, climb, tip release, land unharmed) x2 identical +
      comparator PASS, captures AI-inspected; playtest digests
      UNCHANGED; deck bench noise-sized. En-route: the route framework
      gained key_script (real gameplay keys injected through the real
      input path — the capability mid-air-steering proofs were
      missing). Deferred honestly: canopy-interior climb frames need a
      look-pitch hook to frame the strand itself; Space jump-off is
      unit-lawed only; mid-air steering still untested as a law.

- [x] The vine anchor concept: the last catalog-only family grows in
      the wild (2026-09-13, loop 442): the vine's mesh hangs downward
      from y=0, so the authority gained the missing concept — a vine
      exists only where a fixed-order 8-neighborhood scan finds a
      LIVING canopy anchor (broadleaf/pine; airy birch and dead wood
      carry nothing), and vine_anchor() answers the attach point:
      0.5-0.7 m lateral of the anchor trunk at the anchor's own ground
      plus per-family hang laws (pine 1.5 m inside the skirt cone,
      broadleaf 2.9 m at the crown underside — valid across the
      canonical AND variant sweeps). Vines joined Forest (0.010) and
      Highlands (0.004); Plains grows none by design. The renderer
      draws the vine at its anchor, and the wind sway now weights
      |mesh y| so the TIP swings under the fixed anchor (upward meshes
      byte-identical). Laws: the anchor gate (34 grown / 13 anchorless
      refused in one Forest region), the hang-law shape, and a GPU
      law (presence 0.154, tip sway at two frozen times). The
      wilderness proof gained a traveling CANOPY capture that searched
      the seed, meshed the vine's own ground, and framed the hanging
      strand (AI-inspected). Deck bench: low 6.89 / mid 12.16 /
      high 12.44 ms p50 vs 440's 6.79/12.36/12.62 — noise-sized.
      En-route: the p3d-806 scale law's linear judgment hardened to
      best-of-k tick cost after the full suite caught the mean
      tripping under parallel load (the 20 ms sustain budget stays on
      the mean). Deferred honestly: grass_tuft GLBs stay covered by
      the deliberate Deck-cheap card field; the CANOPY capture frames
      one specimen (a wider drape framing is a polish pass).

- [x] The live fall proof: gravity is the world, not the menu
      (2026-09-13, loop 441): the playtest drops the player 8 m onto
      the plaza with the quest journal OPEN and ends wounded
      ('fell 8 m over the open journal (health 33%)', x2 identical +
      comparator PASS). The standing 438 deferral's root cause was two
      real bugs: the airborne arc was gated behind gameplay_active
      (any open panel froze a fall mid-air — the '+1.7 m zero-gravity'
      freeze), and Space jump was dead code (gravity only ran while
      falling while the walk's snap held everyone down). Fixed by a
      pure fixed-step (1/60 s) integrate_air_arc landing on the
      per-frame ground answer, running OUTSIDE the gameplay gate, with
      one Space branch committing the same airborne state; 3 new unit
      laws (jump lands safe, current-ground landing, refresh
      independence); the OBSERVE line now reports the real verdict.
      Deferred honestly: the lethal plaza-recovery branch has no route
      proof; mid-air steering is untested as a law.

- [x] The thousand-asset families go wild (2026-09-12, loop 440): 20 of
      the 22 new GLB families (920 assets) grow biome-appropriately in
      the played world — forest undergrowth, wetland reeds, ruin heights
      (columns/cairns/arches/crystals/obsidian), snowpeak ice — via a
      29-kind PlantKind authority with census/loader/tag/GPU laws and a
      new UNDERGROWTH wilderness capture; the deck bench enforced a
      near-field variant rule en route (variants beyond lod0's 40 m draw
      the canonical base) that halved the walk's frame time vs the
      pre-change baseline (mid p50 23.44 → 12.36 ms) while drawing more
      instances in fewer buckets. Deferred honestly: vine needs an
      anchor concept (hangs downward), grass_tuft GLBs duplicate the
      deliberate card field.

- [x] ZCode perpetual idle-upgrade pack (2026-09-12): a paste-ready prompt
      that repeatedly ships one bounded improvement without declaring the game
      designed, a 977-line consolidated LOREFORGE canon bible, measured
      performance/dependency/new-crate/portability gates, a machine-readable
      contract, and `make idle-upgrade-check` + Rust guardrails proving the
      pack remains complete and lore-locked (loop 439).

- [x] The samurai cut (2026-09-11): the owner-facing UI layer and HUD quads
      drew as ONE TriangleList triangle — the bottom-right half of every
      menu/button/text field showed raw world along the screen diagonal.
      6-vertex quads + byte-exact quad-coverage proof law + sRGB view fix;
      p3d 592/592, gates 10/10, journey/smoke digests unchanged.

- [x] GLM UI rework (2026-09-09): the owner-facing UI is real — menus,
      HUD, settings, save slots, modals, screenshot harness
      (`--ui-shots`), JSON inspector (`--ui-inspect`); 10-gate battery.
      Deferred honestly: hotbar tools (6–9 reserved), damage-driven
      health, XP beyond first-build, audio, rebinding, save thumbnails
      (see docs/POORCRAFT-3D/GLM-UI-REWORK-PROOF.md).

- [x] M1 window opens and clear color (lf_engine)
- [x] M2 chunk data structure + culled meshing (lf_voxel) + texture array
- [x] M3 voxel raycast (DDA) with tests (not yet wired to input — P1)
- [x] M4 terrain noise heightmap + biomes + strata — 30 biomes today
      (the "all 8 biomes" wording was the M4-era count, stale until the
      audit; visual distinctness of the 30 is build-pack Steps 16–19)
- [x] M5 world persistence (region files hold many chunks, atomic writes, P0)
- [x] M6 day/night cycle math + light level constants (propagation is P3)
- [x] M7 survival data types (stats, inventory stacking)
- [x] M8 smithing data model (8 materials, tool assembly, forge minigame;
      audit fixed the forge UI minting one steel ingot per frame — strike
      is now a click and the forge resets after granting)
- [x] M9 mob data model (6 types incl. Null Knight boss)
- [x] M10 mod manifest + block/item data loading (ember_ores, amberium examples)
- [x] M11 protocol codec + UDP echo server binary
- [x] M12 villager schedules + trading (audit split: the Geode Guardian /
      Cinder Crawler mobs in lf_npc are dead data, never spawned — open
      item in AUDIT.md, spawn-or-cut)
- [x] M13 quest data types (objectives, quest log)
- [x] M14 chronicle events + saga/markdown generation
- [x] Depth-buffered renderer with shared GpuScene (P0)
- [x] Real offscreen headless renderer + scene harness (P0)
- [x] xtask vistest/screenshot commands producing real PNGs (P0)
- [x] real item icons + tooltips + recipe book + minimap/world map/waypoints (P22)
- [x] POORCRAFT 3D owner UI asset handoff: transparent generated logo/HUD/bar/
      key/action/resource/panel/faction sheets, implementation guide, JSON
      naming manifest, and a code guard proving the sheet files exist (loop 411)
- [x] POORCRAFT 3D GLM UI rework pack: drop-in prompt/spec/JSON folder for
      GLM 5.3/Z-code, including screenshot/playtest protocol, local
      MCP-style inspector spec, runtime data export authorization, failure
      baseline screenshot, implementation queue, and a code guard proving the
      pack is complete (loop 412)
- [x] POORCRAFT 3D GLM world/tools/semantic asset pack: drop-in prompt/spec/
      JSON folder for start-menu seed previews, gameplay-semantic assets,
      AMD/NVIDIA profiling rules, screenshot/wireframe/data extraction,
      asset brainstorms, and "no fake assets" laws with a code guard proving
      the pack is complete (loop 415)
- [x] POORCRAFT 3D WT-001 seed preview execution pack: concrete
      implementation-ready subfolder under GLM-WORLD-TOOLS-ASSET-PACK for
      the New World seed-preview harness, including GLM prompt, Rust API
      sketch, UI wireframe, deterministic preview algorithm, screenshot gates,
      Z-code commands, failure modes, checklist, JSON contracts, and a code
      guard proving the subpack files parse and exist (loop 416)
- [x] POORCRAFT 3D WT-002 semantic asset factory lab: concrete
      implementation-ready subfolder under GLM-WORLD-TOOLS-ASSET-PACK for
      high-volume playable asset generation, interaction anchors, inspector
      exports, wireframe/bounds screenshots, material/LOD/collision rules,
      semantic playtest routes, AMD/NVIDIA-informed capture gates, nine JSON
      contracts, and a code guard proving the subpack files parse and exist
      (loop 417)
- [x] POORCRAFT 3D WT-003 game observatory MCP lab: concrete
      implementation-ready subfolder under GLM-WORLD-TOOLS-ASSET-PACK for
      local MCP-style inspection, runtime state exports, screenshots,
      wireframes, overlays, mesh/material dumps, input replay, GPU markers,
      regression evidence bundles, ten JSON contracts, and a code guard proving
      the subpack files parse and exist (loop 418)
- [x] POORCRAFT 3D WT-004 feature expansion matrix: concrete
      implementation-ready subfolder under GLM-WORLD-TOOLS-ASSET-PACK for
      converting broad owner brainstorming into proofable slices across
      worldgen, NPCs, factions, settlements, industry, magic, survival, combat,
      exploration, UI/HUD, mods, multiplayer, Steam, tools, eleven JSON
      contracts, and a code guard proving the subpack files parse and exist
      (loop 419)
- [x] POORCRAFT 3D WT-005 asset prompt atlas: concrete implementation-ready
      subfolder under GLM-WORLD-TOOLS-ASSET-PACK for high-volume original asset
      prompts, UI sprite/key/bar contracts, glTF/Blender import rules,
      material/rig/texture contracts, batch rejection tests, nine JSON
      contracts, and a code guard proving the subpack files parse and exist
      (loop 420)
- [x] POORCRAFT 3D WT-006 Z-code continuous runbook: concrete
      implementation-ready subfolder under GLM-WORLD-TOOLS-ASSET-PACK for
      task selection, proof-first development, failure recovery, evidence
      bundles, sprint cards, completion audits, bookkeeping, commit/push
      discipline, eight JSON contracts, and a code guard proving the subpack
      files parse and exist (loop 421)
- [x] POORCRAFT 3D WT-007 GPU vendor cookbook: concrete implementation-ready
      subfolder under GLM-WORLD-TOOLS-ASSET-PACK for wgpu markers, AMD RGP/FSR
      gates, NVIDIA Nsight/DLSS gates, capture scenes, upscaler readiness,
      shader debug rules, before/after perf evidence, seven JSON contracts, and
      a code guard proving the subpack files parse and exist (loop 422)
- [x] POORCRAFT 3D WT-008 data extraction plugin lab: concrete
      implementation-ready subfolder under GLM-WORLD-TOOLS-ASSET-PACK for
      local-safe mods/plugins/exporters, loopback MCP-style inspector endpoints,
      scene/UI/asset/mesh/material/player/NPC/machine/worldgen/perf/evidence
      extraction, good/bad sample mod packs, telemetry traces, data safety
      rules, eight JSON contracts, and a code guard proving the subpack files
      parse and exist (loop 426)

## ui-world-craft pack (loop 328)

- [x] LOREFORGE title identity: logotype + tagline, vignette, left link
      column, version/seed display; palette unified across ui_kit
- [x] version-seeded preview world (in-memory, never saved) + 90s
      elliptical orbit camera with altitude oscillation
- [x] New World screen (name/seed/reroll/type/mode/difficulty) with real
      difficulty gameplay (spawn gating, damage, hunger)
- [x] Load World picker: seed-rendered cached thumbnails, world-type
      glyphs, metadata, delete confirmation
- [x] Multiplayer screen: direct connect, host world, lobby stub
- [x] two-layer terrain: flat lowlands vs ridged highlands (measured
      0.41 flat fraction across seeds) + ocean shelf
- [x] rivers: meandering lowland channels that actually hold water
- [x] caves: breach ramp, deep slate < y30, lava lakes < y10,
      stalactites/stalagmites
- [x] structures terrain-adapted (support fill, slope platforms,
      underwater refusal) — huts through faction camps
- [x] per-biome ground cover with densities + transition interleaving
      (5 new plant/lava blocks)
- [x] crafting workbench: three zones, batch craft, Add to Queue, earned
      recipe visibility (always-visible / era / first-pickup) with toasts

## P1 — First-person core
- [x] keyboard/mouse input (WASD, jump, sneak, sprint, mouse look, cursor lock;
      audit fix: sneak was captured but never read — now a 0.45x careful walk)
- [x] player AABB physics (gravity, collision, jump, fly; substepped anti-tunneling; 8 tests)
- [x] camera control (first-person from eye position; crosshair lands with P4 HUD)
- [x] block targeting outline via DDA raycast; break/place with player-overlap check
- [x] hotbar (1–6, scroll) with block placement; F2 in-game screenshots

## P2 — World streaming & terrain
- [x] chunk streaming: background generator thread, nearest-first, wish
      radius follows the view-distance setting (audit fix: was hard-wired
      to 5 so High preset never streamed farther)
- [x] worldgen features: trees (canopy in-chunk), caves (3D noise), coal/iron by depth, water at sea level
- [x] sphere-frustum + distance column culling using mesh bounds
- [x] save/load world (region chunks + player state) with autosave and save-on-exit
- [x] block registry: solid/transparent/targetable; water non-solid, raycast skips it

## P3 — Lighting & atmosphere
- [x] flood-fill sky + block light per column (BFS, opacity-aware; tests for falloff/overhangs/emitters)
- [x] torches/lanterns emit real light (14/15) and are placeable (audit
      fix: the lantern block existed with light 15 but had no item or
      recipe — unplaceable; now craftable iron-over-torch)
- [x] RGB block-light propagation: source colors attenuate and max-blend per
      channel across chunk borders; sky remains neutral and the packed vertex
      layout stays the same size (loop 348)
- [x] material light family: warm ordinary torch/lantern, hot Ember Torch,
      cyan Lumen Torch, broad Fireplace, amber lava/glowstone and green
      radiation; the three new sources have textures, recipes and drops
- [x] day/night cycle drives sky color, clear color, sky-light factor + distance fog
- [x] water transparency (alpha-blended pass, back-to-front column sort; underwater tint in P7)
- [x] smooth per-vertex lighting/AO (done by P26's visual identity pass;
      this line had been left unchecked and stale until the audit)
- [x] sun/moon/stars/clouds + weather-driven sky/fog (client atmosphere pass)

## P4 — Survival & inventory UI
- [x] egui HUD (crosshair, hearts, hunger, air bubbles, 9-slot hotbar, mining progress, clock)
- [x] inventory screen: click pick/place/swap/merge, right-click split; shift-click quick-move (P22)
- [x] crafting: the 3x3 grid was replaced by the ui-world-craft workbench
      (three zones, earned recipe visibility, batch craft) — the shaped
      matching engine stays in lf_game for mods/tests; the grid UI is gone
- [x] hold-to-mine with hardness, tool speed multipliers, harvest gating (iron needs stone pick), durability that breaks tools
- [x] item drop entities with gravity + magnet pickup + bobbing render
- [x] hunger drain, regen when fed, fall damage, drowning with air, death screen + respawn
- [x] eating (apple); inventory/stats/time saved with the world
- [ ] beds/spawn setting, crack overlay texture (P5/P7 polish)

## P5 — Content catalog
- [x] furnace with smelting state machine (raw iron->ingot, sand->glass; coal/log/planks fuel; ticks while closed) + UI
- [x] chest block entity (27 slots) + UI; contents spill on break
- [x] planks + glass blocks (glass renders in transparent pass)
- [x] iron tool tier (pick/axe/shovel) + wooden/stone/iron swords + all recipes
- [x] block entities persist with the world; catalog consistency test
- [x] armor (loop 329): full bronze/steel kit — helmet/leggings/boots items,
      icons, Bronze-era recipes; the inventory's four armor slots are
      honored (worn_armor_points sums 36..=39) and drawn in the workbench
      strip with a live readout. Beds/doors/signs, wool/decor variants still open.
- [ ] smithing table integration (with P6 combat loot)

## P6 — Mobs & combat
- [x] mob framework + AI (wander/chase/flee, 1-block hops) + spawning rules
      (day/night table, cap 12; the light-level gating is still open)
- [ ] grid A* pathfinding (mobs hop and beeline today)
- [x] combat: cooldown, knockback, armor mitigation, bow/arrows
- [x] XP levels; Null Knight boss data (arena/phases still open)
- [x] villagers wander by schedule + trading UI

## P7 — Structures, weather, sound, menus
- [x] structures: meadow huts (torch/crafting table/furnace), highlands watchtowers, desert pyramids — deterministic (+1 test)
- [x] title screen (Play/Quit) and pause menu with sensitivity/FOV settings
- [x] loop 329 menu pass: every panel centered (new world + multiplayer were
      top-left anchored), global kit theming for egui windows/widgets,
      Journal quest-log redesign, multiplayer screen developed, resize
      robustness pixel-proven at 640x420 / 800x600 / 1280x800
- [x] UI proof screenshots: hud_preview scene renders the real egui HUD offscreen
- [x] weather particles (rain/snow by biome) — sound lands in P17
- [x] world types superflat/amplified with title-screen selection
- [ ] key rebinding, PT-BR (P19)

## P8 — Quests & chronicle live
- [x] quest events from gameplay (collect/craft/kill) advance objectives with progress (+1 test)
- [x] 5-quest starter chain (timber -> planks -> tools -> iron age -> night hunter)
- [x] quest log UI (J) with objectives, progress and the chronicle
- [x] chronicle records live milestones, exports worlds/<name>/chronicle.md on save
      (audit note: 5 of 11 event types have no producer — GreatTrade,
      Discovery, StructureCompleted, VillageFounded, RuneApplied; and q4's
      "collect iron" can't fire from furnace output, only ground pickup)
- [x] quest/chronicle state persists with the world
- [ ] lore books readable in-game (deferred)

## P9 — Multiplayer
- [x] protocol v3: join/leave, 20/s position snapshots, validated block ops, chat (+4 codec tests)
- [x] authoritative-lite lf_server: canonical world + edit history, newcomer replay, chat relay
- [x] two-client local integration test over real UDP (chat + block sync + positions)
- [x] dedicated loreforge-server binary (bind + seed args)
- [x] client join from title screen, remote players rendered, remote edits applied, chat UI (T)
      (audit note: Welcome.seed is ignored — clients generate local-seed
      terrain, only edited blocks sync; connect is hardcoded localhost +
      name "smith"; see P28 + build-pack Steps 34–35)
- [ ] singleplayer routed through integrated server; mob/world sync; server browser (deferred)

## P10 — Mod API real
- [x] runtime registries: mod blocks (stable ids, solidity/opacity/drops), items, recipes, smelting
- [x] smelting.toml parsing fixed; worldgen ore hooks (auto *_ore veins)
- [x] client loads mods/ at boot — smoke log confirms ember_ores + amberium
- [x] mods/README.md; full-pipeline test (parse->register->place->break->smelt)
- [ ] custom mod textures (generic mod layer for now)

## P11 — Performance & release
- [x] light cache with edit invalidation (+test) — and it caught two real
      lighting bugs (unsampled light closure; section-local y offset)
- [x] cargo xtask package: portable dist/ zip (binaries + mods + docs)
- [x] CI release matrix (ubuntu/macOS/windows) uploading artifacts
- [x] honest RELEASE.md with run instructions, controls, features, gaps
- [ ] puffin profiling pass, greedy meshing (deferred; frame times fine at view 5)

## P25 — Correctness & honesty sweep
- [x] server SetBlock validates against the real registry (mod blocks >= 100
      accepted; the old `block <= 18` cap silently dropped every mod edit);
      dedicated server loads mods/ at boot like the client
- [x] `lf_steam/steam` feature actually compiles (steamworks 0.12 optional dep);
      STEAM.md corrected (CI still builds default-feature binaries only)
- [x] generator version stamped into saves (`genver.dat` per world; mismatch
      warns — revisited unedited chunks regenerate, edited chunks are safe)
- [x] lantern block got a real texture layer (was falling through to stone)
- [x] root Cargo.toml dependency table pruned to what is actually used (the
      old table listed 14 deps nothing referenced, with drifted versions)
- [x] mods/README `_ore` auto-registration claim is now real code in
      lf_modapi::apply_mod; misleading `tests/golden` stub removed
- [x] vistest PNGs are pixel-analyzed after rendering (non-uniform, multi-color
      check in lf_vistest::verify_render) — "it rendered" is enforced by code
- [x] the pixel gate immediately caught two real pathtracer bugs that had made
      every raytraced scene (and in-game Live RT) render one flat color since
      P18: the WGSL DDA initialized t_max with a signed numerator (negative
      for negative ray components), and the camera basis was scaled by a
      double `to_radians()` on the already-radians fovy (basis at ~1.4%,
      all rays parallel). Both fixed; RT proofs show real terrain again.

## P26 — Visual identity (per approved direction: hybrid-selective)
- [x] per-face materials: `tex_of(BlockState, Face)` — grass top/side/bottom
      correct on all six faces, log rings on every species' ends (atlas 48
      layers: +grass_top, +log_top, +crack_0..3)
- [x] alpha-cutout foliage: shader discards alpha < 0.5; all six leaf
      textures hole-punched per species (18-29% deterministic holes);
      see-through leaves with reliable depth writes (water/ice unaffected,
      glass pane becomes frame-only cutout)
- [x] foliage wind: GpuVertex sway weight + Env.time; vertex-shader wave
      phased by world position (stable across chunk borders); frozen when
      the particles setting is off (low quality tier)
- [x] smooth lighting: per-vertex AO (classic side/side/corner) + per-corner
      light averaging over the 4 touching cells (was flat per-face)
- [x] mining feedback on the target: stage 0..3 crack decal (inflated cutout
      cube) + debris particles (billboards, gravity, ground stop, cap 128);
      the subtle HUD progress bar stays for accessibility
- [x] mipmaps: 5-level CPU box-filtered chain per layer, mag-nearest /
      min-linear sampling (distance shimmer gone)
- [x] two new proofs: `foliage_canopy`, `mining_feedback` (22 scenes total)
- [ ] connected-surface projection on large man-made materials
      (stone/marble/planks) — the hybrid-selective art direction's second half

## P27 — Camera culling fix
- [x] objects-no-longer-disappear-when-looking-up: column frustum culling
      now uses the exact AABB bounding sphere (sqrt(128 + half_h^2) + sway
      margin) instead of an under-sized axis-only sphere, and the frustum
      planes are normalized (the near plane's raw normal is ~2x unit);
      regression test proves corner-inside => kept across pitches 5-85 deg
      with a pinned pre-fix failure case

## Deferred (P23 notes, honest)
- [ ] console `new`/`load` adopt connected-server seed in multiplayer
  (Welcome.seed now carries it; client terrain is still local-only).
- [ ] second-level autocomplete (command arguments) — first token only.
- [ ] save-slot thumbnails in the picker (renders a map preview per slot).
- [ ] Gamepad/mobile input; key remapping UI.

## Deferred (P22 notes, honest)
- [ ] migrating GameState::menu_reveal (f32 clock) onto ui_kit::Reveal — the
  clock + per-open reset already provides the same behavior; pure churn, kept
  for a rainy day. Reveal stays tested in ui_kit.
- [ ] minimap rotation / zoom controls; in-game waypoint beacon rendering
  (map + minimap pips exist, world-space beams do not).
- [ ] Windows exe runtime: host lacks mingw-w64 (macOS runner); macOS dmg +
  Linux tarball ship instead.

## P28 — V1REBRAND rendering & UX gate (docs/V1REBRAND/02, +2 numbering per DECISIONS)
- [x] execution plan written: docs/V1REBRAND/11-EXECUTION-PLAN.md (P28-P39
      map onto roadmap docs 02-10 with the +2 phase offset)
- [x] wind-sway honesty fix: P26 claimed foliage wind, but shader.wgsl
      vs_main never read the sway attribute (loc 6) or time_sway — leaves
      could not move. Shader now offsets sway-weighted vertices with a
      world-position-phased double sine (max ~0.08 blocks, inside the 0.1
      cull margin). Proof: foliage_sway_animates_between_frames renders the
      canopy at two wind phases through the real GPU pipeline and demands
      pixels differ (same-phase control must be pixel-identical).
- [x] clouds setting un-no-op'd: the toggle now actually clears/rebuilds
      the cloud batch (was rebuilt unconditionally)
- [x] unload radius tracks view distance: UNLOAD_RADIUS=8 const replaced by
      view_distance + UNLOAD_MARGIN(3); column_in_view takes the view
      distance (view 8 previously had zero unload headroom)
- [x] first-launch fix: Settings opened from the title screen now returns
      to the title (Back and Esc via close_settings) instead of dropping
      the player into the world
- [x] boot loads the booted slot's player extras (was: legacy worlds/default
      read before boot_slot(), so slotted players booted with default
      inventory/settings until Play was clicked)
- [ ] chunk-border lighting: cross-column flood (3x3 neighborhood) replacing
      "seams accepted"; night seam vistest + regression test
- [ ] transparency/sort audit documented in DECISIONS (water sort + particle
      rules ahead of Steam-age smoke/steam)
- [ ] frame-time target: xtask perf (p50/p95 ms at Medium) + DECISIONS entry
      (host iGPU is the "low" device)
- [ ] quality tiers: PathTraced preset (Low/Medium/High/Path-Traced)
- [ ] key rebinding (input.rs Action/Keymap + Controls tab + persistence)
- [ ] save-slot thumbnails
- [ ] minimap rotation/zoom + waypoint beacons
- [ ] UI language audit (on-kit quest log/book/console/trade/tech tree,
      HUD text shadows, Theme::MANA)
- [ ] connected-surface textures (stone/marble/planks; carried from P26)

## Build-pack Stage A — reality audit (docs/poorcraft-build-pack, Step 1-2)
- [x] Step 1: AUDIT.md written at repo root — every prior [x] claim
      re-verified (code + live session + captures); BACKLOG corrected in
      the same commit; stale lines fixed (M4 "8 biomes" -> 30, P3
      smooth-AO now checked, M8/M12/P2/P9 caveats added)
- [x] Step 1 fixes shipped with the audit: HUD hidden behind the title
      menu; title orbit camera clamped above ring terrain (was buried on
      hilly worlds -> flat-dark backdrop, repro tool in
      lf_worldgen/examples); render culling uses the render camera's eye;
      streamer wish radius follows view distance; sneak wired to a slow
      walk; smithing strike-per-click + grant-once reset; lantern
      craftable; random_seed() sequence counter; 201 fossil ev_*.png
      "proofs" removed (zero code references — Evolution-era residue)
- [ ] Step 2 remainder: audio engine + break/place sounds (pack Step 4);
      biome visual identity (pack Steps 16-19); [spawn-or-cut Geode
      Guardian / Cinder Crawler — DONE loop 447, spawned];
      q4 Collected from furnace output/trade;
      multiplayer Welcome.seed + address entry; chronicle dead event
      types; dawn/dusk light ramp; F2 re-render includes water/crack

## Fluids & block gravity (user request; P30 Steam-Age fluid groundwork landed early)
- [x] granular blocks fall: registry::has_gravity (sand/red_sand/snow/dirt/
      grass/moss/mycelium — ores deliberately excluded, MC rule); breaking
      support detaches the column into animated FallingBlock entities that
      land through the player edit path (remesh + MP broadcast); settle_
      gravity is the headless twin (tests + vistest)
- [x] water physics: level 0..7 flow states in BlockState flags; event-driven
      cellular sim (fall first, spread with decay, dry up when unsupported);
      flowing surfaces render lowered (stepped water) with step-covering
      side faces; 64-cell tick budget; oceans/lakes are sources
- [x] bucket + water_bucket (craftable from 3 iron; scoop a source /
      pour a source) — first player tool for the fluid system
- [x] vistest proofs: water_flow (aqueduct -> flume -> dam pooling,
      settled through the real sim before meshing) and falling_sand
      (collapsed pile + mid-air faller), 24 scenes total

## Goal-file Sections 0–4 (2026-08-26, loop 311)
- [x] S0 re-audit of the four flagged items (AUDIT.md updated): bottom
      mining bar CONFIRMED and removed; texture stretching NOT
      reproducible in the raster path (per-block quads tile by
      construction — mesh test + visual proof added, greedy-mesh
      precondition recorded); biome grade absence CONFIRMED and fixed;
      mod-load visibility CONFIRMED and fixed
- [x] S2: mining/bow progress = crosshair-centered radial ring
      (ui_kit::paint_mining_reticle + geometry test; hud_preview proof
      shows it mid-break; bottom-of-screen bars fully removed)
- [x] S3: per-biome color grade (shader grade uniform; warm/cool/lush/
      eerie/teal/neutral table; ~0.3s boundary lerp; clear-color mirror;
      GPU hue/sat proof test biome_grade_shifts_midframe_color)
- [x] S4: mods/smoke_test (1 block + 1 item) + [MOD SMOKE TEST] OK boot
      line (client + dedicated server) + CI test on the real folder +
      README pointer
- [x] S1: per-block texture tiling proven (mesh test
      multi_block_walls_tile_per_block_not_stretched + texture_tiling
      scene, AI-verified per-block repetition on a 7-wide wall and floor)
- [x] S5 spot: Live RT decision + greedy-mesh UV precondition in DECISIONS
- [x] STATUS.md rewritten to match verified reality (was stale: 121 tests/
      14 scenes/live-RT-deferred)

## Loop 312 — build-pack Step 3 remainder, Step 4 (audio), Step 7, Step 8, Step 9
- [x] Step 4 AUDIO: new lf_audio crate — procedural PCM one-shots (no
      asset files) per material category (wood/stone/metal/glass/soft),
      break + place variants, rodio playback with silent fallback when no
      output device, 30ms rate limit, driven by the persisted volume
      sliders; client plays on every real break/place; category dispatch
      unit-tested (block_categories_dispatch_correctly etc., 4 tests);
      CI ubuntu jobs install libasound2-dev
- [x] Step 3 remainder: impact pulse — heavy blocks with heavy tools kick
      a short decaying camera shake (break_shake/shake_decay/shake_offset,
      envelope unit-tested; jitters the look target only)
- [x] Step 7: FOV-to-projection verified against hand-computed reference
      values at fov 90 and 60 (projection_matches_reference_values_at_two_
      fovs in lf_engine) — the double-to_radians class is guarded on the
      raster path now
- [x] Step 8: transparency_layers proof scene — water pool behind a glass
      wall with particles on both sides; AI-verified layering (water
      through glass, near particles over the pane, far ones through it)
- [x] Step 9: persistent HeadlessRenderer refactor (device+atlas once —
      the first perf run measured setup, not frames), xtask perf + make
      perf, Medium(radius-5) numbers recorded in DECISIONS (target device
      = this host's iGPU); live >=30fps confirmation pending next play
      session's F3 reading

## Loop 313 — build-pack Steps 13, 14, 15 (settings completeness + picker + minimap)
- [x] Step 13 KEY REBINDING: new lf_client::input (Action x Keymap,
      defaults = the original hardcoded keys); window_event + movement read
      the keymap; Settings > Controls tab with click-then-press capture
      rows; persisted via Settings.keymap_pairs in ClientSave (serde
      default; junk input falls back to defaults). Digits 1-9 + Escape
      stay fixed by design. Persistence test:
      rebind_and_quality_tier_persist_through_client_save
- [x] Step 13 PATH-TRACED TIER: Quality::PathTraced added — Medium raster
      base + RtMode::Live; Low/Medium/High now explicitly set rt Off;
      active tier stored in Settings.quality and shown in the preset row
- [x] Step 14 THUMBNAILS: save_world captures a 256x144 live-view PNG to
      worlds/<slot>/thumb.png (throttled ~2 min so the 30s autosave stays
      cheap); the slot picker lazily loads and shows it beside
      name/type/seed/last-played
- [x] Step 14 FIRST-LAUNCH WALKTHROUGH: documented in DEVLOG (title ->
      Play/New/Load/Multiplayer/Settings, no dead ends; Settings-from-
      title returns to title — fixed in the loop-309 audit; legacy
      worlds/default pre-load removed loop 308)
- [x] Step 15 MINIMAP: rotate-with-view (custom rotated mesh + shared
      marker rotation + N chip rides the rim) + zoom 0.5-3x (Interface
      settings), both persisted
- [x] Step 15 BEACONS: per-color translucent atlas layers
      (waypoint_0..5) + world-space beams rebuilt per frame from the
      player's waypoints and drawn in the transparent pass; proof scene
      waypoint_beacons (AI-verified: three colored translucent beams,
      no artifacts)

## Loop 314 — build-pack Steps 16-19 (biome identity, spawns, weather)
- [x] Step 16 BIOME SURFACES: JUNGLE_GRASS (deep saturated) + SAVANNA_GRASS
      (dry gold) blocks (ids 42/43) with atlas layers; Jungle/Savanna/
      WindsweptSavanna now wear them; Swamp keeps MOSS; MushroomHollow now
      generates on previously-unused MYCELIUM; wildflower cutout plant
      (FLOWER, id 44) sparsely covers FlowerForest — breaking its twin
      with Forest. GENERATOR_VERSION -> 2 (unedited chunks regenerate).
- [x] Step 16 CONTACT SHEET: biome_contact_sheet scene — 30 strips paved
      with each biome's REAL surface+filler from the table; pixel check
      measures exactly 30 distinct quantized strip colors; AI-verified all
      palette families present with no identical groups
- [x] Step 17 EXCLUSIVITY: Tundra gets SpruceSparse conifers (vs dense
      SnowyTaiga); boulder fields are SnowySlope/WindsweptHills/
      WindsweptSavanna's exclusive feature; regression test
      biome_identity_markers_are_distinct enforces pairwise uniqueness
      (surface+filler+tree+structure+exclusive) with only two documented
      families exempt (depth-banded oceans incl. FrozenOcean; coastal
      StonyShore/Mountains) — the test caught two real twins during
      development and both were fixed
- [x] Step 18 SPAWNS: roll_spawn is biome-aware (cold biomes: woolbeasts
      only; temperate: boars + rare woolbeasts); night hostiles global;
      tested (day_spawns_are_biome_appropriate). Structures were already
      biome-gated (loop-309 audit)
- [x] Step 19 WEATHER: cold = the actual biome field (Biome::is_cold via
      the map's generator), not the old surface-block proxy; proofs:
      weather_snow (flakes over a snow field) + weather_dry (clear desert)
      join clouds_weather (rain)

## Loop 315 — P29 Water Age (V1REBRAND doc 04 / build-pack Step 23)
- [x] research prerequisite GRAPH: Era::Water branch (prereq Industrial,
      independent of Electrical — doc 03's either-order rule);
      ResearchState.branches (serde-default, pre-branch saves load);
      can_unlock/unlocked/unlock() with material costs (16 planks + 24
      stone + 4 iron — cheap/early per doc 04); tech-tree screen gains the
      branch card with a live Unlock button; 5 new tests incl. save-compat
- [x] machines: WaterWheel (12 EU/s while water touches it, river-gated,
      free — lowest tier below the coal generator's 20) + BatteryCell
      (4000 EU) + a PURE lf_game::machines::distribute_power (producers
      first, batteries cover gaps = blackout prevention, surplus recharges
      batteries in the 4-block field) — 3 tests
- [x] blocks: WATER_WHEEL (45) + BATTERY (46) through the full content
      pipeline (registry, atlas layers + procedural textures, items,
      Water-era-gated recipes, drops); machine UI panels (spin-up + charge
      bars); client power tick now runs every source through the pure step
- [x] RT palette: ids 42-46 hand-set + a stable hash fallback for all
      future ids so new blocks are never invisible/wrong in path tracing
- [x] proof: water_wheel_power scene (river carved, wheel + battery +
      crusher, the real power step spins the wheel for 30 sim-seconds) —
      AI-verified as a riverside power station

## Loop 316 — P30 Steam Age (V1REBRAND doc 04 / build-pack Step 24)
- [x] Era::Steam branch (prereq Industrial, either-order vs Water —
      tested; 12 iron + 4 gears + 16 coal); tech-tree branch cards now
      list Water AND Steam with live unlock buttons
- [x] machines: Pipe (1000 mB, equal-share between neighbors — no
      pressure sim per DECISIONS), Boiler (fuel via the existing
      fuel_seconds table + water -> steam, idle dissipates), SteamEngine
      (16 EU/s at full steam — wheel 12 < engine 16 < coal 20, asserted);
      4 tests incl. the full boiler->engine->machine chain through
      distribute_power
- [x] blocks PIPE(47)/BOILER(48)/STEAM_ENGINE(49) through the full
      pipeline (atlas layers, procedural textures — flywheel underflow
      caught by the atlas test and fixed, items, Steam-gated recipes,
      drops); client steam pass (pipe equalization, boiler feeds from
      adjacent sources like a pump + pipes, engines drink adjacent
      boilers); steam puffs rise from burning boilers (particles-gated);
      UI panels (pipe water level, boiler fire+steam+fuel, engine output)
- [x] proof: steam_chain scene — water -> pipes -> fueled boiler (pre-run
      through the real machine code) -> engine -> crusher with live puffs;
      AI-verified as a working boiler room (caught + fixed an infinite
      feed loop in the scene itself during rendering)

## Loop 317 — P28 chunk-border lighting + Step 20 lore books
- [x] P28/Step 6 CROSS-COLUMN LIGHTING: compute_column_light now floods
      sky+block light through a 3x3-column volume (48x256x48) and extracts
      the center slice — the P3 "seams accepted" decision is superseded
      (DECISIONS updated); edit invalidation widened to the full 3x3 column
      neighborhood (light travels 15 blocks). Regression:
      torch_light_crosses_chunk_borders (neighbor column gets 13 at the
      border). Proof: night_border_seam scene — torch straddling a border
      at night, measured max adjacent-column brightness step 1.92 (a seam
      would cliff >8). perf bench after the change: p50 47.7ms incl.
      readback+PNG at Medium radius-5 — no regression.
- [x] Step 20 LORE BOOKS: lore/books.toml with three real tomes (Tome of
      the First Forge / Tome of the Null / The River Wardens' Ledger —
      anchored to the existing Smith + Null + river lore threads);
      lf_client::lore loads them at boot; right-click a tome to page
      through an on-kit reader (prev/next, page x of y); tomes are
      Lorekeeper trades; pixel-art icons with a cover gem. Test:
      lore_books_load_from_the_real_file (3 tomes, pages >40 chars,
      item mapping, lore anchors present). Proof: lore_book scene reading
      the REAL file — AI-verified finished reader with readable story
      text.

## Loop 318 — P31 Oil Age + Step 25 power-grid overlay
- [x] OIL worldgen (id 50): biome-gated crude pools replace deep stone
      (y 8..44, desert/swamp only) + rare surface seeps (1/700 columns);
      regression test `oil_is_biome_gated_and_banded` scans 144 chunks
      and asserts every crude block sits in a desert/swamp column.
      Existing worlds keep their generated chunks (no GEN_VERSION marker
      exists in this codebase; oil appears in newly generated terrain —
      same policy as every prior ore addition).
- [x] Pipes v2: fluid typing (FluidKind Water/Crude on separate channels,
      serde-defaulted so existing pipe entities load unchanged); channels
      never mix (test). Boiler drinks Water; pump/refinery move Crude.
- [x] Crude oil fluid sim: step_cell is now fluid-generic — oil creeps
      only OIL_SPREAD=3 cells (sluggish vs water's 7), fall-first, and
      water/oil never convert each other (test). Sand sinks through oil.
- [x] PUMPJACK (51) / REFINERY (52) / COMBUSTION_GENERATOR (53): pump
      lifts 120 mB/s while powered + adjacent to an oil source, feeds
      neighbor pipes; refinery = crude 240 mB + power -> refined fuel +
      tar per 6s batch (exact mass-balance test); combustion burns
      refined_fuel only (45s/unit) at 26 EU/s — steam 16 < coal 20 <
      combustion 26 < nuclear (P32). Full-chain headless test through the
      real distribute_power.
- [x] Research: Era::Oil branch (bincode-appended, saves safe) gated
      Industrial AND (Steam OR Electrical) — meets_prereqs handles the
      either-or edge; cost = 4 refined fuel + 8 iron + 1 frame (you must
      have RUN the chain to earn it); pump/refinery gate at Industrial
      (extraction is iron-age kit), combustion generator at Oil. Tech
      tree shows the third branch card with the either-or hint.
- [x] Items/crafting: pump/refinery/combustion recipes; oil_bucket
      (scoop crude sources or pour; right-click a refinery to feed its
      tank +1000 mB), refined_fuel, tar (P34 construction fodder).
      Bucket arm handles all three buckets.
- [x] Client wiring: BlockEntity Pump/Refinery/Combustion; oil pass
      (refineries drink from adjacent pipes pre-power, pumpjacks lift
      post-power into adjacent pipes); machine UI panels (refinery with
      fuel/tar slots + pour button); spill-on-break for the new
      containers; keymap rebinding-aware.
- [x] Step 25 POWER-GRID OVERLAY: G toggles translucent tint cubes over
      every machine in the power field — green = granted >= 90% of draw,
      red = starved (same ratio rule as the client overlay). Rebuilt
      every 15 frames while on; rides the transparent pass with the
      waypoint beams. Rebindable Action::GridOverlay.
- [x] Proofs: oil_chain (pool -> pumpjack -> pipes -> refinery ->
      combustion -> powered furnace, 200 sim-seconds pre-run through the
      real machine code; AI-verified coherent chain with flare smoke)
      and grid_overlay (green cube on the powered furnace, red on a
      starved crusher out of range; AI-verified both cubes). The scene
      needed an honest bootstrap: one combustion generator (26 EU/s)
      cannot feed three 10 EU/s consumers — a coal generator runs the
      pumpjack while the oil chain spins up, exactly the balance the
      overlay exists to show (DECISIONS entry).
- [ ] Deferred: derrick silhouette reads small at distance (AI feedback)
      — texture polish for a later visual pass; GEN_VERSION-style save
      re-generation marker still doesn't exist in this codebase.

## Loop 319 — P32 Nuclear (the ceiling)
- [x] URANIUM_ORE (id 54) via the deep band: y 8..24, threshold 0.68
      (rare, tiny) in the standard ore pass; drops raw_uranium; smelts to
      uranium_ingot; assembler makes fuel_rod (2 ingots + 1 iron).
- [x] REACTOR (55): 32 EU/s — the top of the ladder (wheel 12 < steam 16
      < coal 20 < combustion 26 < reactor 32; tier test). Heat/output
      curve: fission +4 heat/s, full coolant -5/s (a cooled core holds
      equilibrium — caught by test, constant fixed from 3), passive -0.5/s.
      Auto-SCRAM at 80, unscram below 60, MELTDOWN at 100. Residual decay
      heat +0.8/s while scrammed WITH rods loaded: a scrammed core without
      coolant still melts (test proves the sequence scram -> meltdown).
      Coolant: 60 mB/s from adjacent pipes (water channel) or water blocks.
- [x] Meltdown in the world: apply_meltdown destroys the r=3 sphere,
      crusts up to 14 RADIATION (56) residue blocks through the crater
      (they glow — emission 7 — and damage anyone within ~3 blocks until
      scrubbed), blast debris + camera shake + a chronicle Meltdown event
      (new EventType, bincode-appended).
- [x] Research: Era::Nuclear branch (Nuclear-era gated reactor + fuel
      rod) requiring Oil AND the new reactor_safety certification
      (glass 8 + basic_circuit 2 + book 1, studied in the tech tree).
      reactor_safety is serde-defaulted — old saves load uncertified.
      DECISIONS: nuclear is the ceiling (Pillar 5) — nothing above it.
- [x] Client: nuclear pass (coolant from adjacent water/pipes, tick,
      meltdown applied live, venting steam particles while scrammed),
      reactor UI (heat/coolant/output bars, fuel-rod slot, SCRAM +
      restart buttons with the honest warning), radiation damage in
      survival_tick.
- [x] Proofs: reactor_control (uranium vein in a cut wall, water+pipe
      cooling line, reactor run to thermal equilibrium through the real
      tick code — asserted heat<30/buffer>1000 — furnace+crusher
      consumers; AI-verified core window + green-flecked vein) and
      meltdown_aftermath (crater + ~dozen glowing residue blocks +
      wrecked machines at dusk; AI-verified). Framing lesson recorded:
      scene builders place at world.surface_height but the framing
      convention uses gen.surface_top — new scenes must mirror the
      steam_chain pattern verbatim.
- [ ] Deferred: radiation suit/scrubbing tools; reactor neighbor
      destruction of other machines' block entities (entities in the
      crater are dropped today via generic spill-on-break only for the
      reactor itself).

## Loop 320 — P33 Magic foundation
- [x] MANA: lf_game::magic (MAX_MANA 30, regen 1.5/s) + PlayerStats.mana
      (persisted via ClientSave.mana). HUD bar in Theme::MANA violet under
      the XP bar — appears only once a spell is learned (magic is found,
      not innate). Tests: pool/cost gating, bounded set stability.
- [x] THE BOUNDED FOUR (doc 05): Firebolt (8 mana — a harder-hitting
      arrow with impact sparks), Gale-step (12 — gaze-ray blink up to 8
      blocks, wall-safe), Ward (20 — 5s of full damage absorption while
      the timer runs), Hearthlight (15 — softens ONE raw ore by hand via
      smelting::smelt_result and lights the targeted cell with a
      temporary lumen that burns out after 90s). 3 cast slots on
      Z/X/C (rebinding-aware, Action::Spell1..3), spellbook screen on B
      (on-kit slide panel: mana bar, slot cards, learned list,
      assign/clear).
- [x] SAVE MIGRATION (latent bug found + fixed): bincode EOFs on old
      bytes when fields are added — every past ClientSave field addition
      silently reset old worlds' extras on load. Extras are now JSON
      (serde defaults apply), with a frozen LegacyClientSave bincode
      shape migrating pre-magic worlds. Tests prove the legacy bytes
      fail the current struct (the legacy path is load-bearing), that
      older JSON tolerates missing new fields, and that the spellbook +
      runed tools persist.
- [x] WIZARD: VillagerJob::Wizard (Ysolde, max 2 per world) settles
      towers (spawn marker = the enchanting table), sells all four
      scrolls + reagents. Wizard towers worldgen: 5x5 stone shell, 9
      tall, spiral stair, torch-lit enchanting table on top —
      FlowerForest (1/53) / Highlands (1/97); unit test scans 400 chunks
      (seed 42 -> 3 towers, biome-gated). Scrolls: right-click to learn
      (chronicle Discovery; auto-assign to a free slot).
- [x] ENCHANTING: ENCHANTING_TABLE (57, craftable + towers) opens the
      imbue minigame (ImbueMinigame mirrors ForgeMinigame: channel
      55..75 band, 3 pulses bind, reset guards per-frame minting).
      Runes (rune_of_haste x1.3 mining while held, rune_of_warding +2
      armor while held) bind to the HELD tool (runed_tools map,
      persisted; CustomTool.rune fill + RuneApplied chronicle event —
      the pre-cut hook is proven by test).
- [x] CROSSOVER ITEMS (doc 05): LUMEN_BLOCK (58 — fuelless light-15,
      crafted from glitch dust + glass + torch; also Hearthlight's
      temporary form) and WARDING_PYLON (59 — hostile mobs refuse to
      spawn within ~3 blocks; crafted around a null_shard core). Magic
      that plays along with the machines, not instead of them.
- [x] Proofs: wizard_tower (AI-verified tower + table + torches at
      dusk), spellbook (AI-verified finished screen: title, mana bar,
      three Z/X/C slot cards, four learned spells), spell_effects
      (AI-verified: lumen glow, enchanting table, orange firebolt arc,
      pale ward ring — pixel-checked; dusk dimming caught by scan
      thresholds, scene moved to 0.62 golden hour).
- [ ] Deferred to P33b+: more runes (the enum is the extension point),
      wizard quest hooks (trades teach today), spell targeting beyond
      the crosshair cell.

## Loop 321 — P34 Construction
- [x] SHAPE SYSTEM: Shape (Cube/SlabBottom/SlabTop/Stair x4) in
      BlockState's high flag nibble (bits 28..31 — fluid levels keep the
      low nibble; shape 0 = plain cube so every old save is untouched).
      mesh_section emits shaped geometry on its own path (1-2 boxes,
      exactly the exterior faces, no coincident interior quads, culled
      against opaque full cubes, AO/smoothed-light blended); the plain
      cube path is untouched. Physics: intersects_solid resolves the
      player AABB against registry::collision_boxes (slab = half plane,
      stair = slab + back box). Tests: meshing (half-box top, stair =
      11 exterior faces, winding outward, culling), physics (slab at
      half height, stair open/rise halves).
- [x] SHAPED PLACEMENT: stone/planks slabs + stone stairs items with
      shaped_placement() (stairs orient by yaw, tested across quadrants);
      placing a slab onto a matching bottom slab merges into a full cube
      (slab_merge, tested).
- [x] SCAFFOLDING (60): climbable (hold jump to rise, sneak to descend
      — physics hook), breaking one removes the connected column above
      and refunds every block.
- [x] SYMMETRY (V): a mirror plane at the player's x; place AND break
      mirror across it; the plane renders as a translucent wall (overlay
      batch). Rebindable Action::Symmetry.
- [x] BLUEPRINTS: two-corner capture (16^3 clamp) -> bincode file under
      worlds/<slot>/blueprints/; holding the blueprint shows the ghost
      (translucent cubes where it would paste, capped at 600); paste
      places into air cells only and consumes the exact per-block bill
      (drop-table-derived). lf_game::construction with capture/file/bill
      tests.
- [x] STATUE CARVING (61): chisel + CarveMinigame (detail 65..85 band,
      3 taps, per-frame-mint reset — mirroring forge/imbue); a completed
      carve turns the targeted stone into a Chiseled Statue; chronicle
      Discovery event.
- [x] DECORATION REGISTRY v2 / MODAPI LIGHT: BlockDef.light was parsed
      and dropped — it now flows through ModBlockDef.light into
      emission() (mod blocks emit their declared light). New
      mods/decor_pack example (glowing banner light 12, plinth, rug) +
      test loading the real folder and asserting the emission.
- [x] Proof: build_tools scene (slab staircase, oriented stairs,
      scaffold tower, statue, green ghost cubes) — AI-verified all four
      construction elements; pixel art for every new item (icons test).
- [ ] Deferred: slopes/arbitrary-corner shapes beyond stairs (the
      Shape enum is the extension point), decoration texture overrides
      (decor blocks currently use the mod texture slot), blueprint
      rotation on paste.

## Loop 322 — P35 Smart building
- [x] CONDUITS (62): power-field relays — distribute_power_relayed runs
      the same three-phase field but reachability hops through conduit
      chains (BFS, <=4 hops of POWER_RANGE each; unified field + relays
      per DECISIONS). Tests: 10 blocks bridged by two conduits, broken
      chains and the 4-hop cap rejected, the relaying distribute test.
- [x] ELEVATOR (63): powered-by-field vertical ride — jump on a platform
      launches physics-exactly to the next platform up (velocity from
      the height), sneak descends. next_elevator_y shaft tests.
- [x] CLIMATE UNIT (64): a unit with a producer in its range within 4
      blocks of the player regenerates health on a cadence (climate_
      comfort tests: unpowered does nothing, producer near the unit
      comforts, too far doesn't).
- [x] COMPUTER SCREEN (65): the dynamic texture path — SceneResources::
      write_atlas_layer rewrites one atlas layer (mips regenerated) at
      runtime; the screen block shows live data as a styled 16x16
      readout (page 1 research pips, page 2 chronicle rows, page 3 the
      green/red grid split) rewritten only when a data signature changes
      (hash-gated upload). Right-click cycles pages. compose_screen_face
      unit test reads the pixels back.
- [x] Proof: modern_wing ("one wing wired for electricity") — glass
      wall, slab mezzanine, the generator feeding upper machines ONLY
      through the conduit chain (asserted in-scene), elevator shaft,
      climate unit, computer; AI-verified all five elements.
- [ ] Deferred: screen text glyphs (the readout is styled pips/bars),
      elevator door animation, conduit visual connection stretching.

## Loop 323 — P36 Dragons
- [x] FLIGHT AI (lf_game::dragons): circle/swoop/perch state machine —
      Circling holds the 14-block ring at roost+8, close players (<20)
      provoke a SWOOP_TIME dive that re-provokes while the threat
      lingers and releases when they retreat; Perched breathes fire on a
      3s period at <7 blocks and launches back into the ring when
      cornered. 3 AI tests pin all of this.
- [x] MULTI-PART RENDERING: dragon_parts(t, yaw) — body, head (forward,
      bobbing), two wings (sine flap), three tail segments (sway) — one
      shared layout fn used by BOTH the client entity batch and the
      vistest proofs (the proof shows the real assembly). Parts test
      asserts flap amplitude, head-lead, tail-trail, yaw rotation.
- [x] FIRE BREATH: perched dragons damage the player in range with ember
      particles streaming from the mouth; breath gated by the AI test.
- [x] ROOSTS: stone crag + egg clutch (DRAGON_EGG 66, ember-cracked
      texture) in Mountains (1/89) / SnowyPeaks (1/101); 400-chunk
      gating test (seed 99 -> 2 clutches). try_settle_dragons: one
      dragon per roost (marker = egg), max two alive, Discovery
      chronicle on settling.
- [x] BOSS: MobType::Dragon (400 HP, 18 dmg, size 2.2 — above the Null
      Knight), drops dragon_scale + iron, BossSlain saga event on melee
      AND projectile kills ("the dragon of the peaks falls — the saga
      turns a page").
- [x] MOUNT (user-approved spike, DECISIONS entry): the flight x
      streaming audit held (ring 14 << view+3 margins; 8 part-cubes is
      mesh noise) — bare-hand right-click bonds the ride, the rider
      tracks the dragon each tick, sneak dismounts.
- [x] Proofs: dragon_roost (AI-verified: crag + cracked eggs + the full
      multi-part assembly readable as a dragon) and dragon_flight
      (pixel-verified: 8116 body-red px + 228 white-hot breath px on
      the shared-assembly mid-flap pose).
- [ ] Deferred: dragon roost loot chests, wing-tilt banking in the
      layout, breath setting blocks alight.

## Loop 324 — P37 Paths & specialization
- [x] PATHS (lf_game::paths): Engineer/Architect/Battlemage/Artisan
      standings on ClientSave (serde-defaulted, JSON extras) — no decay,
      no lock-in. Accrual events: MachineRan->Engineer (cadence-sampled
      in the power loop), BlockPlaced->Architect (every placement),
      SpellCast/BossSlain->Battlemage, ItemCrafted/ItemEnchanted->
      Artisan; tier crossings (25/step) write chronicle milestones.
      Tests: event->path mapping + weights, tier crossing, respec.
- [x] GATE GENERALIZATION + CRAFT/PLACE ENFORCEMENT: Gate::Era|Path|
      Open with passes() over research.unlocked (fixing the REAL
      branch-era bench bug — boilers/pipes were uncraftable because the
      grid compared against the MAINLINE era) + path standing. Enforced
      at the craft grid (locked veil + gate label) AND at placement
      (refuse + hint; it was UI-only before). Ornate tier: precision_
      gear / master_blueprint / battlestaff / master_chisel (recipes +
      icons) gated at 25 path standing.
- [x] RESPEC: pay 8 iron + 1 null_shard, standings reset, the focused
      path accrues double (tested). Paths screen on P: four cards with
      standing bars, tiers, focus buttons, respec note.
- [x] PROTOCOL v4 TRADING: PROTOCOL_VERSION 4; TradeOffer/Accept/Cancel
      client messages + TradeOffered/TradeResolved server messages
      (bincode round-trip test). Server escrow: offers registered +
      validated (target online), accept delivers items to BOTH sides,
      cancel/decline frees both. REAL-UDP test in lf_server (two
      sockets: offer->receive, accept->both deliveries, cancel->both
      freed). Client applies TradeResolved to the inventory and shows
      offers as hints.
- [x] Proofs: paths_screen (AI-verified: four cards, bars, tier text,
      violet focused Battlemage, respec note) and trade_p2p (the
      escrowed offer panel, rendered + verified).
- [ ] Deferred: client-side trade-offer SEND UI (receive/apply is wired;
  server + protocol + tests fully cover the trading deliverable),
  ornate-item gameplay effects beyond the tier-3 tools.

## Loop 325 — Finish line: Steps 34-39
- [x] STEPS 34-36 (lobbies/P2P/invites): lf_steam::lobbies — a
      transport-neutral lobby model (create/join/membership churn/leave)
      where UDP lobby codes ARE the host address, plus the full invite
      flow (mint/receive/accept/decline). 3 tests. The Steamworks-armed
      mapping stays behind the `steam` feature (off by default; UDP
      fallback unchanged), per the existing DECISIONS entry.
- [x] STEP 37 (Workshop UGC): lf_steam::workshop — WorkshopItem +
      scan_installed(): UGC folders with mod.toml load identically to
      bundled mods (Steam subscriptions land in the same shape); test
      scans a temp dir, ignores non-mods, tolerates a missing dir.
- [x] STEP 38 (mods/README rewrite): full authoring guide — quick-start
      scaffold, manifest reference, blocks (with the light field that
      now truly emits), items/smelting, decoration packs, UGC/Workshop
      install, multiplayer determinism, gate interactions, testing.
- [x] STEP 39 (xtask new-mod): `cargo run -p xtask -- new-mod <id>
      [--name]` scaffolds manifest + example block/item, refuses
      overwrites; `make new-mod id=... name=...` added. Verified live
      (scaffold + duplicate refusal) + the scaffold shape parses and
      registers through the real loader (lf_modapi test).
- [ ] Deferred: client title-screen lobby UI wiring (the model + UDP
      codes are done; Steam feature-on arms unverified without the SDK).

## Loop 326 — P28 leftovers + Step 40 (the honesty pass)
- [x] STEP 11 (connected surfaces): stone + planks faces against the
      SAME block sample edgeless variants (atlas 85->86 with stone_conn/
      planks_conn; the mesher picks per-face from the live neighbor;
      contract test).
- [x] STEP 12 (HUD legibility): text_shadowed helper (hard shadow) on
      the air gauge and the new chronicle toast; the on-kit audit
      conclusion: quest log / book / console / trade / tech tree / map /
      spellbook / imbue / carve / paths all use the ui_kit slide-panel
      system (built P22-P37 on the same kit).
- [x] STEPS 21-22 (chronicle + lore surfacing): chronicle_event now
      toasts milestones across the HUD while playing (fading, 4s), and
      the cross-system anchor test proves the Smith / Null / river
      threads span books + tome items + the Lorekeeper's trades.
- [x] STEP 27 (item belt backbone): BELT block + recipe; belts hold a
      stack and push one item per 1.5s into the first adjacent machine
      input that accepts it (furnace/e-furnace/crusher/assembler A+B/
      boiler fuel) — pure belt_push tested, client pass wired, stacks
      spill on break.

## Step 40 — the final honesty pass
The full evidence trail, restated plainly:
- 256 tests green across the workspace; 47/47 vistest proof scenes
  (every gameplay system has a rendered, pixel- or AI-verified proof).
- Smoke: the release binary boots and stays alive (12s) every loop.
- KNOWN HONEST LIMITS (also in STATUS.md): the Steamworks `steam`
  feature arms are written but unverified without the Steam SDK (UDP is
  the default + tested transport; the lobby model is transport-neutral
  and tested); the client-side trade-offer SEND UI is unwired (receive/
  apply is; protocol + server escrow + real-UDP test fully cover the
  trading deliverable); dragon roosts/loot chests, breath ignition, and
  blueprint rotation are deferred; connected textures cover exactly
  stone + planks; the perf target is met on this iGPU host only.

## lore-and-visuals build (2026-08-27, loop 327)

Done (verified — tests + vistest PNGs listed in DEVLOG):

- [x] A1 lore data layer: lf_lore + lore/*.toml (factions, world events,
      NPC roster, dialogue); standing in ClientSave; Nameless start −50
- [x] A2 faction territory tint on minimap + world map (30% blend,
      height shading still reads); unclaimed biomes untinted
- [x] A3/C4 faction standing HUD widget (name, symbol, colored bar,
      standing number, pulse on change via faction_pulse) — bottom-right
- [x] A4 twelve faction quests load, fire their objective types (incl.
      new Break/Place/Interact/Reach-tag/any-food events), completing
      moves standing (+15 issuer, documented ripples)
- [x] B1 companion model + serde round-trip (trust/morale/wage/state/
      tasks/cargo all persist in ClientSave)
- [x] B2 hire flow: standing ≥75 gate, fee deduction, villager→companion
      transition, chronicle entry; 4th hire refused with the doc line
- [x] B3 command menu on interact (follow/stay/rest/mine/chop/haul/
      guard/pay-now/dismiss) + trust/morale readout; low morale refuses
      work ("I need rest.")
- [x] B4 follow AI: 2-4 block standoff (never clings), defends against
      the player's attacker, working tasks break real blocks into cargo,
      contextual dialogue lines in chat
- [x] B5 morale-zero quit (chronicle + faction −5 + trust memory −15);
      unpaid wages −10 morale/day with warning; pay-now +2 trust
- [x] C1 38 new blocks with distinct non-stretched textures (contact
      sheet vistest_faction_blocks.png); catalog/recipe tests green
- [x] C2 6 villager faction skins (+2 named NPC skins), 6 companion
      skins + trust-badge variants at ≥50, 6 mob skins with distinct
      silhouettes, 9 biome-tint variants (vistest_entity_skins.png)
- [x] C3 six faction structures in home biomes (determinism + biome
      gating tested) with banner markers settling faction NPCs
      (vistest_<structure>.png x6, NPC cube in frame)
- [x] C4 companion HUD tiles; ember particles (vistest_ember_glow.png);
      AO verified present (mesher+shader); biome grade verified per
      biome incl. Volcanic (automated vistest grade test)
- [x] D1 standing-driven NPC behavior: ≤−30 refuses trade + hostile
      dialogue line; ≥+50 friendly pricing (10% discount)
- [x] D2 chronicle integration: standing titles on threshold crossings,
      companion hired/dismissed/quit, quest completions, structure
      discoveries — with world-event references by name + Era/Year
- [x] D3 map structure icons (faction-color diamonds) + territory tint
      re-verified with structures placed (vistest_faction_map.png)

Deferred (honest notes):

- [ ] The Unmarked's 5-choice dialogue interview (nameless_q2 completes
      via interaction; the variable-outcome interview tree is future
      dialogue work — the quest itself is playable)
- [ ] Ashen library's "readable lore book" is the chest + existing tome
      system; no library-exclusive book text written yet
- [ ] Companion Craft command is stubbed in the menu (recipes they know
      exist in the roster data; autonomous crafting is future work)
- [ ] Haul moves cargo to the companion's cargo-clearing behavior; chest
      targeting is simplified (nearest-chest pathing not implemented)
- [ ] Nameless camp chest loot is spawn-table based (raiders drop
      torn_archive_page); the chest itself initializes empty
- [ ] Named-NPC uniqueness ("one per world, largest camp") is
      first-settled-wins, not largest-camp-search

## Loop 330 — Phase A timber (master fix plan)

- [x] Valheim-style tree felling: `lf_game::timber` (find_tree / fall_plan /
  tree_parts, all pure + tested), client FallingTree entity with rigid
  rotated-cube animation around the stump hinge, landing as horizontal log
  blocks (ids 111-120, X/Z per species) with directional mesher faces so
  ring ends face along the log, canopy shatter + TreeCreak/TreeCrash
  sounds + camera shake. Proofs: tree_fall_mid (seeded angles +
  GPU animation-diff test), tree_fall_landed, falling_blocks_deep.
- [x] Deep falling-block animation: per-faller tumble (deterministic
  fibonacci-hashed axis), one 0.18-restitution bounce with dust, scalar
  physics untouched; perf gate p50 116.8ms vs 111 baseline (noise).
- [x] Fixed en route: birch/spruce/dark/cherry logs had no items (breaking
  them dropped stone) — four species log items + planks recipes now exist.
- [ ] Deferred (honest): remote clients see the fell result (block edits)
  but not the fall animation (the breaking client runs the entity);
  horizontal logs from player placement orient by face but there is no
  axe/stripping variant; giant-spruce falls render up to ~70 cubes (still
  noise vs chunk meshes, noted in DEVLOG).

## Loop 329 deferred (honest)
- [ ] beds/spawn setting, doors/signs, wool decor (P5 leftovers, untouched)
- [ ] music/ambient audio: the Music volume slider still drives nothing
  (the loop-329 Sfx set covers ui/body/movement feedback only)
- [ ] vistest UI proofs for the title-flow screens render kit-driven
  replicas (real layout helper + real ItemIcons), not the literal
  GameState::draw_* screens — pixel-testing the real screens needs a
  windowless GameState constructor (GameState::new requires a winit
  window + surface today)
- [ ] multiplayer connect still hardcodes the player name "smith"
- [ ] armor has no per-slot equip restrictions (any piece in any armor slot)

## Loop 332 — ai-npc-assets (Sections A-G, docs/ai-npc-assets/)

- [x] A black-square artifact: root causes addressed — the compositor
  alpha fix shipped in loop 331 (CompositeAlphaMode::Opaque), plus Live-RT
  invalidation on world transitions (stale voxel clip + stale egui image
  covered the viewport after load_world) and empty-column-batch guards at
  all three upload sites. `no_black_square` scene + pure-black run-length
  assertion on 8 daytime gameplay scenes.
- [x] B mob AI: `MobBehaviourState` machine (Idle/Wander/Chase/Attack/
  Flee/Investigate/Disengage, all 11 transitions), DDA line-of-sight
  (cached per tick, 32-block cap), faction standing modulating aggro
  radius (`effective_aggro_radius`, +100 = ignore unless attacked), group
  aggro (first-order neighbours, 0.5s reaction, ≤5 pack, no chains), A*
  pathfinding in `lf_game::mob_pathfind` (cardinal + 1-up jumps, 256-node
  cap, cached 2s / goal-drift invalidation, direct-steer fallback).
  Client wired: standing lookup + group propagation per frame.
- [x] C NPC behaviour: enriched 5-slot day (sleep/eat/work/socialize/
  return) with locations, `NpcActivityState` driving movement + render
  pose + dialogue posture (sleeping NPCs refuse trade), reaction lines
  (structure damage, combat panic, gifts, companion quit, +75 ack), NPC
  memory (last two interactions, 5-day window, greeting references)
  persisted via the ClientSave villager JSON. Gift = use-item-on-villager.
- [x] D testing: vistest scenes `mob_ai_visible` (real 120-tick mob sim,
  world-state assert), `npc_schedule_time` (midday = Work slot);
  `--smoke` headless flag (300 ticks: worldgen seed 42 superflat, 1
  passive + 1 hostile mob AI, NPC schedule, planks craft, block mine —
  exit-code + log-pattern checked); `make smoke` now runs the logic
  smoke AND the 12s GUI liveness check.
- [x] E connected textures: neighbour bitmask (corner rule), derived
  47-tile CTM table (const-evaluated, bijective, 0xFF→0 / 0x00→46), strip
  art generated per block with exposed-edge shading + interior dapple,
  second texture binding (192×512 strip atlas) + shader branch on CTM
  markers, mesher computes the bitmask (with diagonal neighbour sections)
  and bakes per-tile UVs. All 8 E5 blocks. Tests: `connected_texture_
  uv_3x3` (centre = interior tile rect, isolated = tile 46 rect) + table
  bijection test + vistest scene `connected_textures_grass_3x3`.
- [x] F asset generator: `xtask gen-texture` (grass-ctm-strip /
  stone-ctm-strip / entity-skin / block-noise, seeded xorshift64 +
  integer hash noise), `gen-ctm <block>`, `gen-all-textures` (skip
  existing). Deterministic: `asset_generator_grass_output` pins seed-42
  bit-identity; no pure black/white in output.
- [x] G wrap-up: full suite + vistest + smoke green; runtimes rebuilt.

Deferred (honest notes):

- [ ] E uses a single 192×512 strip TEXTURE + shader branch instead of the
  reference doc's per-block separate files (assets/ctm/*.png are exports
  for human review, not runtime-loaded) — the runtime has no PNG loader;
  the export/import split is documented in DEVLOG.
- [ ] CTM applies to top faces only (E5 grass-side stays dirt-side by
  spec); side/bottom faces of accord_stone walls are not connected.
- [ ] NPC schedule is the canonical table in lf_npc (+ per-slot client
  resolution); per-archetype TOML schedule overrides in lore/npcs.toml
  are not parsed yet (the TOML path exists for dialogue/quests only).
- [ ] "Hostile faction NPCs join the fight" (C3) is not implemented —
  there is no hostile-faction villager roster to react; only flee/panic.
- [ ] Gift flow consumes from the hotbar slot via right-click; there is
  no dropped-item-pickup path for NPCs.
- [ ] The honored "+75 acknowledgement" flag is session-state (lost on
  save/reload); memory itself persists.
- [ ] gen-all-textures covers the 8 strips + 6 skins; block-noise has no
  placeholder-block registry to drive batch generation (on-demand only).
- [ ] NullKnight keeps the generic behaviour machine (freezing it with no
  boss AI would be a regression); `use_boss_ai` gates dragons only.

## Loop 334 — king-quest mega-loop

- [x] 50 community mods (88 blocks / 79 items / 10 smelts; ores reach
  worldgen, lights reach the light engine, tools carry damage/durability)
  with a load-all contract test.
- [x] 15 new biomes with 18 new blocks, 9 new tree species, per-biome
  ground cover, climate-grid classification, extended structure gates,
  and the 46-strip contact sheet.
- [x] 4 animals (chicken, wolf, dog, bear): multi-part cube rendering,
  skins, spawn rules, combat behaviour.
- [x] The Accord Bastion walled city + frontier watchtowers + desert
  ruins, biome-gated by seeded hash (a biome may or may not carry one).
- [x] The Vassal system: recruit at Honored standing, daily deterministic
  yields, collect from the vassal, persistent on the villager save.
- [x] Steam honest pass: Workshop UGC dir loads in client + server;
  `lf_steam --features steam` compiles.

Deferred (honest notes):

- [ ] Steam P2P transport, overlay and achievements: no Steam client,
  SDK runtime or real AppID on this host (dev AppID 480). The transport
  selector compiles but is not exercised end-to-end.
- [ ] Multi-chunk city sprawl: the Accord Bastion is a single-chunk
  walled town (the structure system is per-chunk); a sprawling capital
  needs a cross-chunk placement system.
- [x] Unique block art per mod pack — closed in loop 335 with one
  deterministic, palette-ruled atlas layer per namespaced mod block.
- [ ] More tree shape variants per biome (9 new species shipped; one
  shape each) and vassal loyalty/wage mechanics (flat deterministic
  yields today).
- [ ] Villager TOML schedule overrides per archetype (canonical enriched
  schedule is the single default table).

## Loop 335 — asset-gap closure

- [x] Unique generated 16x16 atlas art per mod block (100 blocks),
  deterministic per namespaced id, palette-ruled (3+ colors, no pure
  black/white), pairwise-distinct (tested), one atlas layer per block.
- [x] 7 ring-top layers for the new tree species (per-face routing) and
  12 packs gained a signature block (mod blocks 88 -> 100).
- [x] Fixed the loop-B atlas drift (hand-counted layer constants were
  +4 off; all king-quest layers now derive from layer_of(name)) and
  raised max_texture_array_layers to 512 (the atlas is 294 deep).
- [x] Asset ledger: 320 new discrete assets across loops 334-335 — the
  300 target is cleared.
- [ ] Steam P2P/overlay/achievements remain BLOCKED on having a Steam
  client, an SDK runtime and a real AppID (documented in loop 334).

## Loop 338 — authored-depth assets (normal maps, people, world items)

- [x] Generated tangent-space RGB normal maps for every base, mod, CTM,
  entity and item atlas layer; raster lighting reads the linear map for
  cheap material relief while Live RT remains optional.
- [x] Seven villager-job outfits + neutral network-player skin; villagers,
  companions and remote players render with shared six-part articulated
  humanoids (yaw, gait, crouch), not block-textured cubes.
- [x] Non-block drops render their real inventory sprites on crossed,
  double-sided alpha-cutout cards; block drops remain recognizable cubes.
- [x] Fixed CTM marker collision (165+ overlapped real atlas layers; markers
  now live at 4096+) and entity `push_cube` position being ignored.
- [x] `entity_skins` close proof: eight articulated characters + eight item
  silhouettes; 83/83 vistest, smoke, and p50 50.2ms / p95 50.6ms perf green.
- [ ] Continuation is staged in `docs/ASSET-RENDERING-PLAN.md`: per-part
  character UVs/attachments, held hero-item meshes + distance LOD, authored
  material channels, cheap projected/contact shadows, and quality budgets.

## Loop 335b — Steam exercised live

- [x] Steamworks end-to-end exercised on this host with the real client:
  init, Steam ID, stats request, matchmaking lobby create/leave, and
  live transport selection (preferred_transport() -> SteamP2p). Probe:
  `cargo run -p lf_steam --features steam --example steam_probe`.
- [x] Client boot logs the selected transport + feature wiring.
- [ ] Overlay activation: needs launching the game THROUGH the Steam
  client (works for non-Steam games too) — user-side step, not code.
- [ ] Game achievements/leaderboards: need a real partner AppID
  (current dev AppID is Valve's 480/Spacewar).
- [ ] ISteamNetworkingSockets as the in-game multiplayer transport
  (replacing UDP): the binding/init/selection are proven; only the
  socket swap remains.

## Loop 336 — P2P transport implemented; live two-process exercise blocked externally

- [x] ISteamNetworkingSockets transport in lf_steam::net_steam (SteamHost
  lobby+listen+accept+decode; SteamClientNet lobby-join discovery +
  connect_direct; protocol-v4 codec bytes unchanged; UDP untouched).
- [x] Probes steam_host / steam_client implementing the exchange.
- [ ] Two-process live exchange: BLOCKED — Steam refuses self-connections
  (both processes here share one logged-in identity), and the
  gameserver-identity workaround hits a steamworks-rs 0.12 pipe
  limitation. Finishes with two distinct Steam identities (second
  account / partner AppID + second machine), one command, no code change.
- [ ] Achievements: partner AppID. Overlay: launch-through-Steam.

## Loop 337b — roadmap + Steam loopback test SHIPPED

- [x] docs/ROADMAP-100.md: the researched 100-step plan (10 phases).
- [x] ISteamNetworkingSockets loopback pair shim (raw CreateSocketPair
  via steamworks-sys) + `steam_pair_test` example — protocol-v4
  Hello/Welcome exchange PASSES in-process, no second account.
- [x] Fixed: create_local_pair client lifetime (drop = SteamAPI_Shutdown
  = segfault); client handle now returned with the pair.
- [ ] Two-process cross-session P2P: needs two distinct Steam
  identities (user resource). Roadmap step 87 documents the recipe.

## Loop 339 — mob animation overhaul SHIPPED

- [x] Articulated walking: chicken/wolf/dog/bear + boar + woolbeast (leg
      swing in trot pairs, distance-driven phase, real facing).
- [x] Hurt flash: 21 red hurt-layer copies, flicker while hurt_flash.
- [x] Death animation: topple + rest + removal, all three kill paths;
      firebolt immortal-corpse bug fixed; void-fallen mobs culled.
- [x] Nameless raiders walk as humanoids; villagers face their direction;
      remote players' gait estimated from deltas.
- [x] vistest mob_anim + mob_hurt_death with pixel claims (frozen-leg
      detector via silhouette-width variance).
- [ ] Dragon corpse topple (freezes mid-flap 1.5s instead of falling).
- [ ] Additive/entity-tint hurt shader (would replace the layer swap and
      allow partial-flash blending in the opaque pass).

## Loop 340 — GMod-style physics item drops SHIPPED

- [x] lf_game::props: rigid props with floor+wall restitution, tumble,
      settle-flat, sleep; cap-5 stacks growing to block size.
- [x] Client carry: hold-RMB gravity-gun grab (6 blocks, LOS-checked),
      release-to-throw, walk-close pickup; magnet vacuum removed.
- [x] vistest item_physics with real stepped physics + pixel claims.
- [ ] Prop-vs-prop collision (stacks can overlap footprints).
- [ ] Network sync for drops (protocol v5; also covers mobs/villagers).
- [ ] Carried-prop highlight in the outline pass.

## Loop 341 — HUD declutter + inventory screen SHIPPED

- [x] Info line minimal by default; dense readout behind F3.
- [x] Inventory-first E screen (armor column, portrait, storage, hotbar,
      craft-by-hand route via UiOpen::HandCraft).
- [x] Furnace + chest converted to the kit panel shell.
- [x] inventory_screen vistest scene + claims; mirrored info line synced.
- [ ] Machine windows (13), trade, companion menu, tech tree still on
      egui::Window chrome — same shell conversion pending.
- [ ] Build-mode HUD (shape picker, symmetry indicator).
- [ ] Shaped 3x3 crafting grid (recipe list covers discoverability today).

## Loop 342 — texture patterns SHIPPED

- [x] Flatness audit found 8 noise-only barks + clump-less soil.
- [x] Species-true patterns for all of them; variance-floor test.
- [ ] Machine/trade/companion/tech-tree/book/smithing kit conversion.
- [ ] Build-mode HUD (shape picker, symmetry indicator).

## Loop 343 — HUD completion SHIPPED

- [x] kit_shell helper; machine (x13)/trade/companion/tech-tree/lore-book/
      smithing converted — no egui::Window chrome outside chat input.
- [x] Building HUD: BuildShape chips (R cycles / click), symmetry chip
      (V / click), slab+stairs placement for any held solid block.
- [x] build_hud vistest scene + claims; build_shape_state unit test.
- [ ] Shaped 3x3 crafting grid; drop networking (protocol v5);
      prop-vs-prop collision; carried-prop outline; dragon corpse topple.

## Loop 344 — clear sky + sun-tracked voxel lighting SHIPPED

- [x] Authored pixel-art sun, crescent moon, and star atlas assets.
- [x] Celestial render marker bypasses terrain distance fog/color grading but
      keeps depth occlusion; the sky stays available even at low view distance.
- [x] Raster face/normal relief follows the visible sun through the day from
      one shared `sun_direction(time)` source; corrected nighttime star timing.
- [x] `sun_visibility` aggressive-fog proof + east/west GPU lighting regression;
      371 tests and 89/89 visual scenes green.
- [ ] True projected raster cast shadows remain deferred: the shipped raster
      path is intentionally cheap directional relief; real soft cast shadows
      remain available in Live RT.
- [x] First-minute onboarding SHIPPED (loop 350, N01): persisted tutorial
      state machine (Move → Look → Gather → Craft → Build) fed by real
      gameplay facts (horizontal displacement, camera travel, natural-drop
      pickup, craft output, solid placement), keymap-adaptive prompt card
      + pinned starter-objective line (real shared painters, top-center,
      click-dismiss, Gameplay settings restart), new vistest proofs
      hud_onboarding + hud_small_onboarding.

## Loop 345 — kingdoms, walking NPCs, and the kingdom compass SHIPPED

- [x] NPC freeze fixed: `lf_npc::locomotion` (one-block step-up, ≤3-block
      descent, cliff refusal, no-tunnel gravity, 20-tick stuck sidestep with
      per-NPC side bias) driven by the rewritten `update_villagers`; hamlet
      villagers homed at their hamlet instead of the world origin.
- [x] NPC logic: idle villagers shuffle near home (under the en-route
      threshold, no ping-pong), guards patrol a four-post circuit during the
      Patrol slot, panicking NPCs flee directly away from the player.
- [x] Kingdoms: one deterministic citadel per 12x12-chunk region (16-name
      pool), full in-chunk build (crenellated walls + towers, gated banner
      wall, keep with THRONE marker + dais, 2 houses, well, market stalls,
      irrigated farm); THRONE/BANNER_KINGDOM/KINGDOM_BRICK blocks through the
      registry→atlas→items pipeline.
- [x] Kingdom court settles at first throne sight (new VillagerJob::Monarch
      Queen Ilsa + royal trades, 2 guards, farmer, trader, smith, all homed
      at the citadel); kingdoms persist in ClientSave, chronicle entries,
      gold-crown map markers with name + distance.
- [x] kingdom_compass item: craftable from any wood block over an iron ingot;
      held HUD dial (gold rim, red needle to the nearest kingdom, name +
      meters) deterministic from the seed — works from spawn.
- [x] vistest: kingdom_citadel, npc_walkers (real locomotion sim asserts),
      kingdom_compass_hud (real client paint fn) — 92/92 with pixel claims.
- [ ] Deferred: kingdom walls are not multi-chunk (citadels stay in one
      chunk footprint, matching the structure convention); kingdom roads do
      not carve into neighboring chunks; the path tracer's 128-entry palette
      still omits ids ≥128 (pre-existing — kingdoms render in the raster
      path, the default); monarch court lines (greetings/quests specific to
      Queen Ilsa) would need lore/npcs.toml archetypes.

## Loop 346 — packed normal/AO materials and hero textures SHIPPED

- [x] Redraw stone, grass top/side, dirt, sand, planks, coal ore, and iron ore
      as structured 16x16 pixel materials rather than per-pixel noise.
- [x] Pack tangent-space normal RGB + micro-AO alpha in the existing linear
      material atlas; deterministic fallback generation covers every built-in,
      procedural, mod, and dynamically replaced albedo layer.
- [x] Add explicit authored-material constructors to the live and headless
      renderers, with layer-count/dimension validation.
- [x] Protect transparent cutout edges, derive CTM maps per tile, and build
      correct vector-renormalized material mip chains.
- [x] Combine micro-AO with geometry AO in the sun-tracked raster shader using
      the existing material lookup; warm A/B p50 102.9ms versus loop 345's
      104.0ms in the same harness.
- [x] Four asset regressions + authored-channel GPU regression + raking-light
      material_gallery scene; 387/0 tests and 93/93 visual proofs green.
- [ ] Deferred: disk-backed PNG material-pack/mod manifest loading, arbitrary
      atlas resolutions above the current 16x16 contract, roughness/metalness,
      and true raster shadow maps are separate jobs. First-minute onboarding
      remains the next usability pass.

## Loop 347 — hitboxes, walls, wheels & castle siting
- [x] `is_solid`/`is_opaque` exclude by `!is_plant` — lavender/sunflower
      stop being invisible solid cubes that culled the ground under them.
- [x] `registry::pick_boxes` + `raycast_voxel_boxes`: shape-aware picking
      (slab solid half, torch stick, plant inset) and a matching shaped
      selection wireframe (one box per collision box).
- [x] `MobEntity::physics_step`: axis-separated AABB wall collision with
      hop step-up (shared box resolver extracted from the player);
      dragons clamp against terrain; wedge pop-up safeguard.
- [x] `crosshair_mob` occlusion filter — mobs behind the aimed block no
      longer steal LMB into the 0.5s attack cooldown (the creative
      2-blocks/sec cap).
- [x] Wheel: every notch counts, remainder kept for trackpads, consumed
      before the UI frame, no per-notch set_title.
- [x] `hotbar_caption`: one line above the hotbar — item name on switch,
      else the looked-at block's name.
- [x] Castle siting: dense 6x6 footprint validation, 2-chunk region
      border margin, 160-block spawn clearance, hillside carving above
      base, no trees/cover in the courtyard, GENERATOR_VERSION 6.
- [x] 399/0 tests (+12), 93/93 vistest, smoke OK, runtimes rebuilt,
      pushed.
- [ ] Deferred: shaped pick boxes for mod blocks (full-cell fallback),
      shaped picking at the secondary raycast call sites, and companions
      still use their own single-cell mover. First-minute onboarding
      remains the next queued pass.

## Loop 349 — real sound-effect bank (ElevenLabs SFX API)
- [x] 33 generated MP3s in `assets/sounds/` embedded via include_bytes
      (missing file = build error): block break/place ×5 materials,
      footsteps ×5, ui/eat/hurt/xp, tree creak/crash, and 12 newly
      audible events — splash, bow shoot, arrow hit, melee swing, mob
      hit/death, dragon roar, item pickup, craft done, chest open,
      anvil clang, player death.
- [x] `lf_audio` decode pipeline: rodio/symphonia MP3 → mono, silence
      trim (peak-relative), near-silence rejection, 0.85 level
      normalization; synth kept as deterministic fallback per event.
- [x] `tools/gen_sounds.py` + `make sounds`: cache-aware generator, key
      from env only. Free-tier key spend: 620/10,000 chars (46 calls).
- [x] 410/0 tests (+4), 94/94 vistest, smoke OK, runtimes rebuilt,
      pushed.
- [ ] Deferred: ambient loops (wind/birds/torch crackle), positional
      3D audio (distance/panning — currently all sounds are full-mix
      one-shots), music. First-minute onboarding remains queued next.

## Loop 350 — first-minute onboarding + nightly-beta N01 SHIPPED

- [x] Persisted tutorial state machine (Move→Look→Gather→Craft→Build)
      advanced only by real gameplay facts; keymap-adaptive HUD card +
      pinned starter objective (shared painters, click-dismiss, settings
      restart); ClientSave persistence + legacy migration; 12 tests,
      hud_onboarding + hud_small_onboarding proofs.
- [x] docs/NIGHTLY-BETA goal pack (14 docs) + `xtask night-plan-check`.

## Loop 351 — transactional crafting + real queue (nightly-beta N02)

- [x] lf_game::crafting transactional engine: `execute` (validate every
      ingredient → prove output room → consume exactly → grant exactly,
      batched past the u8 add_item boundary with zero loss — fixed the
      old grant loop that silently dropped outputs past 255), typed
      `CraftOutcome`/`CraftBlock` reasons, `max_batches` integer-safe
      craft-all, Inventory count_of/free_capacity/remove_count helpers.
      Blocked crafts consume NOTHING (missing-ingredient AND no-room).
- [x] Client rewired: craft buttons + Craft All run the engine; missing
      ingredients now name the exact items; the placeholder queue is REAL
      (1.25 s/job while playing, engine-verified completion, blocked jobs
      show live reasons, free cancel — documented rule: enqueue reserves
      nothing, consumption happens only at completion), persists via
      ClientSave (unchanged shape), unknown recipes drop honestly.
- [x] 8 new engine tests (exact consume/grant, blocked-consumes-nothing,
      >255 outputs, integer-safe max_batches, rapid double-craft, mod
      recipe transactionally, reason copy) + 3 client tests (queue
      status running/blocked, catalog lookup + unknown, queue save
      round-trip).
- [ ] Deferred to N03: modal workbench layouts, world scrim, queue strip
      polish/proofs, E/Escape input-recovery integration tests, the
      crafting_missing_ingredients / crafting_queue visual scenes.

## Loop 352 — modal workbench + input recovery (nightly-beta N03)

- [x] Modal hierarchy: the workbench draws strongly-opaque framed panels
      (paint_wb_panel) over a 215-alpha world scrim; hud_visible now hides
      the survival HUD (hearts/hotbar/XP/minimap) behind every container/
      station screen (HandCraft/CraftingTable/Furnace/Chest/Machine/
      Smithing/Imbue/Carve) — no duplicate HUD beneath the modal.
- [x] Discovery: search field + filter chips (All / Can make / New / ★Fav)
      + station chips (Any/Craft/Smelt/Alloy/Crush); favorites persist in
      RecipeBook; partial-ingredient rows now show the ~ amber mark.
- [x] Compact 640x420 drill-down: categories collapse to a chip row, the
      single pane shows list OR detail (← back), the strip is one hotbar
      row — layout from the shared pure workbench_layout() (unit-tested
      at 640/800/1280 widths, zones never overlap or clip).
- [x] Primary action ownership: Enter fires Craft-qty when the search box
      does not own the keyboard; the deferred action runs exactly once
      per frame outside the layout closures.
- [x] Input recovery: the rebindable inventory key (E) now closes every
      container/station screen (inventory_key_closes, pure + tested);
      Escape already closed everything — the E/Escape recovery contract
      is covered by test across every UiOpen variant.
- [x] Proofs: crafting_workbench rebuilt on the REAL layout math +
      crafting_workbench_small + crafting_missing_ingredients +
      crafting_queue (99 scenes total; all four Z.ai-reviewed PASS).
      435 tests.
- [ ] Deferred to a later polish pass: era filter chip (era gates already
      render on locked rows), substitutions column, time/power
      requirement rows (machines show power elsewhere), pause control on
      queue jobs, and the inventory screen's own duplicate-hotbar cleanup.

## Loop 353 — contextual HUD channels (nightly-beta N04)

- [x] hud_channels.rs (pure): Focus/prompt model (companion > villager >
      functional block > mine > place) with LIVE-keymap chips and blocked
      reasons (gate barred / blocked by player); transient manager
      (rep toasts capped at 3, settlement banner, hit-direction) with
      fade ticking; danger_warning() strict priority (drowning > critical
      health > starving > low health > threats) with severity shapes.
- [x] Shared painters: interaction prompt beside the crosshair (opaque
      keycap chip), hit-direction arc (world-true bearing minus live yaw),
      attack-readiness ring, reputation toast (crest + signed delta +
      reason + threshold title), settlement banner, danger line with a
      backing plate (the image review caught it drowning in busy terrain
      — fixed same job).
- [x] Wiring: throttled threat scan (hostiles + LOS), kingdom-entry
      banner (once per session, gates-barred state from standings), hit
      direction from each frame's attacker, standing deltas now carry
      reasons through add_standing (quest/trade/gift/discovery/destroy/
      rival ripples).
- [x] Proofs: hud_contextual (1280x800) + hud_contextual_small (640x420)
      + hud_danger + hud_reputation (103 scenes; all Z.ai-reviewed).
- [ ] Deferred: boss/elite identity line, heal/damage numeric feedback,
      garrison alert copy, and interaction prompts for doors/gates (no
      door blocks yet).

## Loop 354 — world identity + the seed laboratory (nightly-beta N05)

- [x] lf_worldgen::identity: canonical WorldIdentity (seed_u64 +
      generator_version + world_type + mod_fingerprint), saved to
      identity.dat BEFORE generation; legacy saves fall back field-by-
      field (seed.dat / genver.dat / fingerprint 0); explicit
      VersionMismatch policy (Current | Legacy{saved}); documented salted
      channels from the full 64-bit seed; seed-text rules moved here
      (numeric exact / word hash stable / empty rolls once) with slots
      delegating so UI and worldgen can never disagree.
- [x] lf_worldgen::seedlab: SeedMetrics over fixed lattices (height/biome
      order-independent hashes, stats, biome histogram, water/river/cave
      fractions incl. the REAL is_cave predicate now public on WorldGen,
      surface-block mix, nearest kingdom, spawn proxy) + Jensen–Shannon
      distance + SeedCorpusReport with calibrated thresholds and a
      same-seed bit-identical control.
- [x] Determinism tests: same seed bit-identical across instances
      (incl. negative/±1M/±i32::MAX coordinates), generation order does
      not matter (commutative fold over columns), 12-seed reduced corpus
      must clear diversity floors, corpus shape (0/u64::MAX/word hashes,
      distinct).
- [x] xtask seedlab + make seedlab: full 64-seed report to
      target/seedlab_report.json (transient by contract). Measured v6:
      2016 pairs · height L1 mean .159 / p05 .090 (floor .020) · biome JS
      mean .324 / p05 .214 (floor .025) — PASS.
- [x] Client adoption: create_world stamps identity before generating,
      load_world restores it + announces generator-version drift plainly,
      multiplayer Welcome now ADOPTS the server seed (restarts the
      streamer on change), mod fingerprints (recipes game-side + ore
      hooks worldgen-side) fold into the identity, F3 shows the full
      identity line.
- [ ] Deferred to N06: the rendered evidence (seed_atlas_8 panoramas,
      spawn_quality_8 with real spawn scoring, seed_same_control pixel
      proof) and any diversity REPAIR — the lab measures, N06 fixes what
      it finds.

## Loop 355 — real spawn selection + the rendered seed evidence (nightly-beta N06)

- [x] WorldGen::find_spawn: deterministic expanding spiral (step 4, radius
      ≤96) — dry land above sea, non-ocean biome, off rivers, tree-free
      cell (new pure tree_at predicate mirroring chunk placement), clear
      of kingdom footprints; reports nearest wood ring ≤96; a dry-land
      fallback exists for extreme worlds and is flagged. The lab's
      0/64-seeds-passed origin-spawn proxy is gone: spawn_ok now measures
      the REAL selection.
- [x] Client adoption: create_world and load_world place the player (and
      set the respawn point) at find_spawn with an arrival hint naming
      the biome + wood distance — ocean-origin seeds no longer drop the
      player into water at (0,0).
- [x] Proofs: seed_atlas_8 (8 labeled real-generator biome/height maps at
      the lab's ±256 macro scale, cross-panel categorical-disagreement
      gate), seed_same_control (right half regenerated from the SAME seed
      must be cell-hash identical at build + render seamless), and
      spawn_quality_8 (8 find_spawn verdicts, setup asserts every safety
      invariant before rendering). 106/106 vistest; all image-reviewed.
- [x] Fixed a real bounds bug the atlas check exposed in its own sampler
      (.min(mx+ms-x0) inverted the range — every panel but the first
      sampled zero pixels) plus chased a stale-fingerprint false lead.
- [ ] Deferred: rendered PANORAMA atlases (the map atlas proves macro
      shape; true 8-viewport 3D compositing needs renderer work),
      river_source_to_mouth + biome_transitions scenes (N07), and any
      terrain-shape repair beyond spawn (v6 diversity floors were already
      met by wide margins — nothing to repair this pass).

## Loop 356 — biome identity contract + transition proof (nightly-beta N07)

- [x] BiomeIdentity row (surface/filler/tree silhouette/feature density/
      feature set/freeze/cold) + the pairwise-distinct contract as a
      test: no two of the 46 biomes may share the whole visible tuple —
      the "thirty names, twenty-seven meadows" failure mode now fails CI.
      The scan found exactly one surviving collision (Ocean vs DeepOcean)
      plus Ocean/WarmOcean after the first fix: three distinct ocean
      floors now (Ocean dirt, WarmOcean sand, DeepOcean stone).
- [x] Confetti ceiling enforced by test: ground cover ≤ 0.35 everywhere
      (Jungle .40, LavenderFields .45, SunflowerPlains .40 trimmed) —
      identity reads from negative space.
- [x] GENERATOR_VERSION → v7 (ocean floors + densities change chunks);
      seedlab still PASSES its floors on v7 (2016 pairs, height L1 p05
      .090, biome JS p05 .214).
- [x] Proofs: biome_contact_sheet upgraded from pavement to full identity
      (surface + filler + SIGNATURE TREE silhouette from TreeKind blocks
      + ground-cover features per strip) and new biome_transitions (four
      boundary pairs with dithered mixing bands). Both Z.ai-reviewed
      PASS. 107/107 vistest.
- [ ] Deferred: per-biome fog/sky grading (env() has no biome input —
      engine work), gameplay-affecting resource rows per biome (mining
      tables are global today), and merging (no merge was needed: every
      biome now differs on the visible tuple).

## 2026-09-07 — Natural-world rebuild (NWR pack) baseline recorded

- DONE (NWR-001): baseline audit + capability inventory + the
  anti-false-art guardrail (pc3d_render::inventory). No rendering changed.
- NEXT (NWR-002): original GLB asset factory (tree/rock/house), validated
  source-to-GLB pipeline, windowed render proof.
- DEFERRED to their milestones (per rebuild_roadmap.json): natural
  terrain surface migration (NWR-003/004), sparse caves + conforming
  water (NWR-005), materials/atmosphere (NWR-006), wilderness assets
  (NWR-007), settlement kit (NWR-008), NPC presentation (NWR-009), Deck
  quality contract (NWR-010), rebuild vertical slice (NWR-011).
- The prior visual reset's honest deferrals remain tracked in
  docs/POORCRAFT-3D-VISUAL-RESET/VISUAL-GATES-REPORT.md §Known limits.

## 2026-09-07 — NWR-002 asset factory shipped

- DONE: tools/assetgen (original GLB factory), pc3d_assets::v2 (schema-v2
  validator + honesty law), pc3d_render::glb (loader + LOD fallback),
  --play-assets windowed proof; three assets integrated.
- DEFERRED to their milestones: texture maps (NWR-006 material library),
  instanced placement (NWR-007), asset colliders feeding player collision
  (NWR-005), glTF skinning/animation (NWR-009).

## 2026-09-07 — NWR-003 surface spike shipped

- DONE: pc3d_render::surface (seam-shared boundary grids, authoritative
  sampling, delta edits + bounded dirty-remesh, same-data collision,
  delta persistence), --play-surface windowed proof, discriminators.
- DEFERRED: spike LOD, caves (NWR-005), host-authoritative edit wiring
  (NWR-004/005), texture materials (NWR-006).

## 2026-09-08 — NWR-004 surface migration shipped

- DONE: pc3d_render::surface_stream (rings/budget/queue/frustum, LOD +
  skirts, distance-gated eviction), explicit TerrainPolicy, collision from
  the full ring, --play-surface-stream windowed proof.
- DEFERRED: surface collision wired into the walking player (NWR-005),
  mid-ring mesh sampler optimization, caves/water on the surface path
  (NWR-005), texture LOD detail (NWR-006).

## 2026-09-08 — NWR-005 caves/water/foundations shipped

- DONE: pc3d_render::world_features (CaveRegion face-net extraction
  welded to final_solid; ConformingWater strips following surface
  patches with local refresh_after_edit; check_foundation verdicts),
  CollisionSurface trait + SurfaceRegion implementation (the body walks
  the surface and feels edits), --play-caves windowed proof, make
  p3d-caves. GPU + unit + windowed evidence; 9/9 gates.
- DEFERRED: per-cave discovery cache (world-wide cave scan cost),
  live-player switch from AuthorityGround to the surface path (NWR-011
  slice integration), automatic dirty-patch water subscription in the
  app loop, dedicated cave batch pass (wilderness assets), asset
  colliders feeding player collision (carried from NWR-002), texture
  materials (NWR-006).

## 2026-09-08 — NWR-006 materials/atmosphere shipped

- DONE: pc3d_render::atmosphere (tier table + budgets, texel-snapped
  sun shadow map, exp2 fog, procedural material atlas from
  pc3d_assets metadata, water glint, cutout foliage pipeline),
  --play-materials windowed proof, make p3d-materials. LEGACY default
  keeps all prior pixels; 9/9 gates green.
- DEFERRED: one cascade (near field only), sky-band fog, masked
  foliage shadows, GLB textured leaves (NWR-007), live-shell tier
  binding (NWR-011).

## 2026-09-08 — NWR-007 wilderness shipped

- DONE: pc3d_world::flora (pure biome placement + landmarks), nine
  assetgen GLBs + wilderness_batch.json (validated), pc3d_render::flora
  (instanced buckets, wind grass, bounded scan with eviction/reload,
  shadow casting, authority-derived collision), --play-wilderness
  windowed proof, make p3d-wilderness. 9/9 gates green.
- DEFERRED: per-plant frustum culling, grass collision (never blocks
  by contract), landmark on the single-asset slot, GLB cutout leaves,
  live-shell flora attach (NWR-011), grass far-LOD.

## 2026-09-08 — NWR-008 settlement kit shipped

- DONE: ten socket-declaring kit GLBs + settlement_batch.json
  (validated), assemble_kit from the authoritative plans (exhaustive
  mapping, socket-chained walls, gate passage refinement, D-033
  markers, river dock + water wheel), instanced draws with shadows,
  SettlementGround adapter, --play-settlement windowed proof,
  make p3d-settlement. 9/9 gates green.
- DEFERRED: gate-approach terrain leveling (foundations work), a real
  well piece (banner stand-in), road rendering, module texture
  variants, live-shell kit attach (NWR-011).

## 2026-09-08 — NWR-009 NPC rig shipped

- DONE: the rig + role gear + deterministic intent-driven animation,
  impostors, per-color instanced crowd (<= ~8 draws for any crowd),
  capsule collision adapter, crowd_tick over the authoritative
  brains, --play-people windowed proof, make p3d-people. 9/9 gates.
- DEFERRED: skinned meshes, NPC shadows in the sun pass, chunked
  updates for very large crowds, NPC-vs-NPC avoidance (sim's domain),
  live-shell rig attach (NWR-011).

## 2026-09-08 — NWR-010 Deck quality contract shipped

- DONE: pc3d_render::deck (contract + presets + report, tested), the
  crowd pose-rate lever, --deck-bench 3-tier benchmark + documented
  report (measured: 167/149/146 fps low/mid/high; caps held; Low
  readable), make p3d-deck-bench. 9/9 gates.
- DEFERRED: internal render-scale path (declared 1.0), CPU/GPU time
  split (no profiler), mid-ring mesh sampler optimization.

## 2026-09-08 — NWR-011 rebuild slice shipped

- DONE: assemble_rebuild (full stack on the showcase route), the
  surface live walk, foundation-gated building, crowd ticking in the
  live loop, --play-rebuild automated route + live, REVIEW-CHECKLIST.md
  with the owner gate, make p3d-rebuild / p3d-rebuild-live. 9/9 gates.
- DEFERRED: live terrain-edit persistence on B, cave live collision,
  rotating water wheel. THE OWNER'S MANUAL PLAY PASS is the gate
	before 'playable'.
