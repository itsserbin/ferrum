---
title: Codebase Impact Analysis - Mouse Click Handling in Ferrum Terminal
task_file: User request to investigate broken click functionality
scratchpad: /home/user/apps/ferrum/.specs/scratchpad/a8f3c2d1.md
created: 2026-02-16
status: complete
---

# Codebase Impact Analysis: Mouse Click Handling in Ferrum Terminal

## Summary

**Root Cause Identified**: The `suppress_click_to_cursor_once` flag mechanism is broken, causing clicks to be ignored after window focus events.

- **Files Involved**: 3 files need modification
- **Bug Severity**: High (breaks core click-to-cursor functionality)
- **Risk Level**: Low (fix is straightforward)

---

## Root Cause: suppress_click_to_cursor_once Flag

### The Bug

The `suppress_click_to_cursor_once` flag is set to `true` when the window gains focus (to prevent cursor jumps from "stale" mouse positions), but the clearing mechanism is unreliable:

**Problem Flow**:
1. User clicks window to focus → `WindowEvent::Focused(true)` fires
2. `suppress_click_to_cursor_once` set to `true` (lifecycle.rs:77)
3. Flag should be cleared by `WindowEvent::CursorMoved` (cursor.rs:7)
4. **BUT**: Timing issues or missing CursorMoved events leave flag stuck at `true`
5. Click release checks flag → cursor move is suppressed (terminal_click.rs:99-101)
6. **Result**: Clicks don't work!

### Evidence

| File | Line | Issue |
|------|------|-------|
| `src/gui/lifecycle.rs` | 77 | Sets `suppress_click_to_cursor_once = true` on focus |
| `src/gui/events/mouse/cursor.rs` | 7 | Clears flag on cursor move (unreliable timing) |
| `src/gui/events/mouse/terminal_click.rs` | 99-101 | Suppresses click-to-cursor if flag is true |

---

## Complete Event Flow: Click to PTY

```
User clicks terminal
    ↓
[WINIT EVENTS]
    ↓
WindowEvent::Focused(true)              [lifecycle.rs:63-81]
  └─ suppress_click_to_cursor_once = true  ← BUG: Set here
    ↓
WindowEvent::CursorMoved                [lifecycle.rs:96-98]
  └─ on_cursor_moved()                  [cursor.rs:5-78]
      └─ suppress_click_to_cursor_once = false  ← Should clear, but timing issues
    ↓
WindowEvent::MouseInput(Pressed)        [lifecycle.rs:99-101]
  └─ on_mouse_input()                   [events/mouse/input.rs:5-17]
      └─ on_left_mouse_input()          [events/mouse/input.rs:92-113]
          ├─ Check context menu         [input.rs:99] (early return if hit)
          ├─ Check tab bar              [input.rs:103] (early return if hit)
          ├─ Check resize zones         [input.rs:108] (early return if hit)
          └─ handle_terminal_left_click [events/mouse/terminal_click.rs:22-108]
              ├─ pixel_to_grid()        [interaction/geometry.rs:6-18]
              ├─ Check mouse_reporting  [terminal_click.rs:32]
              │   └─ If yes: send_mouse_event() → PTY
              └─ If no: Handle selection
                  └─ is_selecting = true
    ↓
WindowEvent::MouseInput(Released)       [lifecycle.rs:99-101]
  └─ handle_terminal_left_click()       [terminal_click.rs:93-106]
      ├─ Check if should_move_cursor    [terminal_click.rs:94-95]
      │   → Only if no selection drag happened
      ├─ Check suppress flag            [terminal_click.rs:99-101]
      │   → If true: SKIP cursor move ← BUG TRIGGERS HERE
      └─ move_cursor_to()               [interaction/cursor_move.rs:7-70]
          ├─ Calculate arrow sequences
          └─ pty_writer.write_all()     [cursor_move.rs:67-68]
```

---

## Files to be Modified

```
src/gui/
├── lifecycle.rs                   # MODIFY: Remove suppress flag set on focus
├── events/
│   └── mouse/
│       ├── terminal_click.rs      # MODIFY: Remove suppress flag check
│       └── cursor.rs              # MODIFY: Remove suppress flag clear
└── state.rs                       # MODIFY: Remove suppress field
```

---

## Key Interfaces & Functions

### Mouse Event Handlers

| Location | Function | Current Behavior | Issue |
|----------|----------|------------------|-------|
| `src/gui/events/mouse/input.rs:5-17` | `on_mouse_input` | Dispatches by button type | ✅ Works correctly |
| `src/gui/events/mouse/input.rs:92-113` | `on_left_mouse_input` | Routes to sub-handlers | ✅ Works correctly |
| `src/gui/events/mouse/terminal_click.rs:22-108` | `handle_terminal_left_click` | Handles terminal clicks | ❌ Suppresses due to flag |

### Click-to-Cursor Implementation

