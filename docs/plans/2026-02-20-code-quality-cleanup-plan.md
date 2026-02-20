# Code Quality Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove all `#[allow(clippy::too_many_arguments)]`, `#[allow(dead_code)]`, and unnecessary `_` prefixes by introducing context structs and cleaning up dead code.

**Architecture:** Introduce `RenderTarget<'a>` (buffer surface), `ScrollbarState` (scrollbar params), and `TabSlot<'a>` (CPU tab rendering context). Update `Renderer` trait and all implementations (CPU/GPU). Remove dead code.

**Tech Stack:** Rust, clippy, cargo test

---

### Task 1: Remove Dead Code

**Files:**
- Modify: `src/gui/state.rs:72-82`
- Modify: `src/gui/pane.rs:56-63`
- Modify: `src/gui/renderer/traits.rs:1`

**Step 1: Remove dead methods from state.rs**

Delete lines 72-82 (the `terminal()` and `terminal_mut()` methods with `#[allow(dead_code)]`):

```rust
// DELETE these lines from state.rs:
    /// Convenience: immutable reference to the focused pane's terminal.
    #[allow(dead_code)] // Used in later tasks
    pub(super) fn terminal(&self) -> Option<&Terminal> {
        self.focused_leaf().map(|l| &l.terminal)
    }

    /// Convenience: mutable reference to the focused pane's terminal.
    #[allow(dead_code)] // Used in later tasks
    pub(super) fn terminal_mut(&mut self) -> Option<&mut Terminal> {
        self.focused_leaf_mut().map(|l| &mut l.terminal)
    }
```

**Step 2: Remove DividerHit from pane.rs**

Delete lines 56-63:

```rust
// DELETE these lines from pane.rs:
/// Result of a divider hit-test, describing which divider was hit.
#[allow(dead_code)]
pub(super) struct DividerHit {
    pub direction: SplitDirection,
    pub ratio: f32,
    pub position: u32,
    pub available_size: u32,
}
```

**Step 3: Remove module-level allow from traits.rs**

Delete line 1: `#![allow(dead_code)]`

**Step 4: Build and test**

Run: `cargo build 2>&1 | grep -E "^error"` — expect no errors
Run: `cargo test` — expect 341 tests pass

**Step 5: Commit**

```bash
git add src/gui/state.rs src/gui/pane.rs src/gui/renderer/traits.rs
git commit -m "refactor: remove dead code (unused methods, structs, module-level allows)"
```

---

### Task 2: Add RenderTarget and ScrollbarState Structs

**Files:**
- Modify: `src/gui/renderer/types.rs` — add new structs

**Step 1: Add structs to types.rs**

Add at the end of the file (before any closing brace):

```rust
/// Pixel buffer surface passed to rendering methods.
///
/// Groups the `(buffer, width, height)` triple that appears in every
/// renderer method, eliminating repeated parameters.
pub struct RenderTarget<'a> {
    pub buffer: &'a mut [u32],
    pub width: usize,
    pub height: usize,
}

/// Scrollbar rendering parameters.
///
/// Groups scroll state, opacity, and hover flag that are always passed
/// together to scrollbar drawing methods.
pub struct ScrollbarState {
    pub scroll_offset: usize,
    pub scrollback_len: usize,
    pub grid_rows: usize,
    pub opacity: f32,
    pub hover: bool,
}
```

**Step 2: Verify the struct is exported**

`types.rs` is already re-exported via `pub use types::*;` in `mod.rs`.

Run: `cargo build 2>&1 | grep -E "^error"` — expect no errors

**Step 3: Commit**

```bash
git add src/gui/renderer/types.rs
git commit -m "refactor: add RenderTarget and ScrollbarState structs"
```

---

### Task 3: Update Renderer Trait Signatures

**Files:**
- Modify: `src/gui/renderer/traits.rs`

**Step 1: Update all trait method signatures**

Replace every method that takes `(buffer: &mut [u32], buf_width: usize, buf_height: usize)` with `(target: &mut RenderTarget<'_>)`. Also update scrollbar methods to use `ScrollbarState`.

Add import at top:
```rust
use super::types::{RenderTarget, ScrollbarState};
```

Updated signatures (replace the old ones):

