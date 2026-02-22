# Auto-Update Feature Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show an in-app toast banner when a new Ferrum version is available, with Details and
Install buttons — the Install button triggers a platform-specific silent update and relaunches.

**Architecture:** The existing `src/update.rs` already fetches GitHub release metadata and
delivers `AvailableRelease` to the app. We add: (1) a layout + rendering layer for the banner
(same pattern as the existing tooltip overlay), (2) mouse-click handling on the banner buttons,
(3) a platform-specific installer in `src/update/installer.rs`, (4) an `auto_check` toggle in
`AppConfig`, and (5) an "Updates" settings tab on every platform.

**Tech Stack:** Rust, winit, wgpu (GPU path), softbuffer (CPU path), ureq (HTTP download),
serde/serde_json (config), objc2 (macOS settings), Win32 (Windows settings), gtk4 (Linux
settings).

---

## Reference: Existing Patterns to Follow

Before starting any task, read these files once:
- `src/gui/renderer/shared/overlay_layout.rs` — `TooltipLayout` + `compute_tooltip_layout`
- `src/gui/renderer/gpu/overlays.rs` — `draw_tab_tooltip_impl`
- `src/gui/events/render_shared.rs` lines 86–97 (`FrameParams`) and 447–455 (tooltip step 6)
- `src/gui/events/mouse/input.rs` — how mouse clicks are dispatched

---

## Task 1: i18n Strings

Add banner strings to `Translations` and both locale files.

**Files:**
- Modify: `src/i18n/translations.rs` (add fields after `update_available`)
- Modify: `src/i18n/en.rs` (fill English values)
- Modify: `src/i18n/uk.rs` (fill Ukrainian values)

**Step 1: Add fields to `Translations` struct**

In `src/i18n/translations.rs`, in the `// --- Update ---` section, replace:
```rust
    /// Format string — use `{}` as placeholder for the version tag.
    pub update_available: &'static str,
```
with:
```rust
    /// Format string — use `{}` as placeholder for the version tag.
    pub update_available: &'static str,
    /// "Details" button label on the update banner.
    pub update_details: &'static str,
    /// "Install" button label on the update banner.
    pub update_install: &'static str,
    /// "Installing…" label shown while update is in progress.
    pub update_installing: &'static str,
    /// Settings tab label for Updates.
    pub settings_tab_updates: &'static str,
    /// "Current version" label in Updates settings tab.
    pub update_current_version: &'static str,
    /// "Check for Updates" button label in Updates settings tab.
    pub update_check_now: &'static str,
    /// "Auto-check for updates" toggle label.
    pub update_auto_check: &'static str,
```

**Step 2: Add the same fields to `all_non_empty` field slice**

In `translations.rs`, append to the `fields` slice in `all_non_empty`:
```rust
            self.update_details,
            self.update_install,
            self.update_installing,
            self.settings_tab_updates,
            self.update_current_version,
            self.update_check_now,
            self.update_auto_check,
```

**Step 3: Fill English translations**

In `src/i18n/en.rs`, in the `// --- Update ---` section, after the `update_available` line add:
```rust
        update_details: "Details",
        update_install: "Install",
        update_installing: "Installing…",
        settings_tab_updates: "Updates",
        update_current_version: "Current version",
        update_check_now: "Check for Updates",
        update_auto_check: "Auto-check for updates",
```

**Step 4: Fill Ukrainian translations**

In `src/i18n/uk.rs`, same location:
```rust
        update_details: "Деталі",
        update_install: "Встановити",
        update_installing: "Встановлення…",
        settings_tab_updates: "Оновлення",
        update_current_version: "Поточна версія",
        update_check_now: "Перевірити оновлення",
        update_auto_check: "Автоперевірка оновлень",
```

**Step 5: Verify compilation**

```bash
cargo build 2>&1 | grep -E "error|warning"
```
Expected: zero errors, zero warnings.

**Step 6: Commit**

```bash
git add src/i18n/translations.rs src/i18n/en.rs src/i18n/uk.rs
git commit -m "feat(i18n): add update banner and settings strings"
```

---

## Task 2: `UpdatesConfig` in AppConfig

Add a new config section so the auto-check toggle persists across sessions.

**Files:**
- Modify: `src/config/model.rs` (add `UpdatesConfig` struct and field)

**Step 1: Add `UpdatesConfig` struct**

In `src/config/model.rs`, before `pub struct AppConfig`, add:
```rust
/// Configuration for the update checker.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdatesConfig {
    /// Whether Ferrum should automatically check for updates in the background.
    #[serde(default = "defaults::updates_auto_check")]
    pub auto_check: bool,
}

impl Default for UpdatesConfig {
    fn default() -> Self {
        Self { auto_check: defaults::updates_auto_check() }
    }
}
```

**Step 2: Add default function**

In the `mod defaults` block at the bottom of `model.rs`, add:
```rust
    pub(super) fn updates_auto_check() -> bool { true }
```

**Step 3: Add field to `AppConfig`**

In the `AppConfig` struct, add:
```rust
    #[serde(default)]
    pub updates: UpdatesConfig,
```

**Step 4: Verify compilation**

```bash
cargo build 2>&1 | grep -E "error|warning"
```
Expected: zero errors, zero warnings.

**Step 5: Run tests**

```bash
cargo test config 2>&1 | tail -5
```
Expected: all pass.

**Step 6: Commit**

```bash
git add src/config/model.rs
git commit -m "feat(config): add UpdatesConfig with auto_check toggle"
```

---

## Task 3: `UpdateBannerLayout` (layout math + tests)

Add the banner geometry computation to the shared renderer module.
This is **cross-platform** (no `#![cfg(not(target_os = "macos"))]` gate).

