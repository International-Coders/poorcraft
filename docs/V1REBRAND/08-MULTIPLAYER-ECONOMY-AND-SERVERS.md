# Multiplayer, Economy & Servers

## Foundation already in place

Protocol v3 (join/leave, position snapshots, validated block ops, chat),
an authoritative-lite dedicated server with edit history and newcomer
replay, and a Steam P2P transport option already exist. This doc covers
what the *content* additions in this roadmap (Paths, power, magic,
building) mean for that existing multiplayer layer — it does not propose
re-architecting the network model.

## Trading & the Path economy

- Villager trading already exists. Extend the same trading UI pattern to
  **player-to-player trading**, which becomes meaningful once Paths (`07`)
  create real specialization — an Engineer trading power components for
  an Artisan's enchants is the payoff of the whole Path system.
- No proposal here for a currency system beyond what may already exist via
  villager trading — barter (item-for-item) is the simpler, more
  in-fiction default; revisit via `DECISIONS.md` only if playtesting shows
  a real need for money.

## Shared infrastructure

- Power grids (`04`) and buildings (`06`) placed by one player should be
  usable/extendable by teammates on a server by default, consistent with
  the existing shared-world block-edit model — no new permission system is
  proposed here. If griefing becomes a real problem in friend-testing
  (per `10`), a lightweight claim/permission block is the likely first
  fix — flag for `DECISIONS.md`, don't build preemptively.
- Server-side validation of new content (machines, spells, Path-gated
  recipes) must follow the same pattern the P25 audit already established
  for `SetBlock` — validate against the real registry, don't trust the
  client, and make sure the dedicated server loads whatever new mod/
  content data the client does.

## Server browser & discovery

- Already flagged as deferred in BACKLOG.md (P9/P23). This roadmap doesn't
  change its priority — it's still a "nice to have once there's something
  to browse for," and friend-testing (`10`) doesn't need it since friends
  will be given a direct address.

## Mods and this roadmap

- Everything in `04`–`07` should be added the way `ember_ores` and
  `amberium` already are: through the runtime mod-registry pattern
  (`lf_modapi`), even if it ships as "built-in" content — this keeps the
  door open for the community (or future paid DLC-equivalent content, if
  that's ever revisited) to add new Paths' recipes, new power sources, or
  new spells without engine changes.

## Guardrail

Don't let multiplayer-specific systems (trading, permissions) grow ahead
of what singleplayer needs — per Pillar 2, this is one world with two ways
to play it, not two separate games with a shared renderer.
