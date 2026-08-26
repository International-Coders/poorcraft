# Steam & Release Plan

## Foundation already in place

Steam-ready deployment already exists: a Spacewar (480) dev loop,
feature-gated Steam transport, and depot docs (`docs/STEAM.md`,
`steam_appid.txt`). The `lf_steam`/`steam` feature is off by default and
falls back to UDP — that default should hold through friend-testing (see
below) and only flip once a real Steam App ID replaces the Spacewar
placeholder.

## Immediate goal: friends-and-family testing

This is the near-term milestone this whole roadmap should point at first —
not a public Steam launch yet:

- [ ] P26 (rendering/UX stabilization) complete — don't hand a build to
      friends with known visual bugs; first impressions are what you're
      testing for.
- [ ] At least one full age beyond current Electric tier playable
      end-to-end (Water Age is the cheapest to reach this with, per `09`'s
      P27) so testers see *new* content, not just a polished version of
      what already exists.
- [ ] A packaged build for each tester's OS via the existing
      `cargo xtask package` / runtime bundling flow in AGENTS.md.
- [ ] A short, honest "known issues" note included with each build —
      matches the project's existing honesty discipline (BACKLOG's
      "deferred, honestly" sections) and sets tester expectations so
      feedback is about the game, not about bugs you already know about.
- [ ] A simple feedback channel (a shared doc, a Discord, whatever's
      already in use) — not a blocker to build, just don't skip collecting
      structured feedback.

## Pricing philosophy

- **Low price, not free.** You've already decided on "very, very low
  price" while the exact number is still being worked out — treat that as
  settled direction, and revisit the specific number in `DECISIONS.md`
  once there's a real content-complete build to compare against similar
  early-access voxel/survival titles' launch pricing.
- **No monetization beyond the up-front price at this stage** (Pillar 5 in
  `01`) — no cosmetic shop, no DLC plan yet. Keep the store page and
  business model simple until the game has proven itself with friends and
  early players.
- Consider Steam Early Access framing once public — it matches the
  project's own honest, still-in-progress reality (visible in STATUS.md's
  "deferred polish" section) better than a "1.0, finished" framing would.

## Steam store page checklist (for when public launch is closer)

- [ ] Real App ID (replaces Spacewar 480 placeholder) — needs a Steamworks
      account/app registration, a business step, not a code step.
- [ ] Store page copy that reflects the actual pillars in `01` — lead with
      "craft, automate, and adventure in one world," not a generic
      "sandbox survival game" description.
- [ ] Capsule art / trailer — should show the mashup directly (a shot of a
      water wheel next to a wizard tower reads better than either alone).
- [ ] System requirements reflect the P26-defined "low" target device, not
      the path-traced showcase mode.
- [ ] Update `docs/STEAM.md` and `steam_appid.txt` together when the real
      App ID lands, and flip the `steam` feature default only after that.

## Name change

The game is currently "poorcraft" / "LOREFORGE" as a placeholder. When a
final name is chosen:

- [ ] Update `Cargo.toml` package names only if actually renaming crates/
      binaries (weigh churn vs. benefit — AGENTS.md's build commands
      reference `loreforge`/`loreforge-server` binary names throughout).
- [ ] Update Steam store page, capsule art, and `DECISIONS.md` together.
- [ ] This design-doc folder needs no changes — every file here was
      written to refer to "the game," not the placeholder name.

## Guardrail

Don't let store-page/business polish (capsule art, App ID registration)
compete for time against P26–P35 before there's a friends-tested build
worth putting a store page in front of. Sequence: **P26 → at least one new
age playable → friend test → iterate → then Steam page work.**
