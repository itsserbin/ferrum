# Function Decomposition & Deduplication Design

## Problem

After the previous round of file-level decomposition, several large functions remain:
- `render_cpu_frame` (255 lines) and `render_gpu_frame` (250 lines) share ~100 identical lines
- `on_cursor_moved` (157 lines) mixes 6+ responsibilities
- `handle_rename_input` (194 lines) handles navigation, deletion, and input in one match
- `handle_ctrl_shortcuts` (156 lines) groups unrelated shortcut categories

## Phase 1: Deduplicate render frame preparation

**New file:** `src/gui/events/render_shared.rs`

Create `TabBarFrameState` struct holding all tab bar metadata for rendering:
- `tab_infos: Vec<TabInfo>`, `tab_tooltip: Option<String>`
- `drag_info: Option<(usize, f64, f32)>`, `tab_offsets: Option<Vec<f32>>`
- `show_tooltip: bool`, `tab_bar_visible: bool`

Extract `FerrumWindow::build_tab_bar_state(bw) -> TabBarFrameState` (~100 lines from each render fn).

Extract `scrollbar_opacity(tab: &TabState) -> f32` (duplicated ~15 lines).

Extract `should_show_cursor(blink_start: Instant, style: CursorStyle) -> bool` (duplicated ~6 lines).

**Result:** Each render function drops from ~255 to ~130 lines. Zero duplication.

## Phase 2: Split on_cursor_moved (157 → ~30 dispatcher + small handlers)

In `src/gui/events/mouse/cursor.rs`:

- `on_cursor_moved` becomes a dispatcher (~30 lines)
- `handle_scrollbar_drag(&mut self, my: f64)` — scrollbar dragging logic (~30 lines)
- `handle_scrollbar_hover(&mut self, mx: f64, my: f64)` — hover detection (~25 lines)
- `handle_mouse_reporting(&mut self, row: usize, col: usize) -> bool` — mouse mode (~20 lines)

Resize edge detection and context menu hover remain inline (already small).

## Phase 3: Split handle_rename_input (194 → ~40 dispatcher + focused handlers)

In `src/gui/events/keyboard/rename.rs`:

- `handle_rename_navigation(rename, key, ctrl, shift)` — ArrowLeft/Right, Home/End (~60 lines)
- `handle_rename_deletion(rename, key, ctrl)` — Backspace, Delete with selection (~40 lines)
- `handle_rename_text_input(rename, key)` — Character input (~15 lines)
- Main match becomes dispatcher: Enter→commit, Escape→cancel, else→delegates (~40 lines)

Helper functions (prev/next_char_boundary, word boundaries) stay as-is.

## Phase 4: Split handle_ctrl_shortcuts (156 → ~20 dispatcher + groups)

In `src/gui/events/keyboard/shortcuts.rs`:

- `handle_clipboard_shortcuts(key, physical) -> Option<bool>` — Copy, Paste, Cut (~25 lines)
- `handle_tab_shortcuts(key, physical, ...) -> Option<bool>` — Ctrl+T/W/N/digit/Tab (~50 lines)
- `handle_super_text_navigation(physical) -> Option<bool>` — Super+A/E/B/F/D/K/U (~30 lines)
- `handle_super_arrow_shortcuts(key) -> Option<bool>` — Super+Arrow/Backspace/Delete (~30 lines)
- Main function chains `if let Some(r) = ...` calls (~20 lines)

## Constraints

- No new public API changes
- All 212+ tests must pass after each phase
- Both GPU and CPU-only builds must compile cleanly
