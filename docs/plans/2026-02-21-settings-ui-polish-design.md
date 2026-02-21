# Settings UI Polish — Design Document

## Goal

Polish the settings overlay to be visually consistent with the terminal's existing UI elements (tab bar, security popup, drag overlay). Add hover effects, drop shadow, and theme-consistent styling.

## Approach

Minimal changes to existing structure. Reuse visual patterns already in the codebase:
- Drop shadow: black rect +2px offset at 24% opacity (from tab drag overlay)
- Hover opacity: lerped values on interactive elements (from tab bar)
- Border: accent-colored instead of white (from security popup pattern)
- All colors from ThemePalette — both themes automatically consistent

## Changes

### 1. Panel Shadow
Draw black RoundedRectCmd at +2px y-offset, 24% opacity before main panel. Same pattern as `gpu/overlays.rs` tab drag ghost.

### 2. Panel Border Color
Change from white (0xFFFFFF) at 0.078 opacity → `active_accent` at 0.12 opacity.

### 3. Stepper Button Hover
- Normal: `bar_bg` at 0.6 opacity
- Hovered: `bar_bg` at 1.0 opacity
- New state: `hovered_stepper: Option<(usize, StepperHalf)>` on SettingsOverlay

### 4. Dropdown Button Hover
- Normal: `bar_bg` at 0.6
- Hovered: `bar_bg` at 0.85

### 5. Close Button (X) Hover
- Normal: `bar_bg` at 0.6
- Hovered: `close_hover_bg` at 0.8

### 6. Active Category Indicator
Left vertical bar 2px wide, `active_accent` color, next to active category row.

### 7. Item Row Hover
Full-width background highlight `bar_bg` at 0.2 on hovered item row.

## Non-Goals
- No animated transitions (hover_progress) — keep simple static hover
- No layout structure changes
- No new ThemePalette colors
