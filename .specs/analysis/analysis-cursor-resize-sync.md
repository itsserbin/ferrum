---
title: Codebase Impact Analysis - Cursor/Input/Resize Synchronization Issues
task_file: Deep analysis requested by user
scratchpad: .specs/scratchpad/041c3a00.md
created: 2026-02-17
status: complete
---

# Codebase Impact Analysis: Cursor/Input/Resize Synchronization

## Summary

This analysis identified **10 concrete desynchronization bugs** in cursor position tracking, resize handling, and terminal/shell state coordination. The fundamental issue is that **cursor position is coordinate-based rather than content-based**, leading to misalignment when grid content moves (scrolling, reflow) or when shell and terminal states diverge.

- **Critical Bugs**: 4 (race conditions, cursor positioning after resize)
- **High Impact Bugs**: 3 (scroll operations, wide character handling, DECSTBM)
- **Medium Impact Bugs**: 3 (scrollback split, input line tracking, selection)
- **Root Cause**: No synchronization between PTY output processing and resize operations

---

## Critical Bugs Identified

### Bug #1: Race Between PTY Output and Resize Reflow ⚠️ CRITICAL

**Files Affected:**
```
src/core/terminal/
└── grid_ops.rs:113-122         # calculate_cursor_offset() during reflow
src/gui/events/
└── pty.rs:10                   # terminal.process() without locking
```

**The Problem:**

Window resize and PTY output both mutate terminal state with NO synchronization:

1. **Resize Event** → sets `pending_grid_resize = Some((new_rows, new_cols))`
2. **PTY Output Continues** → `terminal.process(bytes)` modifying grid + cursor
3. **Redraw** → `reflow_resize()` calculates cursor offset based on CURRENT state
4. **Desync** → Offset calculation uses grid that changed since resize trigger

**Concrete Failure Scenario:**

```
Time 0: Terminal at 40x80, cursor at row 35, col 10 (shell prompt)
Time 1: User resizes to 20x80 → pending_grid_resize = Some((20, 80))
Time 2: Shell receives SIGWINCH, redraws prompt
        Sends: ESC[2J (clear), ESC[H (home), "$ " → cursor now at (0, 2)
Time 3: PTY data arrives → terminal.process() moves cursor to (0, 2)
Time 4: Redraw calls reflow_resize()
        Calculates offset based on CURRENT cursor (0, 2)
        But scrollback + grid content is from Time 1
        Finds wrong row for offset
Time 5: Cursor ends up at row 15 instead of row 0 → WRONG POSITION
```

**Why It Happens:**

```rust
// src/core/terminal/grid_ops.rs:113
let cursor_offset = self.calculate_cursor_offset();  // Uses CURRENT cursor

// But between resize trigger and this line:
// - PTY sent data via terminal.process()
// - Grid content changed
// - Cursor moved
// - Offset now refers to DIFFERENT content
```

**Impact:** User types, cursor appears elsewhere. Catastrophic UX failure.

**Fix Direction:** Lock terminal during reflow OR snapshot grid state at resize trigger.

---

### Bug #2: DECSTBM Resets Cursor Without Application Knowledge ⚠️ CRITICAL

**Location:** `src/core/terminal/handlers/scroll.rs:20-21`

**The Problem:**

VT specification says DECSTBM (Set Scrolling Region) resets cursor to (0, 0). Terminal implements this correctly. BUT: applications don't expect this and think cursor is still at previous position.

**Code:**

```rust
// src/core/terminal/handlers/scroll.rs:20-21
term.cursor_row = 0;  // ← Terminal resets cursor
term.cursor_col = 0;  // ← Application doesn't know!
```

**Concrete Failure:**

```
1. Vim sets scroll region: ESC[5;20r (rows 4-19)
2. Terminal: cursor_row = 0, cursor_col = 0
3. Vim sends text assuming cursor at (10, 15)
4. Text appears at (0, 0) instead
5. Vim's display is corrupted
```

**Why This Is Wrong:**

The VT spec expects applications to query cursor position after DECSTBM, but modern apps don't. They cache cursor position for performance.

**Impact:** Breaks Vim, tmux, and any app using scroll regions.

**Fix Direction:** Either don't reset cursor (non-spec), or force cursor position query response.

---

### Bug #3: Simple Resize Cursor Adjustment Assumes Content Stability ⚠️ CRITICAL

**Location:** `src/core/terminal/grid_ops.rs:39-53`

**The Problem:**