**Files:**
- Create: `src/gui/renderer/shared/banner_layout.rs`
- Modify: `src/gui/renderer/shared/mod.rs` (add `pub mod banner_layout`)

**Step 1: Create `banner_layout.rs`**

```rust
//! Layout computation for the update-available banner.
//!
//! The banner is a small rounded rect centred horizontally at the top of the
//! terminal area (just below the tab bar on non-macOS; at the top edge on
//! macOS). It contains: an "Update vX.Y.Z" label, a [Details] button, an
//! [Install] button, and a [✕] dismiss button.
//!
//! The layout is computed once per frame (only when a release is available
//! and the banner is not dismissed) and shared between the CPU and GPU
//! render paths.

use crate::gui::renderer::shared::tab_math::TabLayoutMetrics;

/// Pre-computed geometry for the update-available banner.
#[derive(Debug, Clone)]
pub struct UpdateBannerLayout {
    /// Background rectangle.
    pub bg_x: i32,
    pub bg_y: i32,
    pub bg_w: u32,
    pub bg_h: u32,
    /// Corner radius.
    pub radius: u32,
    /// "Update vX.Y.Z available" label position.
    pub label_x: u32,
    pub label_y: u32,
    /// The full label text (e.g. "Update v0.3.2 available").
    pub label_text: String,
    /// [Details] button rect.
    pub details_x: u32,
    pub details_y: u32,
    pub details_w: u32,
    pub btn_h: u32,
    /// [Install] button rect (same height as details).
    pub install_x: u32,
    pub install_y: u32,
    pub install_w: u32,
    /// [✕] dismiss button rect.
    pub dismiss_x: u32,
    pub dismiss_y: u32,
    pub dismiss_w: u32,
}

/// Computes the banner layout.
///
/// Returns `None` when the buffer is too small to fit anything.
///
/// `tab_bar_h` is the pixel height of the tab bar (0 on macOS where the
/// native tab bar does not consume our buffer space).
pub fn compute_update_banner_layout(
    tag_name: &str,
    m: &TabLayoutMetrics,
    buf_width: u32,
    buf_height: u32,
    tab_bar_h: u32,
) -> Option<UpdateBannerLayout> {
    if buf_width == 0 || buf_height == 0 {
        return None;
    }

    let pad_x = m.scaled_px(10);
    let pad_y = m.scaled_px(6);
    let btn_pad_x = m.scaled_px(8);
    let gap = m.scaled_px(6);
    let radius = m.scaled_px(6);

    // Label text
    let label_text = format!("Update {} available", tag_name);
    let label_chars = label_text.chars().count() as u32;
    let label_w = label_chars * m.cell_width;

    // Buttons: "Details", "Install", "✕"
    let details_text_w = "Details".chars().count() as u32 * m.cell_width;
    let install_text_w = "Install".chars().count() as u32 * m.cell_width;
    let dismiss_text_w = m.cell_width; // single char "✕"

    let btn_h = m.cell_height + pad_y;
    let details_w = details_text_w + btn_pad_x * 2;
    let install_w = install_text_w + btn_pad_x * 2;
    let dismiss_w = dismiss_text_w + btn_pad_x * 2;

    let total_w = pad_x * 2 + label_w + gap + details_w + gap + install_w + gap + dismiss_w;
    if total_w > buf_width {
        return None;
    }

    let bg_h = btn_h + pad_y * 2;
    let margin_top = m.scaled_px(6);
    let bg_y = (tab_bar_h + margin_top) as i32;
    let bg_x = ((buf_width - total_w) / 2) as i32;

    let label_x = bg_x as u32 + pad_x;
    let label_y = bg_y as u32 + pad_y + (btn_h - m.cell_height) / 2;

    let details_x = label_x + label_w + gap;
    let details_y = bg_y as u32 + pad_y;
    let install_x = details_x + details_w + gap;
    let install_y = details_y;
    let dismiss_x = install_x + install_w + gap;
    let dismiss_y = details_y;

    Some(UpdateBannerLayout {
        bg_x,
        bg_y,
        bg_w: total_w,
        bg_h,
        radius,
        label_x,
        label_y,
        label_text,
        details_x,
        details_y,
        details_w,
        btn_h,
        install_x,
        install_y,
        install_w,
        dismiss_x,
        dismiss_y,
        dismiss_w,
    })
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics_1x() -> TabLayoutMetrics {
        TabLayoutMetrics { cell_width: 9, cell_height: 20, ui_scale: 1.0, tab_bar_height: 36 }
    }

    fn metrics_2x() -> TabLayoutMetrics {
        TabLayoutMetrics { cell_width: 18, cell_height: 40, ui_scale: 2.0, tab_bar_height: 72 }
    }

    #[test]
    fn zero_buf_returns_none() {
        let m = metrics_1x();
        assert!(compute_update_banner_layout("v0.3.2", &m, 0, 600, 36).is_none());
        assert!(compute_update_banner_layout("v0.3.2", &m, 800, 0, 36).is_none());
    }

    #[test]
    fn basic_layout_has_positive_dimensions() {
        let m = metrics_1x();
        let l = compute_update_banner_layout("v0.3.2", &m, 800, 600, 36)
            .expect("should fit in 800px");
        assert!(l.bg_w > 0);
        assert!(l.bg_h > 0);
        assert!(l.radius > 0);
        assert!(!l.label_text.is_empty());
    }

    #[test]
    fn banner_positioned_below_tab_bar() {
        let m = metrics_1x();
        let l = compute_update_banner_layout("v0.3.2", &m, 800, 600, 36)
            .expect("layout");
        assert!(l.bg_y >= 36);
    }

    #[test]
    fn banner_horizontally_centred() {
        let m = metrics_1x();
        let l = compute_update_banner_layout("v0.3.2", &m, 800, 600, 36)
            .expect("layout");
        let right_space = 800i32 - (l.bg_x + l.bg_w as i32);
        // Centre tolerance ±1 pixel
        assert!((l.bg_x - right_space).abs() <= 1);
    }

    #[test]
    fn hidpi_scales_dimensions() {
        let m1 = metrics_1x();
        let m2 = metrics_2x();
        let l1 = compute_update_banner_layout("v0.3.2", &m1, 800, 600, 36).unwrap();
        let l2 = compute_update_banner_layout("v0.3.2", &m2, 1600, 1200, 72).unwrap();
        assert!(l2.bg_h > l1.bg_h);
        assert!(l2.radius > l1.radius);
    }

    #[test]
    fn buttons_inside_background() {
        let m = metrics_1x();
        let l = compute_update_banner_layout("v0.3.2", &m, 800, 600, 36).unwrap();
        assert!(l.details_x >= l.bg_x as u32);
        assert!(l.install_x >= l.bg_x as u32);
        assert!(l.dismiss_x >= l.bg_x as u32);
        let bg_right = (l.bg_x as u32) + l.bg_w;
        assert!(l.dismiss_x + l.dismiss_w <= bg_right);
    }
}
```

