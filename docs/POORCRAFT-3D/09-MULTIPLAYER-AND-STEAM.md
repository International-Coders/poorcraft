# Multiplayer and Steam

## One world, one rule set

Solo, LAN, dedicated-server, and Steam sessions must run the same simulation
rules. Solo play should start an integrated authoritative host in-process;
multiplayer should place that host behind a transport adapter.

## Authority

The host owns:

- world edits and terrain revisions;
- players, inventories, stats, and rewards;
- fluids, machines, and energy state;
- mobs, NPCs, companions, settlements, and faction state;
- quests, events, and persistent history;
- save/load and reconnect recovery.

Clients own presentation, input mapping, prediction where safe, interpolation,
and local UI state. A client may request an action, never silently grant the
result.

## Replication contract

The network layer needs versioned commands and snapshots, sequence/acknowledge
behavior, duplicate suppression, interest management, content handshake,
resync, reconnect, and transaction recovery. Transport details must not leak
into gameplay rules.

## Steam scope

The player-facing path should eventually cover initialization, lobby creation,
invites, joining, loading, version/content mismatch, leaving, reconnect, and a
clear non-Steam fallback. A real two-account test is external evidence and
must be reported separately from local compilation or mock tests.

## Solo quality bar

The solo host must be exercised by the same command/event tests as a dedicated
host. A feature is not multiplayer-ready if it only works when two clients
happen to render the same local guess.

## Implementation order

1. Build the authoritative host for solo play first.
2. Test save/reload and deterministic replay of every command.
3. Add local two-client transport with latency, loss, duplicate, and reconnect
   tests.
4. Add dedicated-host persistence and interest management.
5. Put Steam behind the same transport interface.
6. Run the real two-account proof only after the non-Steam path is complete.

## Multiplayer design questions

Player count, shared versus rival realms, permissions, theft, building rights,
war declarations, and host migration materially change the persistence and
authority model. Do not assume answers from other survival games; record the
owner’s decisions before public-server work begins.

## Scale contract

Hosts choose their maximum population according to server ability. Support for
128-player worlds is an optimization and operations target, not a blanket
promise. It must be earned through staged 4/16/32/64/128-player proofs with
interest management, permission/anti-grief controls, terrain/fluid/NPC
authority, server profiling, and published hardware requirements. See
`21-FIRST-PERSON-WAR-KARMA-AND-SERVER-SCALE.md`.
