---
title: Research - Terminal Window Resize Handling (WezTerm & Kitty)
task_file: Ad-hoc user request
scratchpad: .specs/scratchpad/7904b3a1.md
created: 2026-02-15
status: complete
---

# Research: Terminal Window Resize Handling

## Executive Summary

WezTerm and Kitty handle terminal resize with contrasting philosophies: WezTerm emphasizes scrollback integration, while Kitty prioritizes visual stability (maintainer: "I find text moving around disorienting"). For horizontal resize, modern terminals like Alacritty implement text reflow using a WRAPLINE flag to track line continuations, while classic terminals (xterm, st) simply truncate/extend without reflow.

**Key findings:**
- **Vertical resize**: All modern terminals move lines to scrollback on shrink; behavior splits on grow (pull from scrollback vs add blanks)
- **Horizontal reflow**: Requires WRAPLINE flag per line; modern expectation but complex (~300-500 LOC)
- **Minimum viable**: Scrollback buffer + vertical resize handling = essential; horizontal reflow = nice-to-have
- **Critical pitfall**: Cursor position during reflow causes bugs across all implementations

**For Ferrum**: Implement scrollback buffer first (high impact, ~100 LOC). Defer horizontal reflow (medium impact, high complexity).

## Related Existing Research

- [research-ferrum-architecture-review.md](./research-ferrum-architecture-review.md) - Overall architecture context
- [research-terminal-crate-ecosystem.md](./research-terminal-crate-ecosystem.md) - Related terminal crates
- [research-vte-013-api.md](./research-vte-013-api.md) - Parser being used

---

## 1. Vertical Resize Behavior

### How Different Terminals Handle It

| Terminal | Shrinking (Fewer Rows) | Growing (More Rows) |
|----------|------------------------|---------------------|
| **Alacritty** | Lines → history, pull from history on grow | Pull from scrollback first, then add blanks |
| **Kitty** | Lines → history | **Blank lines at bottom** (intentional design) |
| **xterm/VTE** | Lines → scrollback | Scrollback lines appear at top |
| **Windows Terminal** | Lines → history | Empty lines at bottom |
| **st (suckless)** | Simple truncation | Simple extension (blanks) |

### Content Preservation When Shrinking

**Universal behavior**: When vertical space reduces, lines that no longer fit move to scrollback/history buffer.

**Implementation**: As viewport shrinks from N to M rows, the top (N-M) lines are moved to scrollback in order. This prevents data loss.

### Scrollback Interaction

**Key mechanism** (from WezTerm documentation):
> "When a newline is processed, if the cursor position would move off the bottom of the screen, and the scroll margins are set to match the full height of the viewport, then the top row of the grid is moved into immutable scrollback."

Lines in scrollback become **immutable** — they cannot be modified by escape sequences, only by future resize operations.

**Critical insight**: During normal operation, only lines at the **bottom** of the screen trigger scrollback movement (when newline would scroll content off). During resize, lines at the **top** move to scrollback (when vertical space shrinks).

### Cursor Position Adjustment

**Standard approach**: Clamp cursor to new viewport dimensions.

```rust
self.cursor_row = self.cursor_row.min(new_rows.saturating_sub(1));
self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));
```

**Edge case**: Multi-line editing (readline) can lose cursor position. If cursor not on last line when narrowing, wrapping can push cursor down; on re-widening, cursor stays on last line rather than returning to original content.

### Design Philosophy Divide

