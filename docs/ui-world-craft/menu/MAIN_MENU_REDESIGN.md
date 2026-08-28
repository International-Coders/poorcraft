# Main Menu Redesign — Reference Document

## The problem in one sentence

The current main menu looks like every other voxel game's title screen
because it makes the same default decisions: centered layout, rounded
rect buttons with a filled background, pure white text, the game's working
title in a default font. None of these are wrong in isolation. All of them
together produce "generic."

## What "not AI-generated" means for a game menu

Study the menus of games that are immediately, visually distinct:
- **Hollow Knight**: left-aligned, hand-painted background, nothing is
  perfectly symmetric, font has personality and imperfection.
- **Disco Elysium**: the title hangs in a strange position, the background
  art is deliberately ugly-beautiful, the buttons look like a physical
  object (a poster, a form).
- **Dwarf Fortress (newer UI)**: everything has weight, everything looks
  like it was made by a craftsperson who cared about tools.

LOREFORGE should feel like the third one: **a craftsperson's interface.**
Heavy. Real. A little rough at the edges. Not sterile.

## The LOREFORGE visual identity (apply consistently to every screen)

### Color palette
```
Background deep:     #1a1410  (very dark warm brown-black)
Background mid:      #2a2018  (slightly lighter, for panels)
Background panel:    #332a1c  (visible panel background)
Text primary:        #f0ead6  (warm off-white, like aged parchment)
Text muted:          #8a7f6e  (muted warm grey for secondary text)
Text disabled:       #4a4438  (very muted, for locked/unavailable)
Accent primary:      #c4602a  (ember-orange — same as Ember Covenant color)
Accent secondary:    #8b4513  (iron-brown — same as Ironborn)
Border/line:         #4a3f2e  (warm dark brown for dividers and borders)
Success:             #6b8e23  (earthy green — same as Free Holds)
Warning:             #c4a02a  (amber-gold)
Danger:              #8b2020  (dark red)
```

These are the ONLY colors that appear in the game's UI. `ui_kit.rs` must
define these as named constants. Any other color is a bug.

### Typography
- One font family throughout. Recommended: a slightly condensed, slightly
  heavy font with real character — NOT Roboto, NOT Open Sans, NOT any
  "clean" sans-serif. If no font is currently embedded, add one. Suggested
  options that are freely licensed: **Cinzel** (Roman-inspired, fits the
  medieval-industrial aesthetic), **Josefin Sans** (geometric but with
  personality), or **Liberation Serif** (accessible, readable, slightly
  formal).
- Font sizes: title = 48pt, subtitle = 14pt, body = 13pt, small = 11pt,
  micro = 9pt. These are the ONLY font sizes. No intermediate sizes.
- Line height: 1.4× for body text. 1.1× for titles (tighter).

### Spacing system
All spacing is a multiple of 8px: 8, 16, 24, 32, 48, 64. No 5px, no 10px,
no 15px. This is not visible to the player but produces visual consistency
that reads as "designed."

### Border/divider style
Thin single-pixel lines in `Border/line` (#4a3f2e). No thick borders.
No double borders. One divider between sections, not decorative flourishes.

## Title screen layout spec

```
┌─────────────────────────────────────────────────────────────────┐
│  [live rotating world render — fills 100% of screen]            │
│                                                                  │
│  [dark vignette fades in from all 4 edges — gradient, not hard] │
│                                                                  │
│  LOREFORGE                    (top-left, 10% from left, top 18%)│
│  Build. Rule. Endure.         (subtitle, below logotype, muted) │
│                                                                  │
│                                                                  │
│  New World                    (left col, 10% from left)         │
│  ─────────                    (underline, animates on hover)    │
│  Load World                                                      │
│  Multiplayer                                                     │
│  Settings                                                        │
│  Quit                                                            │
│                                                                  │
│                          LOREFORGE v0.x.x  (bottom-right, micro)│
└─────────────────────────────────────────────────────────────────┘
```

## Button interaction spec (precise)

```
Default state:
  text: #f0ead6, 16pt
  underline: none
  x-position: 10% from left edge

Hover state (transition: 120ms ease-out):
  text: #f0ead6 → #fff8ee (very subtle brightening)
  underline: appears, width animates from 0 to text-width
  underline color: #c4602a (accent primary)
  underline thickness: 1px
  x-position: shifts +4px (text moves right slightly)
  cursor: pointer

Active/click state (transition: 60ms ease-in):
  x-position: shifts back -2px
  text: #c4602a (briefly flashes accent color)
  duration: 60ms then transitions back to hover state before navigation

Disabled state:
  text: #4a4438 (text-disabled)
  underline: none
  no hover response
```

## What must NOT appear on the title screen

- Any UI panel with a solid background color (buttons float on the world)
- Pure white (#ffffff) text
- Pure black (#000000) anywhere
- Drop shadows
- Gradients on text
- Animated or blinking text
- The word "poorcraft" in any form
- A centered button column
- Any icon, logo, or symbol next to the button text
- Border around the button text

## Settings screen (brief spec, matches above visual language)

Settings opens as a fullscreen overlay (same vignette background), with:
- Left sidebar: settings categories (Graphics, Audio, Controls, Game,
  About) using the same hover-underline button style.
- Right panel: category settings displayed as labeled rows with controls
  (sliders, toggles, dropdowns) styled in the LOREFORGE palette.
- "Back" text-link bottom-left, same style as menu buttons.
- "Apply" button bottom-right, styled distinctly (accent color underline
  stays visible always, not just on hover, to indicate it's an action
  button not just navigation).