```rust
fn render(
    &mut self,
    target: &mut RenderTarget<'_>,
    grid: &Grid,
    selection: Option<&Selection>,
    viewport_start: usize,
);

fn draw_cursor(
    &mut self,
    target: &mut RenderTarget<'_>,
    row: usize,
    col: usize,
    grid: &Grid,
    style: CursorStyle,
);

fn render_in_rect(
    &mut self,
    _target: &mut RenderTarget<'_>,
    _grid: &Grid,
    _selection: Option<&Selection>,
    _viewport_start: usize,
    _rect: PaneRect,
    _fg_dim: f32,
) {
}

fn draw_cursor_in_rect(
    &mut self,
    _target: &mut RenderTarget<'_>,
    _row: usize,
    _col: usize,
    _grid: &Grid,
    _style: CursorStyle,
    _rect: PaneRect,
) {
}

fn render_scrollbar_in_rect(
    &mut self,
    _target: &mut RenderTarget<'_>,
    _state: &ScrollbarState,
    _rect: PaneRect,
) {
}

fn render_scrollbar(
    &mut self,
    target: &mut RenderTarget<'_>,
    state: &ScrollbarState,
);

fn draw_tab_bar(
    &mut self,
    target: &mut RenderTarget<'_>,
    tabs: &[TabInfo],
    hovered_tab: Option<usize>,
    mouse_pos: (f64, f64),
    tab_offsets: Option<&[f32]>,
    pinned: bool,
);

fn draw_tab_drag_overlay(
    &mut self,
    target: &mut RenderTarget<'_>,
    tabs: &[TabInfo],
    source_index: usize,
    current_x: f64,
    indicator_x: f32,
);

fn draw_tab_tooltip(
    &mut self,
    target: &mut RenderTarget<'_>,
    mouse_pos: (f64, f64),
    title: &str,
);

fn draw_security_popup(
    &mut self,
    target: &mut RenderTarget<'_>,
    popup: &SecurityPopup,
);
```

Also remove all `#[allow(clippy::too_many_arguments)]` from the trait — both the trait-level one (line 25) and the per-method ones.

Update `scrollbar_thumb_bounds` to use `target.height` instead of `buf_height` param — actually this method doesn't take buffer, so it stays as-is.

Update `hit_test_security_popup` to use its existing params (no buffer involved, stays as-is).

**Step 2: Expect compile errors (trait impls not yet updated)**

Run: `cargo build 2>&1 | grep "^error" | head -5` — expect errors about mismatched signatures. This is expected — we fix them in Tasks 4-5.

**Step 3: Commit (WIP)**

```bash
git add src/gui/renderer/traits.rs
git commit -m "refactor: update Renderer trait to use RenderTarget and ScrollbarState (WIP — impls follow)"
```

---

### Task 4: Update CPU Renderer Trait Impls

**Files:**
- Modify: `src/gui/renderer/cursor.rs`
- Modify: `src/gui/renderer/scrollbar.rs`
- Modify: `src/gui/renderer/terminal.rs` (the `render()` and `render_in_rect()` methods)
- Modify: `src/gui/renderer/tab_bar/mod.rs`
- Modify: `src/gui/renderer/tab_bar/drag_overlay.rs`
- Modify: `src/gui/renderer/security.rs`

For every CPU renderer method that implements the Renderer trait, change the signature to match the new trait. Inside the method body, destructure or access `target.buffer`, `target.width`, `target.height`.

**Pattern for each method:**

Old:
```rust
pub fn render_scrollbar(
    &self,
    buffer: &mut [u32],
    buf_width: usize,
    buf_height: usize,
    scroll_offset: usize,
    ...
```

New:
```rust
pub fn render_scrollbar(
    &self,
    target: &mut RenderTarget<'_>,
    state: &ScrollbarState,
    ...
) {
    let buffer = &mut *target.buffer;  // or use target.buffer directly
    let buf_width = target.width;
    let buf_height = target.height;
    // ... rest of body unchanged, using local bindings
```

Apply this to ALL trait-implementing methods in these files. Also remove all `#[allow(clippy::too_many_arguments)]` from these methods.

**Important details for each file:**

**cursor.rs** — `draw_cursor()` and `draw_cursor_in_rect()`:
- Change signature to `(target: &mut RenderTarget<'_>, row, col, grid, style)` and `(target: &mut RenderTarget<'_>, row, col, grid, style, rect)`
- Bind `let (buffer, buf_width, buf_height) = (&mut *target.buffer, target.width, target.height);` at start of each

