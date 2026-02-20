# Fast Window Close Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix slow window close and cmd.exe flashing on Windows when closing Ferrum with multiple split panes.

**Architecture:** Extract PTY sessions from pane trees before dropping windows, move kill+wait to a background thread, replace PowerShell-based process detection with native Win32 API via `windows-sys`, and attempt graceful shutdown before hard kill on Windows.

**Tech Stack:** Rust, windows-sys crate (Win32 ToolHelp API), portable-pty 0.9.0, std::thread

---

### Task 1: Add `drain_sessions()` to PaneNode

Extract all PTY sessions from a pane tree without dropping the tree. This is the foundation for background cleanup.

**Files:**
- Modify: `src/gui/pane.rs` — add `drain_sessions()` method on `PaneNode` (after `leaf_ids()` at ~line 190)
- Test: inline `#[cfg(test)]` in `src/gui/pane.rs`

**Step 1: Write the failing test**

Add to the existing `mod tests` block at the bottom of `src/gui/pane.rs` (after the last test ~line 911):

```rust
#[test]
fn drain_sessions_empties_all() {
    let mut tree = PaneNode::new_leaf(1);
    tree.split(1, SplitDirection::Horizontal, 2);
    tree.split(2, SplitDirection::Vertical, 3);

    // All test leaves have session = None, but drain should still work.
    let sessions = tree.drain_sessions();
    assert_eq!(sessions.len(), 3);

    // After draining, all leaves should have session = None.
    for id in tree.leaf_ids() {
        let leaf = tree.find_leaf(id).unwrap();
        assert!(leaf.session.is_none());
    }
}

#[test]
fn drain_sessions_single_leaf() {
    let mut tree = PaneNode::new_leaf(1);
    let sessions = tree.drain_sessions();
    assert_eq!(sessions.len(), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test drain_sessions -- --nocapture`
Expected: FAIL — `drain_sessions` method not found.

**Step 3: Write minimal implementation**

Add to `impl PaneNode` block in `src/gui/pane.rs`, after the `leaf_ids()` method (~line 190):

```rust
/// Extracts all PTY sessions from the pane tree via `.take()`.
///
/// Returns a `Vec<Option<pty::Session>>` — one entry per leaf.
/// After this call every leaf's `session` field is `None`, so
/// dropping the tree will no longer block on `Session::drop()`.
pub(super) fn drain_sessions(&mut self) -> Vec<Option<pty::Session>> {
    match self {
        PaneNode::Leaf(leaf) => vec![leaf.session.take()],
        PaneNode::Split(split) => {
            let mut sessions = split.first.drain_sessions();
            sessions.extend(split.second.drain_sessions());
            sessions
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test drain_sessions -- --nocapture`
Expected: PASS (both tests).

**Step 5: Commit**

```bash
git add src/gui/pane.rs
git commit -m "feat: add drain_sessions() to PaneNode for background cleanup"
```

---

### Task 2: Simplify Session::drop() — kill-only, no wait

Remove the blocking `child.wait()` from Drop so that if sessions are dropped on the UI thread (edge case), it doesn't block.

**Files:**
- Modify: `src/pty/mod.rs:255-267` — simplify Drop impl

**Step 1: Modify Session::drop()**

Replace the current `Drop` impl at `src/pty/mod.rs:255-267` with:

```rust
impl Drop for Session {
    fn drop(&mut self) {
        // Kill only — no wait(). This is a safety net for sessions that
        // weren't extracted for background cleanup (e.g. during a panic).
        // Blocking wait() was removed to prevent UI thread hangs.
        if let Err(e) = self.child.kill() {
            if e.kind() != std::io::ErrorKind::InvalidInput {
                eprintln!("Failed to kill PTY child process: {}", e);
            }
        }
    }
}
```

**Step 2: Run all tests**

Run: `cargo test`
Expected: All existing tests pass. No test exercises `wait()` directly.

**Step 3: Commit**

```bash
git add src/pty/mod.rs
git commit -m "fix: remove blocking wait() from Session::drop to prevent UI hangs"
```

---

### Task 3: Add `Session::shutdown()` for explicit background cleanup

Add a method that does the full kill+wait sequence, meant to be called from a background thread.

**Files:**
- Modify: `src/pty/mod.rs` — add `shutdown()` method on Session (after `process_id()` at ~line 171)

**Step 1: Add shutdown() method**

Add after the `process_id()` method in `impl Session` at `src/pty/mod.rs:171`:

