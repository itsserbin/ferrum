---
title: Research - Alacritty Window Resize Handling
task_file: User request - Research Alacritty window resize implementation
scratchpad: /home/user/apps/ferrum/.specs/scratchpad/020831f8.md
created: 2026-02-15
status: complete
---

# Research: Alacritty Window Resize Handling

## Executive Summary

Alacritty handles window resize using a **Vec-based ring buffer with manual index management** for efficient scrolling and content preservation. The implementation supports **optional text reflow** (controlled by boolean parameter) with sophisticated handling of wrapped lines and wide characters. Vertical resize moves content to/from scrollback by rotating the ring buffer offset; horizontal resize can either truncate/pad (no reflow) or intelligently wrap/unwrap lines (with reflow). Cursor positions are clamped to new dimensions, and PTY notification (SIGWINCH via TIOCSWINSZ) happens synchronously after grid resize completes. The architecture evolved from simple truncation to sophisticated reflow, providing a clear learning path for terminal emulator implementation.

## Related Existing Research

- `/home/user/apps/ferrum/.specs/research/research-terminal-crate-ecosystem.md` - Terminal library alternatives including alacritty_terminal
- `/home/user/apps/ferrum/.specs/research/research-vte-013-api.md` - VTE parser used by Alacritty

---

## Documentation & References

