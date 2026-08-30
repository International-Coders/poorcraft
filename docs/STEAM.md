# Steam Deployment Guide

LOREFORGE is Steam-ready. This document covers the dev/test loop (Spacewar),
the transport switch, and shipping.

## Development with Spacewar (AppID 480)
1. `steam_appid.txt` in the repo root contains `480` — Valve's shared test
   app. Any Steamworks call made by a process launched outside the Steam
   client uses this id.
2. The Steam **client must be running** and logged in for `SteamAPI_Init()`
   to succeed (per Valve's docs: https://partner.steamgames.com/doc/sdk/api).
3. Run: `cargo run --release -p loreforge`. Multiplayer uses the UDP
   transport unless the `steam` feature reports a live Steamworks init.

## Enabling the Steam transport
```bash
cargo run --release --features lf_steam/steam -p loreforge
```
`lf_steam::preferred_transport()` returns `SteamP2p` only when Steamworks
initialized (client running, SDK reachable); otherwise the game transparently
uses the existing UDP netcode over the same lf_protocol frames.

## Building the Steam depot
```bash
cargo run --release -p xtask -- package     # dist/loreforge-<ver>-<os>.zip
```
Ship the zip contents (bin/, mods/) as your depot:
- Depot "Loreforge Game": `bin/loreforge`, `bin/loreforge-server`, `mods/`
- `steam_appid.txt` is NOT shipped (Steam injects the real AppID at launch)
- Launch option: `bin/loreforge`

## Upload (steamcmd)
```bash
steamcmd +login <account> +run_app_build /path/to/app_build.vdf +quit
```

## Before the store page
- Replace 480 with your assigned AppID in the Steamworks dashboard (delete
  steam_appid.txt from depots).
- The `steam` feature compiles (verify locally with
  `cargo check -p lf_steam --features steam` — P25 wired the steamworks
  dependency in). CI builds and ships **default-feature** binaries only;
  run that check before shipping a Steam build.

---

## Loop 335 — exercised end-to-end on this host (2026-08-30)

The `steam` feature now LINKS and RUNS against a live Steam client:
`cargo run -p lf_steam --features steam --example steam_probe` (run from
the repo root, so steam_appid.txt = 480 is visible; libsteam_api.dylib is
copied next to the binary automatically by the build layout).

Probe result on this host (Steam client running, logged-in session):

- `preferred_transport()` -> **SteamP2p** (live)
- Steamworks init: PASS (steamclient.dylib loaded)
- Steam ID: 76561198061541771
- User stats request: PASS
- Matchmaking lobby create + leave: PASS (real lobby id round-trip;
  the callback pump must run while waiting — see the probe source)
- Overlay: reported disabled — by design for direct launches; the
  overlay activates when the game is launched THROUGH the Steam client
  (add the built binary as a non-Steam game and launch it there)

Still requires a Steamworks partner account to unlock:

- A real AppID (dev AppID is Valve's 480/Spacewar) — achievements and
  any game-specific stats schema need it.
- Steam P2P sockets as the in-game multiplayer transport: the binding,
  init and transport selection are proven; swapping the UDP socket for
  ISteamNetworkingSockets remains the one structural step (protocol v4
  escrow already rides either transport).
