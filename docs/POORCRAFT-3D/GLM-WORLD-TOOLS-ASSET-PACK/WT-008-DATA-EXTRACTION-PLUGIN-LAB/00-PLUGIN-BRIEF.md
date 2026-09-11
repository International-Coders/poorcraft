# Plugin Brief

The owner explicitly authorizes tools, mods, and plugins that help GLM inspect
the running game. WT-008 defines what those tools may export and how they must
stay safe.

## Allowed

- local CLI exporters;
- local loopback MCP-style endpoints;
- debug-only plugin modules;
- sample mod packs;
- JSON/CSV/PNG/OBJ/GLB evidence output;
- local traces, metrics, and logs;
- validators that reject broken content.

## Purpose

- understand the actual runtime state;
- prove or disprove claims;
- help GLM compare screenshots, wireframes, and data;
- make asset generation and feature expansion measurable;
- catch fake houses, fake NPCs, fake forges, fake UI, and fake optimizations.

## Not Allowed By Default

- network upload;
- public listeners;
- reading unrelated user files;
- storing secrets;
- mutating player saves during tests;
- weakening gates to make extraction look green.
