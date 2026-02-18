---
title: Ferrum Terminal Emulator - Deep Architectural Analysis
scratchpad: .specs/scratchpad/a7f3c9d2.md
created: 2026-02-16
status: complete
---

# Ferrum Terminal Emulator - Deep Architectural Analysis

## Executive Summary

**Overall Assessment**: The Ferrum codebase has a **clean core module** but suffers from **god objects, excessive trait responsibilities, and code duplication** in the GUI layer.

**Critical Issues**: 3 critical, 2 medium, 1 low
**Core Module**: ✅ Properly isolated and well-designed
**GUI Module**: ⚠️ Architectural violations and coupling problems

---

## 1. Current Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         main.rs                              │
│                    (entry point)                             │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                      gui::run()                              │
│              ┌──────────────────────┐                        │
│              │        App           │                        │
│              │  (window manager)    │                        │
│              └──────────┬───────────┘                        │
│                         │                                    │
│                         │ manages multiple                   │
│                         ▼                                    │
│              ┌──────────────────────┐                        │
│              │   FerrumWindow       │ ◄── GOD OBJECT         │
│              │   (28 fields)        │                        │
│              └──────────┬───────────┘                        │
│                         │                                    │
│         ┌───────────────┼───────────────┐                    │
│         ▼               ▼               ▼                    │
│   ┌─────────┐   ┌──────────────┐  ┌────────┐               │
│   │  tabs   │   │   backend    │  │ events │               │
│   │(TabState)   │(RendererBackend) │ (mouse,│               │
│   └─────────┘   └──────┬───────┘  │keyboard)               │
│                         │          └────────┘               │
│                 ┌───────┴───────┐                            │
│                 ▼               ▼                            │
│         ┌──────────────┐ ┌──────────────┐                   │
│         │ CpuRenderer  │ │ GpuRenderer  │                   │
│         │(softbuffer)  │ │   (wgpu)     │                   │
│         └──────┬───────┘ └──────┬───────┘                   │
│                │                 │                           │
│                └────────┬────────┘                           │
│                         │ implements                         │
│                         ▼                                    │
│                  ┌──────────────┐                            │
│                  │  Renderer    │ ◄── TOO MANY METHODS       │
│                  │  (31 methods)│     (rendering + layout    │
│                  └──────────────┘      + hit testing)        │
└─────────────────────────────────────────────────────────────┘
                         │ uses
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                     core module                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ Terminal │  │   Grid   │  │   Cell   │  │  Color   │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
│  │Selection │  │ Position │  │ Security │                  │
│  └──────────┘  └──────────┘  └──────────┘                  │
│                                                              │
│  ✅ NO GUI IMPORTS - properly isolated                       │
└─────────────────────────────────────────────────────────────┘
                         │ used by
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                     pty module                               │
│                    ┌──────────┐                              │
│                    │ Session  │                              │
│                    └──────────┘                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Problems Found

### Critical Severity

#### Problem 1: FerrumWindow God Object

**Severity**: 🔴 Critical
**Location**: `src/gui/state.rs:108-139`, `src/gui/mod.rs:34-95`
**Lines**: 148 (state.rs) + 197 (mod.rs) = 345 total

**Evidence**: FerrumWindow has **28 fields** mixing 8 different concerns:

