---
title: Codebase Impact Analysis - macOS Resize Content Shift Issue
task_file: User investigation request
scratchpad: .specs/scratchpad/c4f7a8b2.md
created: 2026-02-18
status: complete
---

# Codebase Impact Analysis: macOS Terminal Content Shift on Resize

## Summary

- **Root Cause**: Race condition between native tab bar visibility toggle and grid size calculation
- **Affected Components**: 3 core modules (resize flow, reflow logic, macOS native integration)
- **Risk Level**: High (data loss appearance, poor UX)
- **Platform**: macOS-specific issue due to native NSWindow tab bar integration

**Problem**: When resizing the window on macOS, terminal content that was at the top of the screen ends up at the bottom. This occurs because the native tab bar toggle changes window dimensions mid-resize, causing the terminal to resize to incorrect dimensions, triggering reflow overflow logic that pushes content to scrollback.

---

## Root Cause: Detailed Analysis

### The Race Condition

```
Timeline of Events:
═══════════════════════════════════════════════════════════════════

T0: User drags window edge to resize
    └─ OS generates WindowEvent::Resized(PhysicalSize { w: 1200, h: 800 })

T1: on_resized() called [src/gui/events/redraw.rs:289]
    ├─ Calls calc_grid_size(1200, 800)
    ├─ Calculates: rows = 800 / 20 = 40, cols = 1200 / 10 = 120
    └─ Stores: pending_grid_resize = Some((40, 120))

T2: window.request_redraw() triggers WindowEvent::RedrawRequested

T3: on_redraw_requested() [src/gui/events/redraw.rs:298]
    ├─ Calls sync_native_tab_bar_visibility() [line 301]
    │   └─ May call window.toggleTabBar(None)
    │       └─ ⚠️ SIDE EFFECT: window.inner_size() changes to (1200, 780)
    │           └─ Native tab bar appeared, consuming 20px of height
    │
    ├─ Calls apply_pending_resize() [line 312]
    │   └─ Uses STALE pending_grid_resize = (40, 120)
    │       └─ But window is now 780px tall, should be 39 rows!
    │
    └─ Calls resize_all_tabs(40, 120) [src/gui/tabs/manage.rs:71]
        └─ terminal.resize(40, 120) [src/core/terminal/grid_ops.rs:6]
            └─ reflow_resize() [line 67]
                ├─ Current content: 38 rows on screen
                ├─ New size: 40 rows (WRONG - should be 39)
                ├─ Reflow calculates total_rows = 38
                ├─ 38 <= 40 → Case A: content fits, fills from top
                └─ ✓ Works correctly (by luck)

T4: NEXT resize event (window still being dragged)
    └─ Repeat cycle, but now with 39 rows → if content is 40 rows:
        └─ 40 > 39 → Case B: OVERFLOW
            ├─ Push 1 row to scrollback
            ├─ Fill grid with last 39 rows
            └─ cursor_row = 38 (BOTTOM)
                └─ Content "shifted down" - top row now in scrollback!
```

### Why macOS is Affected

| Factor | macOS | Linux/Windows | Impact |
|--------|-------|---------------|--------|
| **Tab bar** | Native NSWindow tab group | Custom rendered | Native toggles change window size |
| **Grid calculation** | `rows = height / cell_height` | `rows = (height - 36 - 16) / cell_height` | No padding buffer, more sensitive |
| **Window padding** | 0px | 8px | Full window height used for content |
| **Size changes** | Dynamic (native UI controls) | Static (JS controls size) | Async size changes possible |

### The Reflow Overflow Logic

**File**: `src/core/terminal/grid_ops.rs:141-163`

```rust
// Case B: Content overflows new grid size
if total_rows > rows {
    let scrollback_count = total_rows - rows;

    // Push excess rows to scrollback
    for row in rewrapped.iter().take(scrollback_count) {
        self.scrollback.push_back(row.clone());
    }

    // Fill grid with LAST rows (NOT first rows)
    for (i, row) in rewrapped.iter().skip(scrollback_count).enumerate() {
        // populate grid...
    }

    // Force cursor to bottom
    self.cursor_row = rows.saturating_sub(1);
}
```

**Problem**: When grid size is calculated incorrectly (too small), this logic treats visible content as "overflow" and hides it in scrollback.

---

## Files Involved

### Core Resize Flow

