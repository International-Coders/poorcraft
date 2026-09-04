# All the Mods Research and Sandbox Balance

## Scope

POORCRAFT 3D is not an All the Mods remake and must not copy its names,
recipes, assets, scripts, progression tables, or content. This document takes
only general product and engineering lessons from public descriptions of ATM6
and ATM7.

## What the research actually supports

ATM6 publicly described itself as a very large pack spanning exploration,
magic, technology, and multiplayer, with a named endgame objective. ATM7 is
also described by its creators as a broad pack combining familiar and newer
mods while pursuing stability. Its public repository includes configuration,
default configuration, data/resource, client scripts, server scripts, and
startup scripts. That is evidence of a pack-level integration layer rather
than a pile of wholly independent additions.

Sources consulted on 2026-09-03:

- [ATM6 on CurseForge](https://www.curseforge.com/minecraft/modpacks/all-the-mods-6)
- [ATM7 on CurseForge](https://www.curseforge.com/minecraft/modpacks/all-the-mods-7)
- [ATM6 repository](https://github.com/AllTheMods/ATM-6)
- [ATM7 repository](https://github.com/AllTheMods/ATM-7)
- [ATM7 KubeJS layout](https://github.com/AllTheMods/ATM-7/tree/Staging/kubejs)

## What POORCRAFT 3D should take

### Breadth with understandable entry points

Several deep paths can coexist: technology, magic, exploration, trade,
building, settlement, and empire. A player does not need to use all of them.
The game must make the next possible step discoverable through the world,
crafting book, NPC requests, landmarks, journals, and a city-needs screen.

### An integration layer is necessary

All content must use shared definitions for materials, power/capabilities,
recipes, ownership, world events, and save/network identity. A new machine or
spell should not be allowed to invent a private resource format that bypasses
the rest of the game.

### Soft guidance, not a railroad

POORCRAFT 3D should not force a player through one “correct” chain. Instead,
the city, companions, faction opportunities, environmental conditions, and
personal goals create **opportunity threads**:

- a hungry city requests food, fishing, farms, trade, or magic as possible
  answers;
- a dangerous river invites a bridge, ferry, dam, diversion, or route change;
- a weak wall invites construction, diplomacy, mercenaries, industrial arms,
  magic, or a different political choice;
- a magical discovery can lead to scholarship, danger, faction conflict, or
  dragon-related play.

The player chooses the solution. A request tells them why something matters;
it does not turn the sandbox into a mandatory checklist.

## Original balance model: power has five costs

There is deliberately no universally strongest branch. Each powerful action is
evaluated through five dimensions:

| Dimension | Question |
|---|---|
| Capability | What new problem can this solve? |
| Materials | What rare/common resources, tools, or labor does it require? |
| Infrastructure | What location, network, buildings, people, or maintenance are required? |
| Risk | What can fail, hurt people, attract danger, pollute, destabilize, or create obligations? |
| Social/world consequence | How do factions, NPCs, territory, trade, ecology, or war respond? |

This lets a nuclear reactor, a dragon pact, a magical ritual, a factory, and a
merchant network all be extremely powerful in different ways without deciding
that one is simply “the endgame.”

### Example: open-core nuclear power

An open-core reactor may produce exceptional capability, but it requires fuel,
specialized construction, protection, maintenance, staff or automation, and a
safe exclusion zone. Radiation can make nearby work, travel, housing, farming,
or diplomacy difficult until the player invests in suits, shielding,
containment, cleanup, or chooses a remote site. This is not a punishment for
choosing engineering; it creates interesting engineering, civic, and political
problems.

### Example: magic power

An advanced ritual can solve travel, defense, farming, discovery, or combat
problems, but may require a rare site, seasonal/event conditions, trained NPCs,
faction approval, ritual reagents, a protected sanctuary, or create a visible
magical consequence. It should open different doors rather than be a slower
version of a machine.

## Anti-patterns to avoid

- A single ultimate recipe that invalidates the entire world after completion.
- Hard locking every player behind the same technology order.
- Hundreds of items with no world role or cross-system interaction.
- A city management screen that replaces visible citizens and construction.
- A dangerous system whose only consequence is a smaller UI number.
- A guide that overwhelms a new player with every branch on the first day.

## POORCRAFT 3D balance rule

Balance **options**, not identical damage/output values. Each path must have a
reason to exist in personal survival, settlement needs, faction politics, and
empire growth. The player can specialize, hybridize, ignore paths, or change
direction over time. The world should answer the player’s choices with new
possibilities and costs, not announce that they selected the wrong build.