**Step 2: Register module in `src/gui/renderer/shared/mod.rs`**

Add to the `mod.rs` after the existing module declarations:
```rust
pub mod banner_layout;
```

**Step 3: Run the new tests**

```bash
cargo test banner_layout 2>&1 | tail -10
```
Expected: 5 tests pass.

**Step 4: Commit**

```bash
git add src/gui/renderer/shared/banner_layout.rs src/gui/renderer/shared/mod.rs
git commit -m "feat: add UpdateBannerLayout geometry computation with tests"
```

---

## Task 4: `UpdateInstallState` in Window State

Add state to `FerrumWindow` that tracks whether the banner is dismissed and whether an
install is in progress.

**Files:**
- Modify: `src/gui/state.rs` (add enum + FerrumWindow fields)
- Modify: `src/gui/mod.rs` (initialise new fields in `FerrumWindow::new`)

**Step 1: Add `UpdateInstallState` enum in `src/gui/state.rs`**

After the existing public enums (find `pub(crate) enum SelectionDragMode` as a reference),
add:
```rust
/// Install state for the in-app update banner.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) enum UpdateInstallState {
    #[default]
    Idle,
    Installing,
    Done,
    Failed(String),
}
```

**Step 2: Add fields to `FerrumWindow`**

Inside the `FerrumWindow` struct, add two fields (place them near the end, before `settings_tx`):
```rust
    /// Whether the update banner has been manually dismissed this session.
    pub(super) update_banner_dismissed: bool,
    /// Current state of the background install operation.
    pub(super) update_install_state: UpdateInstallState,
```

**Step 3: Initialise in `FerrumWindow::new`**

In `src/gui/mod.rs`, inside the `FerrumWindow { … }` struct literal in `fn new`, add:
```rust
            update_banner_dismissed: false,
            update_install_state: UpdateInstallState::Idle,
```

**Step 4: Verify compilation**

```bash
cargo build 2>&1 | grep -E "error|warning"
```
Expected: zero errors, zero warnings.

**Step 5: Commit**

```bash
git add src/gui/state.rs src/gui/mod.rs
git commit -m "feat: add UpdateInstallState and banner state fields to FerrumWindow"
```

---

## Task 5: `draw_update_banner` on `Renderer` Trait + CPU Implementation

Add the banner rendering method to the trait and implement it for the CPU path.

**Files:**
- Modify: `src/gui/renderer/traits.rs` (add `draw_update_banner`)
- Modify: `src/gui/renderer/cpu/trait_impl.rs` (implement it)

**Step 1: Add method to `Renderer` trait in `traits.rs`**

After `fn draw_tab_tooltip` (line ~179), add:
```rust
    /// Draws the update-available banner overlay.
    fn draw_update_banner(
        &mut self,
        target: &mut RenderTarget<'_>,
        tag_name: &str,
        install_state: &crate::gui::state::UpdateInstallState,
        tab_bar_h: u32,
    );
```

**Step 2: Implement in `src/gui/renderer/cpu/trait_impl.rs`**

