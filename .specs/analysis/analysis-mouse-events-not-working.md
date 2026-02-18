---
title: Codebase Impact Analysis - Mouse Events Not Working After Refactoring
task_file: User bug report: mouse clicks don't work
scratchpad: .specs/scratchpad/2a43dbb6.md
created: 2026-02-16
status: complete
---

# Codebase Impact Analysis: Mouse Events Not Working After Refactoring

## Summary

**CRITICAL FINDING**: The code compiles successfully and the event handling chain is properly connected. All mouse event handlers exist and are correctly implemented. However, if mouse clicks truly don't work at runtime, the issue is likely one of these:

1. **Module import issue in gui/mod.rs** - The events submodule may not be exposing implementations correctly
2. **Method visibility issue** - Helper methods might not be accessible where needed
3. **Logic bug** - An early return or condition preventing handlers from executing
4. **Git/build cache issue** - Stale binary being run instead of latest code

- **Risk Level**: High (complete loss of user interaction)
- **Root Cause**: Unknown - code structure is valid, needs runtime debugging
- **Files Investigated**: 15 files
- **Event Handler Chain**: Complete and correct

---

## Key Findings

### 1. Event Handling Chain is Correctly Connected ✅

The event flow from winit to handlers is properly set up:

```
WindowEvent::MouseInput (winit)
  ↓
lifecycle.rs:99-100: window_event() receives event
  ↓
lifecycle.rs:100: self.on_mouse_input(event_loop, state, button)
  ↓
events/mouse/input.rs:5-19: on_mouse_input() method exists
  ↓
events/mouse/input.rs:14: dispatches to on_left_mouse_input()
  ↓
events/mouse/input.rs:82-99: on_left_mouse_input() routes by location
  ↓
Handlers: handle_tab_bar_left_click(), handle_terminal_left_click(), etc.
```

**Verification**: `cargo check` passes with no errors or warnings.

### 2. All Handler Methods Are Implemented ✅

| Handler Method | Location | Line | Visibility | Status |
|----------------|----------|------|------------|--------|
| `on_mouse_input` | `events/mouse/input.rs` | 5-19 | `pub(crate)` | ✅ Exists |
| `on_left_mouse_input` | `events/mouse/input.rs` | 82-99 | private | ✅ Exists |
| `on_right_mouse_input` | `events/mouse/input.rs` | 36-66 | private | ✅ Exists |
| `on_middle_mouse_input` | `events/mouse/input.rs` | 21-34 | private | ✅ Exists |
| `handle_tab_bar_left_click` | `events/mouse/tab_bar.rs` | 59-108 | `pub(in crate::gui::events::mouse)` | ✅ Exists |
| `handle_terminal_left_click` | `events/mouse/terminal_click.rs` | 22-110 | `pub(in crate::gui::events::mouse)` | ✅ Exists |
| `handle_context_menu_left_click` | `events/mouse/context_menu.rs` | 5-32 | `pub(in crate::gui::events::mouse)` | ✅ Exists |
| `on_cursor_moved` | `events/mouse/cursor.rs` | 5-78 | `pub(crate)` | ✅ Exists |
| `on_mouse_wheel` | `events/mouse/wheel.rs` | 4-43 | `pub(crate)` | ✅ Exists |

### 3. Module Declarations Are Correct ✅

```
src/gui/mod.rs
├── mod events;              ✅ Line 1
├── mod lifecycle;           ✅ Line 4
└── ...

src/gui/events/mod.rs
├── mod keyboard;            ✅ Line 1
├── mod mouse;               ✅ Line 2
├── mod pty;                 ✅ Line 3
└── mod redraw;              ✅ Line 4

src/gui/events/mouse/mod.rs
├── mod context_menu;        ✅ Line 1
├── mod cursor;              ✅ Line 2
├── mod input;               ✅ Line 3
├── mod tab_bar;             ✅ Line 4
├── mod terminal_click;      ✅ Line 5
└── mod wheel;               ✅ Line 6
```

**All submodules are properly declared and should be compiled.**

### 4. Helper Methods Are Available ✅

| Helper Method | Location | Visibility | Called From |
|---------------|----------|------------|-------------|
| `pixel_to_grid` | `interaction/geometry.rs:6` | `pub(in crate::gui)` | Multiple mouse handlers |
| `apply_pending_resize` | `events/redraw.rs:6` | `pub(in crate::gui)` | `on_mouse_input` |
| `is_mouse_reporting` | `interaction/mouse_reporting.rs:22` | `pub(in crate::gui)` | Terminal click handler |
| `tab_bar_hit_with_fallback` | `events/mouse/tab_bar.rs:35` | `pub(in crate::gui::events::mouse)` | Tab bar click handler |

---

## Possible Root Causes

### Hypothesis 1: Module Not Loaded (UNLIKELY)

**Evidence Against**:
- All `mod` declarations present in parent modules
- `cargo check` succeeds (would fail if modules not found)
- Test compilation shows this pattern works in Rust

