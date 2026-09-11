# Operating Loop

One GLM run equals one shippable job.

## Loop

1. Read `STATE.md`, `BACKLOG.md`, `CHANGELOG.md`, and `DEVLOG.md`.
2. Read the relevant WT pack.
3. Inspect the current code and dirty worktree.
4. Pick one job that can be proven.
5. Write or update implementation code.
6. Write or update tests.
7. Run targeted tests.
8. Produce screenshots, wireframes, overlays, and JSON evidence when relevant.
9. Run the broader guard tests.
10. Update docs and bookkeeping.
11. Stage only intended files.
12. Commit and push.
13. Repeat.

## Required Posture

- Be strict with evidence.
- Be honest with deferrals.
- Prefer small vertical slices over broad unproven scaffolds.
- Keep playability first: mouse, Escape, start menu, seed preview, HUD,
  interaction prompts, and runtime proof.