Find the `impl Renderer for CpuRenderer` block. Add:
```rust
    fn draw_update_banner(
        &mut self,
        target: &mut RenderTarget<'_>,
        tag_name: &str,
        install_state: &crate::gui::state::UpdateInstallState,
        tab_bar_h: u32,
    ) {
        use crate::gui::renderer::shared::banner_layout::compute_update_banner_layout;
        use crate::gui::renderer::cpu::primitives::RoundedRectCmd;

        let (buf_width, buf_height) = (target.width as u32, target.height as u32);
        let m = self.tab_layout_metrics();
        let Some(layout) = compute_update_banner_layout(tag_name, &m, buf_width, buf_height, tab_bar_h)
        else {
            return;
        };

        let colors = self.theme_colors();
        let bg_color = colors.menu_bg;
        let border_color = colors.border;
        let text_color = colors.fg;
        let btn_color = colors.tab_active_bg;

        // Background
        self.draw_rounded_rect_cmd(target, &RoundedRectCmd {
            x: layout.bg_x, y: layout.bg_y,
            w: layout.bg_w, h: layout.bg_h,
            radius: layout.radius, color: bg_color, opacity: 240,
        });
        // Border
        self.draw_rounded_rect_cmd(target, &RoundedRectCmd {
            x: layout.bg_x - 1, y: layout.bg_y - 1,
            w: layout.bg_w + 2, h: layout.bg_h + 2,
            radius: layout.radius + 1, color: border_color, opacity: 180,
        });

        // Label
        let label = if install_state == &crate::gui::state::UpdateInstallState::Installing {
            crate::i18n::t().update_installing.to_string()
        } else {
            layout.label_text.clone()
        };
        self.draw_text(target, layout.label_x, layout.label_y, &label, text_color, 255);

        if install_state == &crate::gui::state::UpdateInstallState::Idle {
            // [Details] button
            self.draw_rounded_rect_cmd(target, &RoundedRectCmd {
                x: layout.details_x as i32, y: layout.details_y as i32,
                w: layout.details_w, h: layout.btn_h,
                radius: layout.radius, color: btn_color, opacity: 200,
            });
            let t = crate::i18n::t();
            self.draw_text(target, layout.details_x + m.scaled_px(8), layout.details_y + m.scaled_px(3),
                           t.update_details, text_color, 255);

            // [Install] button
            self.draw_rounded_rect_cmd(target, &RoundedRectCmd {
                x: layout.install_x as i32, y: layout.install_y as i32,
                w: layout.install_w, h: layout.btn_h,
                radius: layout.radius, color: btn_color, opacity: 200,
            });
            self.draw_text(target, layout.install_x + m.scaled_px(8), layout.install_y + m.scaled_px(3),
                           t.update_install, text_color, 255);

            // [✕] dismiss
            self.draw_rounded_rect_cmd(target, &RoundedRectCmd {
                x: layout.dismiss_x as i32, y: layout.dismiss_y as i32,
                w: layout.dismiss_w, h: layout.btn_h,
                radius: layout.radius, color: btn_color, opacity: 160,
            });
            self.draw_text(target, layout.dismiss_x + m.scaled_px(8), layout.dismiss_y + m.scaled_px(3),
                           "✕", text_color, 255);
        }
    }
```

**Step 3: Verify compilation**

```bash
cargo build 2>&1 | grep -E "error|warning"
```
Expected: zero errors, zero warnings. (GPU renderer will fail to compile until Task 6.)

**Step 4: Commit**

```bash
git add src/gui/renderer/traits.rs src/gui/renderer/cpu/trait_impl.rs
git commit -m "feat: implement draw_update_banner for CPU renderer"
```

---

## Task 6: GPU `draw_update_banner` Implementation

Implement the same method for the GPU renderer following the `draw_tab_tooltip_impl` pattern
in `src/gui/renderer/gpu/overlays.rs`.

**Files:**
- Modify: `src/gui/renderer/gpu/overlays.rs` (add `draw_update_banner_impl`)
- Modify: `src/gui/renderer/gpu/trait_impl.rs` (add trait method delegation)

**Step 1: Add `draw_update_banner_impl` to `overlays.rs`**

At the bottom of `overlays.rs`, add:
```rust
/// Draws the update-available banner using GPU rounded-rect + text commands.
pub fn draw_update_banner_impl(
    renderer: &mut crate::gui::renderer::gpu::GpuRenderer,
    tag_name: &str,
    install_state: &crate::gui::state::UpdateInstallState,
    tab_bar_h: u32,
    buf_width: u32,
    buf_height: u32,
) {
    use crate::gui::renderer::shared::banner_layout::compute_update_banner_layout;
    use crate::gui::renderer::gpu::commands::RoundedRectCmd;

    let m = renderer.tab_layout_metrics();
    let Some(layout) = compute_update_banner_layout(tag_name, &m, buf_width, buf_height, tab_bar_h)
    else {
        return;
    };

    let colors = renderer.theme_colors();
    let opacity = 0.94_f32;

    // Background + border (same two-rect pattern as tooltip)
    renderer.push_rounded_rect_cmd(&RoundedRectCmd {
        x: layout.bg_x - 1, y: layout.bg_y - 1,
        w: layout.bg_w + 2, h: layout.bg_h + 2,
        radius: layout.radius + 1, color: colors.border, opacity: 0.7,
    });
    renderer.push_rounded_rect_cmd(&RoundedRectCmd {
        x: layout.bg_x, y: layout.bg_y,
        w: layout.bg_w, h: layout.bg_h,
        radius: layout.radius, color: colors.menu_bg, opacity,
    });

    // Label
    let label = if install_state == &crate::gui::state::UpdateInstallState::Installing {
        crate::i18n::t().update_installing.to_string()
    } else {
        layout.label_text.clone()
    };
    renderer.push_text(layout.label_x, layout.label_y, &label, colors.fg, opacity);

    if install_state == &crate::gui::state::UpdateInstallState::Idle {
        let t = crate::i18n::t();
        let m_pad = m.scaled_px(8);
        let m_top = m.scaled_px(3);

        // [Details]
        renderer.push_rounded_rect_cmd(&RoundedRectCmd {
            x: layout.details_x as i32, y: layout.details_y as i32,
            w: layout.details_w, h: layout.btn_h,
            radius: layout.radius, color: colors.tab_active_bg, opacity: 0.78,
        });
        renderer.push_text(layout.details_x + m_pad, layout.details_y + m_top,
                           t.update_details, colors.fg, opacity);

        // [Install]
        renderer.push_rounded_rect_cmd(&RoundedRectCmd {
            x: layout.install_x as i32, y: layout.install_y as i32,
            w: layout.install_w, h: layout.btn_h,
            radius: layout.radius, color: colors.tab_active_bg, opacity: 0.78,
        });
        renderer.push_text(layout.install_x + m_pad, layout.install_y + m_top,
                           t.update_install, colors.fg, opacity);

        // [✕]
        renderer.push_rounded_rect_cmd(&RoundedRectCmd {
            x: layout.dismiss_x as i32, y: layout.dismiss_y as i32,
            w: layout.dismiss_w, h: layout.btn_h,
            radius: layout.radius, color: colors.tab_active_bg, opacity: 0.63,
        });
        renderer.push_text(layout.dismiss_x + m_pad, layout.dismiss_y + m_top,
                           "✕", colors.fg, opacity);
    }
}
```