| Location | Function | Signature | Issue |
|----------|----------|-----------|-------|
| `src/gui/interaction/cursor_move.rs:7-70` | `move_cursor_to` | `fn(&mut self, row: usize, col: usize)` | ✅ Works when called |
| `src/gui/events/mouse/terminal_click.rs:104` | Call site | Calls `move_cursor_to(row, col)` | ❌ Call is suppressed |

### Suppress Flag Lifecycle

| Location | Action | When | Issue |
|----------|--------|------|-------|
| `src/gui/state.rs:68` | Declaration | `suppress_click_to_cursor_once: bool` | Should be removed |
| `src/gui/mod.rs:59` | Initialization | `suppress_click_to_cursor_once: false` | Should be removed |
| `src/gui/lifecycle.rs:77` | Set to true | `WindowEvent::Focused(true)` | ❌ Problematic |
| `src/gui/events/mouse/cursor.rs:7` | Set to false | `WindowEvent::CursorMoved` | ❌ Unreliable timing |
| `src/gui/events/mouse/terminal_click.rs:99-101` | Check | Before cursor move | ❌ Blocks clicks |

---

## Related Systems (Working Correctly)

### Mouse Reporting Mode (Terminal Apps)

| Location | Purpose | Status |
|----------|---------|--------|
| `src/core/terminal.rs:29-37` | MouseMode enum (Off/Normal/ButtonEvent/AnyEvent) | ✅ Correct |
| `src/gui/interaction/mouse_reporting.rs:22-27` | `is_mouse_reporting()` checks terminal mode | ✅ Correct |
| `src/gui/events/mouse/terminal_click.rs:32-46` | Sends mouse events to PTY for vim/tmux | ✅ Correct |

### Selection System

| Location | Purpose | Status |
|----------|---------|--------|
| `src/core/selection.rs` | Selection data structure | ✅ Correct |
| `src/gui/interaction/selection.rs:81-159` | `update_drag_selection()` | ✅ Correct |
| `src/gui/events/mouse/terminal_click.rs:56-91` | Selection on press (word/line select) | ✅ Correct |

### Coordinate Conversion

| Location | Purpose | Status |
|----------|---------|--------|
| `src/gui/interaction/geometry.rs:6-18` | `pixel_to_grid()` converts screen to grid coords | ✅ Correct |

---

## Recommended Fix

### Option 1: Remove Suppress Mechanism (Simplest)

**Rationale**: The original problem (stale cursor position after focus) is less severe than completely broken clicks.

**Changes**:

1. **Remove field from state** (`src/gui/state.rs:68`):
   ```rust
   // DELETE this line:
   pub(super) suppress_click_to_cursor_once: bool,
   ```

2. **Remove initialization** (`src/gui/mod.rs:59`):
   ```rust
   // DELETE this line:
   suppress_click_to_cursor_once: false,
   ```

3. **Remove flag set** (`src/gui/lifecycle.rs:77`):
   ```rust
   // DELETE this line:
   self.suppress_click_to_cursor_once = true;
   ```

4. **Remove flag clear** (`src/gui/events/mouse/cursor.rs:7`):
   ```rust
   // DELETE this line:
   self.suppress_click_to_cursor_once = false;
   ```

5. **Remove suppression check** (`src/gui/events/mouse/terminal_click.rs:99-101`):
   ```rust
   // DELETE these lines:
   if self.suppress_click_to_cursor_once {
       self.suppress_click_to_cursor_once = false;
       return;
   }
   ```

**Risk**: First click after window focus might move cursor to unexpected position (minor UX issue).

**Benefit**: Clicks work reliably.

---

### Option 2: Fix the Timing (Better, More Complex)

**Rationale**: Keep the feature but fix the timing issue.

**Changes**:

1. Clear the flag on `MouseInput(Pressed)` instead of on `CursorMoved`:

   **In `src/gui/events/mouse/terminal_click.rs:56`** (inside `ElementState::Pressed` branch):
   ```rust
   ElementState::Pressed => {
       // Clear suppress flag at press time, not on cursor move
       self.suppress_click_to_cursor_once = false;

       self.is_selecting = true;
       // ... rest of code
   }
   ```

2. Remove the clear from `src/gui/events/mouse/cursor.rs:7`.

3. Keep all other code the same.

**Risk**: Slightly more complex logic.

**Benefit**: Preserves the original intent (prevent stale cursor moves) while fixing the bug.

---

### Option 3: Track Specific Click (Most Robust)

**Rationale**: Only suppress the EXACT click that caused the focus event.

**Changes**:

1. Replace `suppress_click_to_cursor_once: bool` with `focus_causing_click: Option<std::time::Instant>` in state.

2. On focus: Set `focus_causing_click = Some(Instant::now())`

3. On click release: Check if click is within 50ms of focus timestamp; if yes, suppress.

4. After any suppression or after 100ms, clear the timestamp.

**Risk**: Most complex solution.

**Benefit**: Most correct behavior - only suppresses the problematic click.

