# Release, Recovery, and Morning Handoff

An autonomous night is successful when it leaves a sequence of green,
understandable commits. A long uncommitted diff is not success.

## Verification ladder per job

Run what the job requires, in order:

```bash
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace
cargo run --release -p xtask -- vistest shots   # visible jobs
make smoke                                      # runtime-sensitive jobs
make perf                                       # performance-sensitive jobs
make runtimes                                   # after finished game-code jobs
ls -la dist/
```

If formatting changes are needed, run the formatter, inspect its diff, then
repeat the check. Generated screenshots are reviewed with the protocol in
`08-ZAI-VISION-AND-DEEP-TESTS.md`.

Pure tooling that does not alter the game binary may skip runtimes, as allowed
by the dev-loop skill, but still needs tests, bookkeeping, commit, and push.

## Commit boundary

Before commit:

- inspect `git status --short` and `git diff --check`;
- inspect diffs for every staged path;
- stage only the active job; never absorb unrelated audio/assets/experiments;
- update STATE, BACKLOG, CHANGELOG, and append DEVLOG after real verified work;
- keep Makefile current if commands/targets changed;
- state exactly what was honestly deferred.

Commit message format:

```text
<area> (loop N): <concrete shipped behavior>
```

Then run `git push github HEAD`. If authentication/network fails, retain the
local green commit, capture the exact error, set a blocker/next action, and do
not start changes whose safety depends on the remote checkpoint.

## Failure recovery

- Compilation/test failure: diagnose within the active job; reduce scope only
  at a coherent boundary. Never commit red.
- Vistest/pixel failure: open the PNG, fix production code or a genuinely
  incorrect assertion, rerun all affected scenes. Never weaken a gate merely
  to pass.
- Z.ai semantic failure: record the observed defect, improve the asset/layout,
  and add a measurable assertion when possible.
- Performance regression: confirm warm/cold and scene equivalence; revert the
  regression or document a measured, approved correctness tradeoff.
- Dirty unrelated file overlap: preserve both sides, avoid the file if
  possible, or stop with exact paths; never checkout/reset another owner's work.
- Low remaining context: finish or shrink the current job, verify, bookkeep,
  commit/push, and put a cold-start-ready next task in STATE.
- Repeated external blocker: after exhausting safe alternatives, stop at the
  last green commit with the exact command/error and required human action.

Never use destructive reset/checkout, force-push, broad recursive deletion,
history rewriting, or save deletion as recovery.

## Morning report template

```markdown
# LOREFORGE Nightly Report — YYYY-MM-DD

## Outcome
- Last completed job:
- Beta gate status: ALPHA | PRE-BETA | BETA CANDIDATE
- Current commit/branch:
- Push status:

## Shipped jobs
| Job | Commit | Player-visible outcome | Tests | Visual proofs | Runtime |
|---|---|---|---:|---|---|

## Evidence
- cargo build:
- cargo test count:
- vistest count and changed PNGs:
- Z.ai review verdicts/confidence:
- smoke:
- perf before/after:
- runtime artifact paths and sizes:

## World and asset audits
- seed corpus result:
- castle placement result:
- asset manifest closure:
- NPC/reputation scenarios:

## Disk/repository
- before/after by category:
- files removed and recovery method:
- protected saves/unrelated changes confirmed untouched:

## Remaining failures or deferrals
- first failing beta gate:
- exact reproduction/evidence:
- STATE.md next_task:
- human action required:
```

## Artifact expectations

When game code changed and the host supports them, report absolute paths for:

- `dist/loreforge.app/`
- `dist/loreforge-macos.dmg`
- `dist/loreforge-linux-x86_64.tar.gz`
- `dist/loreforge.exe` only if the Windows cross toolchain actually succeeded.

Never claim an artifact based on a command intention. Check that it exists and
record size/timestamp. State unsupported targets and the reason plainly.

## Final honesty rule

The morning label follows evidence. If the journey works but two castle realms
remain recolors, report pre-beta. If every visual is attractive but save/load
duplicates reputation, report alpha. The pack exists to make progress durable,
not to manufacture a beta claim.