**Step 2: Add trait delegation in `gpu/trait_impl.rs`**

In the `impl Renderer for GpuRenderer` block, add:
```rust
    fn draw_update_banner(
        &mut self,
        target: &mut RenderTarget<'_>,
        tag_name: &str,
        install_state: &crate::gui::state::UpdateInstallState,
        tab_bar_h: u32,
    ) {
        let (buf_width, buf_height) = (target.width as u32, target.height as u32);
        overlays::draw_update_banner_impl(self, tag_name, install_state, tab_bar_h, buf_width, buf_height);
    }
```

**Step 3: Verify compilation**

```bash
cargo build 2>&1 | grep -E "error|warning"
```
Expected: zero errors, zero warnings.

**Step 4: Commit**

```bash
git add src/gui/renderer/gpu/overlays.rs src/gui/renderer/gpu/trait_impl.rs
git commit -m "feat: implement draw_update_banner for GPU renderer"
```

---

## Task 7: Render Banner in Frame + Wire Into FrameParams

Add the banner as step 7 of `draw_frame_content` and pass `available_release` via
`FrameParams`.

**Files:**
- Modify: `src/gui/events/render_shared.rs` (add `available_release` to `FrameParams`, draw in frame)
- Modify: `src/gui/lifecycle/mod.rs` (pass release to render call)

**Step 1: Add `available_release` to `FrameParams`**

In `render_shared.rs`, extend `FrameParams` struct to add:
```rust
    /// If `Some`, the update banner is shown (unless the window dismissed it).
    pub available_release: Option<&'a crate::update::AvailableRelease>,
    /// Whether the user dismissed the banner this session.
    pub update_banner_dismissed: bool,
    /// Current install state.
    pub update_install_state: crate::gui::state::UpdateInstallState,
```

**Step 2: Add banner draw step at the end of `draw_frame_content`**

After the tooltip step (step 6, line ~453 in `render_shared.rs`), add:
```rust
    // 7) Draw update banner.
    if let Some(release) = params.available_release {
        if !params.update_banner_dismissed
            && params.update_install_state != crate::gui::state::UpdateInstallState::Done
        {
            let tab_bar_h = renderer.tab_bar_height_px();
            renderer.draw_update_banner(
                &mut target,
                &release.tag_name,
                &params.update_install_state,
                tab_bar_h,
            );
        }
    }
```

**Step 3: Pass new fields when constructing `FrameParams`**

Find where `FrameParams { … }` is constructed in `lifecycle/mod.rs` (or wherever render is
called). Add the three new fields:
```rust
        available_release: self.available_release.as_ref(),   // lives on App
        update_banner_dismissed: win.update_banner_dismissed,
        update_install_state: win.update_install_state.clone(),
```

Note: `available_release` lives on `App`, not on `FerrumWindow`. Check the lifecycle render
call site to confirm how `App` data is threaded through (look for how `available_release` is
already passed to `win.sync_window_title(self.available_release.as_ref())` — follow the same
pattern).

**Step 4: Verify compilation**

```bash
cargo build 2>&1 | grep -E "error|warning"
```
Expected: zero errors, zero warnings.

**Step 5: Smoke test manually**

Temporarily lower `CARGO_PKG_VERSION` in `Cargo.toml` to `"0.0.1"`, run `cargo run`, and
verify the banner appears at the top of the window. Restore the real version afterwards.

**Step 6: Commit**

```bash
git add src/gui/events/render_shared.rs src/gui/lifecycle/mod.rs
git commit -m "feat: render update banner in frame overlay pass"
```

---

## Task 8: Mouse Click Handling for Banner Buttons

Handle clicks on the three banner buttons: Details (open URL), Install (start installer),
Dismiss (set flag).

**Files:**
- Create: `src/gui/events/mouse/update_banner.rs`
- Modify: `src/gui/events/mouse/mod.rs` (add `mod update_banner`)
- Modify: `src/gui/events/mouse/input.rs` (call handler early in click chain)

**Step 1: Create `update_banner.rs`**

```rust
use winit::event::ElementState;

use crate::gui::renderer::shared::banner_layout::compute_update_banner_layout;
use crate::gui::state::UpdateInstallState;
use crate::gui::*;
use crate::update::AvailableRelease;

impl FerrumWindow {
    /// Handles a left-button press/release on the update banner.
    ///
    /// Returns `true` if the click was consumed by the banner.
    pub(super) fn handle_update_banner_click(
        &mut self,
        state: ElementState,
        mx: f64,
        my: f64,
        available_release: Option<&AvailableRelease>,
    ) -> bool {
        if state != ElementState::Released { return false; }
        let Some(release) = available_release else { return false; };
        if self.update_banner_dismissed { return false; }
        if self.update_install_state != UpdateInstallState::Idle { return false; }

        let size = self.window.inner_size();
        let tab_bar_h = self.backend.tab_bar_height_px();
        let m = self.backend.tab_layout_metrics();
        let Some(layout) = compute_update_banner_layout(
            &release.tag_name, &m, size.width, size.height, tab_bar_h,
        ) else {
            return false;
        };

        let px = mx as i32;
        let py = my as i32;

        // Check if click is within the banner background at all.
        let in_banner = px >= layout.bg_x
            && px < layout.bg_x + layout.bg_w as i32
            && py >= layout.bg_y
            && py < layout.bg_y + layout.bg_h as i32;
        if !in_banner {
            return false;
        }

        // Dismiss button
        let in_dismiss = px >= layout.dismiss_x as i32
            && px < (layout.dismiss_x + layout.dismiss_w) as i32
            && py >= layout.dismiss_y as i32
            && py < (layout.dismiss_y + layout.btn_h) as i32;
        if in_dismiss {
            self.update_banner_dismissed = true;
            self.window.request_redraw();
            return true;
        }

        // Details button
        let in_details = px >= layout.details_x as i32
            && px < (layout.details_x + layout.details_w) as i32
            && py >= layout.details_y as i32
            && py < (layout.details_y + layout.btn_h) as i32;
        if in_details {
            open_url(&release.html_url);
            return true;
        }

        // Install button
        let in_install = px >= layout.install_x as i32
            && px < (layout.install_x + layout.install_w) as i32
            && py >= layout.install_y as i32
            && py < (layout.install_y + layout.btn_h) as i32;
        if in_install {
            self.start_update_install(release);
            return true;
        }

        // Clicked banner background but not a button — consume to prevent pass-through.
        true
    }
}

/// Opens a URL in the system default browser.
fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/c", "start", url]).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}
```

