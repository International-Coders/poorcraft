# Implementation Queue

Execute these in order unless the owner explicitly redirects.

## UI-001 Screenshotable UI State Harness

Add deterministic UI state screenshots for title, pause, gameplay HUD, settings,
and save/load. Include pixel checks and layout dumps.

## UI-002 Runtime UI Draw List

Create a draw-list abstraction for panels, buttons, text, icons, bars, hotbar
slots, and toasts. It may start simple, but it must be screenshotable.

## UI-003 Real Title And Pause Screens

Replace the temporary bitmap owner overlay with real menu panels and buttons.
Keep Escape/Q behavior correct.

## UI-004 HUD Bars And Hotbar

Implement health/stamina/food debug-backed bars, 9-slot hotbar, selected slot,
crosshair, and toasts.

## UI-005 Settings And Controls

Add mouse sensitivity, invert Y, FOV, UI scale, quality preset, and visible key
binding summary.

## UI-006 Save Slot UI

Add save-slot browser, save confirmation, load confirmation, delete
confirmation, and test-only save directory support.

## UI-007 Inspector Tool

Add local JSON commands for UI state, screenshot capture, layout dump, frame
stats, and input replay.

## UI-008 Visual Polish Pass

Import/slice panel, keycap, icon, and bar assets or recreate them as
renderer-native primitives. Prove with screenshots.
