# Fix Clear Terminal / Reset Terminal Context Menu Actions

**Date:** 2026-02-20

## Problem

The "Clear Terminal" and "Reset Terminal" context menu buttons don't work. Both currently use `write_pty_bytes()` to send escape sequences to the PTY, which sends them as stdin to the shell — the shell's line editor doesn't interpret these as terminal control sequences.

Additionally, the Reset Terminal action sends DECSTR (`ESC[!p`) which isn't even implemented in the terminal's VT parser.

## Design

### Approach: Direct Terminal Manipulation + Shell Signal

Instead of sending escape sequences to the PTY, directly manipulate `leaf.terminal` state, then send `\x0c` (Ctrl+L / form feed) to the PTY so the shell redraws its prompt.

### Clear Terminal

"Cosmetic" clear — erases screen content but preserves all terminal settings.

Actions:
1. Clear grid: `terminal.grid = Grid::new(rows, cols)`
2. Clear scrollback: `terminal.scrollback.clear()`
3. Reset cursor to (0, 0)
4. Reset `leaf.scroll_offset = 0`
5. Reset `leaf.selection = None`
6. Send `\x0c` (Ctrl+L) to PTY for prompt redraw

Preserves: colors, cursor style, mouse mode, DECCKM, scroll regions, alt-screen.

### Reset Terminal

"Nuclear" reset — full terminal state reset (equivalent to `reset` command / ESC c / RIS).

Actions:
1. Call `terminal.full_reset()` — resets grid, scrollback, cursor, alt-screen, colors, mouse mode, DECCKM, scroll regions, cursor style, etc.
2. Reset `leaf.scroll_offset = 0`
3. Reset `leaf.selection = None`
4. Send `\x0c` (Ctrl+L) to PTY for prompt redraw

### Code Changes

1. **`src/core/terminal.rs`**: Make `full_reset()` public (`pub fn full_reset`)
2. **`src/gui/events/menu_actions.rs`**: Replace `write_pty_bytes` calls with direct terminal manipulation