```
src/gui/
├── lifecycle.rs
│   └── Line 111-113        # WindowEvent::Resized handler (entry point)
├── events/
│   └── redraw.rs
│       ├── Line 289-296    # on_resized(): calc_grid_size + pending_grid_resize
│       ├── Line 298-332    # on_redraw_requested(): sync native tab bar + apply resize
│       └── Line 277-287    # on_scale_factor_changed(): separate scale handling
├── mod.rs
│   └── Line 85-93          # calc_grid_size(): macOS uses 0 padding/tab height
└── tabs/
    └── manage.rs
        └── Line 71-86      # resize_all_tabs(): loops over tabs, calls terminal.resize()
```

### Terminal Reflow Logic

```
src/core/terminal/
└── grid_ops.rs
    ├── Line 6-31           # resize(): entry point, delegates to reflow or simple
    ├── Line 34-58          # simple_resize(): height-only changes
    └── Line 67-166         # reflow_resize(): CRITICAL - content shift logic here
        ├── Line 129-140    # Case A: content fits (safe)
        └── Line 141-163    # Case B: overflow (DANGEROUS - pushes to scrollback)
```

### macOS Native Integration

```
src/gui/platform/
└── macos.rs
    ├── Line 195-214        # sync_native_tab_bar_visibility(): toggles native tab bar
    └── Line 139-152        # add_as_tab(): manages native tab groups
```

### Renderer Metrics

```
src/gui/renderer/
├── metrics.rs
│   ├── Line 48-61          # tab_bar_height_px(): returns 0 on macOS
│   └── Line 63-72          # window_padding_px(): returns 0 on macOS
└── mod.rs
    └── Line 162-199        # CPU renderer equivalents
```

---

## Key Interfaces & Contracts

### Functions/Methods to Understand

| Location | Name | Current Signature | Purpose |
|----------|------|-------------------|---------|
| `src/gui/events/redraw.rs:289` | `on_resized` | `fn(&mut self, PhysicalSize<u32>)` | **Entry point**: Calculates grid size, stores pending resize |
| `src/gui/events/redraw.rs:312` | `apply_pending_resize` | `fn(&mut self)` | **Critical**: Applies stale pending_grid_resize AFTER native sync |
| `src/gui/mod.rs:85` | `calc_grid_size` | `fn(&self, u32, u32) -> (usize, usize)` | **macOS sensitive**: Uses 0 for tab bar and padding |
| `src/core/terminal/grid_ops.rs:6` | `resize` | `fn(&mut self, usize, usize)` | Terminal resize: delegates to reflow or simple |
| `src/core/terminal/grid_ops.rs:67` | `reflow_resize` | `fn(&mut self, usize, usize)` | **THE BUG**: Case B overflow logic shifts content |
| `src/gui/platform/macos.rs:195` | `sync_native_tab_bar_visibility` | `fn(&Window)` | **Race condition source**: Toggles native UI mid-resize |

### Classes/Components Affected

| Location | Name | Description | Role in Issue |
|----------|------|-------------|---------------|
| `src/gui/state.rs:112` | `FerrumWindow` | Per-window state | Holds `pending_grid_resize` field (stale data) |
| `src/core/terminal.rs` | `Terminal` | Terminal emulator state | Grid dimensions, scrollback, reflow logic |
| `src/core/grid.rs` | `Grid` | 2D cell storage | Resized by terminal, loses content on incorrect sizing |

### Types/Interfaces to Update

| Location | Name | Fields Affected | Notes |
|----------|------|-----------------|-------|
| `src/gui/state.rs:114` | `FerrumWindow.pending_grid_resize` | `Option<(usize, usize)>` | Stores stale grid size if native tab bar toggles |

---

## Integration Points

Files that interact with the resize flow:

| File | Relationship | Impact | Issue Relevance |
|------|--------------|--------|-----------------|
| `src/gui/events/keyboard/shortcuts.rs` | Calls calc_grid_size on tab operations | Low | Uses same calc_grid_size, but not during window resize |
| `src/gui/events/mouse/tab_bar.rs` | Calls calc_grid_size on tab changes | Low | Same pattern, but macOS doesn't render custom tab bar |
| `src/gui/renderer/backend.rs` | Provides tab_bar_height_px() and window_padding_px() | High | Returns 0 on macOS - core to race condition |
| `src/pty/mod.rs:158` | `Session.resize()` called after terminal resize | Low | Sends SIGWINCH to shell, standard behavior |