**scrollbar.rs** — `render_scrollbar()` and `render_scrollbar_in_rect()`:
- `render_scrollbar(target: &mut RenderTarget<'_>, state: &ScrollbarState)`
- `render_scrollbar_in_rect(target: &mut RenderTarget<'_>, state: &ScrollbarState, rect: PaneRect)`
- Destructure `state` fields at the top of each method body

**terminal.rs** — `render()` and `render_in_rect()`:
- `render(target: &mut RenderTarget<'_>, grid, selection, viewport_start)`
- `render_in_rect(target: &mut RenderTarget<'_>, grid, selection, viewport_start, rect, fg_dim)`

**tab_bar/mod.rs** — `draw_tab_bar()`:
- `draw_tab_bar(target: &mut RenderTarget<'_>, tabs, hovered_tab, mouse_pos, tab_offsets, pinned)`

**tab_bar/drag_overlay.rs** — `draw_tab_drag_overlay()`:
- `draw_tab_drag_overlay(target: &mut RenderTarget<'_>, tabs, source_index, current_x, indicator_x)`

**security.rs** — `draw_security_popup()` (note: the `draw_security_shield_icon` is a separate non-trait method, handle it in Task 7):
- There is no trait method `draw_security_popup` in security.rs — it's in the GPU renderer. Check where the CPU version of `draw_security_popup` is. It's likely dispatched through the trait.

The CPU `draw_security_popup` is defined via the trait default or in the shared overlay layout. Check `src/gui/renderer/cpu/mod.rs` or wherever the CPU renderer's trait impl lives for `draw_security_popup`.

Actually — looking at the trait, `draw_security_popup` is a required method (no default). The CPU impl must exist somewhere. Search for it.

**Step 1: Update all method signatures and bodies**

Apply changes as described above.

**Step 2: Build to check progress**

Run: `cargo build 2>&1 | grep "^error" | head -10`

Expect: fewer errors — GPU impl and callers still need updating.

**Step 3: Commit**

```bash
git add src/gui/renderer/cursor.rs src/gui/renderer/scrollbar.rs src/gui/renderer/terminal.rs src/gui/renderer/tab_bar/mod.rs src/gui/renderer/tab_bar/drag_overlay.rs src/gui/renderer/security.rs
git commit -m "refactor: update CPU renderer trait impls to use RenderTarget/ScrollbarState"
```

---

### Task 5: Update GPU Renderer Trait Impl

**Files:**
- Modify: `src/gui/renderer/gpu/trait_impl.rs`

The GPU renderer ignores `buffer/buf_width/buf_height` for most methods (they use GPU command buffers). With `RenderTarget`, the `_` prefix problem is solved — the parameter is just `target` and the GPU can ignore its fields.

**Special cases:**
- `render_scrollbar()` — currently uses `buf_height` from the trait. With `RenderTarget`, access it as `target.height`.
- `draw_tab_bar()` — uses `buf_width`. Access as `target.width`.
- `draw_tab_drag_overlay()` — uses `buf_width`. Access as `target.width`.
- `draw_tab_tooltip()` — uses `buf_width` and `buf_height`. Access as `target.width`/`target.height`.
- `draw_security_popup()` — uses `buf_width` and `buf_height`. Access as `target.width`/`target.height`.

**For scrollbar methods:** use `state.scroll_offset`, `state.scrollback_len` etc. instead of individual params.

Remove all `_buffer`, `_buf_width`, `_buf_height` prefixed params — they become `target`.

**Step 1: Update all GPU trait impl signatures**

Apply changes to match updated trait.

**Step 2: Build to check**

Run: `cargo build 2>&1 | grep "^error" | head -10`

Expect: only caller errors remain (render_shared.rs).

**Step 3: Commit**

```bash
git add src/gui/renderer/gpu/trait_impl.rs
git commit -m "refactor: update GPU renderer trait impl to use RenderTarget/ScrollbarState"
```

---

### Task 6: Update Callers in render_shared.rs

**Files:**
- Modify: `src/gui/events/render_shared.rs`

**Step 1: Update draw_frame_content signature and body**

Change `draw_frame_content` to create a `RenderTarget` from its `buffer`/`bw`/`bh` params and pass it to all renderer calls. Also create `ScrollbarState` before scrollbar calls.

The function signature stays the same (it still receives raw buffer/bw/bh from the window), but internally creates the wrapper:

```rust
let target = &mut RenderTarget {
    buffer,
    width: bw,
    height: bh,
};
```

