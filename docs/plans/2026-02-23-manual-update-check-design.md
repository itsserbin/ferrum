# Manual Update Check Button — Design

**Goal:** "Check for Updates" button in Settings → Updates tab on all 3 platforms.

## UI

```
Current version: 0.3.2
☑ Auto-check for updates

[Check for Updates]   Checking…
                      You're up to date
                      — або —
                      Update v0.4.0 available   [Install]
```

Status persists until Settings is closed. Empty on open.

## Architecture

**`update.rs`** — add `ManualCheckResult` enum + `spawn_manual_check(tx)` that skips the 24h cache.

**Per-platform Settings window** — button triggers manual check; result shown inline:
- macOS: AtomicUsize counter (same pattern as pin/gear button) + Mutex result state, polled in `about_to_wait`
- Windows: WM_COMMAND on click; thread posts WM_USER back to hwnd with result
- Linux: `connect_clicked` spawns thread; `glib::MainContext::default().spawn_local` updates GTK widgets

**Install button** appears only when `Found`. Calls `crate::update_installer::spawn_installer(&tag_name)`.

## Data Flow

1. User clicks "Check for Updates"
2. Platform signals main loop (atomic / WM_COMMAND / GTK click)
3. Main loop spawns `spawn_manual_check(tx)`
4. Thread runs `fetch_latest_release()` bypassing cache, sends `ManualCheckResult` to `tx`
5. Result stored in `Mutex<Option<ManualCheckResult>>`; atomic flag set
6. Next poll cycle: reads result, updates status label + shows/hides Install button
