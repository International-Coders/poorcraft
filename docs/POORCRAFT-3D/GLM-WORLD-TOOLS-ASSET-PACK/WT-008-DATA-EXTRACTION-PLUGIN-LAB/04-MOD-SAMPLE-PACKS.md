# Mod Sample Packs

Sample packs are test fixtures for validators.

## Good Packs

- `sample_house_good`: house with door, interior, collision, nav, light;
- `sample_npc_good`: NPC with talk state, schedule, role, faction;
- `sample_forge_good`: forge with input/output/heat/workspot;
- `sample_ui_good`: icons with alpha, states, runtime key labels;
- `sample_seed_good`: deterministic preview config.

## Bad Packs

- `bad_house_no_door`;
- `bad_npc_no_talk`;
- `bad_forge_no_output`;
- `bad_ui_baked_key_text`;
- `bad_asset_nan_bounds`;
- `bad_mod_missing_material`;
- `bad_perf_claim_no_before_after`.

Bad packs must fail validation. If they pass, the validator is broken.