Then replace all calls:
- `renderer.render(buffer, bw, bh, ...)` → `renderer.render(target, ...)`
- `renderer.draw_cursor(buffer, bw, bh, ...)` → `renderer.draw_cursor(target, ...)`
- `renderer.render_scrollbar(buffer, bw, bh, offset, len, rows, opacity, hover)` → `renderer.render_scrollbar(target, &ScrollbarState { scroll_offset: offset, scrollback_len: len, grid_rows: rows, opacity, hover })`
- etc.

**Important:** `target` is `&mut` so after each mutable borrow, the borrow ends and you can create a new one. Since the calls are sequential, this works fine.

Actually, since the buffer is borrowed mutably through `target`, and we need to pass `target` to multiple sequential calls, we have two options:
1. Create a new `RenderTarget` before each call
2. Create it once and pass `&mut *target` (re-borrow)

Option 1 is cleaner. Just construct `RenderTarget { buffer, width: bw, height: bh }` inline for each call, or create a local and re-use via `&mut` reborrow.

Add import:
```rust
use crate::gui::renderer::types::{RenderTarget, ScrollbarState};
```

**Step 2: Build and test**

Run: `cargo build` — expect clean
Run: `cargo test` — expect 341 tests pass

**Step 3: Commit**

```bash
git add src/gui/events/render_shared.rs
git commit -m "refactor: update render_shared.rs callers to use RenderTarget/ScrollbarState"
```

---

### Task 7: Add TabSlot and Update CPU Tab Bar Internal Methods

**Files:**
- Modify: `src/gui/renderer/types.rs` — add `TabSlot`
- Modify: `src/gui/renderer/tab_bar/mod.rs` — update `draw_tab_bar` loop to use `TabSlot`
- Modify: `src/gui/renderer/tab_bar/tab_content.rs` — update method signatures
- Modify: `src/gui/renderer/tab_bar/rename_field.rs` — update method signatures

**Step 1: Add TabSlot to types.rs**

```rust
/// Per-tab layout slot used during CPU tab bar rendering.
///
/// Groups the tab-specific position and state data that internal
/// `CpuRenderer` tab drawing methods all receive.
pub struct TabSlot<'a> {
    pub index: usize,
    pub tab: &'a TabInfo<'a>,
    pub x: u32,
    pub width: u32,
    pub is_hovered: bool,
}
```

**Step 2: Update tab_bar/mod.rs draw_tab_bar loop**

In `draw_tab_bar()`, after computing `tab_x`, `is_hovered`, create a `TabSlot` and pass it to child methods:

```rust
let slot = TabSlot {
    index: i,
    tab,
    x: tab_x,
    width: tw,
    is_hovered,
};
```

Update calls from:
```rust
self.draw_tab_background(buffer, buf_width, bar_h, tab, tab_x, tw, tab_bar_height);
```
to:
```rust
self.draw_tab_background(target, &slot);
```

And similarly for `draw_tab_rename_field`, `draw_tab_number`, `draw_tab_content`.

Remove the `_tab_bar_height` parameter from all these methods (it was always unused/prefixed with `_`).

**Step 3: Update tab_content.rs methods**

Update `draw_tab_number`, `draw_tab_content`, `draw_tab_title`, `draw_security_badge`, `draw_close_button` to use `RenderTarget` + `TabSlot` where applicable.

For `draw_tab_title`, `draw_security_badge`, `draw_close_button` — these are called from within `draw_tab_content` and don't need `TabSlot` since they receive already-computed values. They still benefit from `RenderTarget` replacing `buffer/buf_width/bar_h`.

**Step 4: Update rename_field.rs**

Update `draw_tab_rename_field` to take `(target: &mut RenderTarget<'_>, slot: &TabSlot<'_>)`.

Internal helpers `draw_rename_background`, `draw_rename_text`, `draw_rename_cursor` take `RenderTarget` instead of `buffer/buf_width/bar_h`.

**Step 5: Remove all #[allow(clippy::too_many_arguments)] from these files**

**Step 6: Build and test**

Run: `cargo build` — expect clean
Run: `cargo test` — expect 341 tests pass

**Step 7: Commit**

```bash
git add src/gui/renderer/types.rs src/gui/renderer/tab_bar/mod.rs src/gui/renderer/tab_bar/tab_content.rs src/gui/renderer/tab_bar/rename_field.rs
git commit -m "refactor: add TabSlot, update CPU tab bar internals to use RenderTarget/TabSlot"
```

