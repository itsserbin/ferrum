# Settings UI Redesign + Gear Icon — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a gear icon button to the tab bar for opening settings, and redesign the settings overlay controls (steppers instead of sliders, dropdowns instead of segmented buttons, close button, improved colors).

**Architecture:** Two independent work streams — (A) gear icon in tab bar (geometry → hit test → rendering → event handling) and (B) settings overlay control redesign (layout structs → rendering → mouse handling). Both use the existing vector-drawing infrastructure.

**Tech Stack:** Rust, fontdue, wgpu (GPU), softbuffer (CPU), objc2 (macOS native toolbar)

---

## Part A: Gear Icon in Tab Bar

### Task 1: Add SettingsButton to TabBarHit + gear_button_rect geometry

**Files:**
- Modify: `src/gui/renderer/types.rs:182-199` (TabBarHit enum)
- Modify: `src/gui/renderer/shared/tab_math.rs:8-59` (constants), `:114-123` (tab_strip_start_x), `:192-203` (after pin_button_rect)

**Step 1: Add `SettingsButton` variant to `TabBarHit`**

In `src/gui/renderer/types.rs`, add after `PinButton` (line ~193):
```rust
/// Clicked on the settings gear button (non-macOS).
#[cfg(not(target_os = "macos"))]
SettingsButton,
```

**Step 2: Add constants to `tab_math.rs`**

After `PIN_BUTTON_GAP` constant (line ~54), add:
```rust
/// Size of the settings gear button (non-macOS).
#[cfg(not(target_os = "macos"))]
pub const GEAR_BUTTON_SIZE: u32 = 24;
/// Gap between gear button and next element (non-macOS).
#[cfg(not(target_os = "macos"))]
pub const GEAR_BUTTON_GAP: u32 = 8;
```

**Step 3: Add `gear_button_rect()` function**

After `pin_button_rect()` function, add:
```rust
/// Gear button sits to the right of pin button (non-macOS).
#[cfg(not(target_os = "macos"))]
pub fn gear_button_rect(m: &TabLayoutMetrics) -> Rect {
    let pin = pin_button_rect(m);
    let x = pin.x + pin.w + m.scaled_px(GEAR_BUTTON_GAP);
    let size = m.scaled_px(GEAR_BUTTON_SIZE);
    let y = (m.tab_bar_height.saturating_sub(size)) / 2;
    Rect { x, y, w: size, h: size }
}
```

**Step 4: Update `tab_strip_start_x()` to account for gear button**

The gear button goes after pin, before tab strip. Update `tab_strip_start_x()` to include gear button space. Currently it returns:
```rust
m.scaled_px(PIN_BUTTON_SIZE) + m.scaled_px(PIN_BUTTON_GAP) + platform_start
```
Change to:
```rust
m.scaled_px(PIN_BUTTON_SIZE) + m.scaled_px(PIN_BUTTON_GAP)
    + m.scaled_px(GEAR_BUTTON_SIZE) + m.scaled_px(GEAR_BUTTON_GAP)
    + platform_start
```

**Step 5: Build and verify**

Run: `cargo build && cargo build --no-default-features`

**Step 6: Commit**

```
feat: add gear button geometry and TabBarHit::SettingsButton
```

---

### Task 2: Gear icon geometry in ui_layout.rs

**Files:**
- Modify: `src/gui/renderer/shared/ui_layout.rs` (add after PinIconLayout section)

**Step 1: Define `GearIconLayout` struct**

After the `PinIconLayout` section (~line 70), add a new struct for a simple gear icon. The gear is drawn as a circle with 6 rectangular teeth around it, plus a hollow center:

```rust
/// Pre-computed gear icon layout.
pub struct GearIconLayout {
    /// Outer teeth: each tooth is a small filled rect (x, y, w, h).
    pub teeth: [(f32, f32, f32, f32); 6],
    /// Outer ring circle.
    pub ring_cx: f32,
    pub ring_cy: f32,
    pub ring_outer_radius: f32,
    pub ring_inner_radius: f32,
    /// Center hole circle.
    pub hole_cx: f32,
    pub hole_cy: f32,
    pub hole_radius: f32,
    /// Icon color.
    pub color: u32,
}
```

