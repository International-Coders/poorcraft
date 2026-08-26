# Steam Multiplayer Integration — Detail for Steps 34–36

## What already exists (don't rebuild this)

Per AGENTS.md/BACKLOG.md, the project already has: protocol v3 (join/
leave, 20/s position snapshots, validated block ops, chat), an
authoritative-lite `lf_server` (canonical world + edit history, newcomer
replay, chat relay), a dedicated `loreforge-server` binary, and a
feature-gated `lf_steam`/`steam` optional dependency (steamworks 0.12)
that's OFF by default and falls back to UDP. This stage turns that
existing P2P *transport option* into an actual **lobby and matchmaking
experience** — what you called "Steam Space Force" (Spacewar app-ID-based
Steamworks integration) working end-to-end for hosting and easy joining.

## Step 34 — Lobby creation and discovery

- Use the Steamworks `ISteamMatchmaking` API (via the `steamworks` crate
  already a dependency behind the `steam` feature) to:
  - Create a lobby when a player chooses "Host" from the multiplayer
    screen, with lobby metadata (world name, player count, game version)
    set as lobby data fields.
  - List lobbies from friends currently playing (`RequestLobbyList` with
    a friends filter, or simply reading friends' rich presence) in the
    multiplayer screen's UI, built with the Step 12 design system.
  - Joining a listed lobby should require no manual IP entry — select it,
    click join, connect.
- Keep the existing dedicated-server / direct-UDP-connect path fully
  intact and visible as a separate option in the same screen — Steam
  lobbies are for the easy "play with friends" case, self-hosted/dedicated
  servers remain for anyone who wants that (server communities, LAN
  parties without Steam).

## Step 35 — Steam P2P as the default transport inside a lobby

- When a game is joined via a Steam lobby, route the actual gameplay
  traffic (the existing protocol v3 packets) over Steam's P2P networking
  (`ISteamNetworkingSockets` or the existing P2P transport option already
  behind the `steam` feature) instead of requiring the host to port-
  forward UDP.
- This is additive to the existing transport, not a replacement: the
  dedicated-server UDP path is unaffected and its existing two-client
  local integration test must keep passing unchanged.
- Concretely: the client's `net.rs` needs a second transport
  implementation alongside the existing UDP one, selected based on
  whether the connection originated from a Steam lobby join or a direct
  address — same protocol codec, different wire transport underneath.

## Step 36 — Friend invite and "Join Game" flow

- Support the Steam overlay's "Invite Friends" from within a hosted
  lobby, and "Join Game" from a friend's Steam friends-list context menu
  when they're already in a lobby.
- This is largely Steamworks API wiring (`ISteamFriends` rich presence +
  invite APIs) once Step 34's lobby system exists — the lobby ID needs to
  be set as the player's rich-presence connect string so Steam's own UI
  can offer the one-click join.

## Testing reality check

Full Steamworks lobby/invite testing typically requires two real Steam
accounts and the Spacewar (480) test AppID already configured
(`steam_appid.txt`) — CI can't fully exercise this the way `cargo test
--workspace` exercises the UDP path. Document a manual two-machine (or
two-Steam-account) test procedure in DEVLOG.md for each step, and keep
automated coverage for everything that doesn't require an actual
Steamworks session (protocol codec tests, transport-selection logic).

## Guardrail

Don't let Steam lobby work regress the dedicated-server path — self-hosted
multiplayer without Steam must remain fully functional, since not every
future player or server operator will be running through Steam.