---

### Task 8: Update CPU Primitives and Remaining Internal Methods

**Files:**
- Modify: `src/gui/renderer/cpu/primitives.rs`
- Modify: `src/gui/renderer/tab_bar/primitives/rounded.rs`
- Modify: `src/gui/renderer/tab_bar/primitives/shapes.rs`
- Modify: `src/gui/renderer/security.rs`

**Step 1: Update primitives.rs**

`draw_char()` (8 args) — takes `buffer, buf_width, buf_height, x, y, character, fg`. Change to `target: &mut RenderTarget<'_>, x, y, character, fg`. (5 args — under limit.)

`draw_bg()` (6 args) — already under limit but update for consistency: `target: &mut RenderTarget<'_>, x, y, color`.

`draw_rounded_rect()` (11 args) — takes `buffer, buf_width, buf_height, x, y, w, h, radius, color, alpha`. Change to `target: &mut RenderTarget<'_>, x, y, w, h, radius, color, alpha` (8 args). OR use existing `RoundedRectCmd` struct and migrate callers to `draw_rounded_rect_cmd()`.

`draw_rounded_impl()` (static, 11 args) — takes buffer directly, not &self. Change to `target: &mut RenderTarget<'_>` for first 3 params. (9 args — still high but includes a function pointer). This is an internal static method, the function pointer makes it hard to group further. Consider keeping as-is since it's private, OR inline the coverage function variants.

**Recommended approach for draw_rounded_rect/draw_rounded_impl:** Since `RoundedRectCmd` already exists with `draw_rounded_rect_cmd`, migrate the 2-3 callers of `draw_rounded_rect()` to use `draw_rounded_rect_cmd()` instead. Then `draw_rounded_rect()` becomes unused and can be deleted. `draw_rounded_impl` stays as an internal implementation detail of `draw_rounded_rect_cmd` and `draw_top_rounded_rect`. For `draw_rounded_impl`, use `RenderTarget` for the first 3 params.

**Step 2: Update rounded.rs**

`draw_top_rounded_rect()` (11 args → 8 with RenderTarget) — replace `buffer, buf_width, buf_height` with `target: &mut RenderTarget<'_>`.

**Step 3: Update shapes.rs**

`draw_stroked_line()` (static, 10 args → 7 with RenderTarget) — replace `buffer, buf_width, buf_height` with `target: &mut RenderTarget<'_>`.

**Step 4: Update security.rs**

`draw_security_shield_icon()` (9 args → 6 with RenderTarget) — replace `buffer, buf_width, buf_height` with `target: &mut RenderTarget<'_>`.

**Step 5: Update all callers of these methods**

Every call site that previously passed `buffer, buf_width, buf_height` (or `bar_h`) needs updating. These are primarily within the tab_bar/ and cursor.rs files already updated in previous tasks.

**Step 6: Remove all #[allow(clippy::too_many_arguments)]**

**Step 7: Build and test**

Run: `cargo build` — expect clean
Run: `cargo test` — expect 341 tests pass

**Step 8: Commit**

```bash
git add src/gui/renderer/cpu/primitives.rs src/gui/renderer/tab_bar/primitives/rounded.rs src/gui/renderer/tab_bar/primitives/shapes.rs src/gui/renderer/security.rs
git commit -m "refactor: update CPU primitives and security to use RenderTarget"
```

---

### Task 9: Update GPU Internal Methods

**Files:**
- Modify: `src/gui/renderer/gpu/ui_commands.rs`
- Modify: `src/gui/renderer/gpu/tab_rendering.rs`
- Modify: `src/gui/renderer/gpu/grid_packing.rs`
- Modify: `src/gui/renderer/gpu/scrollbar.rs`

**Step 1: ui_commands.rs — push_rounded_rect, push_line, push_glyph**

These are GPU-specific command buffer methods that don't use pixel buffers at all. Their `too_many_arguments` is from shape parameters (x, y, w, h, r, color, alpha).

For `push_rounded_rect` (7 args): use existing `RoundedRectCmd` struct — change to `push_rounded_rect_cmd(&mut self, cmd: &RoundedRectCmd)`. OR since these map to GPU `GpuDrawCommand` fields directly, create a thin wrapper. The simplest fix: the args are `(x, y, w, h, r, color, alpha)` — exactly what `RoundedRectCmd` contains. So accept `&RoundedRectCmd`.

