# Data Contracts

These shapes keep the beta systems data-driven while preserving crate-layer
rules. Exact Rust/TOML syntax may adapt to neighboring code; semantics and
validation must remain.

## Asset manifest

Suggested `assets/manifest.toml` row:

```toml
[[asset]]
id = "accord.wall.primary"
kind = "block_texture"
path = "assets/textures/accord_wall.png"
consumer = "lf_assets:block.ACCORD_STONE"
dimensions = [16, 16]
layout = "single_tile"
palette = "accord_stone_v1"
alpha = "opaque"
origin = "project_authored" # or procedural_generated / third_party
generator = ""
seed = 0
license = "LOREFORGE project asset"
animation = []
fallback = "core.stone"
proof = "faction_material_gallery"
status = "beta"
reviewed_by = "zai+human"
reviewed_on = "YYYY-MM-DD"
```

Validation owns file existence, duplicate IDs, dimensions/layout, consumer
closure, allowed alpha/origin/status values, license/source, and proof scene.
`lf_assets` remains the render-side consumer; gameplay crates never depend on
client/UI code.

## Realm policy

Suggested `lore/realms.toml` or extension to existing faction data:

```toml
[[realm]]
id = "gravebound_court"
display_name = "The Gravebound Court"
alignment_tags = ["undead", "ordered", "oathbound"]
home_biomes = ["deep_cave", "ash_waste"]
castle_grammar = "tomb_city"
starting_standing = -40

[realm.values]
lawful_self_defense = 0
civilian_murder = 8       # respect/fear axis, not necessarily trust
kill_bound_undead = -30
grave_offering = 12
grave_desecration = -25
honor_oath = 15
break_oath = -35
```

Policy outputs independent deltas for standing, trust, fear, respect,
resentment, warrant severity, rumor priority, and dialogue posture. Clamp and
threshold semantics live in one tested module.

## Moral event and knowledge

Conceptual serialized records:

```text
MoralEvent {
  id, actor, action, target, target_kind, target_faction, location, day_tick,
  combat_context, legal_context, contract_id, severity, evidence_tags
}

Knowledge {
  event_id, knower, source_kind, source_entity, confidence, learned_day_tick,
  reported_to, expires_day_tick
}
```

Event IDs are stable and idempotent. Applying the same event/knowledge twice
cannot duplicate standing, quests, loot, warrants, or chronicle entries.
Witness discovery lives near world/entity sensing; realm interpretation lives
in faction/game logic; UI only presents validated outcomes.

## Castle layout plan

```text
CastlePlan {
  realm_id, seed, grammar_version, site, terrain_summary, bounds,
  modules[], connections[], entrances[], roads[], nav_anchors[],
  npc_roles[], dwellings[], resource_links[], protected_cells[], layout_hash
}

Module {
  id, kind, oriented_bounds, ports[], foundation_rule, roof_rule,
  material_roles, required_neighbors, nav_anchors, activity_anchors
}

PlacementReport {
  accepted, rejection_reasons[], changed_bounds, support_depth_max,
  support_ratio, blocked_required_cells[], reachable_required_anchors,
  river_cells_changed, protected_cells_preserved
}
```

Planning is pure/deterministic and heavily property-tested. Voxel mutation
consumes an accepted plan and returns a report. Worldgen owns terrain and
placement; voxel remains substrate; client spawns/presents NPCs through stable
anchors without making `lf_engine` depend on gameplay.

## World identity and diversity report

```text
WorldIdentity { seed_u64, generator_version, world_type, mod_fingerprint }

SeedMetrics {
  identity, sample_bounds, height_hash, biome_hash, river_hash, cave_hash,
  structure_hash, height_stats, biome_histogram, water_fraction,
  nearest_kingdom_distance, spawn_quality
}

SeedCorpusReport {
  schema_version, commit, corpus[], metric_thresholds,
  pairwise_summary, failures[]
}
```

Write reports beneath a tracked small diagnostics/docs location only when they
are intentional evidence; large transient data goes to a temporary directory.

## NPC intent and navigation profile

```text
NpcIntent { kind, target_anchor, reason, priority, started_tick, timeout_tick }
MovementProfile { width, height, step_up, safe_drop, can_swim, can_open_doors }
PathBudget { max_nodes_per_slice, max_slices, retry_cooldown, cache_scope }
```

The rendered activity is derived from intent and locomotion state. Dialogue
cannot directly mutate these records without issuing a validated command.

## Vision review

Use the record in `08-ZAI-VISION-AND-DEEP-TESTS.md`. If automated tooling is
added, prefer JSON with schema version, file hash, scene state, questions,
answers, confidence, verdict, and model/tool identifier. Never store API keys
or private request data.

## Versioning and migrations

- Add schema versions before persistent beta data lands.
- Use defaults only when a safe legacy meaning exists; otherwise migrate or
  fail with an actionable message.
- Tests load at least one legacy fixture and current round-trip fixture.
- Unknown future fields should be handled according to the serializer's
  documented compatibility policy.
- Generator, save, network, mod, and asset-manifest versions are different
  concerns; never bump one as a substitute for another.