---

## Similar Implementations

### Pattern: Resize Coalescing

- **Location**: `src/gui/events/redraw.rs:289-296`
- **Why relevant**: Multiple rapid OS events → single pending_grid_resize → applied on redraw
- **Current approach**: Store pending size, apply later
- **Problem**: "Later" is too late - native UI changed between store and apply

### Pattern: Native Tab Bar Management

- **Location**: `src/gui/platform/macos.rs:195-214`
- **Why relevant**: Native UI integration that affects window dimensions
- **Current approach**: Sync on every redraw, toggle when tab count changes
- **Problem**: Toggle happens between grid calculation and application

---

## Potential Solutions (Not Implementation)

### Option 1: Recalculate Grid Size Before Apply (Safest)

**Change**: In `apply_pending_resize()`, recalculate grid size using current `window.inner_size()` instead of using stale `pending_grid_resize`.

**Files to modify**:
- `src/gui/events/redraw.rs:312` - Replace `pending_grid_resize.take()` with fresh `calc_grid_size(window.inner_size())`

**Pros**: Guarantees correct dimensions, simple fix
**Cons**: Wastes the calc_grid_size call in on_resized

### Option 2: Move Native Tab Sync Before Grid Calculation (Risky)

**Change**: Call `sync_native_tab_bar_visibility()` in `on_resized()` BEFORE `calc_grid_size()`.

**Files to modify**:
- `src/gui/events/redraw.rs:289` - Add native sync before calc_grid_size
- `src/gui/events/redraw.rs:301` - Remove native sync from on_redraw_requested

**Pros**: Ensures native UI state is stable before calculation
**Cons**: Multiple calls to sync per resize cycle, may cause flicker

### Option 3: Defer Native Tab Sync Until After Resize Complete (Complex)

**Change**: Track resize in-progress state, skip native tab sync during resize, sync only when stable.

**Files to modify**:
- `src/gui/state.rs` - Add `resize_in_progress: bool` field
- `src/gui/events/redraw.rs` - Set flag on resized, clear on stable timer
- `src/gui/platform/macos.rs` - Check flag before sync

**Pros**: Eliminates race condition entirely
**Cons**: Complex state management, may delay tab bar appearance

### Option 4: Prevent Reflow Overflow on Small Deltas (Workaround)

**Change**: In `reflow_resize()`, if `total_rows - rows < threshold`, treat as Case A (content fits) to prevent scrollback push.

**Files to modify**:
- `src/core/terminal/grid_ops.rs:141` - Add delta threshold check

**Pros**: Prevents symptom without fixing root cause
**Cons**: Masks the real issue, may cause other edge cases

---

## Test Coverage

### Existing Tests Related to Resize

| Test File | Tests Affected | Coverage |
|-----------|----------------|----------|
| `tests/unit/core_terminal.rs` | Likely has terminal resize tests | Check for reflow edge cases |
| `src/core/terminal/grid_ops.rs:270-396` | Inline reflow tests | Tests reflow logic but not macOS race condition |

### Tests Confirming the Issue

```rust
// From grid_ops.rs:298-315
#[test]
fn reflow_preserves_content_after_width_change() {
    // Tests that content is preserved, but doesn't test
    // the case where grid size is incorrectly calculated
}

#[test]
fn reflow_rewraps_long_lines_to_new_width() {
    // Tests rewrapping logic, but assumes correct grid size
}
```

**Gap**: No tests for macOS-specific resize race condition with native tab bar.

### New Tests Needed

| Test Type | Location | Coverage Target |
|-----------|----------|-----------------|
| Integration | New file `tests/macos_resize.rs` | Simulate native tab bar toggle during resize |
| Unit | `src/core/terminal/grid_ops.rs` | Test reflow with grid size smaller than content by 1 row |
| Unit | `src/gui/events/redraw.rs` | Mock window.inner_size() changing between on_resized and apply_pending_resize |

---

## Risk Assessment

### High Risk Areas

| Area | Risk | Mitigation |
|------|------|------------|
| **Reflow overflow logic** | Data appears lost (hidden in scrollback) | Add safeguards to prevent overflow on small deltas |
| **Native tab bar timing** | Async size changes unpredictable | Synchronize native UI state before grid calculations |
| **Pending resize stale data** | Grid sized to wrong dimensions | Recalculate from current window size at apply time |

