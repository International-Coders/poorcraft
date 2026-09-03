# Paste-ready ZCode `/goal`

Run this from `/Users/zari/Desktop/POORCRAFT`. Paste the following text after
invoking `/goal`:

```text
Take LOREFORGE from its current playable alpha toward a defensible beta by
executing docs/NIGHTLY-BETA/10-OVERNIGHT-JOB-QUEUE.md in order. First read
AGENTS.md, STATE.md, BACKLOG.md, CHANGELOG.md, DEVLOG.md, AUDIT.md, every file
in docs/NIGHTLY-BETA/, and the source files relevant to the next job. Run
`make night-plan-check` and inspect git status before changing anything.

Treat the repository as the source of truth. Preserve unrelated dirty files
and never overwrite player saves. Existing systems must be deepened, not
silently rebuilt. Use original LOREFORGE factions and assets; strategy-town
games may inspire systemic depth, but do not copy copyrighted names, layouts,
characters, icons, textures, dialogue, audio, or UI.

Work one N-job at a time. For each job: orient, write a short concrete plan,
implement code and tests together, run the proportional verification ladder,
inspect every changed screenshot with Z.ai image recognition, fix all proof-
discovered bugs, update STATE/BACKLOG/CHANGELOG/DEVLOG and Makefile when
commands change, build fresh runtimes when game code changed, commit only the
job's files, and push with `git push github HEAD`. Never make a docs-only
progress commit. Do not start the next job unless the current commit is green
and pushed. If push authentication fails, keep the green commit and state the
failure honestly in the handoff; do not claim success.

The priority outcomes are: a readable real-game HUD and crafting experience;
seeds that are reproducible yet visibly and statistically distinct; distinct
biomes; terrain-integrated original faction castles; living NPC navigation,
memory, and witnessed reputation; undead and infernal factions that respond
coherently to a player's moral history; a complete asset manifest and asset
quality pipeline; deep behavioral and visual tests; measured performance; and
safe repository cleanup. The detailed acceptance criteria in this pack are
binding.

Continue through as many complete jobs as time and context permit. Never stop
mid-job. When a blocker repeats and cannot be solved safely, record exact
evidence, set the next actionable job in STATE.md, and stop at the last green
checkpoint. Finish with the morning report specified in
docs/NIGHTLY-BETA/12-RELEASE-RECOVERY-AND-HANDOFF.md.
```

## First commands

```bash
pwd
git status --short
make night-plan-check
sed -n '1,220p' AGENTS.md
sed -n '1,220p' STATE.md
sed -n '1,320p' docs/NIGHTLY-BETA/10-OVERNIGHT-JOB-QUEUE.md
```

Do not run `cargo clean` as an opening ritual. The 23 GB `target/` directory
is rebuildable, but clearing it before a job can waste most of the night and
destroy useful incremental caches. Cleanup is N22 and has its own evidence and
recovery requirements.