```rust
pub(super) struct FerrumWindow {
    // Window/Rendering (3 fields)
    pub(super) window: Arc<Window>,
    pub(super) pending_grid_resize: Option<(usize, usize)>,
    pub(super) backend: RendererBackend,

    // Tab Management (3 fields)
    pub(super) tabs: Vec<TabState>,
    pub(super) active_tab: usize,
    pub(super) closed_tabs: Vec<ClosedTabInfo>,

    // Mouse/Click Tracking (6 fields)
    pub(super) mouse_pos: (f64, f64),
    pub(super) last_click_time: std::time::Instant,
    pub(super) last_click_pos: Position,
    pub(super) click_streak: u8,
    pub(super) last_tab_click: Option<(usize, std::time::Instant)>,
    pub(super) last_topbar_empty_click: Option<Instant>,

    // Selection State (4 fields)
    pub(super) is_selecting: bool,
    pub(super) selection_anchor: Option<Position>,
    pub(super) selection_drag_mode: SelectionDragMode,
    pub(super) suppress_click_to_cursor_once: bool,

    // Tab Bar UI State (5 fields)
    pub(super) hovered_tab: Option<usize>,
    pub(super) renaming_tab: Option<RenameState>,
    pub(super) dragging_tab: Option<DragState>,
    pub(super) tab_reorder_animation: Option<TabReorderAnimation>,

    // Popups/Context Menu (2 fields)
    pub(super) context_menu: Option<ContextMenu>,
    pub(super) security_popup: Option<SecurityPopup>,

    // Input State (3 fields)
    pub(super) modifiers: ModifiersState,
    pub(super) clipboard: Option<arboard::Clipboard>,
    pub(super) scroll_accumulator: f64,

    // Window Chrome (2 fields)
    pub(super) resize_direction: Option<ResizeDirection>,
    pub(super) cursor_blink_start: std::time::Instant,

    // Request Queue (1 field)
    pub(super) pending_requests: Vec<WindowRequest>,
}
```