Actually, for GPU these are performance-critical hot paths in the render loop. Adding a struct just to destructure it immediately is overhead. Better approach: since clippy limit is 7 and these have exactly 7 (not counting `&mut self`), check if clippy actually fires. Clippy counts `&mut self` as an argument, so 7 + self = 8 total, which triggers clippy.

Simplest approach: accept `RoundedRectCmd` for `push_rounded_rect` (exact field match). For `push_line` and `push_glyph`, create small structs:

```rust
pub(super) struct LineCmd {
    pub x1: f32, pub y1: f32,
    pub x2: f32, pub y2: f32,
    pub width: f32,
    pub color: u32, pub alpha: f32,
}

pub(super) struct GlyphCmd {
    pub x: f32, pub y: f32,
    pub atlas_x: f32, pub atlas_y: f32,
    pub atlas_w: f32, pub atlas_h: f32,
    pub color: u32, pub alpha: f32,
}
```

OR — even simpler — these methods map 1:1 to `GpuDrawCommand` fields. Just accept a `GpuDrawCommand` directly. But that leaks the command type detail.

**Recommended:** For `push_rounded_rect`, the existing `RoundedRectCmd` from types.rs has `opacity` instead of `alpha` but otherwise matches. We can adapt.

For `push_line` and `push_glyph` — these are internal to the GPU renderer with 2-3 callers each. The pragmatic choice is to use the existing `RoundedRectCmd` where it fits, and for the others, just keep the args but raise clippy's threshold OR restructure the callers.

**Actually, simplest correct approach:** Since these functions are `pub(super)` (GPU-internal), and after counting properly — `push_rounded_rect` has 7 params + &mut self = 8 total... Let me recount:

- `push_rounded_rect(&mut self, x, y, w, h, r, color, alpha)` — 8 including self
- `push_line(&mut self, x1, y1, x2, y2, width, color, alpha)` — 8 including self
- `push_glyph(&mut self, x, y, atlas_x, atlas_y, atlas_w, atlas_h, color, alpha)` — 9 including self

clippy default limit is 7 total (including self). So all three trigger. Use struct wrappers.

**Step 2: tab_rendering.rs — tab_content_commands**

`tab_content_commands(&mut self, tab_index, tab, tab_count, buf_width, tab_x, tw, text_y, is_hovered)` — 9 args. With `TabSlot`, this becomes `tab_content_commands(&mut self, slot: &TabSlot, tab_count, buf_width, text_y)` — 5 args. Clean.

**Step 3: grid_packing.rs — queue_grid_batch**

`queue_grid_batch(&mut self, grid, selection, viewport_start, origin_x, origin_y, max_width, max_height, fg_dim)` — 9 args.

Group origin/size into a rect-like struct — actually `PaneRect` could work but the semantics are different (these are offsets, not absolute positions). Create a small struct:

```rust
// Already partially exists — use fields directly or create GridBatchRegion
```

OR just keep it as-is and count: with `RenderTarget` not applicable here (this is GPU-internal, no pixel buffer). The params are genuinely different each call. Pragmatic fix: group `(origin_x, origin_y, max_width, max_height)` into `PaneRect` since they describe a region:

```rust
fn queue_grid_batch(
    &mut self,
    grid: &Grid,
    selection: Option<&Selection>,
    viewport_start: usize,
    region: PaneRect,  // origin_x, origin_y, max_width, max_height
    fg_dim: f32,
)
```

That brings it to 6 args. Clean.

**Step 4: scrollbar.rs — render_scrollbar_in_rect_impl**

`render_scrollbar_in_rect_impl(&mut self, scroll_offset, scrollback_len, grid_rows, opacity, hover, rect)` — 7 args including self.

With `ScrollbarState`: `render_scrollbar_in_rect_impl(&mut self, state: &ScrollbarState, rect: PaneRect)` — 3 args. Clean.

Also update `render_scrollbar_impl` to use `ScrollbarState` + `buf_height`:
`render_scrollbar_impl(&mut self, buf_height: usize, state: &ScrollbarState)` — 3 args.

**Step 5: Remove all #[allow(clippy::too_many_arguments)]**

**Step 6: Build and test**

Run: `cargo build` — expect clean
Run: `cargo build --no-default-features` — expect clean (CPU-only)
Run: `cargo test` — expect 341 tests pass

