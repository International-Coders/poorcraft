# POORCRAFT 3D Design Constitution

This document is the center of POORCRAFT 3D. When a new idea, model prompt,
or implementation shortcut conflicts with this document, stop and resolve the
conflict before adding more code.

## The game in one sentence

POORCRAFT 3D is a first-person voxel fantasy-industrial sandbox where one
player can grow from a vulnerable explorer into the founder, ruler, ally, or
enemy of living settlements and empires.

## The game is not

- a Minecraft clone with smoother terrain;
- a factory game whose only answer is more automation;
- a castle builder full of empty NPC decorations;
- an RTS map disconnected from the first-person world;
- a survival grind with no social or political consequence;
- a collection of unrelated “cool features” added because they can be made.

## The player fantasy

The player should feel that the world is open, legible, and increasingly theirs
to influence. They can choose who they become:

- a wandering hunter, explorer, fisher, builder, or dungeon delver;
- a mage who commands ancient places, creatures, and ritual systems;
- an engineer who builds waterworks, factories, valve computers, and power
  networks;
- a caravan leader, diplomat, merchant, spy, rebel, or faction champion;
- a ruler who creates a settlement, castle, army, economy, and eventually an
  empire.

None of these identities is a cosmetic class selection. Each changes what the
player does in the world and what the world asks of them.

## Non-negotiable experiences

The beta cannot call itself the intended game unless a player can:

1. Leave a persistent physical mark through building, digging, shaping land,
   diverting water, and constructing useful things.
2. Find natural places that matter: rivers, caves, mountains, roads, ruins,
   magic sites, and faction territory.
3. Choose a meaningful early direction without permanently losing the ability
   to hybridize later.
4. Meet people who have jobs, homes, interests, local knowledge, and credible
   reactions to what happens nearby.
5. Visit a castle that is a working political place, not a tiny procedural
   wall around vendors.
6. Build toward a settlement or empire goal that changes the world beyond the
   player’s personal inventory.
7. Play alone for a long time without hitting a “bring friends or stop” wall.
8. Invite friends into the same authoritative world without duplicate,
   contradictory, or client-only simulation.

## Design laws

### Every system needs a reason in the world

If a mechanic exists only as a menu value, recipe ingredient, or abstract
timer, ask whether it should be visible as a place, person, object, route, or
event in the world.

### Every feature must connect to at least two layers

For example, fishing is not just a minigame: it can connect rivers, food,
trade, NPC jobs, faction culture, travel, and settlement economy. A feature
that only connects to one isolated screen needs a strong reason to exist.

### Progress unlocks decisions, not merely output rate

The reward for building a mill should be new goods, services, routes, people,
or political possibilities—not only a faster number. The reward for gaining a
castle should be responsibilities and choices as well as power.

### Simulation earns trust through clarity

Game abstractions are allowed. Water may provide non-destructive independent
power to several wheels. But the rule must be visible, stable, explainable,
and consistent in solo and multiplayer.

### Depth arrives before catalog size

One living capital, one working river chain, and one meaningful magic/industry
choice are better than dozens of decorative machines, empty castles, or
unconnected spells.

### Performance is part of the fantasy

The game should run well on ordinary computers. A feature that destroys travel
smoothness, loading, server stability, or battery/thermal behavior is not done
because it looks impressive in a screenshot.

## Feature admission filter

Before accepting any feature, answer all five questions:

1. Which player fantasy or non-negotiable experience does it strengthen?
2. Which two or more game layers does it connect?
3. What visible cause and effect teaches the player how it works?
4. What is its bounded CPU, GPU, memory, save, and network cost?
5. What player story becomes possible that was impossible before?

If the answer is “it is cool” but not the other four answers, record it as an
idea rather than beginning implementation.

## End-state feeling

After many hours, a player should be able to look over a changed landscape and
recognize a personal history: a river they redirected, a town they helped grow
or defeated, a faction whose laws changed because of them, a dragon they made
an ally or threat, an industrial network or magical domain they built, and an
empire whose character reflects their choices.
