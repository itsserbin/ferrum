# Page-Based GraphemeCell Terminal Architecture

**Branch:** `feat/page-based-grapheme`
**Date:** 2026-03-05
**Status:** Approved

## Goals

- Grapheme-aware reflow: correct handling of wide chars (CJK, emoji), grapheme clusters
- Page-based memory: unified `PageList` replacing `Grid` + `VecDeque<Row>` for scrollback
- Tracked pins: auto-updating cursor and selection after reflow
- 30 000 rows scrollback by default (~100 MB at 120 cols), configurable in Settings UI
- Keep SIGWINCH compatibility: cursor at col=0 of logical line start after reflow
- Safe Rust only — no mmap, no unsafe

## Architecture Overview

### New Types

#### `GraphemeCell` (replaces `Cell`)

Stores a grapheme cluster inline (up to 8 UTF-8 bytes covers all common Unicode including CJK
and most emoji). Falls back to heap allocation for rare multi-codepoint sequences exceeding
8 bytes (e.g. family emoji with ZWJ sequences).

```
grapheme_bytes: [u8; 8]   — inline UTF-8 storage
grapheme_len:  u8          — byte length in grapheme_bytes (0 = empty/space)
width:         u8          — display columns: 1 = normal, 2 = wide (CJK/emoji), 0 = spacer
fg:            Color       — foreground color (3 bytes)
bg:            Color       — background color (3 bytes)
flags:         u8          — bold, dim, italic, reverse, strikethrough packed as bits
underline:     u8          — UnderlineStyle (None/Single/Double)
overflow:      Option<Box<str>> — heap storage for graphemes > 8 bytes
```

Estimated size: 24-28 bytes per cell.

Wide char representation: a wide char occupies two adjacent cells. The first cell has `width=2`
and the character. The second (spacer) cell has `width=0` and empty `grapheme_bytes`.

When the terminal is too narrow to fit a wide char (< 2 cols), it is replaced with `?`.

#### `Page`

A fixed-size block of 256 rows. Heap-allocated as `Box<PageData>` where `PageData` contains
the rows array.

```rust
struct Page {
    rows: Box<[Row; PAGE_SIZE]>,  // PAGE_SIZE = 256
    len: usize,                   // rows actually in use
}
```

`Row` stores `Vec<GraphemeCell>` and a `wrapped: bool` flag (soft-wrap continuation).

#### `PageList`

Unified storage for all terminal content: scrollback + visible area.

```rust
struct PageList {
    pages:          Vec<Box<Page>>,
    free_list:      Vec<usize>,       // indices of reusable pages
    viewport_start: PageCoord,        // where the visible area begins
    max_rows:       usize,            // scrollback limit (default: 30_000)
    tracked_pins:   Vec<*mut PageCoord>,
}
```

When `max_rows` is exceeded, the oldest page is returned to the free-list (O(1) eviction).

### Tracked Pins

```rust
struct PageCoord {
    page_idx: usize,
    row:      u8,
    col:      u16,
}
```

A `TrackedPin` is a `PageCoord` registered with `PageList`. On every reflow or scrollback
eviction, `PageList` updates all registered pins automatically.

`Terminal` holds three tracked pins:
- `cursor_pin` — current cursor position
- `selection_start_pin` — selection anchor
- `selection_end_pin` — selection end

### File Structure

**New files:**
- `src/core/grapheme_cell.rs` — `GraphemeCell`, `GraphemeStr` (inline/heap enum)
- `src/core/page.rs` — `Page`, `Row`
- `src/core/page_list/mod.rs` — `PageList`, `PageCoord`
- `src/core/page_list/reflow.rs` — grapheme-aware reflow
- `src/core/page_list/iter.rs` — `ViewportIter` for renderer
- `src/core/tracked_pin.rs` — `TrackedPin`

**Removed files:**
- `src/core/cell.rs` → replaced by `grapheme_cell.rs`
- `src/core/grid.rs` → replaced by `page.rs` + `page_list/`
- `src/core/terminal/reflow.rs` → replaced by `page_list/reflow.rs`
- `src/core/terminal/resize.rs` → rewritten using `PageList`

**Modified files:**
- `src/core/terminal.rs` — uses `PageList`, cursor/selection become `TrackedPin`
- `src/core/selection.rs` — `TrackedPin` instead of row/col indices
- `src/gui/events/render_shared.rs` — uses `PageList::viewport_rows()` instead of `Grid`
- `src/gui/renderer/gpu/` and `cpu/` — reads `GraphemeCell` (`&str` instead of `char`)
- `src/config/model.rs` — `max_scrollback` default changed from 1000 to 30_000
- Settings window (macOS/Linux/Windows) — add stepper for `max_scrollback`
- Tests — rewritten for new types, new tests for wide chars and tracked pins

## Reflow Algorithm

### `collect_logical_lines()`

Iterates `PageList` from start via `ViewportIter`. Stitches soft-wrapped rows into logical
lines by accumulating cells until `row.wrapped == false`. Counts display columns via
`cell.width` rather than cell count. Tracks which logical line contains `cursor_pin`.

### `rewrap_lines()`

Pure function: takes logical lines + new column width, produces new `Vec<Row>`.

Column counting uses `cell.width`:
- ASCII/normal: width=1 → column += 1
- Wide char: width=2 → if `col + 2 > new_cols`, wrap first, then place both columns
- Spacer (width=0): skipped (belongs to previous wide char)
- If terminal narrower than 2 cols: replace wide char with `?` (graceful degradation)

After rewrap, all `TrackedPin`s are updated. `cursor_pin.col` is set to 0 (start of logical
line) for SIGWINCH compatibility — readline/zsh SIGWINCH handler sends CR then redraws from
the current row, so placing cursor at col=0 avoids double-prompt artifacts.

## SIGWINCH Compatibility (preserved)

After every reflow:
1. `cursor_pin` points to the first physical row of the cursor's logical line
2. `cursor_pin.col = 0`
3. Shell receives SIGWINCH → sends CR (no-op, already at col 0) → erases to end → redraws

This matches the current behavior in `resize.rs` and must not change.

## Alt Screen

Alt screen gets `simple_resize` (no reflow), same as today. The alt screen has its own
`PageList` with a small fixed size (no scrollback needed).

## Settings UI

`TerminalConfig.max_scrollback` default: `30_000`.
`TerminalConfig` gains constants: `SCROLLBACK_MIN = 1_000`, `SCROLLBACK_MAX = 100_000`,
`SCROLLBACK_STEP = 1_000`.

Settings window gains one stepper + text field for `max_scrollback` in the Terminal tab,
following the same pattern as `cursor_blink_interval_ms`.

## Memory Budget

At 120 columns with 28 bytes/cell:

| Scrollback rows | Memory    |
|----------------|-----------|
| 10 000          | ~34 MB    |
| 30 000          | ~100 MB   |
| 100 000         | ~336 MB   |

Default 30 000 rows is reasonable for a modern machine.

## Key Invariants

1. `cursor_pin.col == 0` after every reflow (SIGWINCH compat)
2. Alt screen never reflowed (simple resize only)
3. Wide char never split across line boundary during reflow
4. Wide char replaced with `?` when terminal < 2 cols wide
5. All `TrackedPin`s valid after reflow and scrollback eviction
6. `cargo clippy` produces zero warnings
7. All existing tests pass; new tests cover wide chars, graphemes, tracked pins

## Out of Scope

- mmap / memory pool (safe Rust is sufficient for 30 000 rows)
- Kitty image protocol
- OSC 8 hyperlinks in scrollback
- Right-to-left (BiDi) text
