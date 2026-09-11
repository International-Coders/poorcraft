# Comparison And Regression Gates

Screenshots are evidence only when compared against rules.

## Comparison Modes

- `same_seed_same_build`: output should be byte-stable or digest-stable;
- `same_seed_new_build`: output may change only with an expected change note;
- `different_seed`: seed preview/world data must change meaningfully;
- `before_after_fix`: target bug evidence must improve and unrelated gates
  must stay green;
- `vendor_before_after`: perf data must improve without visual/interaction
  regressions.

## Required Verdicts

- `pass`;
- `fail`;
- `inconclusive`;
- `skipped_with_reason`.

`inconclusive` is not green. It exists to stop false confidence.

## Evidence Bundle

Each regression gate writes a bundle:

- manifest JSON;
- input route JSON;
- runtime state JSON;
- screenshots;
- wireframes/overlays;
- metrics;
- diff images;
- verdict JSON.