| Resource | Description | Relevance | Link |
|----------|-------------|-----------|------|
| Alacritty Grid Resize Source | Core resize algorithm implementation | Primary source for algorithm | [resize.rs](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/grid/resize.rs) |
| Alacritty Grid Storage | Ring buffer implementation | Understanding grid structure | [storage.rs](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/grid/storage.rs) |
| GitHub Issue #591 | Reflow on resize design decision | Historical context for reflow | [Issue](https://github.com/alacritty/alacritty/issues/591) |
| GitHub Issue #2177 | TIOCSWINSZ optimization | PTY notification efficiency | [Issue](https://github.com/alacritty/alacritty/issues/2177) |
| GitHub Issue #3584 | Cursor reflow correctness | Edge cases in reflow | [Issue](https://github.com/alacritty/alacritty/issues/3584) |
| alacritty_terminal docs | Public API documentation | Integration reference | [docs.rs](https://docs.rs/alacritty_terminal/latest/alacritty_terminal/grid/index.html) |
| TIOCSWINSZ man page | PTY resize ioctl details | PTY communication spec | [man7.org](https://man7.org/linux/man-pages/man2/TIOCSWINSZ.2const.html) |

### Key Concepts

- **Ring Buffer with Zero Offset**: Vec-based storage with modular index arithmetic; avoids expensive rotation by updating single offset value
- **Reflow vs Truncate**: Two strategies for horizontal resize—reflow unwraps/wraps logical lines, truncate simply cuts/pads rows
- **WRAPLINE Flag**: Row metadata tracking whether line was artificially wrapped due to width constraints (enables reflow)
- **Cursor Clamping**: Ensuring cursor position remains within valid grid bounds after resize by clamping to `min(old_pos, new_size - 1)`
- **Scrollback Preservation**: On vertical shrink, top visible lines rotate into history; on grow, lines restore from history first
- **SIGWINCH**: POSIX signal sent to foreground process group when PTY dimensions change (via TIOCSWINSZ ioctl)

---

## 1. Vertical Resize (Height Changes)

### Shrinking Vertically (Fewer Rows)

**What happens to content:**
- Content that no longer fits moves to scrollback history
- Ring buffer rotates: `zero` offset increases, pushing top lines into history region
- No data is lost (subject to max scrollback limit)

**Cursor behavior:**
- If cursor would be below new grid bottom, content scrolls up to keep cursor visible
- Formula: `required_scrolling = max(0, cursor.row + 1 - new_height)`
- Cursor row is clamped: `cursor.row = min(cursor.row, new_height - 1)`
- Both primary cursor and saved cursor (from ESC 7) are clamped

**Algorithm (from resize.rs):**
```rust
fn shrink_lines(&mut self, target: usize) {
    // Calculate scrolling needed to keep cursor visible
    let required_scrolling = (self.cursor.point.line.0 as usize + 1)
        .saturating_sub(target);

    if required_scrolling > 0 {
        // Scroll content up so cursor stays visible
        self.scroll_up(&(Line(0)..Line(self.lines as i32)), required_scrolling);
        self.cursor.point.line = min(self.cursor.point.line, Line(target as i32 - 1));
    }

    // Rotate ring buffer (moves top lines to history)
    self.raw.rotate((self.lines - target) as isize);
    self.raw.shrink_visible_lines(target);
    self.lines = target;
}
```

**Example: 30 rows → 20 rows**
```
BEFORE:
History: [100 old lines]
Visible:
  Row 0:  "line 0"
  Row 1:  "line 1"
  ...
  Row 15: "cursor here" ← cursor at (15, 5)
  ...
  Row 29: "bottom"

AFTER:
History: [110 lines]  ← grew by 10
  ...previous history...
  Row 100: "line 0"   (was visible)
  Row 101: "line 1"   (was visible)
  ...
  Row 109: "line 9"   (was visible)
Visible:
  Row 0:  "line 10"   (was row 10)
  Row 1:  "line 11"   (was row 11)
  ...
  Row 5:  "cursor here" ← cursor at (5, 5) [row decreased by 10]
  ...
  Row 19: "bottom"    (was row 29)
```

### Growing Vertically (More Rows)

**What happens to content:**
- First, attempt to restore lines from scrollback history
- If history insufficient, create new empty lines at bottom
- Existing visible content stays in place (relative to top)

**Cursor behavior:**
- Cursor row increases by number of lines pulled from history
- Formula: `cursor.row += lines_pulled_from_history`
- This keeps cursor on same content line even as grid expands

**Algorithm (from resize.rs):**
```rust
fn grow_lines(&mut self, target: usize) {
    let lines_added = target - self.lines;
    self.raw.grow_visible_lines(target);

    let history_size = self.history_size();
    let from_history = min(history_size, lines_added);

    // If can't pull enough from history, scroll existing content up
    if from_history != lines_added {
        let delta = lines_added - from_history;
        self.scroll_up(&(Line(0)..Line(target as i32)), delta);
    }

    // Move cursor down for lines pulled from history
    self.saved_cursor.point.line += from_history;
    self.cursor.point.line += from_history;

    self.lines = target;
}
```

**Example: 20 rows → 30 rows (with 10 lines in history)**
```
BEFORE:
History: [10 lines]
  Row -10: "old line 0"
  Row -9:  "old line 1"
  ...
  Row -1:  "old line 9"
Visible:
  Row 0:  "current line 0"
  Row 5:  "cursor here" ← cursor at (5, 10)
  ...
  Row 19: "current line 19"

AFTER:
History: [0 lines]  ← all pulled back into visible
Visible:
  Row 0:  "old line 0"    (restored from history)
  Row 1:  "old line 1"    (restored from history)
  ...
  Row 9:  "old line 9"    (restored from history)
  Row 10: "current line 0"
  Row 15: "cursor here" ← cursor at (15, 10) [row increased by 10]
  ...
  Row 29: "current line 19"
```

---

## 2. Horizontal Resize (Width Changes)

Alacritty supports **two modes** for horizontal resize:
1. **Without reflow** (reflow=false): Truncate/pad (simple, traditional)
2. **With reflow** (reflow=true): Wrap/unwrap logical lines (complex, modern)

### Mode 1: Without Reflow (reflow=false)

**Shrinking width:**
- Each row is independently resized to new column count
- Content beyond new width is **permanently lost** (truncated)
- No line wrapping occurs

**Growing width:**
- Each row is padded with empty/default cells to new width
- Lost content does NOT return (truncation is permanent)

**Cursor behavior:**
- Cursor column is clamped: `cursor.col = min(cursor.col, new_width - 1)`

**Algorithm:**
```rust
// Simple Vec::resize on each row
for row in self.raw.iter_mut() {
    row.resize(columns, T::default());  // Truncate or pad
}
self.columns = columns;
```

**Example: 80 cols → 60 cols**
```
BEFORE:
Row 5: "This is a very long line that extends past 60 chars and will be cut[XX]"
        ├─────────────────────────────────────────────────────────┤ ← 60 chars
Cursor at: (5, 65)

AFTER:
Row 5: "This is a very long line that extends past 60 chars and "
        ├────────────────────────────────────────────────────────┤
Cursor at: (5, 59) ← clamped from 65 to 59 (max valid column)

Content "[will be cut[XX]" is LOST permanently.
```

### Mode 2: With Reflow (reflow=true)

**Shrinking width (shrink_columns):**
- Long lines are split into multiple rows
- Original row sets WRAPLINE flag indicating artificial wrap
- Wide characters (CJK, emoji) are handled at boundaries with spacer cells
- Subsequent content shifts down to accommodate new wrapped rows

**Growing width (grow_columns):**
- Rows with WRAPLINE flag are candidates for unwrapping
- Content from next row is pulled back to restore logical line
- WRAPLINE flag is removed after successful unwrap
- Subsequent content shifts up as wrapped rows merge

**Cursor behavior:**
- Cursor position follows content through reflow
- Complex edge cases with saved cursor (ESC 7) - known issues
- Wide character boundaries require special handling

**Algorithm (simplified from resize.rs):**
```rust
fn shrink_columns(&mut self, reflow: bool, columns: usize) {
    if !reflow {
        // Simple truncation (mode 1)
        return;
    }

    for row in rows.drain(..).enumerate().rev() {
        loop {
            // Split row at column boundary
            let wrapped = match row.shrink(columns) {
                Some(wrapped) => wrapped,
                None => {
                    new_raw.push(row);
                    break;
                }
            };

            // Handle wide character at boundary
            if row[Column(columns - 1)].flags().contains(Flags::WIDE_CHAR) {
                // Insert spacer for wide character continuation
                wrapped.insert(0, wide_char_spacer);
            }

            // Mark this row as artificially wrapped
            row.last_mut().unwrap().flags_mut().insert(Flags::WRAPLINE);

            // Continue wrapping if wrapped content still too long
            row = Row::from_vec(wrapped, ...);
        }
    }
}

fn grow_columns(&mut self, reflow: bool, columns: usize) {
    if !reflow {
        // Simple padding (mode 1)
        return;
    }

    for row in rows {
        // Check if this row was artificially wrapped
        let should_unwrap = row.len() < columns
            && row.last().flags().contains(Flags::WRAPLINE);

        if should_unwrap {
            // Remove WRAPLINE flag
            row.last_mut().unwrap().flags_mut().remove(Flags::WRAPLINE);

            // Pull content from next row back to this row
            let num_to_pull = columns - row.len();
            row.append(&mut next_row.drain(0..num_to_pull));
        }
    }
}
```

**Example: 80 cols → 60 cols (with reflow)**
```
BEFORE:
Row 5: "This is a very long line that extends well beyond 60 characters total"
       ├────────────────────────────────────────────────────────┤ ← 60 chars
Cursor at: (5, 65)

AFTER:
Row 5: "This is a very long line that extends well beyond 60 c$" [WRAPLINE flag set]
Row 6: "haracters total"                                        [new row from wrap]
       ^
       └─ Content continues from previous row
Cursor at: (6, 5) ← followed content through wrap (65 - 60 = 5 on next line)
```

**Comparison: Reflow vs No Reflow**

| Aspect | Without Reflow | With Reflow |
|--------|----------------|-------------|
| **Complexity** | Simple (O(rows)) | Complex (O(rows × cols)) |
| **Performance** | Fast | Slower (processes all content) |
| **Content preservation** | Lost on shrink | Preserved via wrapping |
| **Compatibility** | Traditional terminals | Modern terminals |
| **Wide characters** | Simple truncation | Boundary handling needed |
| **Cursor tracking** | Simple clamping | Complex position calculation |
| **Line flags** | Not needed | WRAPLINE flag required |

---

## 3. Grid/Buffer Resize Internals

### Storage Structure

Alacritty uses a **Vec-based ring buffer with manual index management** (not VecDeque):

```rust
pub struct Storage<T> {
    inner: Vec<Row<T>>,      // All rows (history + visible)
    zero: usize,             // Offset to "line 0" (top of visible area)
    visible_lines: usize,    // Number of currently visible lines
    len: usize,              // Total number of active lines
}
```

**Why Vec instead of VecDeque?**
- Avoids VecDeque's internal complexity
- Custom ring buffer allows fine-grained control
- Performance: No expensive `rotate_left()` operations
- Simple modular arithmetic for index computation

### Ring Buffer Operations

**Index computation:**
```rust
fn compute_index(&self, line: usize) -> usize {
    let wrapped = self.zero + line;
    if wrapped < self.inner.len() {
        wrapped
    } else {
        wrapped - self.inner.len()
    }
}
```

**Rotation (moving content to/from scrollback):**
```rust
pub fn rotate(&mut self, count: isize) {
    let len = self.inner.len();
    self.zero = (self.zero as isize + count + len as isize) as usize % len;
}

pub fn rotate_down(&mut self, count: usize) {
    self.zero = (self.zero + count) % self.inner.len();
}
```
- **O(1) operation**: Only updates `zero` offset, no data movement
- Positive rotation: Moves top lines into history (shrink scenario)
- Negative rotation: Restores lines from history (grow scenario)

**Growing visible area:**
```rust
pub fn grow_visible_lines(&mut self, target: usize) {
    // Calculate how many lines needed
    let needed = target - self.visible_lines;

    // Ensure inner Vec has capacity (allocate if needed)
    self.initialize(target, self.columns);

    self.visible_lines = target;
}
```
- Allocates in chunks to avoid frequent reallocation
- Doesn't immediately populate with data (lazy)

**Shrinking visible area:**
```rust
pub fn shrink_visible_lines(&mut self, target: usize) {
    self.visible_lines = target;
    self.shrink_lines();

    // Only deallocate if excess capacity > MAX_CACHE_SIZE (1000 lines)
    let excess = self.inner.len() - self.len;
    if excess > MAX_CACHE_SIZE {
        self.inner.truncate(self.len);
    }
}
```
- Opportunistic deallocation: Keeps 1000 lines cached
- Avoids thrashing when resize toggles repeatedly
- Performance optimization for drag-resize scenarios

### Content Migration Strategy

**When shrinking:**
1. Calculate delta: `delta = old_lines - new_lines`
2. Rotate ring buffer: `storage.rotate(delta)` → O(1)
3. Update visible_lines: `storage.shrink_visible_lines(new_lines)`
4. Content now in "history" region (before zero offset)

**When growing:**
1. Calculate delta: `delta = new_lines - old_lines`
2. Grow storage: `storage.grow_visible_lines(new_lines)` → may allocate
3. Calculate history available: `from_history = min(history_size, delta)`
4. Rotate backward if history exists: `storage.rotate(-from_history)` → O(1)
5. Create empty rows for remainder: `delta - from_history` new rows

**Key insight**: No explicit copying between grids; ring buffer rotation reinterprets which region is "visible" vs "history".

### Alternate Screen Buffer

Alacritty maintains **two separate grids**:
- **Main grid**: Has scrollback history (unlimited by default)
- **Alt grid**: No scrollback (max_scroll_limit = 0)

**Resize behavior:**
```rust
pub fn resize(&mut self, size: TermSize) {
    // Resize BOTH grids
    self.grid.resize(false, size.screen_lines, size.cols);
    self.alt_grid.resize(false, size.screen_lines, size.cols);

    // Clamp both cursor positions
    self.cursor.point = self.cursor.point.grid_clamp(&self.grid);
    self.alt_cursor.point = self.alt_cursor.point.grid_clamp(&self.alt_grid);
}
```

**Implications for alt screen:**
- Fullscreen apps (vim, less, htop) use alt screen
- On vertical shrink: Content that doesn't fit is **lost** (no scrollback)
- Apps typically redraw completely after SIGWINCH anyway
- Horizontal resize: Same truncate/pad or reflow behavior

---

## 4. PTY SIGWINCH Interaction

### Complete Flow (Window Event → Child Process)

```
1. Window System
   └─> WindowEvent::Resized(pixel_width, pixel_height)
         │
         v
2. Alacritty Event Handler
   ├─> Calculate grid dimensions:
   │     rows = pixel_height / font_height
   │     cols = pixel_width / font_width
   │
   └─> Term::resize(TermSize { rows, cols })
         │
         v
3. Grid Resize (SYNCHRONOUS)
   ├─> self.grid.resize(reflow, rows, cols)
   ├─> self.alt_grid.resize(reflow, rows, cols)
   ├─> Clamp cursor positions
   │
   └─> All grid operations complete
         │
         v
4. PTY Notification
   └─> self.pty.on_resize(WindowSize {
         num_rows: rows,
         num_cols: cols,
         cell_width: 0,   // Unused
         cell_height: 0,  // Unused
       })
         │
         v
5. ioctl(master_fd, TIOCSWINSZ, &winsize)
   └─> Kernel updates PTY dimensions
         │
         v
6. Kernel Signal Delivery
   └─> SIGWINCH sent to foreground process group
         │
         v
7. Child Process Signal Handler
   ├─> ioctl(slave_fd, TIOCGWINSZ, &winsize)  // Query new size
   ├─> Update internal state (rows, cols)
   └─> Redraw interface (e.g., shell prompt, vim buffer)
```

### Critical Ordering: Grid BEFORE PTY

**Why this order matters:**
```rust
// CORRECT: Grid resize synchronous, then PTY notification
self.grid.resize(...);              // Grid now has new dimensions
self.pty.on_resize(...);            // PTY notified of new size
                                     // Child queries size → gets new dimensions ✓

// WRONG: Would create race condition
self.pty.on_resize(...);            // Child receives SIGWINCH
                                     // Child queries size → might get OLD dimensions ✗
self.grid.resize(...);              // Grid updated too late
```

### TIOCSWINSZ Optimization Issue

From [GitHub Issue #2177](https://github.com/alacritty/alacritty/issues/2177):

**Problem:**
- Alacritty calls TIOCSWINSZ for **every pixel change** during resize drag
- Even when grid dimensions (rows, cols) haven't changed (only padding changed)
- Causes unnecessary SIGWINCH signals → child process redraws unnecessarily
- Particularly slow over SSH connections

**Solution:**
- Only call TIOCSWINSZ when **grid size actually changes**
- Track previous grid size: `last_rows`, `last_cols`
- Skip ioctl if `new_rows == last_rows && new_cols == last_cols`

**Impact on learning project:**
- **Initial implementation**: Call TIOCSWINSZ on every resize (simple)
- **Optimization later**: Add dirty tracking to skip redundant calls

### WindowSize Structure

```rust
pub struct WindowSize {
    pub num_rows: u16,     // Number of character rows
    pub num_cols: u16,     // Number of character columns
    pub cell_width: u16,   // Pixel width of cell (often 0, unused)
    pub cell_height: u16,  // Pixel height of cell (often 0, unused)
}
```

Corresponds to Unix `struct winsize`:
```c
struct winsize {
    unsigned short ws_row;      /* rows, in characters */
    unsigned short ws_col;      /* columns, in characters */
    unsigned short ws_xpixel;   /* horizontal size, pixels (unused) */
    unsigned short ws_ypixel;   /* vertical size, pixels (unused) */
};
```

**Note**: `ws_xpixel` and `ws_ypixel` are rarely used by applications; most only care about character dimensions.

---

## 5. Cursor Position Preservation

### Primary Cursor

**Clamping strategy:**
```rust
// After grid resize
self.cursor.point.line = min(self.cursor.point.line, Line(new_lines - 1));
self.cursor.point.col = min(self.cursor.point.col, Column(new_cols - 1));
```

**Special cases:**

1. **Vertical shrink with cursor near bottom:**
   - Content scrolls up to keep cursor visible
   - Cursor row decreases but stays on same content line

2. **Horizontal shrink with cursor past new width:**
   - Cursor snaps to rightmost column: `col = new_cols - 1`
   - Without reflow: Cursor position within line is lost
   - With reflow: Cursor follows content to wrapped line

3. **Growing from history:**
   - Cursor row increases by number of lines restored from history
   - Keeps cursor on same content even as absolute position changes

### Saved Cursor (ESC 7 / ESC 8)

Terminal escape sequences allow saving/restoring cursor:
- `ESC 7` (DECSC): Save cursor position
- `ESC 8` (DECRC): Restore cursor position

**Resize behavior:**
```rust
// Both primary and saved cursor are clamped
self.cursor.point = self.cursor.point.grid_clamp(&self.grid);
self.saved_cursor.point = self.saved_cursor.point.grid_clamp(&self.grid);
```

**Known issues (from GitHub):**

From [Issue #3584](https://github.com/alacritty/alacritty/issues/3584):
> "Reflow not wrapping cursor correctly"

From [Ghostty Issue #5718](https://github.com/ghostty-org/ghostty/issues/5718):
> "Terminal resize with reflow doesn't reflow the saved cursor"

**Comparison across terminals:**
| Terminal | Reflows Primary Cursor | Reflows Saved Cursor |
|----------|------------------------|----------------------|
| Alacritty | ✓ (with bugs) | ✗ (clamped only) |
| Kitty | ✓ (off by one bug) | ? |
| WezTerm | ? | ✗ |
| iTerm2 | ✓ | ✗ |
| xterm | ✗ (no reflow) | ✗ |

**Lesson for learning project**: Saved cursor during reflow is an unsolved problem across terminals—acceptable to clamp only for initial implementation.

---

## Comparison: Terminal Emulators Resize Strategies

| Terminal | Vertical Resize | Horizontal Resize | Reflow Support | Cursor Handling | Notes |
|----------|-----------------|-------------------|----------------|-----------------|-------|
| **Alacritty** | Scrollback preservation | Optional reflow | ✓ (configurable) | Clamps, follows with reflow | Modern approach, evolved from no-reflow |
| **xterm** | Traditional scrollback | Truncate/pad only | ✗ | Simple clamp | Traditional, no reflow |
| **Kitty** | Scrollback preservation | Partial reflow | ⚠️ (buggy) | Reflows but off-by-one | Active development |
| **WezTerm** | Scrollback preservation | Optional reflow | ✓ (off by default) | Clamps saved cursor | Configurable behavior |
| **iTerm2** | Scrollback preservation | Reflow by default | ✓ | Doesn't reflow saved cursor | macOS only |
| **Ghostty** | Scrollback preservation | Reflow with issues | ⚠️ (in progress) | Saved cursor issues | New terminal (2025+) |

**Consensus findings:**
- **Vertical resize**: All modern terminals preserve scrollback
- **Horizontal reflow**: No universal standard; each terminal has quirks
- **Saved cursor**: Problematic across all terminals that attempt reflow
- **Wide characters**: Complex boundary cases in all implementations
- **Performance**: Reflow is expensive; requires optimization for large grids

---

## Implementation Recommendations for Ferrum

### Phase 1: Basic Resize (No Reflow) - RECOMMENDED START

**Simplest approach matching traditional terminals:**

```rust
// Grid structure
struct Grid {
    rows: VecDeque<Row>,  // Simple ring buffer (stdlib)
    cursor: Point,
    saved_cursor: Point,
    max_scrollback: usize,
}

// Vertical shrink
fn shrink_vertical(&mut self, new_height: usize) {
    let delta = self.visible_lines - new_height;

    // Move top lines to scrollback (VecDeque makes this simple)
    for _ in 0..delta {
        let line = self.rows.remove(index_of_first_visible);
        self.scrollback.push_back(line);

        // Limit scrollback size
        if self.scrollback.len() > self.max_scrollback {
            self.scrollback.pop_front();
        }
    }

    // Clamp cursor
    self.cursor.row = min(self.cursor.row, new_height - 1);
}

// Vertical grow
fn grow_vertical(&mut self, new_height: usize) {
    let delta = new_height - self.visible_lines;

    // Restore from scrollback first
    let from_scrollback = min(delta, self.scrollback.len());
    for _ in 0..from_scrollback {
        let line = self.scrollback.pop_back().unwrap();
        self.rows.insert(index_of_first_visible, line);
    }

    // Create empty lines for remainder
    let remaining = delta - from_scrollback;
    for _ in 0..remaining {
        self.rows.push_back(Row::new(self.cols));
    }
}

// Horizontal resize (simple)
fn resize_horizontal(&mut self, new_width: usize) {
    for row in &mut self.rows {
        row.resize(new_width, Cell::default());  // Truncate or pad
    }

    // Clamp cursor
    self.cursor.col = min(self.cursor.col, new_width - 1);
}

// PTY notification
fn notify_pty(&mut self, rows: u16, cols: u16) {
    // Only if dimensions actually changed
    if self.last_rows != rows || self.last_cols != cols {
        self.pty.resize(WindowSize { rows, cols, xpixel: 0, ypixel: 0 });
        self.last_rows = rows;
        self.last_cols = cols;
    }
}
```

**Advantages:**
- Simple to implement and understand
- Fast performance (no complex wrapping logic)
- Matches traditional terminals (xterm)
- Predictable behavior
- Good learning foundation

**Limitations:**
- Horizontal shrink loses content permanently
- No logical line preservation across resize
- Less sophisticated than modern terminals

### Phase 2: Add Reflow (Advanced) - DEFER INITIALLY

**Only after Phase 1 is working:**

1. **Add WRAPLINE flag to Cell:**
```rust
struct Cell {
    character: char,
    flags: CellFlags,  // Includes WRAPLINE bit
    // ...
}
```

2. **Track logical lines:**
- Mark rows that were artificially wrapped
- On grow: Unwrap adjacent rows with WRAPLINE flag
- On shrink: Wrap long rows, set WRAPLINE flag

3. **Handle wide characters:**
- Detect double-width characters (CJK, emoji) using unicode-width
- Insert spacer cells when wide char split across line boundary
- Remove spacers when unwrapping

4. **Cursor tracking:**
- Calculate cursor position through wraps/unwraps
- Complex: Need to scan through wrapped rows
- Defer saved cursor reflow (known hard problem)

**Complexity comparison:**
- Phase 1: ~200 lines of code, 1-2 days implementation
- Phase 2: ~800 lines of code, 1-2 weeks implementation + debugging

### Migration Path: VecDeque → Vec+Zero

**After Phase 1 is working, optionally optimize:**

```rust
struct Storage {
    inner: Vec<Row>,
    zero: usize,  // Index of "line 0"
    visible_lines: usize,
}

impl Storage {
    fn logical_to_physical(&self, line: usize) -> usize {
        (self.zero + line) % self.inner.len()
    }

    fn rotate(&mut self, delta: isize) {
        let len = self.inner.len();
        self.zero = (self.zero as isize + delta + len as isize) as usize % len;
    }
}
```

**Performance gain:**
- VecDeque: Slower indexing, two memory regions
- Vec+zero: Fast indexing, contiguous memory
- **Only worth it if profiling shows VecDeque is bottleneck**

---

## Code Examples

### Example 1: Basic Vertical Resize (VecDeque approach)

```rust
use std::collections::VecDeque;

struct Terminal {
    grid: VecDeque<Row>,
    scrollback: VecDeque<Row>,
    cursor: Point,
    cols: usize,
    max_scrollback: usize,
}

impl Terminal {
    fn resize_vertical(&mut self, new_lines: usize) {
        let old_lines = self.grid.len();

        match new_lines.cmp(&old_lines) {
            std::cmp::Ordering::Less => {
                // Shrinking: move top lines to scrollback
                let delta = old_lines - new_lines;
                for _ in 0..delta {
                    if let Some(line) = self.grid.pop_front() {
                        self.scrollback.push_back(line);

                        // Enforce max scrollback
                        if self.scrollback.len() > self.max_scrollback {
                            self.scrollback.pop_front();
                        }
                    }
                }

                // Clamp cursor
                self.cursor.row = self.cursor.row.min(new_lines - 1);
            }

            std::cmp::Ordering::Greater => {
                // Growing: restore from scrollback or create empty
                let delta = new_lines - old_lines;
                let from_scrollback = delta.min(self.scrollback.len());

                // Restore from scrollback
                for _ in 0..from_scrollback {
                    if let Some(line) = self.scrollback.pop_back() {
                        self.grid.push_front(line);
                    }
                }

                // Create empty lines
                for _ in 0..(delta - from_scrollback) {
                    self.grid.push_back(Row::new(self.cols));
                }

                // Adjust cursor position
                self.cursor.row += from_scrollback;
            }

            std::cmp::Ordering::Equal => {}
        }
    }
}
```

### Example 2: Basic Horizontal Resize (No Reflow)

```rust
impl Terminal {
    fn resize_horizontal(&mut self, new_cols: usize) {
        // Resize every row in grid
        for row in &mut self.grid {
            row.cells.resize(new_cols, Cell::default());
        }

        // Resize every row in scrollback
        for row in &mut self.scrollback {
            row.cells.resize(new_cols, Cell::default());
        }

        // Update column count
        self.cols = new_cols;

        // Clamp cursor
        self.cursor.col = self.cursor.col.min(new_cols - 1);
    }
}
```

### Example 3: PTY Resize Notification

```rust
use portable_pty::{PtySize, MasterPty};

impl Terminal {
    fn handle_window_resize(&mut self, pixel_width: u32, pixel_height: u32) {
        // Calculate new grid dimensions
        let new_cols = (pixel_width / self.font_width) as usize;
        let new_rows = (pixel_height / self.font_height) as usize;

        // Check if grid size actually changed
        let size_changed = new_rows != self.rows || new_cols != self.cols;

        if size_changed {
            // Resize grid FIRST
            self.resize_vertical(new_rows);
            self.resize_horizontal(new_cols);

            // Update stored dimensions
            self.rows = new_rows;
            self.cols = new_cols;

            // THEN notify PTY
            let pty_size = PtySize {
                rows: new_rows as u16,
                cols: new_cols as u16,
                pixel_width: pixel_width as u16,
                pixel_height: pixel_height as u16,
            };

            if let Err(e) = self.pty.resize(pty_size) {
                eprintln!("Failed to resize PTY: {}", e);
            }

            // Mark for redraw
            self.dirty = true;
        }
    }
}
```

### Example 4: Alacritty-Style Ring Buffer (Advanced)

```rust
struct Storage {
    inner: Vec<Row>,
    zero: usize,           // Index of "line 0"
    visible_lines: usize,
}

impl Storage {
    fn new(lines: usize, cols: usize) -> Self {
        Self {
            inner: vec![Row::new(cols); lines],
            zero: 0,
            visible_lines: lines,
        }
    }

    fn compute_index(&self, line: usize) -> usize {
        let wrapped = self.zero + line;
        if wrapped < self.inner.len() {
            wrapped
        } else {
            wrapped - self.inner.len()
        }
    }

    fn get_mut(&mut self, line: usize) -> &mut Row {
        let idx = self.compute_index(line);
        &mut self.inner[idx]
    }

    fn rotate(&mut self, delta: isize) {
        let len = self.inner.len() as isize;
        self.zero = ((self.zero as isize + delta % len + len) % len) as usize;
    }

    fn shrink_visible(&mut self, new_visible: usize) {
        let delta = self.visible_lines - new_visible;
        self.rotate(delta as isize);  // Move top lines to "history"
        self.visible_lines = new_visible;
    }

    fn grow_visible(&mut self, new_visible: usize) {
        let delta = new_visible - self.visible_lines;
        let history_size = self.inner.len() - self.visible_lines;
        let from_history = delta.min(history_size);

        if from_history > 0 {
            self.rotate(-(from_history as isize));  // Restore from history
        }

        // If not enough history, need to allocate new rows
        let new_rows_needed = delta - from_history;
        for _ in 0..new_rows_needed {
            self.inner.push(Row::new(self.inner[0].len()));
        }

        self.visible_lines = new_visible;
    }
}
```

---

## Sources

### Primary Sources (Alacritty)
- [alacritty_terminal/src/grid/resize.rs](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/grid/resize.rs) - Core resize implementation
- [alacritty_terminal/src/grid/storage.rs](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/grid/storage.rs) - Ring buffer storage
- [alacritty_terminal/src/grid/row.rs](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/grid/row.rs) - Row operations
- [alacritty_terminal docs - Grid](https://docs.rs/alacritty_terminal/latest/alacritty_terminal/grid/index.html)

### GitHub Issues & Pull Requests
- [Issue #591: Reflow on resize](https://github.com/alacritty/alacritty/issues/591) - Design decision
- [PR #1147: Scrollback implementation](https://github.com/alacritty/alacritty/pull/1147) - Ring buffer adoption
- [PR #657: Implementation of scrollback](https://github.com/alacritty/alacritty/pull/657) - Early implementation
- [PR #1584: Dynamically initialize grid storage](https://github.com/alacritty/alacritty/pull/1584/files) - Performance optimization
- [Issue #2177: Resizing calls TIOCSWINSZ for every pixel](https://github.com/alacritty/alacritty/issues/2177) - PTY optimization
- [Issue #3584: Reflow not wrapping cursor correctly](https://github.com/alacritty/alacritty/issues/3584) - Cursor bugs
- [Issue #4419: Resize / Reflow Issues](https://github.com/alacritty/alacritty/issues/4419) - General resize problems
- [Issue #2567: Text reflow slow with large grids](https://github.com/alacritty/alacritty/issues/2567) - Performance

### Other Terminal Emulators
- [Ghostty Issue #5718: Terminal resize with reflow doesn't reflow saved cursor](https://github.com/ghostty-org/ghostty/issues/5718)
- [Terminal Emulator Comparison: iTerm2 vs WezTerm vs Alacritty](https://medium.com/@dynamicy/choosing-a-terminal-on-macos-2025-iterm2-vs-ghostty-vs-wezterm-vs-kitty-vs-alacritty-d6a5e42fd8b3)
- [From iTerm to WezTerm](https://medium.com/@vladkens/from-iterm-to-wezterm-24db2ccb8dc1)

### PTY Documentation
- [TIOCSWINSZ Linux Manual](https://man7.org/linux/man-pages/man2/TIOCSWINSZ.2const.html)
- [tty_ioctl(4) Manual](https://linux.die.net/man/4/tty_ioctl)
- [Playing with SIGWINCH](https://www.rkoucha.fr/tech_corner/sigwinch.html)

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| Source verification | ✅ | Direct analysis of Alacritty GitHub source code (resize.rs, storage.rs) |
| Recency check | ✅ | Source code from master branch (current as of Feb 2026), recent issues reviewed |
| Alternatives explored | ✅ | Compared 6 terminal emulators (Alacritty, xterm, Kitty, WezTerm, iTerm2, Ghostty) |
| Actionability | ✅ | Complete code examples, phased implementation plan, exact algorithms documented |
| Evidence quality | ✅ | Primary sources (actual source code), GitHub issues with maintainer input, man pages |

**Limitations/Caveats:**
- Reflow behavior has known bugs across ALL terminals (not Alacritty-specific)
- Saved cursor during reflow is unsolved problem industry-wide
- Alacritty's implementation continues to evolve; edge cases may change
- Wide character handling at line boundaries is complex and may have corner cases
- Performance characteristics depend on grid size and content
- Some optimizations (Vec vs VecDeque, resize throttling) only matter at scale

**Key Finding Correction:**
- Initial research assumption: Alacritty has NO reflow
- Verified reality: Alacritty DOES support optional reflow (added via PR #2120)
- Lesson: Always verify assumptions against current source code, not historical discussions
