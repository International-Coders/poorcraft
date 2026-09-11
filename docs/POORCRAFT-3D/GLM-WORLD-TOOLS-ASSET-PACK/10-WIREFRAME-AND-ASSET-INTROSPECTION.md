# Wireframe And Asset Introspection

Wireframe capture and asset introspection let GLM see the game like a developer,
not just like a text parser.

## Wireframe Output

Accepted first formats:

- `.obj` for visible mesh export;
- `.json` for vertices/indices/bounds summaries;
- `.png` for wireframe overlay screenshot.

## Required Asset Metadata

- id;
- family;
- source/generator;
- triangle count;
- material names;
- sockets;
- interaction anchors;
- collision bounds;
- nav openings;
- gameplay semantic law;
- proof screenshots;
- tests;
- known deferrals.

## Required Wireframe Scenes

- house with doorway visible;
- NPC close-up with talk anchor;
- forge/workstation;
- town street with collision/nav openings;
- world preview terrain;
- UI overlay wireframe/bounds.

## Failure Conditions

- Doorway exists visually but collision blocks it.
- NPC has no talk anchor.
- Workstation has no use side.
- Mesh has inverted or missing faces.
- Asset has no scale convention.
- LOD loses gameplay-critical silhouette.