**Step 2: Add `start_update_install` stub (filled in Task 9)**

At the bottom of `update_banner.rs`, add:
```rust
impl FerrumWindow {
    pub(super) fn start_update_install(&mut self, release: &AvailableRelease) {
        // TODO: implemented in Task 9
        // For now, fall back to opening the browser.
        open_url(&release.html_url);
    }
}
```

**Step 3: Register module in `mod.rs`**

In `src/gui/events/mouse/mod.rs`, add:
```rust
mod update_banner;
```

**Step 4: Wire into click chain in `input.rs`**

In `on_left_mouse_input`, before the tab-bar check, add:
```rust
        // Update banner takes priority over everything except existing drag states.
        // Note: `available_release` is threaded from App — see lifecycle call site.
        if self.handle_update_banner_click(state, mx, my, available_release) {
            return;
        }
```

You will need to thread `available_release: Option<&AvailableRelease>` as a parameter through
the call chain from `lifecycle/mod.rs` down to `on_mouse_input` → `on_left_mouse_input`.

**Step 5: Verify compilation**

```bash
cargo build 2>&1 | grep -E "error|warning"
```
Expected: zero errors, zero warnings.

**Step 6: Commit**

```bash
git add src/gui/events/mouse/update_banner.rs src/gui/events/mouse/mod.rs src/gui/events/mouse/input.rs
git commit -m "feat: add update banner click handling (Details, Install, Dismiss)"
```

---

## Task 9: Platform-Specific Installer

Implement `start_update_install`: detect install method, download the new binary/installer
in a background thread, replace, and relaunch.

**Files:**
- Create: `src/update_installer.rs`
- Modify: `src/main.rs` (add `mod update_installer`)
- Modify: `src/gui/events/mouse/update_banner.rs` (replace stub)

**Step 1: Determine release asset URLs**

Read `.github/workflows/build-installers.yml` (do NOT edit it) to find the exact artifact
file names for each platform. The download URLs follow:
```
https://github.com/itsserbin/ferrum/releases/download/{tag}/{asset_name}
```

**Step 2: Create `src/update_installer.rs`**