**Step 7: Commit**

```bash
git add src/gui/renderer/gpu/
git commit -m "refactor: update GPU internal methods to use context structs"
```

---

### Task 10: Update shared layout functions

**Files:**
- Modify: `src/gui/renderer/shared/ui_layout.rs`
- Modify: `src/gui/renderer/shared/overlay_layout.rs`

**Step 1: ui_layout.rs — pin_icon_layout (8 args)**

```rust
pub fn pin_icon_layout(
    cx: f32, cy: f32, scale: f32,
    pinned: bool, hovered: bool,
    pin_active_color: u32, hover_color: u32, inactive_color: u32,
) -> PinIconLayout
```

Group the 3 colors into a struct:
```rust
pub struct PinColors {
    pub active: u32,
    pub hover: u32,
    pub inactive: u32,
}
```

New signature: `pin_icon_layout(cx, cy, scale, pinned, hovered, colors: &PinColors)` — 6 args. Clean.

Update the 1-2 callers to construct `PinColors`.

**Step 2: overlay_layout.rs — compute_drag_overlay_layout (8 args)**

```rust
pub fn compute_drag_overlay_layout(
    m: &TabLayoutMetrics, tab_count: usize, source_index: usize,
    source_title: &str, current_x: f64, indicator_x: f32, buf_width: u32,
) -> Option<DragOverlayLayout>
```

This is 7 args (not counting return). Clippy counts: 7 exactly = OK if limit is 7. Actually check — it has `#[allow]` so it must be triggering. Recount: `m, tab_count, source_index, source_title, current_x, indicator_x, buf_width` = 7 params. Clippy default is 7, so 7 should be fine (limit is "too many" = more than 7, i.e., 8+). Let me verify...

Actually clippy `too_many_arguments` default threshold is 7, meaning 7 or more triggers it. So 7 params triggers the lint. We need to reduce to 6.

Group `source_index` + `source_title` + `current_x` + `indicator_x` — these describe the drag state:
```rust
pub struct DragInfo<'a> {
    pub source_index: usize,
    pub source_title: &'a str,
    pub current_x: f64,
    pub indicator_x: f32,
}
```

New: `compute_drag_overlay_layout(m, tab_count, drag: &DragInfo, buf_width)` — 4 args. Clean.

**Step 3: Remove #[allow(clippy::too_many_arguments)]**

**Step 4: Build and test**

Run: `cargo build` — expect clean
Run: `cargo test` — expect 341 tests pass

**Step 5: Commit**

```bash
git add src/gui/renderer/shared/ui_layout.rs src/gui/renderer/shared/overlay_layout.rs
git commit -m "refactor: update shared layout functions to use context structs"
```

---

### Task 11: Final Verification and Cleanup

**Files:** All modified files

**Step 1: Run full clippy check**

Run: `cargo clippy 2>&1 | grep "too_many_arguments"` — expect ZERO results
Run: `cargo clippy 2>&1 | grep "dead_code"` — expect only legitimate `cfg_attr` uses
Run: `cargo clippy 2>&1 | grep "unused"` — check for any remaining underscore-prefix issues

**Step 2: Run full test suite**

Run: `cargo test` — expect 341 tests pass

**Step 3: Build both targets**

Run: `cargo build` — expect clean
Run: `cargo build --no-default-features` — expect clean

**Step 4: Grep for remaining allows**

Run: `grep -r "#\[allow(clippy::too_many_arguments)\]" src/` — expect ZERO results
Run: `grep -r "#\[allow(dead_code)\]" src/` — expect ZERO results (only `cfg_attr` variants OK)

**Step 5: Commit if any fixups needed**

```bash
git add -A
git commit -m "refactor: final cleanup — zero allow(too_many_arguments) and allow(dead_code)"
```

---

## Execution Notes

- **Build frequently** — this is a large refactor touching 20+ files. Build after each task to catch issues early.
- **GPU padding fields (`_pad0` etc.)** — leave untouched, required for WGSL alignment.
- **`#[allow(unreachable_code)]`** in `close_dialog.rs` and `pty/mod.rs` — leave untouched, legitimate platform patterns.
- **`#[cfg_attr(target_os = "macos", allow(dead_code))]`** — leave untouched, legitimate platform-conditional.
- **Test both build targets** — `cargo build` (GPU) and `cargo build --no-default-features` (CPU-only).