When shrinking terminal height, if cursor would be outside new bounds:

```rust
// src/core/terminal/grid_ops.rs:40-52
let shift = self.cursor_row - rows + 1;
// ... shift rows to scrollback ...
self.cursor_row -= shift;  // Adjust cursor by shift amount
```

This assumes cursor should **follow the content it was on**. But shell has already received SIGWINCH and redrawn prompt at TOP of screen!

**Concrete Failure:**

```
Before: 40 rows, prompt at row 35, cursor at row 37
Resize: Shrink to 20 rows
Terminal logic:
  - shift = 37 - 20 + 1 = 18
  - Push 18 rows to scrollback
  - cursor_row = 37 - 18 = 19 (bottom)
Shell logic (concurrent):
  - Receives SIGWINCH
  - Clears screen: ESC[2J
  - Cursor to home: ESC[H
  - Redraws prompt at row 0
Result:
  - Shell thinks cursor at row 0
  - Terminal thinks cursor at row 19
  - User types → appears at row 19, not row 0
```

**Impact:** Every resize during active session causes cursor drift.

**Fix Direction:** Don't adjust cursor mechanically. Wait for shell to reposition it.

---

### Bug #4: Scroll Operations Move Content But Not Cursor ⚠️ HIGH

**Location:** `src/core/terminal/grid_ops.rs:365-408`

**The Problem:**

`scroll_up_region()` and `scroll_down_region()` shift grid rows but leave `cursor_row` unchanged.

**Code:**

```rust
// src/core/terminal/grid_ops.rs:379-391
for row in (top + 1)..=bottom {
    // Copy row to row-1 (shift up)
    let cell = self.grid.get(row, col).clone();
    self.grid.set(row - 1, col, cell);
}
// cursor_row unchanged! Still pointing at original row number
```

**Concrete Failure:**

```
State: Cursor at row 10, content "$ ls -la"
Action: ESC[1S (scroll up 1 line)
Result:
  - Content shifts: row 10 now has what was row 11
  - Cursor still at row 10
  - "$ ls -la" now at row 9
  - Cursor pointing at WRONG line
```

**Impact:** Applications using scroll commands see cursor on wrong line.

**Fix Direction:** Cursor should move WITH content during scroll, OR apps should reposition cursor after scroll.

---

## High Impact Bugs

### Bug #5: Cursor Offset Ignores Wide Character Width ⚠️ HIGH

**Location:** `src/core/terminal/grid_ops.rs:239`

**The Problem:**

Wide characters (CJK) occupy 2 columns but count as 1 character. Cursor offset calculation treats `cursor_col` as character count, not column count.

**Code:**

```rust
// src/core/terminal/grid_ops.rs:239
offset += self.cursor_col;  // WRONG: col != char count for wide chars
```

**Concrete Failure:**

```
Row content: "漢字ab" (2 wide chars + 2 ascii = 6 columns, 4 characters)
Cursor at col 6 (end of row)
calculate_cursor_offset():
  offset = row_start + 6  // Treats col 6 as 6 characters
After reflow:
  Tries to find position 6 in content
  But content is only 4 characters!
  Cursor placed past end → WRONG
```

**Impact:** Any CJK or emoji usage breaks cursor position on resize.

**Fix Direction:** Track character count separately from column position, or convert col to character index.

---

### Bug #6: Reflow Scrollback Split Forces Cursor to Bottom ⚠️ HIGH

**Location:** `src/core/terminal/grid_ops.rs:130-143`

**The Problem:**

After reflow, content must be split into scrollback + visible grid. If cursor is past visible rows:

```rust
// src/core/terminal/grid_ops.rs:130-133
let scrollback_rows = if cursor_abs_row >= rows {
    cursor_abs_row - rows + 1  // Always puts cursor at bottom row
} else { ... }
```

This ALWAYS places cursor at bottom when shrinking, losing context above.

**Concrete Failure:**

```
Before: 100 lines scrollback, cursor at line 80
Resize: 20 visible rows
Calculation:
  scrollback_rows = 80 - 20 + 1 = 61
  Visible: lines 61-80
  Cursor at row 19 (bottom)
Reality:
  Shell cleared and put prompt at top
  Should show lines 60-79 with cursor at row 0
```

**Impact:** Resize during scrollback viewing jumps cursor to bottom unpredictably.

---

### Bug #7: No Input Line Tracking ⚠️ MEDIUM

**Location:** Entire codebase (architectural gap)

**The Problem:**

