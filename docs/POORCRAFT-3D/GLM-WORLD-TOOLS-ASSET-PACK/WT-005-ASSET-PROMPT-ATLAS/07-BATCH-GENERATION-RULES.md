# Batch Generation Rules

Batch generation is allowed, but acceptance must be strict.

## Batch Shape

- choose one asset family;
- choose variant axes;
- generate source assets;
- write manifest rows;
- run import validation;
- capture screenshots/wireframes/overlays;
- run semantic tests;
- accept, reject, or defer each row.

## Batch Sizes

- first batch: 7 assets, one per core category;
- second batch: 30 assets, enough to exercise variants;
- mature batch: 100+ assets only after validators are green;
- massive batches: require runtime consumers and perf budget first.

## Rejection Sampling

Every batch must include deliberate bad examples in tests:

- house without door;
- NPC without talk anchor;
- forge without output;
- UI icon without alpha;
- mesh with non-finite bounds;
- material id missing;
- texture over budget.

Validators must reject them.