**How to Verify**:
Add println! to `on_mouse_input` at `events/mouse/input.rs:5`:
```rust
pub(crate) fn on_mouse_input(...) {
    eprintln!("DEBUG: on_mouse_input called with button {:?}", button);
    self.apply_pending_resize();
    // ...
}
```

### Hypothesis 2: Build Cache / Binary Mismatch (MEDIUM LIKELIHOOD)

**Evidence For**:
- Git shows staged gui/mod.rs is empty (though working tree has content)
- User said "after refactoring" - might be running old binary

**How to Verify**:
```bash
cargo clean
cargo build
cargo run
```

### Hypothesis 3: Early Return or Condition Bug (HIGH LIKELIHOOD)

**Suspicious Code Patterns**:

1. **In `on_left_mouse_input` (events/mouse/input.rs:82)**:
   ```rust
   if self.handle_context_menu_left_click(event_loop, state, mx, my) {
       return;  // Early return if context menu handled
   }
   ```
   - If `handle_context_menu_left_click` always returns `true`, nothing else runs

2. **In `handle_context_menu_left_click` (events/mouse/context_menu.rs:11)**:
   ```rust
   let Some(menu) = self.context_menu.take() else {
       return false;  // Returns false if no menu, allows fall-through
   };
   ```
   - Logic looks correct

3. **In `on_left_mouse_input` (events/mouse/input.rs:89)**:
   ```rust
   if my < TAB_BAR_HEIGHT as f64 || self.is_window_close_button_with_fallback(mx, my) {
       self.handle_tab_bar_left_click(event_loop, state, mx, my);
       return;
   }
   ```
   - Condition might be wrong - what if TAB_BAR_HEIGHT is 0?

**How to Verify**:
Add debug prints to trace execution flow:
```rust
fn on_left_mouse_input(&mut self, event_loop: &ActiveEventLoop, state: ElementState) {
    let (mx, my) = self.mouse_pos;
    eprintln!("DEBUG: on_left_mouse_input at ({}, {})", mx, my);

    if self.handle_context_menu_left_click(event_loop, state, mx, my) {
        eprintln!("DEBUG: context menu handled, returning");
        return;
    }

    eprintln!("DEBUG: TAB_BAR_HEIGHT={}, my={}", TAB_BAR_HEIGHT, my);
    if my < TAB_BAR_HEIGHT as f64 || self.is_window_close_button_with_fallback(mx, my) {
        eprintln!("DEBUG: handling tab bar click");
        self.handle_tab_bar_left_click(event_loop, state, mx, my);
        return;
    }

    eprintln!("DEBUG: handling terminal click");
    self.handle_terminal_left_click(state, mx, my);
}
```

### Hypothesis 4: Mouse Position Not Updated (HIGH LIKELIHOOD)

**Critical Observation**: `on_left_mouse_input` uses `self.mouse_pos`, which is updated by `on_cursor_moved`.