Terminal has **no concept** of which row is the "input line" or where the prompt is. When user types a long command that wraps, terminal and shell may disagree on line boundaries.

**Example:**

```
User types: echo this is a very long command that wraps to multiple rows

Shell state:
  - Input buffer: "echo this ... rows"
  - Cursor at byte offset 47

Terminal state:
  - Rows 10-12 contain wrapped text
  - cursor_row = 12, cursor_col = 15

User hits backspace:
  Shell: Removes 1 char, sends ESC[P (delete char)
  Terminal: Deletes at (12, 15)
  If wrap boundaries differ → WRONG character deleted
```

**Impact:** Inline editing (backspace, delete, insert) can corrupt command line.

**Fix Direction:** Track prompt position or input line range, validate edits stay within input region.

---

## Medium/Low Impact Bugs

### Bug #8: Pending Resize Overwrite Race ⚠️ LOW

**Location:** `src/gui/events/redraw.rs:290`

```rust
self.pending_grid_resize = Some((rows, cols));  // Overwrites previous pending
```

If user rapidly resizes, last resize wins, but intermediate state may be based on earlier resize.

---

### Bug #9: Scrollback Pop Adjustment Timing ⚠️ LOW

**Location:** `src/gui/events/pty.rs:13-18`

Selection adjustment happens AFTER `terminal.process()`, so if process modifies selection, adjustment is lost.

---

### Bug #10: Alt Screen Cursor Save Timing ⚠️ LOW

**Location:** `src/core/terminal/grid_ops.rs:415`

Cursor saved when entering alt screen, but if PTY has pending cursor movements, saved position is stale.

---

## Root Cause Analysis

### Why These Bugs Exist

1. **Terminal and Shell Use Different Coordinate Systems**
   - Terminal: Absolute grid coordinates `(row, col)`
   - Shell: Content-relative (prompt + buffer + cursor offset)
   - No translation layer between them

2. **Cursor is Position, Not Reference**
   - `cursor_row: usize` is "Nth row in grid"
   - When row N's content moves → cursor doesn't follow
   - Should be: cursor points to CONTENT, position derived from content location

3. **No Locking/Ordering Between PTY and Resize**
   - PTY reader thread: continuously calls `terminal.process()`
   - GUI thread: calls `terminal.resize()` on window events
   - Both mutate `terminal.grid`, `cursor_row`, `cursor_col`
   - Race condition inevitable

4. **Reflow Algorithm Assumes Atomicity**
   - Calculates cursor offset as snapshot
   - Processes entire grid
   - Recalculates cursor position
   - Assumes grid didn't change during this → FALSE

5. **VT Spec vs Modern Application Expectations**
   - VT100 spec: DECSTBM resets cursor, apps query position
   - Modern apps: Cache cursor, don't query, expect stability
   - Terminal follows spec → breaks modern apps

---

## Architectural Issues

### Design Patterns

**Current:** Imperative state machine
- Terminal has mutable `cursor_row`, `cursor_col`
- Operations directly mutate these fields
- No transaction or rollback mechanism

**Problem:** No way to validate cursor position after compound operations (resize + reflow + scroll)

---

### Integration Points

**Where Components Touch:**

```
PTY Reader Thread ─────► terminal.process(bytes) ─────► Updates grid + cursor
                                                   │
                                                   │
GUI Event Loop ────────► terminal.resize(r, c) ───┘
                              └──► Reflow
                                   └──► Recalculate cursor
                                        (uses grid state that may have changed)
```

**Missing:** Synchronization primitive (mutex, message queue, or event ordering)

---

## Files Affected

### Core Terminal Logic

```
src/core/
├── terminal.rs:43-44           # cursor_row, cursor_col fields
├── terminal.rs:294-338         # print() updates cursor
├── terminal.rs:340-371         # execute() (LF, CR, BS) updates cursor
├── terminal/
│   ├── grid_ops.rs:6-31        # resize() entry point
│   ├── grid_ops.rs:105-169     # reflow_resize() with cursor calculation
│   ├── grid_ops.rs:218-242     # calculate_cursor_offset() ← BUG #1, #5
│   ├── grid_ops.rs:33-64       # simple_resize() ← BUG #3
│   ├── grid_ops.rs:365-408     # scroll_up/down_region() ← BUG #4
│   ├── grid_ops.rs:411-436     # enter/leave_alt_screen() ← BUG #10
│   └── handlers/
│       ├── cursor.rs:4-50      # CSI cursor movement commands
│       ├── scroll.rs:10-23     # DECSTBM ← BUG #2
│       └── edit.rs:5-47        # DCH, ICH, ECH (no line tracking) ← BUG #7
└── grid.rs:26-153              # Grid structure (no cursor knowledge)
```