---

## Testing Recommendations

### Manual Tests

1. **Basic Click**: Click in terminal → cursor should move
2. **Focus Click**: Click different window, then click terminal → cursor should move
3. **Selection**: Click and drag → should select text, not move cursor
4. **Double/Triple Click**: Double-click → select word; triple-click → select line
5. **Mouse Reporting**: Run `vim`, move mouse → vim should receive mouse events
6. **Tab Bar**: Click tab bar → should switch tabs, not move cursor
7. **Context Menu**: Right-click → paste clipboard

### Automated Tests (Future)

Create unit tests for:
- `pixel_to_grid()` coordinate conversion
- `move_cursor_to()` arrow sequence generation
- Selection boundary detection
- Mouse event encoding (SGR vs X10)

---

## Risk Assessment

### High Risk Areas

| Area | Risk | Mitigation |
|------|------|------------|
| Focus handling | Removing suppress might cause cursor jumps | Accept as lesser evil than broken clicks |
| Mouse reporting mode | Changes might break vim/tmux mouse support | No changes needed - this works correctly |

### Medium Risk Areas

| Area | Risk | Mitigation |
|------|------|------------|
| Selection timing | Rapid clicks might create unwanted selection | Existing logic handles this with click_streak |

### Low Risk Areas

| Area | Risk | Mitigation |
|------|------|------------|
| Coordinate conversion | Math errors could misplace cursor | Already working correctly, no changes |
| PTY writing | Buffer might overflow | Already using proper flushing |

---

## Architecture Insights

### Design Patterns

1. **Event Dispatch Pattern**: `ApplicationHandler` → `on_mouse_input` → specific button handlers
2. **Strategy Pattern**: Selection drag mode (Character/Word/Line) determines selection behavior
3. **State Machine**: Click streak (1/2/3) drives selection mode
4. **Coordinate Space Transformation**: Pixel coordinates → grid coordinates via affine transform

### Cross-Cutting Concerns

- **Focus Management**: Affects multiple subsystems (cursor suppression, selection reset, context menu)
- **Modifier Keys**: Shift key overrides mouse reporting mode to force local selection
- **Terminal Modes**: Alt-screen, mouse reporting, and cursor key modes interact with click behavior

---

## Similar Patterns in Codebase

### Focus-Related State Resets

**Location**: `src/gui/lifecycle.rs:63-81`

Other transient states reset on focus:
- `modifiers` → empty
- `is_selecting` → false
- `selection_anchor` → None
- `click_streak` → 0
- `context_menu` → None (on blur only)

**Pattern**: Focus changes should clear input state to prevent stuck interactions after Alt-Tab.

### PTY Write Pattern

**Locations**:
- `src/gui/interaction/cursor_move.rs:67-68` (arrow keys)
- `src/gui/interaction/mouse_reporting.rs:16-17` (mouse events)
- `src/gui/events/keyboard/forward.rs` (keyboard input)

**Pattern**: All PTY writes follow:
```rust
let _ = tab.pty_writer.write_all(&bytes);
let _ = tab.pty_writer.flush();
```

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| All affected files identified | ✅ | 4 files need modification (state, lifecycle, cursor, terminal_click) |
| Integration points mapped | ✅ | Focus events, cursor events, mouse events, PTY writes |
| Similar patterns found | ✅ | Focus state reset pattern, PTY write pattern |
| Test coverage analyzed | ⚠️ | No existing tests for click handling |
| Risks assessed | ✅ | Low risk - straightforward fix |

**Limitations/Caveats**:
- Exact winit event ordering may vary by platform/window manager
- Mouse reporting mode behavior not tested with live vim/tmux instances
- No automated tests exist for this functionality

---

## Recommended Implementation Steps

1. **Read** the 4 files listed above to understand current implementation
2. **Choose** fix option (recommend Option 1 for simplicity)
3. **Modify** the 4 files as specified in the fix section
4. **Test** manually with all test cases listed above
5. **Verify** no regressions in mouse reporting mode (test with vim)
6. **Document** the change in comments/commit message

---

## Additional Context

### Why Click-to-Cursor Uses Arrow Keys

Shell mode doesn't support absolute cursor positioning (no CSI sequences for "move cursor to column N"). The only reliable way to move the cursor is to send arrow key sequences (ESC[C for right, ESC[D for left).

**Limitations**:
- Shell mode: Only horizontal moves on current row (line 52-54 in cursor_move.rs)
- Alt-screen mode: Vertical moves work, but horizontal only reliable on same row (line 32-47)

### Mouse Reporting Modes

Terminal applications can request mouse events via DECSET:
- `?1000` (Normal): Press + Release only
- `?1002` (ButtonEvent): Press + Release + Drag
- `?1003` (AnyEvent): All mouse motion
- `?1006` (SGR): Use SGR encoding (more reliable than X10)

Ferrum correctly implements all modes (verified in mouse_reporting.rs and terminal.rs).