**Potential Issue**: If `mouse_pos` is never updated (e.g., if `on_cursor_moved` isn't called), coordinates might be (0.0, 0.0).

**Check in lifecycle.rs:96**:
```rust
WindowEvent::CursorMoved { position, .. } => {
    self.on_cursor_moved(position);
}
```
This looks correct.

**How to Verify**:
```rust
pub(crate) fn on_cursor_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
    eprintln!("DEBUG: cursor moved to ({}, {})", position.x, position.y);
    self.mouse_pos = (position.x, position.y);
    // ...
}
```

---

## Recommended Debugging Steps

### Step 1: Verify Binary is Up-to-Date
```bash
cargo clean
cargo build --release
./target/release/ferrum
```

### Step 2: Add Debug Logging

Add to `src/gui/lifecycle.rs:99`:
```rust
WindowEvent::MouseInput { state, button, .. } => {
    eprintln!("DEBUG: MouseInput event: {:?} {:?}", state, button);
    self.on_mouse_input(event_loop, &event);
}
```

Add to `src/gui/events/mouse/input.rs:5`:
```rust
pub(crate) fn on_mouse_input(...) {
    eprintln!("DEBUG: on_mouse_input called, button={:?}, state={:?}, pos={:?}",
              button, state, self.mouse_pos);
    // ...
}
```

### Step 3: Check TAB_BAR_HEIGHT Value

Add to `src/gui/lifecycle.rs` after window creation:
```rust
eprintln!("DEBUG: TAB_BAR_HEIGHT={}, WINDOW_PADDING={}",
          TAB_BAR_HEIGHT, WINDOW_PADDING);
```

### Step 4: Verify Handler Methods Are Callable

Add test at end of `src/gui/events/mouse/input.rs`:
```rust
#[cfg(test)]
mod test_visibility {
    use super::*;

    #[test]
    fn methods_are_accessible() {
        // This test just needs to compile to prove visibility is correct
        fn check_app_has_methods(app: &mut App) {
            // These should all be accessible
            let _ = app.on_mouse_input;
            let _ = app.on_cursor_moved;
            let _ = app.on_mouse_wheel;
        }
    }
}
```

---

## Files to Investigate

### Primary Event Handling Files

```
src/gui/
├── lifecycle.rs                     # ApplicationHandler::window_event() - entry point
└── events/
    ├── mod.rs                       # INSPECT: Does this need pub use?
    ├── mouse/
    │   ├── mod.rs                   # INSPECT: Does this need pub use?
    │   ├── input.rs                 # Main mouse dispatcher
    │   ├── tab_bar.rs               # Tab and window chrome handling
    │   ├── terminal_click.rs        # Terminal text selection
    │   ├── context_menu.rs          # Right-click menu handling
    │   ├── cursor.rs                # Mouse movement tracking
    │   └── wheel.rs                 # Scroll handling
    └── redraw.rs                    # apply_pending_resize()
```

### Supporting Files

```
src/gui/
├── mod.rs                           # Module declarations - CHECK for pub use
├── interaction/
│   ├── geometry.rs                  # pixel_to_grid()
│   └── mouse_reporting.rs           # is_mouse_reporting()
└── renderer/
    └── tab_bar.rs                   # hit_test functions
```

---

## Critical Questions Needing Runtime Verification

| # | Question | How to Check | Priority |
|---|----------|--------------|----------|
| 1 | Is `on_mouse_input()` actually being called? | Add eprintln! at events/mouse/input.rs:5 | CRITICAL |
| 2 | Is `mouse_pos` being updated correctly? | Add eprintln! at events/mouse/cursor.rs:6 | CRITICAL |
| 3 | What is the value of `TAB_BAR_HEIGHT`? | Print at startup | HIGH |
| 4 | Are handlers returning early unexpectedly? | Add eprintln! before each return | HIGH |
| 5 | Is the correct binary being run? | Run `cargo clean && cargo build` | MEDIUM |
| 6 | Are there any silent panics/errors? | Run with `RUST_BACKTRACE=1` | MEDIUM |

---

## Architectural Notes

### Event Flow Architecture

The codebase uses a **layered event dispatch** pattern:

1. **Presentation Layer** (`lifecycle.rs`): Receives OS events from winit
2. **Dispatch Layer** (`events/mouse/input.rs`): Routes by input type (left/right/middle click)
3. **Handler Layer** (`events/mouse/*.rs`): Processes by UI region (tab bar, terminal, etc.)
4. **Interaction Layer** (`interaction/*.rs`): Low-level helpers (geometry, mouse mode detection)

This is a clean separation of concerns, but requires careful module visibility management.

### Module Visibility Pattern

The codebase uses Rust's restricted visibility extensively:
- `pub(crate)`: Visible to entire crate - used for main entry points
- `pub(in crate::gui)`: Visible to gui module and submodules - used for helpers
- `pub(in crate::gui::events::mouse)`: Visible only within mouse event handlers

**This pattern is correct and should work.** The fact that `cargo check` succeeds confirms this.

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| All affected files identified | ✅ | 15 files in event handling chain |
| Integration points mapped | ✅ | Full call chain traced |
| Similar patterns found | ⚠️ | No similar bugs found - unique issue |
| Test coverage analyzed | ⚠️ | No tests for event handling |
| Risks assessed | ✅ | Complete loss of interaction |
| Compilation verified | ✅ | No errors or warnings |
| Runtime behavior | ❌ | NOT VERIFIED - needs user testing |

**Limitations/Caveats**:
- Cannot reproduce issue in headless environment
- Code structure is valid - if bug exists, it's a runtime logic issue
- Recommend adding debug logging to trace execution flow
- Strong recommendation: Run `cargo clean && cargo build` first

---

## Recommended Next Steps for Developer

1. **Clean rebuild**: `cargo clean && cargo build`
2. **Add debug logging** to trace where execution stops
3. **Check if binary is actually being updated** (timestamps, etc.)
4. **Look for panic/error messages** in terminal output when clicking
5. **Verify `TAB_BAR_HEIGHT` constant** has reasonable value

If none of those reveal the issue, the problem may be:
- **A logic bug in the routing conditions** (lines 85, 89, 94, 98 of input.rs)
- **Mouse coordinates being wrong** (stuck at 0,0 or invalid)
- **An unrelated initialization issue** preventing the app from being interactive

---

## Conclusion

**The code structure is correct and should work.** All event handlers exist, are properly connected, and have appropriate visibility. The compilation succeeds with no warnings.

However, if the user reports mouse clicks genuinely don't work, this suggests one of:
1. **Runtime logic bug** - condition/early return preventing execution
2. **Build system issue** - old binary being run
3. **State initialization bug** - some flag preventing interaction
4. **User misunderstanding** - perhaps mouse clicks DO work but something else is broken

**The analysis cannot determine the root cause without runtime debugging.** I recommend adding extensive debug logging as shown above to trace execution flow.
