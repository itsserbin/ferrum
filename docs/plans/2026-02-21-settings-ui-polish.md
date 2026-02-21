# Settings UI Polish — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Polish the settings overlay with drop shadow, accent border, hover effects on all interactive elements, active category indicator, and item row hover — consistent with existing terminal UI patterns.

**Architecture:** All 7 visual changes funnel through the shared layout computation in `layout.rs`. New hover state fields on `SettingsOverlay` are read by `compute_settings_layout()` to produce hover-aware opacities. Both CPU and GPU renderers iterate the layout structs unchanged — they only need to draw 2 new elements (shadow, category indicator, item row bg).

**Tech Stack:** Rust, existing `FlatRectCmd`/`RoundedRectCmd`/`TextCmd` types, `ThemePalette` colors.

---

### Task 1: Add hover state fields to SettingsOverlay

**Files:**
- Modify: `src/gui/settings/overlay.rs:1-99`

**Step 1: Add `StepperHalf` enum and new fields**

Add above the `SettingsOverlay` struct:

```rust
/// Which half of a stepper control the mouse is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui) enum StepperHalf {
    Minus,
    Plus,
}
```

Add three new fields to `SettingsOverlay`:

```rust
/// Hovered stepper button: (item_index, which half).
pub hovered_stepper: Option<(usize, StepperHalf)>,
/// Index of the item whose dropdown button is hovered.
pub hovered_dropdown: Option<usize>,
/// Whether the close (X) button is hovered.
pub hovered_close: bool,
```

Initialize them in `SettingsOverlay::new()`:

```rust
hovered_stepper: None,
hovered_dropdown: None,
hovered_close: false,
```

**Step 2: Run tests to verify nothing broke**

Run: `cargo test settings -- --nocapture`
Expected: All existing settings tests pass.

**Step 3: Commit**

```bash
git add src/gui/settings/overlay.rs
git commit -m "feat(settings): add hover state fields for stepper, dropdown, close button"
```

---

### Task 2: Track new hover states in mouse handler

**Files:**
- Modify: `src/gui/events/mouse/settings.rs:161-238`

**Step 1: Update `handle_settings_mouse_move` to track stepper/dropdown/close hover**

Import `StepperHalf` at the top of the file:

```rust
use crate::gui::settings::{SettingItem, SettingsCategory, StepperHalf};
```

In `handle_settings_mouse_move`, add new tracking variables after the existing `new_hovered_*` ones:

```rust
let mut new_hovered_stepper: Option<(usize, StepperHalf)> = None;
let mut new_hovered_dropdown: Option<usize> = None;
let mut new_hovered_close = false;
```

Add close button hover check (after the dropdown option hover loop, before the final update block):

```rust
// Check close button hover.
let cb = &layout.close_button;
if mx >= cb.x as f64
    && mx < (cb.x + cb.w) as f64
    && my >= cb.y as f64
    && my < (cb.y + cb.h) as f64
{
    new_hovered_close = true;
}
```

Add stepper and dropdown button hover inside the existing item hover loop. Replace the simple item hover loop with one that also checks controls:

```rust
// Check item hover + control hover.
for (i, item_layout) in layout.items.iter().enumerate() {
    let label_y = item_layout.label.y as f64;
    let row_bottom = label_y + self.backend.cell_height() as f64 * 2.5;
    if my >= label_y && my < row_bottom {
        new_hovered_item = Some(i);
    }

    match &item_layout.controls {
        ItemControlLayout::Stepper {
            minus_btn,
            plus_btn,
            ..
        } => {
            if hit_test_rounded_rect(minus_btn, mx, my) {
                new_hovered_stepper = Some((i, StepperHalf::Minus));
            } else if hit_test_rounded_rect(plus_btn, mx, my) {
                new_hovered_stepper = Some((i, StepperHalf::Plus));
            }
        }
        ItemControlLayout::Dropdown { button, .. } => {
            if hit_test_rounded_rect(button, mx, my) {
                new_hovered_dropdown = Some(i);
            }
        }
    }
}
```

Update the `changed` check and assignment block to include new fields:

```rust
let overlay = self.settings_overlay.as_mut().unwrap();
let changed = overlay.hovered_category != new_hovered_cat
    || overlay.hovered_item != new_hovered_item
    || overlay.hovered_dropdown_option != new_hovered_dropdown_opt
    || overlay.hovered_stepper != new_hovered_stepper
    || overlay.hovered_dropdown != new_hovered_dropdown
    || overlay.hovered_close != new_hovered_close;
overlay.hovered_category = new_hovered_cat;
overlay.hovered_item = new_hovered_item;
overlay.hovered_dropdown_option = new_hovered_dropdown_opt;
overlay.hovered_stepper = new_hovered_stepper;
overlay.hovered_dropdown = new_hovered_dropdown;
overlay.hovered_close = new_hovered_close;
```