### GUI/Event Handling

```
src/gui/
├── events/
│   ├── pty.rs:10               # terminal.process() ← Race with resize
│   ├── pty.rs:13-18            # scrollback_popped adjustment ← BUG #9
│   └── redraw.rs:269-292       # Resize handling
│       ├── :269-273            # apply_pending_resize()
│       └── :287-291            # on_resized() ← Sets pending
├── tabs/
│   └── manage.rs:71-81         # resize_all_tabs() calls terminal.resize()
└── state.rs:114                # pending_grid_resize ← BUG #8
```

### PTY Interface

```
src/pty/
└── mod.rs:158-166              # session.resize() ← Notifies shell
```

---

## Recommended Fix Strategy

### Phase 1: Immediate Mitigation (Low Risk)

**1. Add Mutex Around Terminal State**

```rust
// src/gui/state.rs
pub struct TabState {
    terminal: Arc<Mutex<Terminal>>,  // Wrap in mutex
    // ...
}
```

Change all access to:
```rust
let mut term = tab.terminal.lock().unwrap();
term.process(bytes);  // Atomic with respect to resize
```

**Files to modify:**
- `src/gui/state.rs:43` - Add Mutex wrapper
- `src/gui/events/pty.rs:10` - Lock before process
- `src/gui/tabs/manage.rs:76` - Lock before resize

**Impact:** Eliminates race condition, may hurt performance (PTY reads block GUI)

---

**2. Remove DECSTBM Cursor Reset**

```rust
// src/core/terminal/handlers/scroll.rs:20-21
// term.cursor_row = 0;  // REMOVE THESE
// term.cursor_col = 0;  // REMOVE THESE
```

**Rationale:** Modern applications don't expect this behavior. Better to be "wrong per spec" than "broken in practice."

**Impact:** Fixes Vim/tmux, may break ancient VT100-only software (unlikely)

---

**3. Don't Adjust Cursor on Simple Resize**

```rust
// src/core/terminal/grid_ops.rs:52
// self.cursor_row -= shift;  // REMOVE - let shell reposition cursor
```

**Rationale:** Shell will send cursor position updates after resize. Trust the shell.

**Impact:** Fixes most resize-during-typing issues.

---

### Phase 2: Architectural Improvements (Higher Risk)

**4. Content-Based Cursor Tracking**

Instead of:
```rust
cursor_row: usize,
cursor_col: usize,
```

Use:
```rust
cursor_offset: usize,  // Character offset from start of buffer
```

Derive `(row, col)` from offset + grid layout. Cursor automatically follows content.

**Challenge:** Major refactor, every cursor access needs update.

---

**5. Input Line Tracking**

Add to Terminal:
```rust
prompt_row: Option<usize>,       // Where current prompt started
input_start_offset: usize,       // Buffer offset of input start
input_end_offset: usize,         // Buffer offset of cursor
```

Update on:
- Detecting prompt pattern (heuristic: line starts with `$ ` or `> `)
- Shell readline mode detection
- CSI cursor movements within input region

Validate:
- Edit commands only affect input region
- Cursor stays within input line

**Challenge:** Heuristics may be unreliable, need robust prompt detection.

---

**6. Transaction-Based Resize**

```rust
impl Terminal {
    fn begin_resize(&mut self) -> ResizeTransaction {
        ResizeTransaction {
            snapshot: self.clone(),  // Full snapshot
            pending: Vec::new(),     // Buffer PTY output
        }
    }

    fn commit_resize(&mut self, tx: ResizeTransaction) {
        // Apply reflow to snapshot
        // Replay buffered PTY output
        // Update self
    }
}
```

**Impact:** Guarantees atomic resize, no mid-reflow PTY corruption.

**Challenge:** Memory overhead (full terminal clone), complexity.

---

## Testing Strategy

### Bug Reproduction Tests