**Step 2: Create `gear_icon_layout()` function**

Computes a gear icon centered at `(cx, cy)` with given scale:

```rust
pub fn gear_icon_layout(
    cx: f32,
    cy: f32,
    icon_size: f32,
    color: u32,
) -> GearIconLayout {
    let outer_r = icon_size * 0.42;
    let inner_r = icon_size * 0.30;
    let hole_r = icon_size * 0.14;
    let tooth_w = icon_size * 0.16;
    let tooth_h = icon_size * 0.14;

    let mut teeth = [(0.0f32, 0.0f32, 0.0f32, 0.0f32); 6];
    for (i, tooth) in teeth.iter_mut().enumerate() {
        let angle = (i as f32) * std::f32::consts::TAU / 6.0;
        let tx = cx + angle.cos() * (outer_r + tooth_h * 0.3);
        let ty = cy + angle.sin() * (outer_r + tooth_h * 0.3);
        *tooth = (tx - tooth_w / 2.0, ty - tooth_h / 2.0, tooth_w, tooth_h);
    }

    GearIconLayout {
        teeth,
        ring_cx: cx,
        ring_cy: cy,
        ring_outer_radius: outer_r,
        ring_inner_radius: inner_r,
        hole_cx: cx,
        hole_cy: cy,
        hole_radius: hole_r,
        color,
    }
}
```

**Step 3: Build**

Run: `cargo build`

**Step 4: Commit**

```
feat: add gear icon geometry computation
```

---

### Task 3: Add gear hit testing

**Files:**
- Modify: `src/gui/renderer/shared/tab_hit_test.rs:15-63` (hit_test_tab_bar function)

**Step 1: Add gear button hit test**

In `hit_test_tab_bar()`, after the pin button check (line ~37) and before the plus button check, add:

```rust
#[cfg(not(target_os = "macos"))]
{
    let gear_rect = tab_math::gear_button_rect(m);
    if tab_math::point_in_rect(x, y, gear_rect.to_tuple()) {
        return TabBarHit::SettingsButton;
    }
}
```

**Step 2: Add `gear_button_rect` to the `Renderer` trait**

In `src/gui/renderer/traits.rs`, after `pin_button_rect()` (~line 228), add:

```rust
#[cfg(not(target_os = "macos"))]
fn gear_button_rect(&self) -> (u32, u32, u32, u32) {
    let m = self.tab_layout_metrics();
    tab_math::gear_button_rect(&m).to_tuple()
}
```

**Step 3: Add `gear_button_rect` dispatch to `RendererBackend`**

In `src/gui/renderer/backend.rs`, after `hit_test_tab_bar()`:

```rust
#[cfg(not(target_os = "macos"))]
pub fn gear_button_rect(&self) -> (u32, u32, u32, u32) {
    self.as_renderer().gear_button_rect()
}
```

**Step 4: Build and run tests**

Run: `cargo build && cargo test`

**Step 5: Commit**

```
feat: add gear button hit testing
```

---

### Task 4: Draw gear button in tab bar

**Files:**
- Modify: `src/gui/renderer/tab_bar/buttons.rs` (add draw_gear_button method)
- Modify: `src/gui/renderer/tab_bar/mod.rs` (call draw_gear_button in draw sequence)

**Step 1: Add `draw_gear_button()` in `buttons.rs`**

Follow the same pattern as `draw_pin_button()`:

```rust
/// Draws the settings gear button in the tab bar (non-macOS).
#[cfg(not(target_os = "macos"))]
pub(super) fn draw_gear_button(
    &self,
    target: &mut RenderTarget<'_>,
    mouse_pos: (f64, f64),
    settings_open: bool,
) {
    use super::super::shared::{tab_math, ui_layout};

    let rect = tab_math::gear_button_rect(&self.tab_layout_metrics());
    let (rx, ry, rw, rh) = rect.to_tuple();
    let hovered = tab_math::point_in_rect(mouse_pos.0, mouse_pos.1, (rx, ry, rw, rh));

    // Hover/active background
    if hovered || settings_open {
        let bg = if settings_open {
            self.palette().active_accent.to_pixel()
        } else {
            self.palette().inactive_tab_hover.to_pixel()
        };
        self.draw_rounded_rect(target, rx, ry, rw, rh, 4, bg, if settings_open { 60 } else { 220 });
    }

    let icon_color = if hovered || settings_open {
        self.palette().tab_text_active.to_pixel()
    } else {
        self.palette().tab_text_inactive.to_pixel()
    };
    let icon_size = rw as f32;
    let cx = rx as f32 + rw as f32 / 2.0;
    let cy = ry as f32 + rh as f32 / 2.0;
    let layout = ui_layout::gear_icon_layout(cx, cy, icon_size, icon_color);

    self.draw_gear_icon(target, &layout);
}
```

Then add `draw_gear_icon()` that draws the gear from layout (filled circle for ring, clear circle for hole, filled rects for teeth).

**Step 2: Call from `draw_tab_bar()` in `tab_bar/mod.rs`**

After the pin button draw call (line ~107), add:
```rust
#[cfg(not(target_os = "macos"))]
self.draw_gear_button(target, mouse_pos, settings_open);
```

Note: need to pass `settings_open: bool` through `draw_tab_bar()` — add it as a parameter. It comes from `FerrumWindow::settings_overlay.is_some()`.

**Step 3: Update `draw_tab_bar()` signature in Renderer trait**

Add `settings_open: bool` parameter to `draw_tab_bar()` in `traits.rs` and all callers (`render_shared.rs`, `FrameParams`).

**Step 4: Build and verify**

Run: `cargo build && cargo build --no-default-features`

**Step 5: Commit**

```
feat: render gear icon in tab bar with hover state
```

---

### Task 5: Handle gear button click

**Files:**
- Modify: `src/gui/events/mouse/tab_bar.rs` (add SettingsButton match arm)

**Step 1: Add match arm in `handle_tab_bar_left_click()`**

After the `PinButton` match arm (line ~150), add:
```rust
#[cfg(not(target_os = "macos"))]
TabBarHit::SettingsButton => {
    self.last_topbar_empty_click = None;
    self.last_tab_click = None;
    self.toggle_settings_overlay(config);
}
```

**Step 2: Build and test**

Run: `cargo build && cargo test`

**Step 3: Commit**

```
feat: handle gear button click to toggle settings
```

---

### Task 6: macOS native toolbar gear button

**Files:**
- Modify: `src/gui/platform/macos.rs` (or `macos/pin.rs` depending on structure)

**Step 1: Explore the exact macOS toolbar setup**

Read `src/gui/platform/macos.rs` (or the `pin.rs` sub-module). The existing setup adds a pin button via `NSTitlebarAccessoryViewController`. Add a second button with SF Symbol `"gearshape"`.

**Step 2: Add gear button to toolbar setup**

In `setup_toolbar()`, after creating the pin button, create a second NSButton:
```rust
let gear_image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
    &NSString::from_str("gearshape"),
    None,
);
// Create button, set target/action, add to toolbar
```

**Step 3: Add click handler**

Create `ferrumGearButtonClicked:` selector that posts a user event or directly toggles settings.

**Step 4: Build on macOS**

Run: `cargo build` (macOS only)

**Step 5: Commit**

```
feat: add native gear button to macOS toolbar
```

---

## Part B: Settings Overlay Redesign

### Task 7: Add dropdown state to SettingsOverlay

**Files:**
- Modify: `src/gui/settings/overlay.rs:66-79` (SettingsOverlay struct)

**Step 1: Add `open_dropdown` field**

```rust
/// Index of the currently open dropdown (if any).
pub open_dropdown: Option<usize>,
/// Hovered option within an open dropdown.
pub hovered_dropdown_option: Option<usize>,
```