**Step 2: Run tests**

Run: `cargo test -- --nocapture`
Expected: All tests pass.

**Step 3: Run clippy**

Run: `cargo clippy`
Expected: Zero warnings.

**Step 4: Commit**

```bash
git add src/gui/events/mouse/settings.rs
git commit -m "feat(settings): track stepper, dropdown, close button hover in mouse handler"
```

---

### Task 3: Panel shadow and accent border

**Files:**
- Modify: `src/gui/settings/layout.rs:8-35` (struct) and `src/gui/settings/layout.rs:162-191` (computation)
- Modify: `src/gui/renderer/settings.rs:24-28` (CPU render)
- Modify: `src/gui/renderer/gpu/settings.rs:27-38` (GPU render)

**Step 1: Add `panel_shadow` field to `SettingsOverlayLayout`**

In `layout.rs`, add after `dim_bg` field:

```rust
/// Drop shadow behind the panel (drawn before panel_bg).
pub panel_shadow: RoundedRectCmd,
```

**Step 2: Compute shadow in `compute_settings_layout`**

After the `panel_bg` rect computation (line ~181), before `panel_border`, add:

```rust
let shadow_offset = sp(2) as f32;
let panel_shadow = RoundedRectCmd {
    x: px + shadow_offset,
    y: py + shadow_offset,
    w: pw,
    h: ph,
    radius,
    color: 0x000000,
    opacity: 0.24,
};
```

**Step 3: Change panel border from white to accent**

Change the `panel_border` computation from:

```rust
color: 0xFFFFFF,
opacity: 0.078,
```

to:

```rust
color: palette_active_accent,
opacity: 0.12,
```

**Step 4: Add `panel_shadow` to the struct initialization**

In the `SettingsOverlayLayout { ... }` return block, add `panel_shadow,` after `dim_bg,`.

**Step 5: Render shadow in CPU renderer**

In `src/gui/renderer/settings.rs`, after `self.draw_flat_rect_cmd(target, &layout.dim_bg);`, add:

```rust
// Panel shadow.
self.draw_rounded_rect_cmd(target, &layout.panel_shadow);
```

**Step 6: Render shadow in GPU renderer**

In `src/gui/renderer/gpu/settings.rs`, after the dim background push_rect block, add:

```rust
// Panel shadow.
self.push_rounded_rect_cmd(&layout.panel_shadow);
```

**Step 7: Run tests**

Run: `cargo test -- --nocapture`
Expected: All pass. Existing `panel_border_matches_bg` test still passes (it checks position/size, not color).

**Step 8: Run clippy on both feature sets**

Run: `cargo clippy && cargo clippy --no-default-features`
Expected: Zero warnings.

**Step 9: Commit**

```bash
git add src/gui/settings/layout.rs src/gui/renderer/settings.rs src/gui/renderer/gpu/settings.rs
git commit -m "feat(settings): add drop shadow and accent-colored border to panel"
```

---

### Task 4: Hover-aware button opacities

**Files:**
- Modify: `src/gui/settings/layout.rs` — `build_stepper_control`, `build_dropdown_control`, close button section, and `compute_settings_layout` signature

**Step 1: Pass `palette_close_hover_bg` to `compute_settings_layout`**

Add new parameter to `compute_settings_layout`:

```rust
palette_close_hover_bg: u32,
```

Update callers in:
- `src/gui/renderer/settings.rs` — add `self.palette.close_hover_bg.to_pixel(),`
- `src/gui/renderer/gpu/settings.rs` — add `self.palette.close_hover_bg.to_pixel(),`
- `src/gui/events/mouse/settings.rs` (both call sites at lines ~24-36 and ~169-181) — add `self.backend.palette_close_hover_bg(),`

Check if `palette_close_hover_bg()` accessor exists on `RendererBackend`. If not, add it following the pattern of existing palette accessors in `src/gui/renderer/backend.rs`.

**Step 2: Close button hover**

In the close button section of `compute_settings_layout` (around line 286), change:

```rust
let close_button = RoundedRectCmd {
    ...
    color: palette_bar_bg,
    opacity: 0.6,
};
```

to:

```rust
let (close_color, close_opacity) = if overlay.hovered_close {
    (palette_close_hover_bg, 0.8)
} else {
    (palette_bar_bg, 0.6)
};

let close_button = RoundedRectCmd {
    ...
    color: close_color,
    opacity: close_opacity,
};
```

**Step 3: Stepper button hover**

In `build_stepper_control`, add `overlay: &SettingsOverlay` and `item_index: usize` parameters.

Change minus button opacity:

```rust
let minus_hovered = overlay.hovered_stepper == Some((item_index, crate::gui::settings::StepperHalf::Minus));
let minus_btn = RoundedRectCmd {
    ...
    opacity: if minus_hovered { 1.0 } else { 0.6 },
};
```