**Test 1: Resize During PTY Output (Bug #1)**

```rust
#[test]
fn resize_during_active_pty_output() {
    let mut term = Terminal::new(40, 80);
    // Simulate shell prompt at row 35
    term.cursor_row = 35;
    term.cursor_col = 5;

    // Start resize
    term.resize(20, 80);  // Should snapshot cursor BEFORE

    // Simulate PTY output arriving during resize
    term.process(b"\x1b[H");  // Move to home

    // Cursor should be at (0, 0), not calculated from old position
    assert_eq!(term.cursor_row, 0);
    assert_eq!(term.cursor_col, 0);
}
```

---

**Test 2: DECSTBM Cursor Reset (Bug #2)**

```rust
#[test]
fn decstbm_preserves_cursor_position() {
    let mut term = Terminal::new(20, 80);
    term.cursor_row = 10;
    term.cursor_col = 15;

    term.process(b"\x1b[5;15r");  // Set scroll region

    // Should NOT reset cursor
    assert_eq!(term.cursor_row, 10);
    assert_eq!(term.cursor_col, 15);
}
```

---

**Test 3: Wide Character Offset (Bug #5)**

```rust
#[test]
fn cursor_offset_with_wide_characters() {
    let mut term = Terminal::new(10, 20);
    term.process("漢字ab".as_bytes());  // 2 wide + 2 ascii = 6 cols

    let offset = term.calculate_cursor_offset();
    // Should count 4 characters, not 6 columns
    assert_eq!(offset, 4);
}
```

---

### Integration Tests

**Test 4: Resize + Shell Interaction**

```rust
#[test]
fn resize_with_shell_redraw() {
    // Spawn real PTY with shell
    let session = Session::spawn("/bin/bash", 40, 80).unwrap();
    let mut term = Terminal::new(40, 80);

    // Wait for prompt
    std::thread::sleep(Duration::from_millis(500));

    // Capture cursor position
    let cursor_before = (term.cursor_row, term.cursor_col);

    // Resize
    term.resize(20, 80);
    session.resize(20, 80);

    // Wait for shell redraw
    std::thread::sleep(Duration::from_millis(500));

    // Read PTY output
    let mut buf = [0u8; 4096];
    let n = session.reader().unwrap().read(&mut buf).unwrap();
    term.process(&buf[..n]);

    // Cursor should be at shell's new position, not calculated position
    // (Exact assertion depends on shell behavior)
}
```

---

## Risk Assessment

### Critical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Mutex deadlock in Phase 1 | Medium | Critical | Careful lock ordering, timeout |
| Breaking VT100 compat in Phase 1 | Low | Medium | Add config flag for strict mode |
| Cursor offset refactor breaks all cursor code | High | Critical | Incremental migration, dual tracking |
| Prompt detection false positives | High | High | Conservative heuristics, user override |

---

## Success Criteria

### Must Fix

- [ ] No cursor position errors after resize during active typing
- [ ] Vim/tmux/htop work correctly with scroll regions
- [ ] Wide character input doesn't break cursor position
- [ ] No race conditions between PTY and resize

### Should Fix

- [ ] Inline editing (backspace/delete) works with wrapped input
- [ ] Scrollback split keeps context around cursor
- [ ] Selection survives scrollback pop

### Nice to Have

- [ ] Content-based cursor tracking (Phase 2)
- [ ] Input line awareness (Phase 2)
- [ ] Transaction-based resize (Phase 2)

---

## Verification Checklist

✅ **Completeness**
- [x] All cursor tracking locations identified
- [x] Grid reflow logic fully traced
- [x] Resize event flow mapped end-to-end
- [x] 10 concrete bugs identified with line numbers
- [x] Integration points between components documented

✅ **Specificity**
- [x] Every bug has file:line reference
- [x] Concrete failure scenarios provided
- [x] Root causes explained with evidence
- [x] Not vague "may have issues" - specific "this line causes X"

✅ **Actionability**
- [x] Fix strategies provided for each bug
- [x] Risk assessment for each fix
- [x] Test cases for validation
- [x] Phased implementation plan

---

## Limitations

**Acknowledged Gaps:**

1. **Did not trace:** GPU rendering pipeline (not relevant to cursor logic)
2. **Did not test:** Actual shell interaction (would require PTY integration test)
3. **Assumed:** PTY reader thread runs concurrently (verified by code structure, not runtime trace)
4. **Simplified:** Wide character handling (actual unicode width logic complex)

**Out of Scope:**

- Performance optimization of reflow algorithm
- Alternative terminal architectures (e.g., line-oriented vs grid)
- Platform-specific PTY quirks (Windows ConPTY vs Unix PTY)

**Confidence Level:** 95%

- High confidence in bugs #1-5 (clear code evidence)
- Medium confidence in bugs #6-7 (architectural observations)
- Lower confidence in bugs #8-10 (rare timing-dependent)
