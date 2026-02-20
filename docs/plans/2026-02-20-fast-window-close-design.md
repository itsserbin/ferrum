# Fast Window Close & cmd.exe Flash Fix (Windows)

## Problem

Two issues on Windows when closing Ferrum with multiple split panes:

1. **Slow close**: Window hangs for seconds because `Session::drop()` runs `child.kill()` + `child.wait()` synchronously on the UI thread for each pane, and `has_active_child_processes_windows()` spawns PowerShell (~300ms per pane) to check for running processes.

2. **cmd.exe flash**: Separate cmd.exe console windows briefly appear when panes are killed, because conpty's hidden console windows become visible during `TerminateProcess`.

## Design

### 1. Background Thread Cleanup

Move PTY session cleanup off the UI thread.

**New method `PaneNode::drain_sessions()`**: Recursively walks the pane tree, calls `.take()` on each `leaf.session`, returns `Vec<Session>`.

**Modified `WindowRequest::CloseWindow` handling**:
- Before dropping the window, extract all sessions via `drain_sessions()` from each tab
- Spawn a background thread that runs `kill()` + `wait()` on all extracted sessions
- Then remove the window from the HashMap — Drop no longer blocks because sessions are already taken

**Simplified `Session::drop()`**: Keep `kill()` only (no `wait()`) as a safety net for edge cases where session wasn't properly extracted (panic, etc.). This prevents zombie processes while keeping Drop non-blocking.

### 2. Native Windows API for Process Detection

Replace PowerShell-based process checking with Win32 API.

**Add `windows-sys` as direct dependency** (already a transitive dep via arboard) with features: `Win32_System_Diagnostics_ToolHelp`, `Win32_Foundation`.

**Replace `has_active_child_processes_windows()`**:
- Use `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)` to snapshot the process table
- Iterate with `Process32First`/`Process32Next`
- Check if any process has `th32ParentProcessID == shell_pid`
- This is <1ms vs ~300ms for PowerShell

Unix path stays unchanged (pgrep is already fast).

### 3. cmd.exe Flash Mitigation

Two-layer approach:

**Layer 1 (automatic)**: Background cleanup from Section 1 means the Ferrum window is already gone when kill() happens, making cmd flash less visible.

**Layer 2 (graceful shutdown)**: On Windows, before `child.kill()`, attempt soft termination:
- Send `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, process_group_id)` via windows-sys
- Wait briefly (100ms) for process to exit
- Only call `child.kill()` (TerminateProcess) if the process is still alive
- Graceful exit doesn't trigger console window flash

## Files Changed

- `src/pty/mod.rs` — Session::drop simplification, graceful_shutdown_windows(), native process detection
- `src/gui/pane.rs` — drain_sessions() method on PaneNode
- `src/gui/lifecycle/window_requests.rs` — background thread cleanup on CloseWindow
- `Cargo.toml` — add windows-sys dependency (Windows only)

## Testing

- Unit test: drain_sessions() empties all sessions from pane tree
- Unit test: native Windows process detection matches PowerShell result
- Manual test: close window with multiple split panes — should be instant, no cmd flash
