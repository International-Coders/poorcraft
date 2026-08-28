# World Creation Flow — Reference Document

## The problem this solves

Currently the game goes from "New World" to "playing" with no player
choices. This means: random seed they can't see, can't control the mode,
can't name their world. The flow below gives players just enough control
to feel ownership without overwhelming them with options.

## Design principle: progressive disclosure

The world creation screen shows the most important choices prominently
and hides the less important ones. The order of importance is:
1. World name (ownership — this is *their* world)
2. Seed (reproducibility — they might want a specific world)
3. World type (terrain shape — affects the whole experience)
4. Game mode (survival vs creative — changes what's possible)
5. Difficulty (adjustable later, so least important here)

## Full screen layout

```
┌─────────────────────────────────────────────────────────────────┐
│  [dark vignette background, same world render as title screen]  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  New World                                                │  │
│  │  ─────────────────────────────────────────────────────   │  │
│  │                                                           │  │
│  │  Name                                                     │  │
│  │  [World 1                                              ]  │  │
│  │                                                           │  │
│  │  Seed                                                     │  │
│  │  [14203847923                                      ] [🎲] │  │
│  │                                                           │  │
│  │  World Type                                               │  │
│  │  [  Normal  ] [  Superflat  ] [  Amplified  ]            │  │
│  │                                                           │  │
│  │  Game Mode                                                │  │
│  │  [  Survival  ] [  Creative  ]                           │  │
│  │                                                           │  │
│  │  Difficulty                                               │  │
│  │  [ Peaceful ] [  Easy  ] [ Normal ] [  Hard  ]           │  │
│  │                                                           │  │
│  │                          ← Back       Create World →      │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Panel styling

The form panel uses:
- Background: `#332a1c` (background-panel from the design palette)
- Border: 1px `#4a3f2e` (border/line) on all sides
- Inner padding: 32px (4 × 8px grid unit)
- Panel width: 50% of screen width, centered horizontally
- Panel is vertically centered, not top-anchored

Title "New World" inside the panel:
- Same `#f0ead6` warm off-white as main menu
- 24pt (between body and title sizes — it's a panel header)
- The horizontal rule below is `border/line` color, 1px

## Input field styling

Text inputs:
- Background: `#1a1410` (deep background — slightly darker than the panel)
- Border: 1px `#4a3f2e`, brightening to `#c4602a` on focus
- Text: `#f0ead6`
- Placeholder: `#4a4438` (text-disabled)
- Padding: 8px horizontal, 8px vertical
- No rounded corners. Straight edges, matching the overall aesthetic.

The "🎲 Roll" button next to the seed input:
- Not a button widget — just a text label `[Roll]` in `text-muted` color
  that uses the same hover-underline behavior as the main menu buttons.
- On click: generates a new random u64, fills the seed field with it.

## Toggle button styling (World Type, Game Mode, Difficulty)

These are NOT checkboxes or radio buttons. They are a row of flat text
segments where the selected one is visually distinct:

```
Selected:    [███ Normal ███]   (background: #4a3f2e, text: #f0ead6)
Unselected:   [ Superflat  ]   (no background, text: #8a7f6e)
```

On hover of an unselected option: text brightens to `#f0ead6`.
The border between options: none — they are just text with consistent
padding, visually grouped by proximity.

## Confirmation and back navigation

"Create World" — right-aligned, same hover-underline style as main menu
buttons, BUT with the underline in accent color permanently visible (to
signal "this is an action, not just navigation"). On click: validate
inputs (name not empty, seed is parseable), then begin world generation
and transition into the game.

"Back" — left-aligned, same hover-underline style as main menu buttons,
standard accent underline only on hover.

## Error states

If "Create World" is clicked with invalid inputs:
- Empty name: the name input border flashes to `danger` (#8b2020) for
  500ms, then returns to normal. A one-line error message appears below
  the input in `danger` color: "World needs a name."
- Invalid seed (a non-numeric string after failed parse): convert the
  string to a seed by hashing it (same as the existing seed-from-string
  logic). Never show an error for an invalid seed — always hash strings.

## What's stored in the world save

The following are stored in the world's metadata (alongside the existing
world data format):
```
world_name: String
seed: u64
world_type: WorldType (Normal | Superflat | Amplified)
game_mode: GameMode (Survival | Creative)
difficulty: Difficulty (Peaceful | Easy | Normal | Hard)
created_at: timestamp (seconds since epoch)
last_played_at: timestamp (updated on every world load)
version_created: String (CARGO_PKG_VERSION at world creation)
```

Difficulty and game mode can be changed later in the in-game settings
(add a "World Settings" option to the pause menu). World type and seed
cannot be changed after creation (they determine the terrain).
