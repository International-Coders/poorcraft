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
- The workspace builds green with the feature on and off; CI artifacts
  cover both.
