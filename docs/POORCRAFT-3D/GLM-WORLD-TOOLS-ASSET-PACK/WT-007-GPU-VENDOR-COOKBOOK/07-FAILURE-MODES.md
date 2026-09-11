# Failure Modes

Reject GPU work if:

- marker names are missing or unstable;
- screenshots are stale;
- p50/p95 are missing;
- scene seed or viewport differs between before and after;
- UI pass disappears from captures;
- upscaler hides unreadable UI;
- vendor SDK support is assumed without a spike;
- shader profiling is claimed without pipeline/debug-info status;
- a faster frame breaks gameplay, collision, input, or screenshots;
- "AMD/NVIDIA optimized" appears with no capture bundle.
