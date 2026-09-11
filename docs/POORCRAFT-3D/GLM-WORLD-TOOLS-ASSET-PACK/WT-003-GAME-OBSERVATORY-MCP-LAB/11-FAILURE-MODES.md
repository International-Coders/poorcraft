# Failure Modes

Reject WT-003 work if:

- screenshots are generated without JSON state;
- JSON claims interactions that screenshots/routes do not prove;
- wireframe mode is a fake 2D effect unrelated to mesh edges;
- routes pass by bypassing input handling;
- missing runtime fields are omitted instead of exported as null with reason;
- performance data has no build hash, seed, viewport, or scene;
- vendor optimization is claimed without markers and before/after captures;
- MCP-style tools can mutate saves or listen publicly by default;
- evidence files are old but reused as fresh proof;
- checks produce only pass/fail without inconclusive/skipped-with-reason.