Initialize both to `None` in `SettingsOverlay::new()`.

**Step 2: Update tests**

Fix any tests that assert field counts if needed.

**Step 3: Build**

Run: `cargo build && cargo test`

**Step 4: Commit**

```
feat: add dropdown state to SettingsOverlay
```

---

### Task 8: Redesign layout structs for stepper and dropdown

**Files:**
- Modify: `src/gui/settings/layout.rs:48-69` (ItemControlLayout, EnumButtonLayout)

**Step 1: Replace `ItemControlLayout` enum variants**

Replace the current `Slider` and `EnumButtons` variants with `Stepper` and `Dropdown`:

```rust
pub(in crate::gui) enum ItemControlLayout {
    /// Stepper: [-] value [+] for numeric values.
    Stepper {
        minus_btn: RoundedRectCmd,
        minus_text: TextCmd,
        value_text: TextCmd,
        plus_btn: RoundedRectCmd,
        plus_text: TextCmd,
    },
    /// Dropdown: button with current value + arrow, expandable list.
    Dropdown {
        button: RoundedRectCmd,
        button_text: TextCmd,
        arrow_text: TextCmd,
        /// Populated only when this dropdown is open.
        options: Vec<DropdownOptionLayout>,
    },
}
```

**Step 2: Add `DropdownOptionLayout` struct**

```rust
pub(in crate::gui) struct DropdownOptionLayout {
    pub bg: FlatRectCmd,
    pub text: TextCmd,
    pub is_selected: bool,
    pub is_hovered: bool,
}
```

**Step 3: Add close button to `SettingsOverlayLayout`**

Add field:
```rust
pub close_button: RoundedRectCmd,
pub close_icon_line_a: (f32, f32, f32, f32),
pub close_icon_line_b: (f32, f32, f32, f32),
pub close_icon_color: u32,
```

**Step 4: Remove old `EnumButtonLayout` struct and `SLIDER_TRACK_HEIGHT`, `ENUM_BUTTON_RADIUS` constants**

Replace with:
```rust
const STEPPER_BTN_SIZE: u32 = 20;
const DROPDOWN_HEIGHT: u32 = 24;
const DROPDOWN_OPTION_HEIGHT: u32 = 24;
```

**Step 5: Build (will have errors — layout computation needs updating next)**

Run: `cargo build` — expect compile errors in `compute_settings_layout()` and renderers.

**Step 6: Commit**

```
refactor: replace slider/enum layout with stepper/dropdown structs
```

---

### Task 9: Update layout computation for stepper and dropdown

**Files:**
- Modify: `src/gui/settings/layout.rs:113-273` (compute_settings_layout function)

**Step 1: Rewrite item layout computation**

In the item loop of `compute_settings_layout()`, replace the slider/enum logic with:

For `FloatSlider` / `IntSlider` / `LargeIntSlider` → `ItemControlLayout::Stepper`:
- `minus_btn`: rounded rect at controls_x, size STEPPER_BTN_SIZE
- `minus_text`: "-" centered in minus_btn
- `value_text`: formatted value, centered between buttons
- `plus_btn`: rounded rect after value, size STEPPER_BTN_SIZE
- `plus_text`: "+" centered in plus_btn

For `EnumChoice` → `ItemControlLayout::Dropdown`:
- `button`: rounded rect spanning controls area
- `button_text`: selected option text, left-aligned in button
- `arrow_text`: "v" right-aligned in button
- `options`: populated only if `overlay.open_dropdown == Some(item_index)` — list of `DropdownOptionLayout` below the button

**Step 2: Add close button computation**

At the end of layout computation, calculate close button position in top-right of panel:
```rust
let close_size = scaled(STEPPER_BTN_SIZE);
let close_x = panel_x + panel_w - close_size - scaled(INNER_PAD);
let close_y = panel_y + scaled(INNER_PAD);
```

**Step 3: Build**

