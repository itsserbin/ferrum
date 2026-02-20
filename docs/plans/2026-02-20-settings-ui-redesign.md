# Settings UI Redesign + Gear Icon

## Summary

Two improvements to the settings system:
1. Add a gear icon button in the tab bar to open settings
2. Redesign the settings overlay: replace sliders with steppers, enum buttons with dropdowns, improve colors

## 1. Gear Icon in Tab Bar

### Windows/Linux (custom tab bar)
- 24px gear icon button, positioned **right of pin button** before the tab strip
- Vector-drawn gear: 6-tooth outer ring + inner circle (same style as existing pin/plus/close icons)
- Hover state: rounded rect background (same as pin/plus buttons)
- New `TabBarHit::SettingsButton` variant for hit testing
- Click opens/closes settings overlay (same as Cmd/Ctrl+,)

### macOS (native toolbar)
- Add native toolbar button via objc2 with SF Symbol `gear`
- Placed alongside existing pin button in the toolbar
- Pin button stays in toolbar; gear is added next to it
- Click triggers same `toggle_settings_overlay()` action

## 2. Settings Overlay Redesign

### Layout
Keep the existing sidebar + content two-column layout, but redesign the controls:

```
+----------------------------------------------+
|              Settings                    [x] |
|----------------------------------------------|
|  Font     |  Font Size       [-] 14.0 [+]   |
|  Theme    |  Font Family  [JetBrains Mono v] |
|  Terminal |  Line Padding    [-]  2   [+]    |
|  Layout   |                                  |
|           |                                  |
|           |                    Esc to close   |
+----------------------------------------------+
```

### Control Types

**Stepper (replaces sliders for numeric values):**
- `[-]` button, value display, `[+]` button
- Click -/+ changes value by the configured step
- Value displayed centered between buttons
- Button size: ~20x20px, rounded corners
- Hold/repeat: single step per click (no continuous drag)

**Dropdown (replaces segmented buttons for enums):**
- Button showing current selection + down arrow `v`
- Click opens a dropdown list below the button
- Dropdown list: each option as a row, highlighted on hover
- Selected option has accent indicator
- Click outside or select an option closes the dropdown
- Needs new state: `open_dropdown: Option<usize>` on SettingsOverlay

**Close button [x]:**
- Small X button in top-right corner of the panel
- Same visual style as tab close button

### Colors
- Panel background: `menu_bg` with solid opacity (no transparency of underlying content)
- Dim overlay: full-screen semi-transparent black (50% alpha)
- Stepper buttons [-][+]: `bar_bg` background, `active_accent` on hover
- Dropdown button: `bar_bg` background, `active_accent` border on hover
- Dropdown list: `menu_bg` background, accent highlight on selected/hovered item
- Category sidebar active: `active_accent` at 20% opacity
- Category sidebar hover: subtle `bar_bg` highlight
- Section separators: accent at 15% opacity
- Close button [x]: same style as tab close button hover circle

### Interaction Changes
- Dropdown needs click-outside-to-close handling (separate from panel click-outside)
- Stepper +/- buttons need per-setting step values (already defined in SettingItem)
- Close [x] button calls `close_settings_overlay()`

## Files to Modify

### Gear Icon
- `src/gui/renderer/shared/ui_layout.rs` -- gear icon geometry
- `src/gui/renderer/shared/tab_math.rs` -- gear button rect, reserved space
- `src/gui/renderer/shared/tab_hit_test.rs` -- gear button hit testing
- `src/gui/renderer/tab_bar/buttons.rs` -- gear icon drawing
- `src/gui/renderer/tab_bar/mod.rs` -- integrate gear into tab bar draw
- `src/gui/renderer/mod.rs` -- TabBarHit::SettingsButton variant
- `src/gui/events/mouse/input.rs` -- handle gear button click
- `src/gui/platform/macos.rs` -- native toolbar gear button (objc2)

### Settings Overlay Redesign
- `src/gui/settings/overlay.rs` -- add `open_dropdown: Option<usize>` state
- `src/gui/settings/layout.rs` -- new layout structs for stepper/dropdown/close button
- `src/gui/renderer/settings.rs` -- CPU rendering of new controls
- `src/gui/renderer/gpu/settings.rs` -- GPU rendering of new controls
- `src/gui/events/mouse/settings.rs` -- stepper/dropdown mouse handling
- `src/gui/events/keyboard/settings.rs` -- keyboard navigation updates
