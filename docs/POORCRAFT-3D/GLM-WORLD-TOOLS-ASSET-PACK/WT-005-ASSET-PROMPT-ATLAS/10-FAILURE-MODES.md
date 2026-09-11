# Failure Modes

Reject WT-005 work if:

- prompts are generic and not POORCRAFT-specific;
- batch generation lacks validators;
- assets have no gameplay purpose;
- UI sprites bake key text instead of using runtime bindings;
- icons lack disabled/focus/hover states;
- GLB import succeeds but semantic sidecar is missing;
- material ids do not match the renderer catalog;
- texture compression is claimed without measuring memory/perf;
- prompt batches copy another game's identity;
- rejected assets are silently kept in the runtime catalog.