Run: `cargo build` — renderers still need updating, but layout should compile.

**Step 4: Commit**

```
feat: compute stepper/dropdown/close button layout
```

---

### Task 10: Update CPU settings rendering

**Files:**
- Modify: `src/gui/renderer/settings.rs` (draw_settings_overlay)

**Step 1: Update control drawing loop**

Replace the slider/enum drawing with stepper/dropdown:

For `Stepper`: draw `minus_btn`, `minus_text`, `value_text`, `plus_btn`, `plus_text` using existing `draw_rounded_rect_cmd()` and `draw_text_cmd()`.

For `Dropdown`: draw `button`, `button_text`, `arrow_text`. If `options` is non-empty, draw the dropdown list on top (options backgrounds and texts).

**Step 2: Add close button drawing**

After the close hint, draw the close button X icon using `draw_stroked_line()`.

**Step 3: Build**

Run: `cargo build --no-default-features` (CPU path)

**Step 4: Commit**

```
feat: CPU renderer draws stepper/dropdown controls
```

---

### Task 11: Update GPU settings rendering

**Files:**
- Modify: `src/gui/renderer/gpu/settings.rs`

**Step 1: Mirror CPU changes for GPU path**

Same logic but using GPU draw commands (`push_rounded_rect`, `push_rect`, `push_text`).

**Step 2: Build**

Run: `cargo build` (GPU path)

**Step 3: Commit**

```
feat: GPU renderer draws stepper/dropdown controls
```

---

### Task 12: Update mouse handling for stepper and dropdown

**Files:**
- Modify: `src/gui/events/mouse/settings.rs`

**Step 1: Replace slider/enum click handling with stepper/dropdown**

In `handle_settings_left_click()`:

For `Stepper`:
- Hit test `minus_btn` → decrement value by step, call `apply_config_change()`
- Hit test `plus_btn` → increment value by step, call `apply_config_change()`

For `Dropdown`:
- Hit test `button` → toggle `overlay.open_dropdown`
- If dropdown is open, hit test each `option` → apply selection, close dropdown, call `apply_config_change()`

**Step 2: Add close button click handling**

Hit test the close button rect → call `close_settings_overlay()`.

**Step 3: Handle click-outside-dropdown**

If a dropdown is open and click is inside the panel but outside the dropdown, just close the dropdown (don't close the overlay).

**Step 4: Update `handle_settings_mouse_move()` for dropdown hover**

When a dropdown is open, update `hovered_dropdown_option` based on mouse position.

**Step 5: Extract stepper value mutation into helper**

Create `apply_stepper_change(&mut self, item_index: usize, delta: i8, items: &[SettingItem])` that reads the step from the `SettingItem` and increments/decrements, clamping to min/max.

**Step 6: Build and test**

Run: `cargo build && cargo test`

**Step 7: Commit**

```
feat: stepper/dropdown mouse interaction for settings overlay
```

---

### Task 13: Final polish and tests

**Files:**
- Modify: `tests/unit/core_terminal.rs` or `src/gui/settings/` (inline tests)

**Step 1: Add test for stepper value clamping**

Test that stepper doesn't go below min or above max.

**Step 2: Add test for dropdown state toggle**

Test that `open_dropdown` toggles correctly.

**Step 3: Run full verification**

```bash
cargo build
cargo build --no-default-features
cargo test
cargo clippy
```

**Step 4: Commit**

```
test: add stepper/dropdown settings tests
```

---

## Verification Checklist

1. `cargo build` — GPU build passes
2. `cargo build --no-default-features` — CPU build passes
3. `cargo test` — all tests pass
4. `cargo clippy` — no new warnings
5. Manual: gear icon visible in tab bar, hover highlights, click opens settings
6. Manual: stepper [-][+] changes values live
7. Manual: dropdown opens, shows options, selection changes live
8. Manual: close [x] button closes overlay
9. Manual: Esc still closes overlay
10. Manual: click outside panel closes overlay
11. Manual (macOS): gear button in native toolbar opens settings