Same for plus button:

```rust
let plus_hovered = overlay.hovered_stepper == Some((item_index, crate::gui::settings::StepperHalf::Plus));
let plus_btn = RoundedRectCmd {
    ...
    opacity: if plus_hovered { 1.0 } else { 0.6 },
};
```

Update all 3 call sites (FloatSlider, IntSlider, LargeIntSlider) in `build_item_layouts` to pass `overlay` and `i`.

**Step 4: Dropdown button hover**

In `build_dropdown_control`, change button opacity:

```rust
let btn_hovered = overlay.hovered_dropdown == Some(item_index);
let button = RoundedRectCmd {
    ...
    opacity: if btn_hovered { 0.85 } else { 0.6 },
};
```

`overlay` and `item_index` are already available in `build_dropdown_control`.

**Step 5: Run tests**

Run: `cargo test -- --nocapture`
Expected: All pass. Test helper `compute_test_layout` needs the new `palette_close_hover_bg` argument — add `const TEST_CLOSE_HOVER_BG: u32 = 0x454B59;` and update the call.

**Step 6: Run clippy**

Run: `cargo clippy && cargo clippy --no-default-features`
Expected: Zero warnings.

**Step 7: Commit**

```bash
git add src/gui/settings/layout.rs src/gui/renderer/settings.rs src/gui/renderer/gpu/settings.rs src/gui/events/mouse/settings.rs src/gui/renderer/backend.rs
git commit -m "feat(settings): hover effects on stepper, dropdown, and close buttons"
```

---

### Task 5: Active category left indicator

**Files:**
- Modify: `src/gui/settings/layout.rs` — `CategoryLayout` struct and `build_category_layouts`
- Modify: `src/gui/renderer/settings.rs` — category rendering loop
- Modify: `src/gui/renderer/gpu/settings.rs` — category rendering loop

**Step 1: Add indicator field to `CategoryLayout`**

```rust
pub(in crate::gui) struct CategoryLayout {
    pub bg: FlatRectCmd,
    pub text: TextCmd,
    #[allow(dead_code)]
    pub is_active: bool,
    /// Left accent bar for the active category (None if not active).
    pub indicator: Option<FlatRectCmd>,
}
```

**Step 2: Compute indicator in `build_category_layouts`**

After the `bg` computation, before the return:

```rust
let indicator = if is_active {
    Some(FlatRectCmd {
        x: panel_x,
        y: row_y,
        w: 2.0,
        h: row_h as f32,
        color: accent,
        opacity: 1.0,
    })
} else {
    None
};

CategoryLayout {
    bg,
    text,
    is_active,
    indicator,
}
```

**Step 3: Render indicator in CPU renderer**

In `settings.rs`, inside the categories loop, after `self.draw_flat_rect_cmd(target, &cat.bg);`:

```rust
if let Some(ref ind) = cat.indicator {
    self.draw_flat_rect_cmd(target, ind);
}
```

**Step 4: Render indicator in GPU renderer**

In `gpu/settings.rs`, inside the categories loop, after the bg push_rect:

```rust
if let Some(ref ind) = cat.indicator {
    self.push_rect(ind.x, ind.y, ind.w, ind.h, ind.color, ind.opacity);
}
```

**Step 5: Run tests**

Run: `cargo test -- --nocapture`
Expected: All pass.

**Step 6: Commit**

```bash
git add src/gui/settings/layout.rs src/gui/renderer/settings.rs src/gui/renderer/gpu/settings.rs
git commit -m "feat(settings): add active category left indicator bar"
```

---

### Task 6: Item row hover background

**Files:**
- Modify: `src/gui/settings/layout.rs` — `ItemLayout` struct and `build_item_layouts`
- Modify: `src/gui/renderer/settings.rs` — item rendering loop
- Modify: `src/gui/renderer/gpu/settings.rs` — item rendering loop

**Step 1: Add `row_bg` field to `ItemLayout`**

```rust
pub(in crate::gui) struct ItemLayout {
    /// Optional hover background for this item row.
    pub row_bg: Option<FlatRectCmd>,
    pub label: TextCmd,
    pub controls: ItemControlLayout,
}
```

**Step 2: Compute row hover background in `build_item_layouts`**

After computing `row_y` and `label_y`, before the label `TextCmd`:

```rust
let is_hovered = overlay.hovered_item == Some(i);
let row_bg = if is_hovered {
    Some(FlatRectCmd {
        x: area_x,
        y: row_y,
        w: area_w,
        h: row_h as f32,
        color: bar_bg,
        opacity: 0.2,
    })
} else {
    None
};
```

Add `row_bg` to the `ItemLayout` construction.

**Step 3: Render row background in CPU renderer**

In `settings.rs`, at the start of the items loop, before label text:

```rust
if let Some(ref bg) = item.row_bg {
    self.draw_flat_rect_cmd(target, bg);
}
```

**Step 4: Render row background in GPU renderer**

In `gpu/settings.rs`, at the start of the items loop, before label push_text:

```rust
if let Some(ref bg) = item.row_bg {
    self.push_rect(bg.x, bg.y, bg.w, bg.h, bg.color, bg.opacity);
}
```

**Step 5: Run tests**

Run: `cargo test -- --nocapture`
Expected: All pass.

**Step 6: Commit**

```bash
git add src/gui/settings/layout.rs src/gui/renderer/settings.rs src/gui/renderer/gpu/settings.rs
git commit -m "feat(settings): add item row hover background"
```

---

### Task 7: Tests and final verification

**Files:**
- Modify: `src/gui/settings/layout.rs` — test module

**Step 1: Add test for shadow offset**

```rust
#[test]
fn panel_shadow_has_offset() {
    let overlay = default_overlay();
    let layout = compute_test_layout(&overlay);
    let offset = layout.panel_shadow.x - layout.panel_bg.x;
    assert!(offset > 0.0, "shadow should be offset to the right");
    assert_eq!(layout.panel_shadow.x - layout.panel_bg.x,
               layout.panel_shadow.y - layout.panel_bg.y,
               "shadow X and Y offsets should be equal");
    assert_eq!(layout.panel_shadow.color, 0x000000);
    assert!((layout.panel_shadow.opacity - 0.24).abs() < 0.01);
}
```

**Step 2: Add test for accent border color**

```rust
#[test]
fn panel_border_uses_accent_color() {
    let overlay = default_overlay();
    let layout = compute_test_layout(&overlay);
    assert_eq!(layout.panel_border.color, TEST_ACCENT);
    assert!((layout.panel_border.opacity - 0.12).abs() < 0.01);
}
```

**Step 3: Add test for active category indicator**

```rust
#[test]
fn active_category_has_indicator() {
    let overlay = default_overlay();
    let layout = compute_test_layout(&overlay);
    // First category (Font) is active by default.
    assert!(layout.categories[0].indicator.is_some());
    let ind = layout.categories[0].indicator.as_ref().unwrap();
    assert_eq!(ind.w, 2.0);
    assert_eq!(ind.color, TEST_ACCENT);
    // Inactive categories have no indicator.
    assert!(layout.categories[1].indicator.is_none());
}
```

**Step 4: Add test for close button hover**

```rust
#[test]
fn close_button_hover_changes_opacity() {
    let config = AppConfig::default();
    let mut overlay = SettingsOverlay::new(&config);
    let layout_normal = compute_test_layout(&overlay);
    overlay.hovered_close = true;
    let layout_hovered = compute_test_layout(&overlay);
    assert!(layout_hovered.close_button.opacity > layout_normal.close_button.opacity);
}
```

**Step 5: Add test for stepper hover**

```rust
#[test]
fn stepper_hover_changes_opacity() {
    let config = AppConfig::default();
    let mut overlay = SettingsOverlay::new(&config);
    let layout_normal = compute_test_layout(&overlay);
    overlay.hovered_stepper = Some((0, StepperHalf::Minus));
    let layout_hovered = compute_test_layout(&overlay);
    // Font Size (item 0) has a stepper.
    match (&layout_normal.items[0].controls, &layout_hovered.items[0].controls) {
        (
            ItemControlLayout::Stepper { minus_btn: normal, .. },
            ItemControlLayout::Stepper { minus_btn: hovered, .. },
        ) => {
            assert!(hovered.opacity > normal.opacity);
        }
        _ => panic!("expected Stepper"),
    }
}
```

**Step 6: Add test for item row hover**

```rust
#[test]
fn item_row_hover_produces_background() {
    let config = AppConfig::default();
    let mut overlay = SettingsOverlay::new(&config);
    assert!(compute_test_layout(&overlay).items[0].row_bg.is_none());
    overlay.hovered_item = Some(0);
    let layout = compute_test_layout(&overlay);
    assert!(layout.items[0].row_bg.is_some());
    let bg = layout.items[0].row_bg.as_ref().unwrap();
    assert_eq!(bg.color, TEST_BAR_BG);
    assert!((bg.opacity - 0.2).abs() < 0.01);
}
```

**Step 7: Run all tests**

Run: `cargo test -- --nocapture`
Expected: All tests pass including the new ones.

**Step 8: Run clippy on both feature sets**

Run: `cargo clippy && cargo clippy --no-default-features`
Expected: Zero warnings.

**Step 9: Build both targets**

Run: `cargo build && cargo build --no-default-features`
Expected: Both compile successfully.

**Step 10: Commit**

```bash
git add src/gui/settings/layout.rs
git commit -m "test(settings): add tests for shadow, accent border, hover effects, indicator"
```