```rust
//! Platform-specific binary replacement and relaunch logic.
//!
//! Called from the update banner "Install" button handler.
//! Runs download + replace in a background thread. Sends
//! `InstallResult` back via a oneshot channel on completion.

use std::env;
use std::fs;
use std::path::PathBuf;

/// Result sent back to the GUI after the install attempt.
#[derive(Debug)]
pub enum InstallResult {
    Success,
    Failed(String),
}

/// Spawns a background thread to download and install the new version,
/// then relaunch. Calls `on_done` with the result on the calling thread
/// (via the event proxy) — or just relaunches on success.
pub fn spawn_installer(tag_name: &str) {
    let tag = tag_name.to_owned();
    std::thread::Builder::new()
        .name("ferrum-installer".into())
        .spawn(move || {
            if let Err(e) = run_install(&tag) {
                eprintln!("[update] install failed: {e}");
            }
        })
        .expect("spawn installer thread");
}

fn run_install(tag: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    return install_macos(tag);
    #[cfg(target_os = "windows")]
    return install_windows(tag);
    #[cfg(target_os = "linux")]
    return install_linux(tag);
}

// ── macOS ─────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn install_macos(tag: &str) -> anyhow::Result<()> {
    // Try Homebrew first.
    let brew_check = std::process::Command::new("brew")
        .args(["list", "--cask", "ferrum"])
        .output();
    if brew_check.is_ok_and(|o| o.status.success()) {
        let status = std::process::Command::new("brew")
            .args(["upgrade", "--cask", "ferrum"])
            .status()?;
        if status.success() {
            relaunch();
        }
        return Ok(());
    }

    // Fall back: download zip, replace binary, relaunch.
    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" };
    let url = format!(
        "https://github.com/itsserbin/ferrum/releases/download/{tag}/ferrum-{arch}-apple-darwin.zip"
    );
    let zip_bytes = download(&url)?;
    let new_bin = extract_binary_from_zip(&zip_bytes, "Ferrum")?;
    replace_current_binary(&new_bin)?;
    relaunch();
    Ok(())
}

// ── Windows ────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn install_windows(tag: &str) -> anyhow::Result<()> {
    let url = format!(
        "https://github.com/itsserbin/ferrum/releases/download/{tag}/Ferrum-Setup-x64.exe"
    );
    let installer_bytes = download(&url)?;
    let tmp = env::temp_dir().join("ferrum-setup.exe");
    fs::write(&tmp, &installer_bytes)?;
    std::process::Command::new(&tmp)
        .args(["/VERYSILENT", "/CLOSEAPPLICATIONS", "/RESTARTAPPLICATIONS"])
        .spawn()?;
    // The installer handles relaunch.
    std::process::exit(0);
}

// ── Linux ──────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn install_linux(tag: &str) -> anyhow::Result<()> {
    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" };
    let url = format!(
        "https://github.com/itsserbin/ferrum/releases/download/{tag}/ferrum-{arch}-linux.tar.gz"
    );
    let archive_bytes = download(&url)?;
    let new_bin = extract_binary_from_targz(&archive_bytes, "ferrum")?;
    let current = env::current_exe()?;
    let needs_sudo = current.starts_with("/usr") || current.starts_with("/opt");
    if needs_sudo {
        // Write to temp first.
        let tmp = env::temp_dir().join("ferrum-new");
        fs::write(&tmp, &new_bin)?;
        let status = std::process::Command::new("pkexec")
            .args(["cp", tmp.to_str().unwrap(), current.to_str().unwrap()])
            .status()?;
        anyhow::ensure!(status.success(), "pkexec cp failed");
    } else {
        replace_current_binary(&new_bin)?;
    }
    relaunch();
    Ok(())
}

// ── Shared helpers ─────────────────────────────────────────────────────

fn download(url: &str) -> anyhow::Result<Vec<u8>> {
    let user_agent = format!("ferrum/{}", env!("CARGO_PKG_VERSION"));
    let mut resp = ureq::get(url)
        .header("User-Agent", &user_agent)
        .call()?;
    let mut buf = Vec::new();
    resp.body_mut().read_to_end(&mut buf)?;
    Ok(buf)
}

fn replace_current_binary(new_bin: &[u8]) -> anyhow::Result<()> {
    let current = env::current_exe()?;
    // Write to a temp file alongside the current binary, then rename atomically.
    let tmp = current.with_extension("new");
    fs::write(&tmp, new_bin)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
    }
    fs::rename(&tmp, &current)?;
    Ok(())
}

fn relaunch() -> ! {
    let exe = env::current_exe().expect("current_exe");
    let _ = std::process::Command::new(exe).spawn();
    std::process::exit(0);
}

fn extract_binary_from_zip(bytes: &[u8], name: &str) -> anyhow::Result<Vec<u8>> {
    // Use the zip crate if available; otherwise fall back to system unzip.
    // IMPORTANT: add `zip = "2"` to Cargo.toml under [dependencies] if not present.
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name().ends_with(name) {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut buf)?;
            return Ok(buf);
        }
    }
    anyhow::bail!("binary '{}' not found in zip", name)
}

fn extract_binary_from_targz(bytes: &[u8], name: &str) -> anyhow::Result<Vec<u8>> {
    // Use the tar + flate2 crates.
    // IMPORTANT: add `tar = "0.4"` and `flate2 = "1"` to Cargo.toml if not present.
    use flate2::read::GzDecoder;
    use tar::Archive;
    let gz = GzDecoder::new(bytes);
    let mut archive = Archive::new(gz);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        if path.file_name().is_some_and(|n| n == name) {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut buf)?;
            return Ok(buf);
        }
    }
    anyhow::bail!("binary '{}' not found in tar.gz", name)
}
```

**Step 3: Add dependencies if missing**

Check `Cargo.toml` for `zip`, `tar`, `flate2`. Add any that are missing:
```toml
zip    = "2"
tar    = "0.4"
flate2 = "1"
```

**Step 4: Register in `src/main.rs`**

Add:
```rust
mod update_installer;
```

**Step 5: Replace stub in `update_banner.rs`**

Replace the `start_update_install` stub:
```rust
    pub(super) fn start_update_install(&mut self, release: &AvailableRelease) {
        self.update_install_state = UpdateInstallState::Installing;
        self.window.request_redraw();
        crate::update_installer::spawn_installer(&release.tag_name);
    }
```

**Step 6: Verify compilation on the current platform**

```bash
cargo build 2>&1 | grep -E "error|warning"
```
Expected: zero errors, zero warnings.

**Step 7: Commit**

```bash
git add src/update_installer.rs src/main.rs src/gui/events/mouse/update_banner.rs Cargo.toml
git commit -m "feat: platform-specific installer (Homebrew/zip/installer/pkexec)"
```

---

## Task 10: Settings "Updates" Tab — macOS

Add an "Updates" tab to the native macOS settings window (`NSTabView`).

**Files:**
- Modify: `src/gui/platform/macos/settings_window.rs`

**Step 1: Add `auto_check_updates` control to `NativeSettingsState`**

In the `NativeSettingsState` struct, add:
```rust
    // Updates tab
    auto_check_updates_check: Retained<NSButton>,
```

**Step 2: Build the Updates tab inside `open_settings_window`**

Find where the Security tab is built (look for `settings_tab_security` string). After it, add:

```rust
// Updates tab
let updates_item = unsafe { NSTabViewItem::new(mtm) };
let updates_label = NSString::from_str(crate::i18n::t().settings_tab_updates);
unsafe { updates_item.setLabel(&updates_label) };

let updates_view = unsafe { NSView::new(mtm) };
let auto_check_row = create_checkbox_row(
    &updates_view,
    crate::i18n::t().update_auto_check,
    config.updates.auto_check,
    NSPoint::new(20.0, content_y),
    view_width,
    mtm,
);
let version_label_str = format!(
    "{}: v{}",
    crate::i18n::t().update_current_version,
    env!("CARGO_PKG_VERSION")
);
let version_label = unsafe { NSTextField::labelWithString(&NSString::from_str(&version_label_str), mtm) };
unsafe { version_label.setFrame(NSRect::new(NSPoint::new(20.0, content_y - 30.0), NSSize::new(view_width, 20.0))) };
unsafe { updates_view.addSubview(&version_label) };
unsafe { updates_item.setView(&updates_view) };
unsafe { tab_view.addTabViewItem(&updates_item) };
```

**Step 3: Read the checkbox value when saving config**

