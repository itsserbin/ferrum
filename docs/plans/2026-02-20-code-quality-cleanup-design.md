# Code Quality Cleanup Design

## Problem

32 instances of `#[allow(clippy::too_many_arguments)]`, 3 instances of `#[allow(dead_code)]`, and ~23 unnecessary `_` prefixes on function parameters across the renderer codebase.

## Solution: Context Structs + Dead Code Removal

### New Structs

#### 1. `RenderTarget<'a>` — pixel buffer surface

```rust
pub struct RenderTarget<'a> {
    pub buffer: &'a mut [u32],
    pub width: usize,
    pub height: usize,
}
```

Used by every rendering method. Replaces the repeated `(buffer, buf_width, buf_height)` triple. Defined in `src/gui/renderer/mod.rs` or a new `types.rs`.

#### 2. `ScrollbarState` — scrollbar rendering params

```rust
pub struct ScrollbarState {
    pub scroll_offset: usize,
    pub scrollback_len: usize,
    pub grid_rows: usize,
    pub opacity: f32,
    pub hover: bool,
}
```

Used by `render_scrollbar()` and `render_scrollbar_in_rect()`. Replaces 5 separate parameters.

#### 3. `TabSlot<'a>` — individual tab position/state for CPU renderer

```rust
pub struct TabSlot<'a> {
    pub index: usize,
    pub tab: &'a TabInfo<'a>,
    pub x: u32,
    pub width: u32,
    pub is_hovered: bool,
}
```

Used by internal CPU renderer methods: `draw_tab_number()`, `draw_tab_content()`, `draw_tab_background()`, `draw_tab_rename_field()`. Replaces 4-5 parameters.

### Trait `Renderer` Changes

All methods that accept `(buffer, buf_width, buf_height)` switch to `RenderTarget<'_>`. Scrollbar methods additionally use `ScrollbarState`.

Before → After examples:
- `render(buffer, bw, bh, grid, selection, viewport)` → `render(target, grid, selection, viewport)`
- `render_scrollbar(buffer, bw, bh, offset, len, rows, opacity, hover)` → `render_scrollbar(target, state)`
- `render_scrollbar_in_rect(buffer, bw, bh, offset, len, rows, opacity, hover, rect)` → `render_scrollbar_in_rect(target, state, rect)`

### `draw_rounded_rect` / `draw_rounded_impl`

These have 10-11 params but `RoundedRectCmd` already exists. Migrate callers to use `draw_rounded_rect_cmd()` with the existing struct, then remove the raw-args variant.

### Dead Code Removal

- `state.rs`: Delete `terminal()` and `terminal_mut()` methods (unused, marked "for later tasks")
- `pane.rs`: Delete `DividerHit` struct (unused)
- `traits.rs`: Remove `#![allow(dead_code)]` module-level attribute

### Underscore Prefix Cleanup

- `gpu/trait_impl.rs`: Remove `_` from ~23 `_buffer`, `_buf_width`, `_buf_height` params (they become part of `RenderTarget`, GPU impl can just ignore `target` fields without prefix)
- `tab_content.rs`, `rename_field.rs`: Remove `_tab_bar_height` parameter entirely (never used)

### `#[allow(unreachable_code)]` (2 instances)

Keep as-is. These are legitimate platform-conditional fallback patterns in `close_dialog.rs` and `pty/mod.rs`.

### GPU Padding Fields (`_pad0`, `_pad1`, etc.)

Keep as-is. Required for WGSL struct alignment.

## Scope

### Files to modify

**Trait & types:**
- `src/gui/renderer/mod.rs` — add struct definitions
- `src/gui/renderer/traits.rs` — update trait signatures

**CPU renderer:**
- `src/gui/renderer/cursor.rs`
- `src/gui/renderer/scrollbar.rs`
- `src/gui/renderer/terminal.rs`
- `src/gui/renderer/cpu/primitives.rs`
- `src/gui/renderer/tab_bar/mod.rs`
- `src/gui/renderer/tab_bar/tab_content.rs`
- `src/gui/renderer/tab_bar/rename_field.rs`
- `src/gui/renderer/tab_bar/drag_overlay.rs`
- `src/gui/renderer/security.rs`
- `src/gui/renderer/tab_bar/primitives/rounded.rs`
- `src/gui/renderer/tab_bar/primitives/shapes.rs`
- `src/gui/renderer/shared/ui_layout.rs`
- `src/gui/renderer/shared/overlay_layout.rs`

**GPU renderer:**
- `src/gui/renderer/gpu/trait_impl.rs`
- `src/gui/renderer/gpu/ui_commands.rs`
- `src/gui/renderer/gpu/tab_rendering.rs`
- `src/gui/renderer/gpu/grid_packing.rs`
- `src/gui/renderer/gpu/scrollbar.rs`

**Callers:**
- `src/gui/events/render_shared.rs`

**Dead code:**
- `src/gui/state.rs`
- `src/gui/pane.rs`

## Success Criteria

- Zero `#[allow(clippy::too_many_arguments)]` in codebase
- Zero `#[allow(dead_code)]` (except legitimate platform-conditional `cfg_attr`)
- No unnecessary `_` prefixed parameters
- `cargo clippy` passes clean
- `cargo test` passes
- `cargo build` and `cargo build --no-default-features` both succeed
