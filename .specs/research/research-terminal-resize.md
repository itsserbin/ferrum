---
title: Research - Terminal Resize Behavior in VT100/xterm Emulation
task_file: User request (ad-hoc research)
scratchpad: /home/user/apps/ferrum/.specs/scratchpad/7375bace.md
created: 2026-02-15
status: complete
---

# Research: Terminal Resize Behavior in VT100/xterm Emulation

## Executive Summary

Terminal resize behavior is NOT strictly defined by VT100 or POSIX beyond the SIGWINCH notification mechanism. The reference implementation (xterm) does NOT reflow text on resize - it simply truncates/pads lines and notifies the shell via SIGWINCH. Modern terminals vary: some reflow (Alacritty, Kitty), others don't (st, xterm). The simplest robust implementation: (1) resize grid buffer, (2) move lines to/from scrollback when height changes, (3) clamp cursor to keep visible, (4) call TIOCSWINSZ ioctl. The kernel then sends SIGWINCH to the shell, which queries the new size and redraws. Text reflow is OPTIONAL and adds significant complexity (~500+ lines vs ~100 lines).

## Related Existing Research

- [research-portable-pty-v0.8.md](research-portable-pty-v0.8.md) - PTY library API and usage
- [research-vte-013-api.md](research-vte-013-api.md) - VT escape sequence parser

---

## Documentation & References