In the section that reads control values back into `AppConfig` (find the `paste_protection_check`
pattern), add:
```rust
        config.updates.auto_check = unsafe { state.auto_check_updates_check.state() } != 0;
```

**Step 4: Verify compilation (macOS only)**

```bash
cargo build 2>&1 | grep -E "error|warning"
```
Expected: zero errors, zero warnings.

**Step 5: Commit**

```bash
git add src/gui/platform/macos/settings_window.rs
git commit -m "feat(settings/macos): add Updates tab with auto-check toggle"
```

---

## Task 11: Settings "Updates" Tab — Windows

Add an "Updates" tab to the Win32 settings window.

**Files:**
- Modify: `src/gui/platform/windows/settings_window.rs`

**Step 1: Add an Updates tab item**

Find where the Security tab is inserted via `TCM_INSERTITEMW`. After it, insert the Updates tab:
```rust
insert_tab(hwnd_tab, n_tabs, crate::i18n::t().settings_tab_updates);
n_tabs += 1;
let tab_updates = n_tabs - 1;
```

**Step 2: Create the Updates panel**

Following the existing pattern for creating per-tab panel controls, create:
- A `BS_AUTOCHECKBOX` checkbox for `auto_check` (label: `t().update_auto_check`).
- A static text label showing `"Ferrum v{CARGO_PKG_VERSION}"`.

```rust
let hwnd_auto_check = create_checkbox(
    hwnd_parent, crate::i18n::t().update_auto_check,
    CONTENT_X, row_y(0), PANEL_W, config.updates.auto_check,
);
let version_str = format!("Ferrum v{}", env!("CARGO_PKG_VERSION"));
let hwnd_version = create_static_label(hwnd_parent, &version_str, CONTENT_X, row_y(1), PANEL_W);
```

**Step 3: Show/hide with other tab panels in `WM_NOTIFY/TCN_SELCHANGE`**

In the tab-switching match arm, add:
```rust
tab_updates => {
    show_window(hwnd_auto_check, true);
    show_window(hwnd_version, true);
    // hide all other panels
}
```

**Step 4: Read checkbox value on save**

```rust
config.updates.auto_check = is_checked(hwnd_auto_check);
```

**Step 5: Verify compilation (cross-compile or CI)**

```bash
cargo build --target x86_64-pc-windows-gnu 2>&1 | grep -E "error|warning"
```
Or check via CI after pushing.

**Step 6: Commit**

```bash
git add src/gui/platform/windows/settings_window.rs
git commit -m "feat(settings/windows): add Updates tab with auto-check toggle"
```

---

## Task 12: Settings "Updates" Tab — Linux (GTK4)

Add an "Updates" tab to the Linux GTK4 settings window.

**Files:**
- Modify: `src/gui/platform/linux/settings_window.rs`

**Step 1: Add a `build_updates_tab` function**

Following the pattern of other tab builder functions in that file:
```rust
fn build_updates_tab(config: &AppConfig) -> gtk4::Widget {
    use gtk4::prelude::*;

    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    vbox.set_margin_top(12);
    vbox.set_margin_start(12);

    // Version label
    let version_str = format!("{}: v{}",
        crate::i18n::t().update_current_version, env!("CARGO_PKG_VERSION"));
    let version_label = gtk4::Label::new(Some(&version_str));
    version_label.set_halign(gtk4::Align::Start);
    vbox.append(&version_label);

    // Auto-check switch row
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let label = gtk4::Label::new(Some(crate::i18n::t().update_auto_check));
    label.set_hexpand(true);
    label.set_halign(gtk4::Align::Start);
    let sw = gtk4::Switch::new();
    sw.set_active(config.updates.auto_check);
    row.append(&label);
    row.append(&sw);
    vbox.append(&row);

    vbox.upcast()
}
```

**Step 2: Append tab to notebook**

Find where `notebook.append_page(...)` is called for other tabs. Add:
```rust
let updates_tab = build_updates_tab(config);
notebook.append_page(
    &updates_tab,
    Some(&gtk4::Label::new(Some(crate::i18n::t().settings_tab_updates))),
);
```

**Step 3: Read switch state on save**

In the config-reading closure, add:
```rust
        config.updates.auto_check = auto_check_switch.is_active();
```

**Step 4: Verify compilation (Linux only, or via CI)**

```bash
cargo build 2>&1 | grep -E "error|warning"
```

**Step 5: Commit**

```bash
git add src/gui/platform/linux/settings_window.rs
git commit -m "feat(settings/linux): add Updates tab with auto-check toggle"
```

---

## Task 13: Wire `auto_check` to Skip Background Check

If the user turns off auto-check, skip the background thread.

**Files:**
- Modify: `src/gui/lifecycle/mod.rs` (pass config to `spawn_update_checker`)
- Modify: `src/update.rs` (guard with `auto_check`)

**Step 1: Guard `spawn_update_checker` in lifecycle**

In `App::new` (in `gui/mod.rs`), the call is:
```rust
update::spawn_update_checker(update_tx);
```

Change to:
```rust
if config.updates.auto_check {
    update::spawn_update_checker(update_tx);
}
```

**Step 2: Verify compilation**

```bash
cargo build 2>&1 | grep -E "error|warning"
```
Expected: zero errors, zero warnings.

**Step 3: Run all tests**

```bash
cargo test 2>&1 | tail -5
```
Expected: all pass.

**Step 4: Run clippy**

```bash
cargo clippy 2>&1 | grep -E "error|warning"
```
Expected: zero warnings.

**Step 5: Commit**

```bash
git add src/gui/mod.rs
git commit -m "feat: respect auto_check config to skip background update checker"
```

---

## Final Verification

```bash
cargo test 2>&1 | tail -10
cargo clippy 2>&1 | grep -E "error|warning"
```

Both must pass clean before marking the feature complete.