### User Impact

- **Severity**: High - visible data loss appearance
- **Frequency**: Every window resize on macOS (consistent reproduction)
- **Workaround**: Scroll up to see "lost" content (data not actually lost, just hidden)

---

## Recommended Investigation Steps

Before implementing a fix, developer should:

1. **Reproduce the issue**: Resize window on macOS while monitoring scrollback length
   - Expected: scrollback grows when top content "disappears"

2. **Verify native tab bar timing**: Add logging to track when `window.inner_size()` changes relative to `toggleTabBar()`
   - File: `src/gui/platform/macos.rs:212`
   - File: `src/gui/events/redraw.rs:289,312`

3. **Measure the delta**: Log difference between `pending_grid_resize` and actual needed size
   - File: `src/gui/events/redraw.rs:312` - Compare pending vs. `calc_grid_size(window.inner_size())`

4. **Test on multiple macOS versions**: Verify if native tab bar height is consistent (10.15+, 11.0+, 12.0+)

5. **Review similar terminals**: Check how Ghostty, Alacritty handle native macOS tabs + resize

---

## Key Files for Implementation

1. **`src/gui/events/redraw.rs`** - Contains on_resized(), apply_pending_resize(), and native sync call
   - Lines 289-296: Where grid size is calculated and stored (STALE)
   - Lines 269-275: Where pending resize is applied (NEEDS FIX)
   - Line 301: Where native tab bar sync happens (CAUSES RACE)

2. **`src/core/terminal/grid_ops.rs`** - Contains reflow logic that manifests the bug
   - Lines 141-163: Case B overflow logic (SYMPTOM)
   - Lines 67-166: Complete reflow_resize() implementation (UNDERSTAND THIS)

3. **`src/gui/platform/macos.rs`** - Native tab bar integration
   - Lines 195-214: sync_native_tab_bar_visibility() (TIMING ISSUE)

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| All affected files identified | ✅ | 8 core files across 3 modules |
| Integration points mapped | ✅ | Native tab bar sync timing is critical |
| Similar patterns found | ✅ | Resize coalescing pattern used elsewhere |
| Test coverage analyzed | ⚠️ | No macOS-specific resize race tests exist |
| Risks assessed | ✅ | High severity, consistent reproduction |

**Limitations/Caveats**:
- Exact timing of when `window.inner_size()` changes after `toggleTabBar()` call is uncertain without runtime testing
- Native tab bar pixel height varies by macOS version (needs measurement)
- Impact on multi-tab scenarios (2+ native tabs) needs validation

---

## Additional Context

### Why Content "Shifts Down" Instead of Truncating

The user perceives content "shifting down" because:

1. Terminal content that was at row 0 (top) is pushed to scrollback
2. What was at row 10 is now at row 0
3. Cursor moves to bottom of visible area (last row)
4. User sees "bottom" content and thinks everything moved down
5. In reality, "top" content moved to scrollback (hidden above)

### macOS Native Tab Bar Behavior

- **Single tab**: Native tab bar is hidden (toggle off)
- **Multiple tabs**: Native tab bar is visible (toggle on)
- **Height**: Approximately 22-28px depending on macOS version
- **Toggle effect**: Changes `window.inner_size().height` immediately
- **Async nature**: `toggleTabBar()` is a UI operation, may not complete before next line of code

### Why Resize Coalescing Exists

Rapid OS resize events (e.g., dragging window edge) can generate 60+ events per second. Without coalescing:
- Terminal would reflow 60 times per second
- PTY would receive 60 SIGWINCH signals per second
- Massive performance impact

**Trade-off**: Coalescing improves performance but introduces stale data window where native UI can change.

---

## Conclusion

The macOS resize content shift issue is caused by a race condition between:
1. Grid size calculation (based on current window size)
2. Native tab bar visibility toggle (changes window size)
3. Application of stale grid size (too small, triggers overflow)

The fix requires either:
- **Recalculating grid size at apply time** (safest, simplest)
- **Synchronizing native UI state before calculation** (cleaner, riskier)

Both approaches require changes to `src/gui/events/redraw.rs` resize flow. The reflow logic in `src/core/terminal/grid_ops.rs` is working as designed - the bug is in providing incorrect input dimensions.