| Resource | Description | Relevance | Link |
|----------|-------------|-----------|------|
| xterm FAQ | Official xterm documentation | Reference implementation behavior | [Link](https://invisible-island.net/xterm/xterm.faq.html) |
| TIOCSWINSZ man page | Linux manual for window size ioctl | PTY resize mechanism | [Link](https://man7.org/linux/man-pages/man2/TIOCSWINSZ.2const.html) |
| GNU C Library - Signals | SIGWINCH signal specification | Signal semantics | [Link](https://www.gnu.org/software/libc/manual/html_node/Miscellaneous-Signals.html) |
| Readline Signal Handling | How readline handles SIGWINCH | Shell redraw behavior | [Link](https://docs.rtems.org/releases/4.5.1-pre3/toolsdoc/gdb-5.0-docs/readline/readline00030.html) |
| Alacritty resize.rs | Full reflow implementation | Complex algorithm reference | [Link](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/grid/resize.rs) |
| VT100 Technical Manual | Original hardware terminal specs | Historical context | [Link](https://vt100.net/docs/vt100-tm/) |

### Key Concepts

- **SIGWINCH**: Window change signal sent by kernel to foreground process group when terminal size changes (via TIOCSWINSZ ioctl). Default action is to ignore; applications must handle it.
- **TIOCSWINSZ/TIOCGWINSZ**: ioctl operations to set/get terminal window size using `struct winsize { ws_row, ws_col, ws_xpixel, ws_ypixel }`.
- **Text Reflow**: Optional feature where lines are re-wrapped when terminal width changes. xterm does NOT reflow; modern terminals vary.
- **Scrollback Buffer**: Ring buffer of off-screen lines. Lines move here when terminal shrinks vertically or when cursor scrolls past bottom.
- **Cursor Clamping**: Ensuring cursor position stays within valid grid bounds (0 <= x < cols, 0 <= y < rows) after resize.
- **NorthWest vs SouthWest**: xterm resize modes. SouthWest (default) keeps bottom line fixed; NorthWest keeps top line fixed.

---

## POSIX/xterm Specification for Resize

### What the Specifications Say

**VT100**: Does NOT specify resize behavior (hardware terminal with fixed size: 80x24 or 132x24).

**POSIX**: Originally omitted SIGWINCH as windowing-specific. POSIX 2024 added `tcgetwinsize()` and `tcsetwinsize()` to `termios.h`, but does NOT specify terminal display behavior during resize.

**xterm (Reference Implementation)**:
- Uses SIGWINCH signal on BSD-derived systems
- Two modes: **SouthWest (default)** - bottom line stays fixed, scrollback pulled when growing; **NorthWest** - top line stays fixed, lines dropped from bottom when shrinking
- Does NOT reflow text on horizontal resize
- Implements dtterm window-manipulation control sequences (CSI t)

### SIGWINCH Mechanism

1. Terminal emulator detects window resize event (e.g., from window system)
2. Terminal emulator resizes internal grid buffer
3. Terminal emulator calls `ioctl(pty_fd, TIOCSWINSZ, &new_winsize)`
4. **Kernel** sends SIGWINCH signal to foreground process group
5. Application (e.g., bash) receives SIGWINCH
6. Application calls `ioctl(stdin_fd, TIOCGWINSZ, &winsize)` to query new size
7. Application redraws screen with new dimensions

**Critical Detail**: SIGWINCH does NOT interrupt system calls like `read()`. In bash 5+, SIGWINCH handler uses `SA_RESTART`, so `read()` continues. Readline only handles SIGWINCH when `read()` returns (i.e., when user presses a key). Result: Terminal won't redraw until keypress.

**Implication for Terminal Emulator**: Don't expect immediate redraw. Shell redraws on its own schedule after receiving SIGWINCH.

### struct winsize Details

```c
struct winsize {
    unsigned short ws_row;      // rows, in characters
    unsigned short ws_col;      // columns, in characters
    unsigned short ws_xpixel;   // horizontal size, pixels (usually 0/unused)
    unsigned short ws_ypixel;   // vertical size, pixels (usually 0/unused)
};
```

- `ws_xpixel` and `ws_ypixel` are typically unused (set to 0)
- Only `ws_row` and `ws_col` matter for most applications
- On success, `ioctl()` returns 0; on error, returns -1

---

## Buffer Adjustment Strategy

### Vertical Resize (Height Change)

**Growing (new_rows > old_rows)**:
- Pull lines from scrollback buffer (if available) onto top of visible grid
- Adjust cursor.y downward by number of lines pulled
- If scrollback exhausted, add blank lines at bottom
- Effect: Command history becomes visible again

**Shrinking (new_rows < old_rows)**:
- Check if cursor would be off-screen (cursor.y >= new_rows)
- If yes: scroll grid upward, push top lines to scrollback, clamp cursor.y to new_rows - 1
- If no: push excess bottom lines to scrollback
- Effect: Cursor stays visible, content preserved in scrollback

### Horizontal Resize (Width Change)

**Without Reflow (Simple)**:
- Growing: Pad each row with blank cells to new width
- Shrinking: Truncate each row to new width
- Clamp cursor.x to new_cols - 1
- Effect: Simple, predictable, content may be lost when shrinking

**With Reflow (Complex)**:
- Track which lines have soft wrap (WRAPLINE flag)
- Growing: Pull wrapped content from next line back to current line
- Shrinking: Wrap overflowing content to next line
- Recalculate cursor position during reflow
- Effect: Content preserved, but complex algorithm and can break columnar output

### Cursor Position

**Simplest Approach** (xterm-style):
```
cursor.x = min(cursor.x, new_cols - 1)
cursor.y = min(cursor.y, new_rows - 1)
```

**If cursor.y >= new_rows** (would be off-screen):
```
scroll_amount = cursor.y - new_rows + 1
push top scroll_amount lines to scrollback
scroll grid upward
cursor.y = new_rows - 1
```

**Rationale**: Shell will redraw after SIGWINCH anyway, so exact logical position doesn't matter. Just keep cursor in viewport.

---

## Is Text Reflow Required?

### Short Answer: NO

Text reflow is an **optional enhancement**, not a requirement for a correct terminal emulator.

### Evidence

| Terminal | Reflow? | Status |
|----------|---------|--------|
| xterm | No | Reference implementation, still maintained |
| st (suckless) | No (patch available) | Simple, correct terminal |
| rxvt | No | Classic terminal |
| urxvt | Yes (with limitations) | Reflow only after scrollback accumulates |
| iTerm2 | Partial (buggy) | Doesn't reflow wider, leaves line breaks |
| Alacritty | Yes | Full reflow, complex algorithm |
| Kitty | Yes | Smart reflow, prompt detection |
| WezTerm | Yes | Full reflow with event coalescing |
| Ghostty | Yes | Modern reflow |

**Conclusion**: Reference implementation (xterm) and simple terminals (st, rxvt) do NOT reflow. Reflow is a modern enhancement, not a specification requirement.

### User Experience Comparison

**Without Reflow**:
- **Pros**: Simple, predictable, better for columnar output (tables, htop), less CPU
- **Cons**: Long lines truncated when shrinking horizontally, widening doesn't recover text

**With Reflow**:
- **Pros**: Content preserved, better for long command lines, modern expectation
- **Cons**: Complex algorithm, can break formatted output, more CPU, cursor tracking tricky

### Recommendation for Learning Project

**Start without reflow**:
- Simpler implementation (~100 lines vs ~500+ lines)
- Fewer edge cases and bugs
- Follows xterm reference behavior
- Good enough for most use cases

**Add reflow later** (if desired):
- Use Alacritty's `resize.rs` as reference
- Expect significant complexity increase
- Test extensively with vim, htop, long prompts

---

## Simplest Correct Implementation

### Algorithm Overview

1. **Receive window resize event** (from winit)
2. **Calculate new grid dimensions** (window_size / cell_size)
3. **Resize grid buffer**:
   - Height: move lines to/from scrollback, keep cursor visible
   - Width: truncate or pad rows, clamp cursor
4. **Notify PTY** via TIOCSWINSZ ioctl
5. **Kernel sends SIGWINCH** to shell
6. **Shell queries size and redraws**

### Pseudo-code

```
fn resize(new_rows: usize, new_cols: usize) {
    let old_rows = grid.rows;

    // VERTICAL RESIZE
    if new_rows < old_rows {
        // Shrinking: keep cursor visible
        if cursor.y >= new_rows {
            let scroll = cursor.y - new_rows + 1;
            for _ in 0..scroll {
                scrollback.push(grid.remove_line(0));
            }
            cursor.y = new_rows - 1;
        }
        grid.truncate_rows(new_rows);
    }
    else if new_rows > old_rows {
        // Growing: pull from scrollback
        let needed = new_rows - old_rows;
        let pulled = min(needed, scrollback.len());
        for _ in 0..pulled {
            grid.insert_line(0, scrollback.pop());
            cursor.y += 1;
        }
        // Fill remaining with blanks
        for _ in pulled..needed {
            grid.push_blank_line(old_cols);
        }
    }

    // HORIZONTAL RESIZE (no reflow)
    for row in grid.rows {
        row.resize(new_cols, Cell::blank());
    }

    // CLAMP CURSOR
    cursor.x = min(cursor.x, new_cols - 1);
    cursor.y = min(cursor.y, new_rows - 1);

    grid.rows = new_rows;
    grid.cols = new_cols;

    // NOTIFY PTY
    pty.resize(PtySize { rows: new_rows, cols: new_cols, .. });
}
```

### Relationship: Grid → PTY → Shell

```
┌─────────────┐
│ Window      │  1. Resize event from window system
│ System      │
└──────┬──────┘
       │
       v
┌─────────────┐
│ Terminal    │  2. Resize internal grid buffer
│ Emulator    │     - Adjust rows/cols
│ (Ferrum)    │     - Move lines to/from scrollback
└──────┬──────┘     - Clamp cursor position
       │
       │  3. ioctl(pty_fd, TIOCSWINSZ, &winsize)
       v
┌─────────────┐
│ PTY         │  4. Kernel sends SIGWINCH to
│ (portable-  │     foreground process group
│  pty)       │
└──────┬──────┘
       │
       │  SIGWINCH signal
       v
┌─────────────┐
│ Shell       │  5. Signal handler triggered
│ (bash)      │  6. ioctl(stdin, TIOCGWINSZ, &size)
│             │  7. Readline redraws prompt
└─────────────┘
```

**Key Points**:
- Grid resize happens BEFORE PTY notification
- Terminal emulator does NOT send SIGWINCH (kernel does)
- Shell handles all redrawing (terminal doesn't know what to redraw)
- Cursor must be in valid position before TIOCSWINSZ

---

## Code Examples

### With portable-pty v0.9.0

```rust
use portable_pty::{PtySize, MasterPty};

pub struct Terminal {
    grid: Grid,
    scrollback: VecDeque<Row>,
    cursor: Position,
    pty: Option<Box<dyn MasterPty>>,
}

impl Terminal {
    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        let old_rows = self.grid.rows();
        let old_cols = self.grid.cols();

        // === VERTICAL RESIZE ===
        if new_rows < old_rows {
            // Shrinking vertically
            let excess = old_rows - new_rows;

            // Keep cursor visible
            if self.cursor.y >= new_rows {
                let scroll_amount = self.cursor.y - new_rows + 1;
                for _ in 0..scroll_amount {
                    if let Some(line) = self.grid.remove_line(0) {
                        self.scrollback.push_back(line);
                    }
                }
                self.cursor.y = new_rows.saturating_sub(1);
            }

            self.grid.truncate_rows(new_rows);
        }
        else if new_rows > old_rows {
            // Growing vertically
            let needed = new_rows - old_rows;
            let mut pulled = 0;

            // Pull from scrollback
            while pulled < needed && !self.scrollback.is_empty() {
                if let Some(line) = self.scrollback.pop_back() {
                    self.grid.insert_line(0, line);
                    self.cursor.y += 1;
                    pulled += 1;
                }
            }

            // Fill remaining with blank lines
            for _ in pulled..needed {
                self.grid.push_blank_line(old_cols);
            }
        }

        // === HORIZONTAL RESIZE (no reflow) ===
        for row_idx in 0..self.grid.rows() {
            let row = self.grid.get_row_mut(row_idx);
            if new_cols > old_cols {
                // Pad with blanks
                row.resize(new_cols, Cell::blank());
            } else {
                // Truncate
                row.truncate(new_cols);
            }
        }

        // === CLAMP CURSOR ===
        self.cursor.x = self.cursor.x.min(new_cols.saturating_sub(1));
        self.cursor.y = self.cursor.y.min(new_rows.saturating_sub(1));

        // === NOTIFY PTY ===
        if let Some(ref pty) = self.pty {
            let size = PtySize {
                rows: new_rows as u16,
                cols: new_cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            };

            // Ignore error - PTY might be closed
            let _ = pty.resize(size);
        }
    }
}
```

### Edge Cases to Handle

```rust
// 1. Minimum size enforcement
pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
    let new_rows = new_rows.max(1);
    let new_cols = new_cols.max(1);
    // ... rest of resize logic
}

// 2. Cursor on last line when shrinking
if self.cursor.y >= new_rows {
    // Must scroll to keep visible
    let scroll_amount = self.cursor.y - new_rows + 1;
    // ... scrolling logic
}

// 3. Empty scrollback when growing
while pulled < needed && !self.scrollback.is_empty() {
    // ... pull logic
}
// If scrollback empty, fall through to adding blanks

// 4. Wide characters on horizontal truncate
// When truncating, check if cutting mid-wide-char
if new_cols < old_cols {
    for row in &mut self.grid.rows {
        if row.len() >= new_cols && row[new_cols-1].is_wide_char_spacer() {
            row[new_cols-1] = Cell::blank(); // Replace spacer
        }
        row.truncate(new_cols);
    }
}
```

---

## Recommendations

### For Ferrum (Learning Project)

**Priority 1: Simple Resize Without Reflow**

Implement the algorithm shown above:
- Vertical: Move lines to/from scrollback, keep cursor visible
- Horizontal: Truncate/pad, clamp cursor
- PTY: Call `pty.resize()` after grid adjustment
- Result: Correct, robust, ~100 lines of code

**Rationale**:
- Follows xterm (reference implementation) behavior
- Significantly simpler than reflow (~500+ fewer lines)
- Fewer edge cases and bugs
- Scrollback still works (most important for users)
- Good enough for learning project

**Testing Approach**:
```rust
#[test]
fn test_resize_shrink_vertical() {
    let mut term = Terminal::new(10, 80);
    term.cursor = Position { x: 5, y: 8 };

    term.resize(5, 80); // Shrink to 5 rows

    assert_eq!(term.cursor.y, 4); // Clamped to last row
    assert_eq!(term.scrollback.len(), 4); // 4 lines pushed to scrollback
}

#[test]
fn test_resize_grow_from_scrollback() {
    let mut term = Terminal::new(5, 80);
    term.scrollback.push_back(Row::from_str("old line"));

    term.resize(6, 80); // Grow by 1 row

    assert_eq!(term.grid.line(0).text(), "old line"); // Pulled from scrollback
    assert_eq!(term.scrollback.len(), 0); // Scrollback consumed
}
```

**Priority 2 (Future): Add Horizontal Reflow**

If horizontal resize UX becomes important:
- Study Alacritty's `resize.rs` implementation
- Implement WRAPLINE flag to track soft wraps
- Add reflow logic for grow/shrink columns
- Expect ~400-500 additional lines
- Test extensively with long command lines, vim, htop

**Priority 3 (Optional): Saved Cursor Support**

If supporting alternate screen (for vim, less, etc.):
- Track both primary cursor and saved cursor (DECSC/DECRC)
- Clamp both cursors on resize
- Note: Reflow complicates saved cursor position (see Ghostty issue #5718)

---

## Potential Issues

| Issue | Impact | Mitigation |
|-------|--------|------------|
| Cursor off-screen after resize | High - terminal unusable | Always clamp cursor position and scroll content to keep cursor visible |
| Scrollback lost on shrink | Medium - history gone | Push lines to scrollback ring buffer, don't discard |
| Shell doesn't redraw | Medium - stale display | Trust SIGWINCH mechanism; test with bash 5+; user can press Enter to trigger redraw |
| Wide characters corrupted | Low - visual glitch | Check for wide char spacers when truncating, replace with blank |
| Resize during VT sequence parse | Low - rare edge case | vte parser should handle gracefully; test with rapid resize |
| Very small size (1x1) | Low - edge case | Enforce minimum size (e.g., 2x10) or handle gracefully |
| Resize before PTY ready | Low - startup race | Check if PTY exists before calling resize() |

---

## Sources

### Primary Sources (Official Documentation)
- [XTerm FAQ](https://invisible-island.net/xterm/xterm.faq.html) - xterm official documentation
- [xterm(1) manual](https://www.x.org/archive/X11R6.8.1/doc/xterm.1.html) - xterm manual page
- [TIOCSWINSZ(2) Linux manual](https://man7.org/linux/man-pages/man2/TIOCSWINSZ.2const.html) - ioctl documentation
- [ioctl_tty(4) Linux manual](https://linux.die.net/man/4/tty_ioctl) - TTY ioctl reference
- [GNU C Library - Miscellaneous Signals](https://www.gnu.org/software/libc/manual/html_node/Miscellaneous-Signals.html) - SIGWINCH spec
- [Readline Signal Handling](https://docs.rtems.org/releases/4.5.1-pre3/toolsdoc/gdb-5.0-docs/readline/readline00030.html) - Readline docs

### Implementation References
- [Alacritty resize.rs](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/grid/resize.rs) - Full reflow implementation
- [Alacritty commit 08a1225](https://github.com/alacritty/alacritty/commit/08a122574880d299181c59bec186c0f9e8bef77c) - PTY resize messages
- [st dev mailing list](https://lists.suckless.org/dev/1703/31237.html) - st resize approach
- [st fork with reflow](https://github.com/ashish-yadav11/st) - Reflow patch for st

### Terminal Behavior Discussions
- [urxvt reflow discussion](https://forums.gentoo.org/viewtopic-t-964078.html) - urxvt limitations
- [Kitty discussions #3848](https://github.com/kovidgoyal/kitty/discussions/3848) - Shell integration and reflow
- [bash 5 SIGWINCH change](https://github.com/dylanaraps/fff/issues/48) - SA_RESTART behavior

### GitHub Issues (Cross-Reference)
- [Microsoft Terminal #10848](https://github.com/microsoft/terminal/issues/10848) - Cursor position on resize
- [Microsoft Terminal #4354](https://github.com/microsoft/terminal/pull/4354) - Scrollback preservation
- [Alacritty #8576](https://github.com/alacritty/alacritty/issues/8576) - Resize scrollback bug
- [Ghostty #5718](https://github.com/ghostty-org/ghostty/issues/5718) - Saved cursor and reflow

### Rust Resources
- [crossterm docs](https://docs.rs/crossterm) - Terminal manipulation in Rust
- [ratatui docs](https://docs.rs/ratatui) - TUI framework with auto-resize
- [memterm docs](https://docs.rs/memterm) - Virtual terminal emulator crate

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| Source verification | ✅ | Official docs cited: xterm, Linux man pages, GNU C Library, Readline. Source code: Alacritty, Kitty, st |
| Recency check | ✅ | Sources from 2017-2026, emphasis on 2020s. POSIX 2024 changes documented. xterm still actively maintained |
| Alternatives explored | ✅ | 8+ terminals compared: xterm, st, rxvt/urxvt, iTerm2, Alacritty, Kitty, WezTerm, Ghostty, Microsoft Terminal |
| Actionability | ✅ | Complete algorithm provided with pseudo-code and Rust implementation. Edge cases documented. Testing approach included |
| Evidence quality | ✅ | Claims backed by official specs (POSIX, xterm), source code analysis (Alacritty), and implementation discussions. Facts vs inferences clearly distinguished |

**Limitations/Caveats**:
- VTE parser interaction during resize not deeply researched (assumed to handle gracefully)
- No quantitative performance benchmarks (qualitative complexity analysis only)
- Alternate screen buffer resize behavior assumed similar to primary screen
- Testing methodology not exhaustive (suggested basic tests only)