```rust
/// Performs a full graceful shutdown: kill the child process and wait
/// for it to exit. Meant to be called from a background thread to
/// avoid blocking the UI.
pub fn shutdown(mut self) {
    if let Err(e) = self.child.kill() {
        if e.kind() != std::io::ErrorKind::InvalidInput {
            eprintln!("Failed to kill PTY child process: {}", e);
        }
    }
    if let Err(e) = self.child.wait() {
        if e.kind() != std::io::ErrorKind::InvalidInput {
            eprintln!("Failed to wait on PTY child process: {}", e);
        }
    }
    // Prevent Drop from running kill() again — consume self.
    // Actually, Drop will still run but kill() on an already-killed
    // process returns InvalidInput which is silently ignored.
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: PASS — no behavior change, just a new public method.

**Step 3: Commit**

```bash
git add src/pty/mod.rs
git commit -m "feat: add Session::shutdown() for explicit background cleanup"
```

---

### Task 4: Background thread cleanup on window close

Modify `WindowRequest::CloseWindow` handling to extract sessions before dropping the window, then clean them up on a background thread.

**Files:**
- Modify: `src/gui/lifecycle/window_requests.rs:62-72` — drain sessions + spawn background thread
- Reference: `src/gui/pane.rs` (drain_sessions), `src/pty/mod.rs` (Session::shutdown)

**Step 1: Modify CloseWindow handling**

In `src/gui/lifecycle/window_requests.rs`, replace the `WindowRequest::CloseWindow` arm (lines 62-72) with:

```rust
WindowRequest::CloseWindow => {
    // Extract all PTY sessions before dropping the window
    // so that Session::drop() doesn't block the UI thread.
    let sessions: Vec<crate::pty::Session> = if let Some(win) = self.windows.get_mut(&window_id) {
        win.tabs
            .iter_mut()
            .flat_map(|tab| tab.pane_tree.drain_sessions())
            .flatten() // Option<Session> → Session
            .collect()
    } else {
        Vec::new()
    };

    // Spawn background thread for cleanup (kill + wait).
    if !sessions.is_empty() {
        std::thread::Builder::new()
            .name("pty-cleanup".into())
            .spawn(move || {
                for session in sessions {
                    session.shutdown();
                }
            })
            .ok();
    }

    // Now drop the window — sessions are already extracted,
    // so Drop won't block.
    #[cfg(target_os = "macos")]
    {
        if let Some(win) = self.windows.remove(&window_id) {
            platform::macos::remove_toolbar_item(&win.window);
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        self.windows.remove(&window_id);
    }
}
```

Note: `drain_sessions()` is `pub(super)` scoped to `gui`, but `window_requests.rs` is inside `gui::lifecycle` which is inside `gui`, so it has access. The `flatten()` call converts `Vec<Option<Session>>` to an iterator of `Session` values, skipping `None`s.

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

**Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add src/gui/lifecycle/window_requests.rs
git commit -m "feat: background thread PTY cleanup on window close"
```

---

### Task 5: Add `windows-sys` dependency for native process detection

**Files:**
- Modify: `Cargo.toml` — add windows-sys as a Windows-only dependency

**Step 1: Add dependency**

Add after the `[target.'cfg(target_os = "linux")'.dependencies]` block in `Cargo.toml` (after line 40). There should already be a `[target.'cfg(target_os = "macos")'.dependencies]` section — add the Windows section before it:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { version = "0.59", features = [
    "Win32_System_Diagnostics_ToolHelp",
    "Win32_Foundation",
] }
```

Note: Use version 0.59 which is stable and widely compatible. The exact version may need adjustment based on what's already in Cargo.lock as a transitive dep — check `cargo tree -p windows-sys` after adding.

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles (windows-sys features are only used on Windows, but `cargo check` should resolve the dependency on any platform).

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add windows-sys for native process detection on Windows"
```

---

### Task 6: Replace PowerShell with native Windows API for process detection

Replace `has_active_child_processes_windows()` which spawns PowerShell (~300ms) with `CreateToolhelp32Snapshot` (<1ms).

**Files:**
- Modify: `src/pty/mod.rs:229-253` — rewrite `has_active_child_processes_windows()`

**Step 1: Replace the Windows implementation**

Replace the entire `has_active_child_processes_windows` function at `src/pty/mod.rs:229-253` with:

```rust
#[cfg(windows)]
fn has_active_child_processes_windows(shell_pid: u32) -> bool {
    use std::mem;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
        TH32CS_SNAPPROCESS,
    };

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == -1_isize as _ {
            return false;
        }

        let mut entry: PROCESSENTRY32 = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32>() as u32;

        if Process32First(snapshot, &mut entry) == 0 {
            CloseHandle(snapshot);
            return false;
        }

        loop {
            if entry.th32ParentProcessID == shell_pid {
                CloseHandle(snapshot);
                return true;
            }
            if Process32Next(snapshot, &mut entry) == 0 {
                break;
            }
        }

        CloseHandle(snapshot);
        false
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles. On non-Windows, the `#[cfg(windows)]` block is excluded.

**Step 3: Commit**

```bash
git add src/pty/mod.rs
git commit -m "perf: replace PowerShell with native Win32 API for process detection"
```

---

### Task 7: Graceful shutdown on Windows to prevent cmd.exe flash

Before hard-killing the child process, attempt a graceful CTRL_BREAK signal which lets cmd.exe exit without flashing its console window.

**Files:**
- Modify: `src/pty/mod.rs` — add graceful shutdown logic to `Session::shutdown()`
- Modify: `Cargo.toml` — add `Win32_System_Console` feature to windows-sys

**Step 1: Update windows-sys features in Cargo.toml**

In `Cargo.toml`, add `Win32_System_Console` and `Win32_System_Threading` to the windows-sys features:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { version = "0.59", features = [
    "Win32_System_Diagnostics_ToolHelp",
    "Win32_System_Console",
    "Win32_System_Threading",
    "Win32_Foundation",
] }
```

**Step 2: Rewrite `Session::shutdown()` with graceful Windows path**

Replace the `shutdown()` method with platform-aware logic:

```rust
/// Performs a full graceful shutdown: kill the child process and wait
/// for it to exit. Meant to be called from a background thread.
pub fn shutdown(mut self) {
    #[cfg(windows)]
    self.shutdown_windows();

    #[cfg(not(windows))]
    self.shutdown_unix();
}

#[cfg(not(windows))]
fn shutdown_unix(&mut self) {
    if let Err(e) = self.child.kill() {
        if e.kind() != std::io::ErrorKind::InvalidInput {
            eprintln!("Failed to kill PTY child process: {}", e);
        }
    }
    if let Err(e) = self.child.wait() {
        if e.kind() != std::io::ErrorKind::InvalidInput {
            eprintln!("Failed to wait on PTY child process: {}", e);
        }
    }
}

#[cfg(windows)]
fn shutdown_windows(&mut self) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Console::{GenerateConsoleCtrlEvent, CTRL_BREAK_EVENT};
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};

    // Try graceful shutdown first: send CTRL_BREAK to the process group.
    // This lets cmd.exe exit cleanly without flashing its console window.
    if let Some(pid) = self.child.process_id() {
        unsafe {
            // Send CTRL_BREAK to the process group.
            let _ = GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid);

            // Wait up to 150ms for the process to exit gracefully.
            let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            if handle != 0 {
                WaitForSingleObject(handle, 150);
                CloseHandle(handle);
            }
        }
    }

    // Hard kill if still alive (kill() handles the "already dead" case).
    if let Err(e) = self.child.kill() {
        if e.kind() != std::io::ErrorKind::InvalidInput {
            eprintln!("Failed to kill PTY child process: {}", e);
        }
    }
    if let Err(e) = self.child.wait() {
        if e.kind() != std::io::ErrorKind::InvalidInput {
            eprintln!("Failed to wait on PTY child process: {}", e);
        }
    }
}
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles. On non-Windows, only `shutdown_unix` is compiled.

**Step 4: Run all tests**

Run: `cargo test`
Expected: All tests pass.

**Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/pty/mod.rs
git commit -m "fix: graceful CTRL_BREAK shutdown on Windows to prevent cmd.exe flash"
```

---

### Task 8: Final integration test and cleanup

Verify everything works together, run all tests, make a final commit.

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

**Step 2: Build release to check no warnings**

Run: `cargo build --release 2>&1 | head -30`
Expected: Clean build, no warnings related to our changes.

**Step 3: Verify the cleanup flow manually (read-through)**

Review the complete close flow:
1. `request_close_window()` → `has_active_child_processes()` (now native API on Windows, <1ms)
2. Dialog shown if needed → user confirms
3. `WindowRequest::CloseWindow` → drain all sessions → spawn cleanup thread → drop window
4. Background thread: `shutdown()` → graceful CTRL_BREAK (Windows) → kill → wait
5. `Session::drop()` on UI thread: only kill() — non-blocking safety net

**Step 4: Final commit (if any cleanup needed)**

```bash
git add -A
git commit -m "chore: final cleanup for fast window close feature"
```