**Kitty approach** (from maintainer in issue #1047):
> "I like the current behavior of not changing what is shown on the screen when increasing the window size, I find text moving around disorienting."

When growing vertically, Kitty adds blank lines at bottom. Content stays stationary.

**Alacritty/xterm approach**: When growing vertically, pull lines from scrollback to show more context.

**Recommendation**: Both are valid. Kitty's approach is simpler to implement (~20 LOC). Alacritty's approach has better UX (~100 LOC).

---

## 2. Horizontal Resize & Text Reflow

### What is Text Reflow?

Text reflow is automatically adjusting wrapped text when terminal width changes:
- **Wider**: Long lines that wrapped to multiple rows merge back to fewer rows
- **Narrower**: Lines that fit in old width wrap to multiple rows in new width

### Do Terminals Track Logical vs Visual Lines?

Yes, modern terminals with reflow capability track this with a **wrapped** flag (WRAPLINE).

**Logical line**: What was actually typed or output (e.g., 200-character command line)
**Visual lines**: How it's displayed (e.g., wrapped to 3 rows of 80 columns each)

Without tracking which visual lines are continuations, terminals cannot reflow correctly — they can't distinguish intentional newlines from automatic wrapping.

### Implementation Approaches

**Classic terminals (xterm, st)**: NO REFLOW
- Horizontal resize just truncates or adds blank space
- Simple: ~10 LOC
- Poor UX: content appears broken after resize

**Modern terminals (Alacritty, urxvt, gnome-terminal)**: WITH REFLOW

**Data structure requirement**:
```rust
struct Line {
    cells: Vec<Cell>,
    wrapped: bool,  // CRITICAL: marks this line continues on next
}
```

**Alacritty algorithm** (from resize.rs):

**Growing columns** (reflow wider):
- Process lines in **reverse order** (bottom to top)
- Check if previous line has `wrapped` flag set
- If yes, pull cells from current line to previous line
- Remove wrap flag when line is complete
- Delete now-empty lines

**Shrinking columns** (reflow narrower):
- Process lines **forward** (top to bottom)
- If line exceeds new width, split excess to next line
- Set `wrapped` flag on truncated line
- Handle wide characters: use LEADING_WIDE_CHAR_SPACER if split

**Performance**: Alacritty achieves "immediate" reflow even with 100k lines in scrollback through batching and efficient data structures.

### VT100 Wrapping Mechanism

Terminal wrapping uses **Last Column Flag (LCF)** (sometimes called "VT100 glitch"):
1. Character at rightmost column: Set LCF=1, don't advance cursor
2. Next character with LCF=1: Move to next line first, then draw

This "deferred wrap" allows full-width lines without extra blank lines.

### Edge Cases

**Wide characters**: CJK characters and emoji take 2 cells. Cannot split across lines. Alacritty inserts LEADING_WIDE_CHAR_SPACER marker when a wide char would split.

**Alt screen mode**: Applications like vim do their own layout. Terminal reflow can interfere and cause visual artifacts. Alacritty disables reflow in alt screen for this reason.

---

## 3. Simplest Correct Approach

### Is Text Reflow Mandatory?

**No.** Classic terminals like xterm and st work fine without reflow.

However, modern user expectations favor reflow. Without it, horizontal resize feels "broken" — content appears truncated or has excessive blank space.

### Minimum Viable Behavior

**MUST HAVE** (feels broken without):
1. **Scrollback buffer**: Store lines that scroll off top (~50-100 LOC)
2. **Vertical shrink → scrollback**: Lines pushed off top go to history (~50 LOC)
3. **Cursor clamping**: Keep cursor in valid range (Ferrum already has this)

**NICE TO HAVE** (modern expectation):
1. **Vertical grow → pull from scrollback**: Show more context when expanding (~50 LOC)
2. **Horizontal reflow**: Rewrap text on width change (~300-500 LOC)
3. **Wrapped line tracking**: Add `wrapped` flag to lines (~50 LOC, foundation for reflow)

**CAN SKIP** (advanced features):
1. Configurable reflow toggle (VTE has this, probably overkill)
2. Marker/bookmark tracking during resize (niche use case)

### Recommended Implementation Path

**Phase 1: Scrollback** (essential, low complexity)
- Add scrollback Vec<Line> separate from viewport
- When scrolling at bottom, push top line to scrollback
- Limit scrollback size (e.g., 10000 lines)
- On vertical shrink, move excess lines to scrollback

**Phase 2: Wrapped Line Tracking** (foundation, medium complexity)
- Change Grid to Vec<Line> with `wrapped: bool` field
- Set flag when wrapping at edge during print
- No immediate user benefit, enables Phase 3

**Phase 3: Horizontal Reflow** (optional, high complexity)
- Implement Alacritty-style algorithm
- Process lines in reverse for grow, forward for shrink
- Handle wide characters carefully
- Test with large scrollback (performance)

### Complexity vs Impact

| Feature | LOC | Complexity | User Impact | Priority |
|---------|-----|------------|-------------|----------|
| Scrollback buffer | 50-100 | Low | **High** | **1** |
| Vertical resize + scrollback | 100-150 | Medium | **High** | **2** |
| Wrapped line tracking | 50-100 | Medium | None (foundation) | 3 |
| Horizontal reflow | 300-500 | High | Medium | 4 |

---

## 4. Common Pitfalls & Edge Cases

### 1. Cursor in Scrollback After Reflow

**Problem**: Reflow can calculate cursor position into immutable scrollback region.

**Impact**: Next write corrupts history.

**Solution**: Always clamp cursor to viewport after any resize operation.

### 2. Wide Character Splitting

**Problem**: 2-cell wide characters (CJK, emoji) can split across line boundaries during reflow.

**Impact**: Rendering corruption, wrong characters displayed.

**Solution**: Never split wide chars. Use LEADING_WIDE_CHAR_SPACER marker or force wide char to next line.

### 3. Cursor Position Loss (Multi-line Editing)

**Problem**: Shell readline uses multiple lines. Resize changes line count, cursor jumps to wrong location.

**Impact**: Editing breaks, text overwrites wrong location.

**Solution**: Track cursor relative to content, not absolute position. This is **hard** — many professional terminals still have bugs here.

### 4. Alt Screen Resize Artifacts

**Problem**: Applications like vim manage their own layout. Terminal reflow interferes.

**Impact**: Visual glitches during resize.

**Solution**: Disable reflow in alt screen mode (Alacritty does this) OR accept artifacts as minor issue.

### 5. Performance with Large Scrollback

**Problem**: Reflow is O(n) in scrollback size. 100,000 lines can cause hangs.

**Impact**: Terminal freezes during resize.

**Solution**:
- Use efficient data structures (ring buffer)
- Batch operations (CircularList.splice batching in xterm.js)
- Limit scrollback size
- Consider lazy reflow (only reflow visible region)

### 6. Save/Restore Cursor During Resize

**Problem**: ESC 7 (save cursor) / ESC 8 (restore cursor) positions become invalid after resize.

**Impact**: Cursor restore jumps to wrong position.

**Solution**: Clamp saved cursor positions on resize. Ferrum already tracks saved_cursor, just need to clamp it:
```rust
self.saved_cursor.0 = self.saved_cursor.0.min(new_rows.saturating_sub(1));
self.saved_cursor.1 = self.saved_cursor.1.min(new_cols.saturating_sub(1));
```

### 7. Scroll Region Reset

**Problem**: Applications set custom scroll regions (DECSTBM). Resize makes regions invalid.

**Impact**: Scroll region may exceed new viewport size.

**Solution**: Reset to full viewport on resize. Ferrum already does this correctly:
```rust
self.scroll_top = 0;
self.scroll_bottom = rows - 1;
```

---

## Implementation Guidance

### Phase 1: Add Scrollback Buffer

**Data Structure:**
```rust
pub struct Terminal {
    pub grid: Grid,                  // Visible viewport
    scrollback: Vec<Vec<Cell>>,      // History (immutable)
    max_scrollback: usize,           // e.g., 10000
    scrollback_offset: usize,        // Current scroll position
    // ... existing fields ...
}
```

**Scroll Up Logic:**
```rust
fn scroll_up_region(&mut self, top: usize, bottom: usize) {
    // Only push to scrollback if scrolling full screen from top
    if top == 0 && bottom == self.grid.rows - 1 {
        // Save top line to scrollback
        let top_line: Vec<Cell> = (0..self.grid.cols)
            .map(|col| self.grid.get(0, col).clone())
            .collect();

        self.scrollback.push(top_line);

        // Trim if exceeds limit
        if self.scrollback.len() > self.max_scrollback {
            self.scrollback.remove(0);
        }
    }

    // Existing scroll logic
    for row in (top + 1)..=bottom {
        for col in 0..self.grid.cols {
            let cell = self.grid.get(row, col).clone();
            self.grid.set(row - 1, col, cell);
        }
    }
    for col in 0..self.grid.cols {
        self.grid.set(bottom, col, Cell::default());
    }
}
```

**Resize with Scrollback:**
```rust
pub fn resize(&mut self, rows: usize, cols: usize) {
    let old_rows = self.grid.rows;

    // Handle vertical shrink: move lines to scrollback
    if rows < old_rows {
        for _ in 0..(old_rows - rows) {
            let line: Vec<Cell> = (0..self.grid.cols)
                .map(|col| self.grid.get(0, col).clone())
                .collect();
            self.scrollback.push(line);

            // Shift grid up
            for row in 1..self.grid.rows {
                for col in 0..self.grid.cols {
                    let cell = self.grid.get(row, col).clone();
                    self.grid.set(row - 1, col, cell);
                }
            }
        }
    }

    // Resize grid (existing logic)
    self.grid = self.grid.resized(rows, cols);

    // Handle alt grid
    if let Some(ref mut alt) = self.alt_grid {
        *alt = alt.resized(rows, cols);
    }

    // Cursor clamping (existing)
    self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
    self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));

    // Clamp saved cursor (NEW)
    self.saved_cursor.0 = self.saved_cursor.0.min(rows.saturating_sub(1));
    self.saved_cursor.1 = self.saved_cursor.1.min(cols.saturating_sub(1));

    // Reset scroll region (existing)
    self.scroll_top = 0;
    self.scroll_bottom = rows - 1;
}
```

### Phase 2: Wrapped Line Tracking

**Data Structure:**
```rust
#[derive(Clone)]
pub struct Line {
    pub cells: Vec<Cell>,
    pub wrapped: bool,  // Marks continuation of previous line
}

impl Line {
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![Cell::default(); cols],
            wrapped: false,
        }
    }
}

pub struct Terminal {
    pub lines: Vec<Line>,           // Changed from Grid
    pub rows: usize,
    pub cols: usize,
    scrollback: Vec<Line>,          // Same structure
    // ...
}
```

**Set Flag During Wrap:**
```rust
fn print(&mut self, c: char) {
    let width = UnicodeWidthChar::width(c).unwrap_or(1);

    if self.cursor_col + width > self.cols {
        // Mark current line as wrapped
        self.lines[self.cursor_row].wrapped = true;

        // Move to next line
        self.cursor_col = 0;
        self.cursor_row += 1;

        if self.cursor_row >= self.rows {
            self.scroll_up();
            self.cursor_row = self.rows - 1;
        }
    }

    // Render character
    self.lines[self.cursor_row].cells[self.cursor_col] = Cell {
        character: c,
        fg: self.current_fg,
        bg: self.current_bg,
    };

    self.cursor_col += width;
}
```

### Phase 3: Horizontal Reflow (Deferred)

See Alacritty's `alacritty_terminal/src/grid/resize.rs` for complete implementation reference. Core algorithm:

**Reflow Wider:**
- Process lines in reverse (bottom to top)
- Check `wrapped` flag on line[i-1]
- Pull cells from line[i] to line[i-1] if space available
- Remove empty lines, clear wrap flags when lines merge

**Reflow Narrower:**
- Process lines forward (top to bottom)
- Split lines that exceed new width
- Set `wrapped` flag on truncated lines
- Handle wide characters: don't split, use spacer markers

---

## Code Examples

### Example 1: Minimal Scrollback Addition

```rust
use std::collections::VecDeque;

pub struct Terminal {
    pub grid: Grid,
    scrollback: VecDeque<Vec<Cell>>,  // Ring buffer for efficiency
    max_scrollback: usize,
    // ... existing fields ...
}

impl Terminal {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            grid: Grid::new(rows, cols),
            scrollback: VecDeque::with_capacity(10000),
            max_scrollback: 10000,
            // ... existing fields ...
        }
    }

    fn push_to_scrollback(&mut self) {
        // Extract top line
        let line: Vec<Cell> = (0..self.grid.cols)
            .map(|col| self.grid.get(0, col).clone())
            .collect();

        self.scrollback.push_back(line);

        // Trim oldest if needed
        if self.scrollback.len() > self.max_scrollback {
            self.scrollback.pop_front();
        }
    }
}
```

### Example 2: Vertical Resize with Scrollback Integration

```rust
pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
    let old_rows = self.grid.rows;

    // Vertical shrink: push to scrollback
    if new_rows < old_rows {
        let lines_lost = old_rows - new_rows;
        for _ in 0..lines_lost {
            self.push_to_scrollback();

            // Shift grid up
            for row in 1..self.grid.rows {
                for col in 0..self.grid.cols {
                    let cell = self.grid.get(row, col).clone();
                    self.grid.set(row - 1, col, cell);
                }
            }
        }
    }

    // Resize grid
    self.grid = self.grid.resized(new_rows, new_cols);

    // Cursor clamping
    self.cursor_row = self.cursor_row.min(new_rows.saturating_sub(1));
    self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));

    // Saved cursor clamping (important!)
    self.saved_cursor.0 = self.saved_cursor.0.min(new_rows.saturating_sub(1));
    self.saved_cursor.1 = self.saved_cursor.1.min(new_cols.saturating_sub(1));

    // Reset scroll region
    self.scroll_top = 0;
    self.scroll_bottom = new_rows - 1;
}
```

---

## Sources

### Official Documentation
- [WezTerm Scrollback Documentation](https://wezterm.org/scrollback.html)
- [WezTerm window-resized Event](https://wezterm.org/config/lua/window-events/window-resized.html)
- [VTE Terminal API Reference](https://lazka.github.io/pgi-docs/Vte-2.91/classes/Terminal.html)

### Source Code & Implementation
- [Alacritty resize.rs](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/grid/resize.rs)
- [xterm.js Reflow PR #1864](https://github.com/xtermjs/xterm.js/pull/1864)
- [xterm.js Reflow Issue #622](https://github.com/xtermjs/xterm.js/issues/622)
- [st (suckless) tresize commit](https://git.suckless.org/st/commit/9619760e129b0994cd3bc4c827c83960f6a5e98f.html)

### Design Discussions & Issues
- [WezTerm: When lines move to scrollback](https://github.com/wezterm/wezterm/discussions/3356)
- [Kitty: Show more output on resize](https://github.com/kovidgoyal/kitty/issues/1047)
- [Alacritty: Reflow on resize](https://github.com/alacritty/alacritty/issues/591)
- [Alacritty: Cursor wrapping during reflow](https://github.com/alacritty/alacritty/issues/3584)
- [Microsoft Terminal: Cursor position loss on resize](https://github.com/microsoft/terminal/issues/10848)

### Technical Background
- [VT terminal wrapping semantics test](https://github.com/mattiase/wraptest)
- [Line wrap control in terminal](https://www.baeldung.com/linux/line-wrap-control)
- [Terminal emulator scrollback buffer guide](https://dedirock.com/blog/increasing-the-scrollback-buffer-size-in-linux-terminal-emulators-a-step-by-step-guide/)

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| Source verification | ✅ | Official docs (WezTerm, VTE), maintainer statements (Kitty), source code (Alacritty, xterm.js) |
| Recency check | ✅ | All sources 2018+ or current git repositories; design decisions remain valid |
| Alternatives explored | ✅ | 7 terminals compared: WezTerm, Kitty, Alacritty, xterm, urxvt, VTE, st |
| Actionability | ✅ | Phase-by-phase plan, complete Rust code examples, clear priority ordering |
| Evidence quality | ✅ | Source code analysis, maintainer quotes, multiple corroborating sources |

**Limitations/Caveats:**
1. Did not analyze WezTerm source code directly (relied on docs + discussions)
2. Did not locate Kitty screen.py implementation (used maintainer's design philosophy statements)
3. Complexity estimates based on similar codebases, not Ferrum-specific measurements
4. Did not verify portable-pty crate handles TIOCSWINSZ (reasonable assumption)
5. Recommendations favor modern terminal behavior (Alacritty) over classic (xterm)
