# Failure Recovery

Failure is useful if it changes the next action.

## When A Test Fails

1. Read the failing assertion.
2. Reproduce with the smallest command.
3. Inspect code and evidence.
4. Fix the cause, not the assertion text.
5. Re-run the failing test.
6. Re-run the broader guard.

## When A Screenshot Fails

1. Open or inspect the PNG.
2. Compare layout JSON to pixels.
3. Check viewport, DPI, build hash, seed, and route.
4. Fix clipping, missing draw, wrong state, stale evidence, or bad test.
5. Regenerate evidence.

## When A Feature Is Too Large

Split it into the smallest vertical slice that changes real gameplay and has a
proof command. Write the rest as explicit deferrals.