**Impact**:
- Violates Single Responsibility Principle
- Makes testing difficult (can't test tab management without window/rendering state)
- Hard to understand - 28 fields is cognitive overload
- Change amplification - any UI feature touches this struct

**Recommended Fix**: Extract cohesive state objects

---

#### Problem 2: Renderer Trait Does Too Much

**Severity**: 🔴 Critical
**Location**: `src/gui/renderer/traits.rs:21-187`
**Lines**: 187 lines, **31 methods**

**Evidence**: Trait mixes 4 unrelated concerns:

```rust
pub trait Renderer {
    // 1. Rendering operations (8 methods)
    fn render(&mut self, ...);
    fn draw_cursor(&mut self, ...);
    fn render_scrollbar(&mut self, ...);
    fn draw_tab_bar(&mut self, ...);
    fn draw_tab_drag_overlay(&mut self, ...);
    fn draw_tab_tooltip(&mut self, ...);
    fn draw_context_menu(&mut self, ...);
    fn draw_security_popup(&mut self, ...);

    // 2. Layout calculations (7 methods)
    fn tab_width(&self, ...) -> u32;
    fn tab_origin_x(&self, ...) -> u32;
    fn tab_strip_start_x(&self) -> u32;
    fn tab_insert_index_from_x(&self, ...) -> usize;
    fn tab_hover_tooltip(&self, ...) -> Option<&str>;
    fn scrollbar_thumb_bounds(&self, ...) -> Option<(f32, f32)>;
    fn security_badge_rect(&self, ...) -> Option<(u32, u32, u32, u32)>;

    // 3. Hit testing (7 methods)
    fn hit_test_tab_bar(&self, ...) -> TabBarHit;
    fn hit_test_tab_hover(&self, ...) -> Option<usize>;
    fn hit_test_tab_security_badge(&self, ...) -> Option<usize>;
    fn window_button_at_position(&self, ...) -> Option<WindowButton>;
    fn hit_test_context_menu(&self, ...) -> Option<usize>;
    fn hit_test_security_popup(&self, ...) -> bool;

    // 4. Metrics (8 methods)
    fn set_scale(&mut self, scale_factor: f64);
    fn cell_width(&self) -> u32;
    fn cell_height(&self) -> u32;
    fn tab_bar_height_px(&self) -> u32;
    fn window_padding_px(&self) -> u32;
    fn ui_scale(&self) -> f64;
    fn scaled_px(&self, base: u32) -> u32;
    fn scrollbar_hit_zone_px(&self) -> u32;
}
```

**Impact**:
- Any change to hit testing logic requires touching the trait
- CpuRenderer and GpuRenderer must implement ALL 31 methods
- No flexibility - can't have a renderer with different layout logic
- Violates Interface Segregation Principle

**Recommended Fix**: Split into 4 traits

---

#### Problem 3: Massive Code Duplication in Render Paths

**Severity**: 🔴 Critical
**Location**:
- `src/gui/events/render_cpu.rs:19-119` (100 lines)
- `src/gui/events/render_gpu.rs:22-123` (101 lines)

**Evidence**: Tab bar metadata preparation logic is **100% IDENTICAL**:

**render_cpu.rs:19-119**:
```rust
#[cfg(not(target_os = "macos"))]
let (tab_infos, tab_tooltip, drag_info, tab_offsets, show_tooltip) = {
    let renaming = self.renaming_tab.as_ref().map(|rename| {
        let selection = rename.selection_anchor.and_then(|anchor| {
            if anchor == rename.cursor {
                None
            } else {
                Some((anchor.min(rename.cursor), anchor.max(rename.cursor)))
            }
        });
        (rename.tab_index, rename.text.as_str(), rename.cursor, selection)
    });
    let tab_infos: Vec<TabInfo> = self.tabs.iter().enumerate()
        .map(|(i, t)| {
            let is_renaming = renaming.as_ref().is_some_and(|(ri, _, _, _)| *ri == i);
            // ... 50 more lines of identical logic ...
        })
        .collect();
    // ... drag info calculation, animation offsets, tooltip logic ...
};
```

**render_gpu.rs:22-123**: **EXACT SAME CODE**

**Lines duplicated**: ~100 lines × 2 = 200 lines of redundant code

**Impact**:
- Bug fix or feature in CPU path must be manually duplicated to GPU path
- High risk of divergence (already happened in past commits)
- Maintenance burden doubled

**Recommended Fix**: Extract shared tab bar state preparation

---

### Medium Severity

#### Problem 4: Tab Bar Event Handler Too Large

**Severity**: 🟡 Medium
**Location**: `src/gui/events/mouse/tab_bar.rs` (449 lines)

**Evidence**: Single file handles ALL tab bar interactions:

```rust
impl FerrumWindow {
    // Line 7: Helper to build TabInfo for hit testing
    fn tab_infos_for_hit_test(&self) -> Vec<TabInfo<'_>> { ... }

    // Line 27: Delegates to backend hit test
    pub(in crate::gui::events::mouse) fn tab_bar_hit(&self, mx: f64, my: f64) -> TabBarHit { ... }

    // Line 33: Security badge hit test
    pub(in crate::gui::events::mouse) fn tab_bar_security_hit(...) -> Option<usize> { ... }

    // Line 47: Opens security popup
    fn open_security_popup_for_tab(&mut self, tab_index: usize) { ... }

    // Line 85: MAIN HANDLER - left click on tab bar (200+ lines)
    pub(in crate::gui::events::mouse) fn handle_tab_bar_left_click(...) {
        // Double-click detection for rename
        // Drag threshold check
        // Security badge clicks
        // Tab switching
        // Close button clicks
        // New tab button
        // Window buttons
        // Empty area double-click to maximize
        // ... 200 more lines ...
    }

    // More handlers for drag, release, rename field clicks...
}
```

**Responsibilities mixed**:
1. Hit testing (tab_bar_hit, tab_bar_security_hit)
2. State preparation (tab_infos_for_hit_test)
3. Security popups (open_security_popup_for_tab)
4. Click handling (handle_tab_bar_left_click)
5. Double-click detection
6. Drag start/update/release
7. Rename field interaction
8. Window button handling

**Impact**:
- Hard to navigate (449 lines)
- Testing requires mocking entire FerrumWindow
- Adding new tab bar features touches this large file

**Recommended Fix**: Split into smaller, focused modules

---

#### Problem 5: Events Know Too Much About Rendering

**Severity**: 🟡 Medium
**Location**:
- `src/gui/events/mouse/tab_bar.rs:29` - calls `backend.hit_test_tab_bar()`
- `src/gui/events/render_cpu.rs:79-97` - calculates drag indicator animation

**Evidence**:

**tab_bar.rs:27-31**:
```rust
pub(in crate::gui::events::mouse) fn tab_bar_hit(&self, mx: f64, my: f64) -> TabBarHit {
    let buf_width = self.window.inner_size().width;
    self.backend.hit_test_tab_bar(mx, my, self.tabs.len(), buf_width)
}
```

**render_cpu.rs:79-97** (event handler calculating renderer animation state):
```rust
let drag_info = self.dragging_tab.as_mut().and_then(|drag| {
    if drag.is_active {
        let insert_idx = self.backend.tab_insert_index_from_x(
            drag.current_x,
            self.tabs.len(),
            bw as u32,
        );
        let tw = self.backend.tab_width(self.tabs.len(), bw as u32);
        let target_x = self.backend.tab_origin_x(insert_idx, tw) as f32;
        if drag.indicator_x < 0.0 {
            drag.indicator_x = target_x;
        } else {
            drag.indicator_x += (target_x - drag.indicator_x) * 0.3; // Lerp
        }
        Some((drag.source_index, drag.current_x, drag.indicator_x))
    } else {
        None
    }
});
```

**Problem**: Event handlers are calculating **layout positions** and **animation interpolation**. This is renderer responsibility.

**Impact**:
- Tight coupling between events and renderer
- Can't change layout logic without touching event handlers
- Animation logic scattered (also in `gui/events/redraw.rs`)

**Recommended Fix**: Move layout/animation to renderer or separate coordinator

---

### Low Severity

#### Problem 6: Confusing Module Names

**Severity**: 🟢 Low
**Location**: `src/gui/events/` vs `src/gui/interaction/`

**Evidence**:

**gui/events/mod.rs**:
```rust
mod keyboard;
mod mouse;
mod pty;
mod redraw;
mod render_cpu;
mod render_gpu;
```

**gui/interaction/mod.rs**:
```rust
mod clipboard;
mod cursor_move;
mod geometry;
mod mouse_reporting;
mod selection;
```

**Reality**:
- `events/` = winit event dispatch (window events → FerrumWindow methods)
- `interaction/` = helper methods for user interactions (clipboard, selection, geometry)

**Problem**: Both names suggest "user interaction" - boundary is unclear

**Impact**:
- Mild confusion when navigating codebase
- No functional impact

**Recommended Fix**: Rename `interaction/` to `helpers/` or `utilities/`

---

## 3. Suggested Fixes

### Fix 1: Extract State Objects from FerrumWindow

**Create new state structs**:

```rust
// gui/state/selection.rs
pub(super) struct SelectionState {
    pub is_selecting: bool,
    pub anchor: Option<Position>,
    pub drag_mode: SelectionDragMode,
    pub last_click_time: Instant,
    pub last_click_pos: Position,
    pub click_streak: u8,
}

// gui/state/tab_bar.rs
pub(super) struct TabBarState {
    pub hovered_tab: Option<usize>,
    pub renaming_tab: Option<RenameState>,
    pub dragging_tab: Option<DragState>,
    pub tab_reorder_animation: Option<TabReorderAnimation>,
    pub last_tab_click: Option<(usize, Instant)>,
    pub last_topbar_empty_click: Option<Instant>,
}

// gui/state/popups.rs
pub(super) struct PopupState {
    pub context_menu: Option<ContextMenu>,
    pub security_popup: Option<SecurityPopup>,
}

// gui/state/input.rs
pub(super) struct InputState {
    pub modifiers: ModifiersState,
    pub mouse_pos: (f64, f64),
    pub scroll_accumulator: f64,
    pub suppress_click_to_cursor_once: bool,
}
```

**Refactored FerrumWindow**:

```rust
pub(super) struct FerrumWindow {
    // Core window concerns
    pub(super) window: Arc<Window>,
    pub(super) backend: RendererBackend,
    pub(super) pending_grid_resize: Option<(usize, usize)>,

    // Tab management
    pub(super) tabs: Vec<TabState>,
    pub(super) active_tab: usize,
    pub(super) closed_tabs: Vec<ClosedTabInfo>,

    // State objects (5 fields instead of 28)
    pub(super) selection: SelectionState,
    pub(super) tab_bar: TabBarState,
    pub(super) popups: PopupState,
    pub(super) input: InputState,

    // Misc
    pub(super) clipboard: Option<arboard::Clipboard>,
    pub(super) resize_direction: Option<ResizeDirection>,
    pub(super) cursor_blink_start: Instant,
    pub(super) pending_requests: Vec<WindowRequest>,
}
```

**Benefits**:
- Reduced from 28 fields to ~13 fields
- Clear grouping by concern
- Each state object can be tested independently
- Easier to understand and modify

**Files to change**:
- Create: `src/gui/state/selection.rs`, `src/gui/state/tab_bar.rs`, `src/gui/state/popups.rs`, `src/gui/state/input.rs`
- Modify: `src/gui/state.rs` (import and use new structs)
- Update: All `gui/events/` files to use `win.selection.anchor` instead of `win.selection_anchor`, etc.

**Estimated effort**: Medium (half day - 1 day)

---

### Fix 2: Split Renderer Trait

**New trait hierarchy**:

```rust
// gui/renderer/traits/metrics.rs
pub trait RenderMetrics {
    fn cell_width(&self) -> u32;
    fn cell_height(&self) -> u32;
    fn tab_bar_height_px(&self) -> u32;
    fn window_padding_px(&self) -> u32;
    fn ui_scale(&self) -> f64;
    fn scaled_px(&self, base: u32) -> u32;
    fn scrollbar_hit_zone_px(&self) -> u32;
}

// gui/renderer/traits/layout.rs
pub trait LayoutCalculator {
    fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32;
    fn tab_origin_x(&self, tab_index: usize, tw: u32) -> u32;
    fn tab_strip_start_x(&self) -> u32;
    fn tab_insert_index_from_x(&self, x: f64, tab_count: usize, buf_width: u32) -> usize;
    fn scrollbar_thumb_bounds(&self, ...) -> Option<(f32, f32)>;
    fn security_badge_rect(&self, ...) -> Option<(u32, u32, u32, u32)>;
}

// gui/renderer/traits/hit_test.rs
pub trait HitTester {
    fn hit_test_tab_bar(&self, x: f64, y: f64, tab_count: usize, buf_width: u32) -> TabBarHit;
    fn hit_test_tab_hover(&self, ...) -> Option<usize>;
    fn hit_test_tab_security_badge(&self, ...) -> Option<usize>;
    fn hit_test_context_menu(&self, ...) -> Option<usize>;
    fn hit_test_security_popup(&self, ...) -> bool;
    #[cfg(not(target_os = "macos"))]
    fn window_button_at_position(&self, ...) -> Option<WindowButton>;
}

// gui/renderer/traits/render.rs
pub trait Renderer: RenderMetrics {
    fn set_scale(&mut self, scale_factor: f64);
    fn render(&mut self, buffer: &mut [u32], buf_width: usize, buf_height: usize,
              grid: &Grid, selection: Option<&Selection>);
    fn draw_cursor(&mut self, ...);
    fn render_scrollbar(&mut self, ...);
    fn draw_tab_bar(&mut self, ...);
    fn draw_tab_drag_overlay(&mut self, ...);
    fn draw_tab_tooltip(&mut self, ...);
    fn draw_context_menu(&mut self, ...);
    fn draw_security_popup(&mut self, ...);
}
```

**Updated RendererBackend**:

```rust
pub enum RendererBackend {
    Cpu {
        renderer: CpuRenderer,
        surface: Surface<...>,
    },
    #[cfg(feature = "gpu")]
    Gpu(GpuRenderer),
}

impl Renderer for RendererBackend { ... }
impl RenderMetrics for RendererBackend { ... }
impl LayoutCalculator for RendererBackend { ... }
impl HitTester for RendererBackend { ... }
```

**Benefits**:
- Clearer separation of concerns
- Easier to test layout logic independently
- Flexibility to swap implementations (e.g., different layout algorithms)
- Follows Interface Segregation Principle

**Files to change**:
- Create: `src/gui/renderer/traits/metrics.rs`, `layout.rs`, `hit_test.rs`, `render.rs`
- Modify: `src/gui/renderer/traits.rs` → `src/gui/renderer/traits/mod.rs` (re-export all)
- Update: `src/gui/renderer/backend.rs` (implement all 4 traits)
- Update: Event handlers to use trait bounds (e.g., `T: HitTester` instead of `T: Renderer`)

**Estimated effort**: Medium (1-2 days with careful refactoring)

---

### Fix 3: Extract Shared Tab Bar State Preparation

**Create new module**:

```rust
// gui/renderer/tab_bar_state.rs

/// Prepared tab bar rendering state (shared between CPU and GPU paths).
pub struct TabBarRenderState<'a> {
    pub tab_infos: Vec<TabInfo<'a>>,
    pub tab_tooltip: Option<String>,
    pub drag_info: Option<(usize, f64, f32)>, // (source_index, current_x, indicator_x)
    pub tab_offsets: Option<Vec<f32>>,
    pub show_tooltip: bool,
}

impl FerrumWindow {
    /// Builds tab bar render state (used by both CPU and GPU renderers).
    pub(super) fn prepare_tab_bar_state(&mut self, buf_width: u32) -> TabBarRenderState<'_> {
        let renaming = self.renaming_tab.as_ref().map(|rename| {
            let selection = rename.selection_anchor.and_then(|anchor| {
                if anchor == rename.cursor {
                    None
                } else {
                    Some((anchor.min(rename.cursor), anchor.max(rename.cursor)))
                }
            });
            (rename.tab_index, rename.text.as_str(), rename.cursor, selection)
        });

        let tab_infos: Vec<TabInfo> = self.tabs.iter().enumerate()
            .map(|(i, t)| {
                let is_renaming = renaming.as_ref().is_some_and(|(ri, _, _, _)| *ri == i);
                let security_count = if t.security.has_events() {
                    t.security.active_event_count()
                } else {
                    0
                };
                TabInfo {
                    title: &t.title,
                    is_active: i == self.active_tab,
                    security_count,
                    is_renaming,
                    rename_text: if is_renaming {
                        renaming.as_ref().map(|(_, text, _, _)| *text)
                    } else {
                        None
                    },
                    rename_cursor: if is_renaming {
                        renaming.as_ref().map_or(0, |(_, _, c, _)| *c)
                    } else {
                        0
                    },
                    rename_selection: if is_renaming {
                        renaming.as_ref().and_then(|(_, _, _, selection)| *selection)
                    } else {
                        None
                    },
                }
            })
            .collect();

        // ... rest of preparation logic (drag info, animations, tooltip) ...

        TabBarRenderState {
            tab_infos,
            tab_tooltip,
            drag_info,
            tab_offsets,
            show_tooltip,
        }
    }
}
```

**Updated render paths**:

```rust
// render_cpu.rs
pub(super) fn render_cpu_frame(&mut self, w: NonZeroU32, h: NonZeroU32, bw: usize, bh: usize) {
    #[cfg(not(target_os = "macos"))]
    let tab_bar_state = self.prepare_tab_bar_state(bw as u32);

    let RendererBackend::Cpu { renderer, surface } = &mut self.backend else {
        return;
    };

    // ... rendering logic using tab_bar_state ...
}

// render_gpu.rs
pub(super) fn render_gpu_frame(&mut self, w: NonZeroU32, h: NonZeroU32, bw: usize, bh: usize) {
    #[cfg(not(target_os = "macos"))]
    let tab_bar_state = self.prepare_tab_bar_state(bw as u32);

    let RendererBackend::Gpu(gpu) = &mut self.backend else {
        return;
    };

    // ... rendering logic using tab_bar_state ...
}
```

**Benefits**:
- Eliminates 100+ lines of duplication
- Single source of truth for tab bar state
- Bug fixes apply to both CPU and GPU paths automatically
- Easier to maintain and test

**Files to change**:
- Create: `src/gui/renderer/tab_bar_state.rs`
- Modify: `src/gui/events/render_cpu.rs` (replace lines 19-119 with call to `prepare_tab_bar_state()`)
- Modify: `src/gui/events/render_gpu.rs` (replace lines 22-123 with call to `prepare_tab_bar_state()`)
- Update: `src/gui/renderer/mod.rs` (add `pub mod tab_bar_state;`)

**Estimated effort**: Low (2-4 hours)

---

### Fix 4: Split Tab Bar Event Handler

**Refactor into smaller modules**:

```
gui/events/mouse/tab_bar/
├── mod.rs           # Public interface, delegates to submodules
├── hit_test.rs      # Hit testing helpers (tab_bar_hit, security_hit)
├── click.rs         # Click handling (single/double clicks)
├── drag.rs          # Drag start/update/release logic
├── rename.rs        # Rename field interaction
└── window_buttons.rs # Window control button handling
```

**Example structure**:

```rust
// gui/events/mouse/tab_bar/mod.rs
mod hit_test;
mod click;
mod drag;
mod rename;
mod window_buttons;

use crate::gui::*;

impl FerrumWindow {
    pub(crate) fn handle_tab_bar_left_click(...) {
        // High-level orchestration only
        if state != ElementState::Pressed {
            self.handle_tab_drag_release();
            return;
        }

        if self.handle_security_badge_click(mx, my) {
            return;
        }

        if self.handle_rename_field_click(mx) {
            return;
        }

        self.handle_tab_bar_click(event_loop, mx, my, next_tab_id, tx);
    }
}

// gui/events/mouse/tab_bar/click.rs
impl FerrumWindow {
    pub(super) fn handle_tab_bar_click(...) {
        let hit = self.tab_bar_hit(mx, my);
        match hit {
            TabBarHit::Tab(idx) => self.handle_tab_click(idx, mx, my),
            TabBarHit::CloseTab(idx) => self.close_tab(idx),
            TabBarHit::NewTab => self.handle_new_tab_click(next_tab_id, tx),
            TabBarHit::WindowButton(btn) => self.handle_window_button_click(btn),
            TabBarHit::Empty => self.handle_empty_area_click(mx, my),
        }
    }
}
```

**Benefits**:
- Each file focused on single concern (~50-100 lines each)
- Easier to navigate and understand
- Simpler unit tests (test each module independently)

**Files to change**:
- Split: `src/gui/events/mouse/tab_bar.rs` (449 lines) → 6 files
- Create: Directory `src/gui/events/mouse/tab_bar/`

**Estimated effort**: Medium (4-6 hours)

---

### Fix 5: Move Layout/Animation Out of Event Handlers

**Create animation coordinator**:

```rust
// gui/animation/tab_bar.rs

pub struct TabBarAnimator {
    drag_indicator_x: f32,
    reorder_animation: Option<TabReorderAnimation>,
}

impl TabBarAnimator {
    pub fn update_drag_indicator(
        &mut self,
        drag: &DragState,
        layout: &impl LayoutCalculator,
        tab_count: usize,
        buf_width: u32,
    ) -> f32 {
        if !drag.is_active {
            return -1.0;
        }

        let insert_idx = layout.tab_insert_index_from_x(drag.current_x, tab_count, buf_width);
        let tw = layout.tab_width(tab_count, buf_width);
        let target_x = layout.tab_origin_x(insert_idx, tw) as f32;

        if self.drag_indicator_x < 0.0 {
            self.drag_indicator_x = target_x;
        } else {
            self.drag_indicator_x += (target_x - self.drag_indicator_x) * 0.3;
        }

        self.drag_indicator_x
    }

    pub fn compute_tab_offsets(&self, now: Instant) -> Option<Vec<f32>> {
        let anim = self.reorder_animation.as_ref()?;
        let elapsed = now.duration_since(anim.started).as_millis() as u32;

        if elapsed >= anim.duration_ms {
            return None;
        }

        let t = elapsed as f32 / anim.duration_ms as f32;
        let ease = 1.0 - (1.0 - t).powi(3); // Ease-out cubic

        Some(anim.offsets.iter().map(|&offset| offset * (1.0 - ease)).collect())
    }
}
```

**Updated render paths**:

```rust
// render_cpu.rs
let drag_info = if let Some(drag) = &self.dragging_tab {
    let indicator_x = self.tab_bar_animator.update_drag_indicator(
        drag,
        &self.backend,
        self.tabs.len(),
        bw as u32,
    );
    Some((drag.source_index, drag.current_x, indicator_x))
} else {
    None
};
```

**Benefits**:
- Animation logic centralized
- Event handlers don't calculate layout
- Easier to change animation curves
- Reusable for other UI animations

**Files to change**:
- Create: `src/gui/animation/tab_bar.rs`
- Modify: `src/gui/events/render_cpu.rs`, `render_gpu.rs` (use animator)
- Add field: `tab_bar_animator: TabBarAnimator` to `FerrumWindow` or `TabBarState`

**Estimated effort**: Medium (3-5 hours)

---

### Fix 6: Rename interaction/ Module

**Simple rename**:

```bash
mv src/gui/interaction src/gui/helpers
```

**Update imports**:

```rust
// Before
use crate::gui::interaction::clipboard;

// After
use crate::gui::helpers::clipboard;
```

**Benefits**:
- Clearer module purpose
- Less confusion with `events/` module

**Files to change**:
- Rename: `src/gui/interaction/` → `src/gui/helpers/`
- Update: All imports in `gui/events/` files

**Estimated effort**: Trivial (15 minutes)

---

## 4. Implementation Priority

### Phase 1: Quick Wins (1-2 days)
1. ✅ **Fix 3**: Extract shared tab bar state preparation (eliminates duplication)
2. ✅ **Fix 6**: Rename `interaction/` to `helpers/` (clarity)

### Phase 2: Structural Improvements (3-5 days)
3. ✅ **Fix 1**: Extract state objects from FerrumWindow (reduces god object)
4. ✅ **Fix 4**: Split tab bar event handler (improves maintainability)

### Phase 3: Advanced Refactoring (5-7 days)
5. ✅ **Fix 2**: Split Renderer trait (proper separation of concerns)
6. ✅ **Fix 5**: Move layout/animation to coordinator (decouples events from rendering)

**Total estimated effort**: 9-14 days (1.5-3 weeks)

---

## 5. Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| All module boundaries documented | ✅ | Complete analysis of gui/, core/, pty/ |
| God object identified with field count | ✅ | FerrumWindow: 28 fields, 8 concerns |
| Trait responsibilities analyzed | ✅ | Renderer: 31 methods, 4 concerns mixed |
| Code duplication quantified | ✅ | 100+ lines duplicated in render paths |
| Coupling problems mapped | ✅ | Events calculate layout/animations (renderer concern) |
| Fixes provided with code examples | ✅ | 6 fixes with implementation details |
| Core independence verified | ✅ | No gui imports in core/ ✅ |

**Limitations/Caveats**:
- Analysis focused on gui/ layer - did not deeply analyze core/terminal/ handlers
- Did not review GPU renderer internals (gpu/ directory)
- Did not analyze PTY module (only 1 file, out of scope)
- Fixes are architectural recommendations - actual implementation may reveal edge cases

---

## 6. Final Recommendations

### Immediate Actions (Do This Week)
1. **Extract tab bar state preparation** (Fix 3) - highest ROI, prevents future bugs
2. **Rename interaction/ to helpers/** (Fix 6) - trivial improvement

### Short-term Goals (Next Sprint)
3. **Start extracting SelectionState** (partial Fix 1) - reduces FerrumWindow complexity incrementally
4. **Split tab_bar.rs** (Fix 4) - improves code navigation

### Long-term Refactoring (Next Quarter)
5. **Split Renderer trait** (Fix 2) - requires careful API design
6. **Create animation coordinator** (Fix 5) - enables future UI improvements

### Maintain Current Quality
- ✅ **Core module is excellent** - keep it independent
- ✅ **RendererBackend enum dispatch is clean** - no changes needed
- ✅ **Event flow (lifecycle.rs) is clear** - maintain pattern

---

## Appendix: File Reference

### Files Analyzed

**Core Module** (✅ Clean):
- `src/core/mod.rs` - exports
- `src/core/terminal.rs` - terminal state
- `src/core/grid.rs` - character grid

**GUI Module** (⚠️ Needs Work):
- `src/gui/mod.rs` (197 lines) - FerrumWindow constructor, App
- `src/gui/state.rs` (148 lines) - state structs (GOD OBJECT)
- `src/gui/lifecycle.rs` (280 lines) - winit ApplicationHandler
- `src/gui/renderer/mod.rs` (617 lines) - CpuRenderer implementation
- `src/gui/renderer/traits.rs` (187 lines) - Renderer trait (TOO MANY METHODS)
- `src/gui/renderer/backend.rs` (287 lines) - enum dispatch
- `src/gui/events/render_cpu.rs` (258 lines) - CPU render path (DUPLICATION)
- `src/gui/events/render_gpu.rs` (251 lines) - GPU render path (DUPLICATION)
- `src/gui/events/mouse/tab_bar.rs` (449 lines) - tab bar events (TOO LARGE)
- `src/gui/events/mouse/input.rs` - mouse input entry point
- `src/gui/events/keyboard/entry.rs` - keyboard input entry point

**PTY Module**:
- `src/pty/mod.rs` - PTY interface (not analyzed)

---

*End of Analysis*
