# STATUS.md

## Current Milestone: Completion & Industrial Expansion (P12–P20) — COMPLETE

121 tests passing; 14 real proof scenes in shots/vistest_*.png. The game
now spans: title screen -> 30-biome world with weather/clouds/sun/moon ->
survival (bow/arrows, armor, XP, smithing minigame) -> villages with
trading villagers and lore books -> industrial tier (copper/tin/bauxite/
sulfur ores, coal generators powering electric furnaces/crushers/
assemblers) -> research eras gated by a tech-tree screen with live
have/need costs -> compute-shader voxel path tracing (soft sun shadows,
one-bounce GI, torch emissive; R captures in-game) -> Steam-ready
deployment (Spacewar 480 dev loop, feature-gated transport, depot docs).

Multiplayer: dedicated UDP server with chat/block sync + Steam P2P
transport option. Mods: runtime block/item/recipe/smelting/ore veins.

### Deferred polish (tracked in BACKLOG.md, honestly)
Sound/music (kira), beds/doors, A* mob pathing, live RT view toggle
(R captures instead), PT-BR localization, key rebinding, server browser.

Full history in CHANGELOG.md, including the loop-282 audit that started
this rebuild and every bug the pixel-proof discipline caught since.
