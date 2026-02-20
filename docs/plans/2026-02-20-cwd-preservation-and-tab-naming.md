# CWD Preservation & Dynamic Tab Naming — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Preserve working directory when opening new tabs/windows/panes on all platforms, and show CWD as dynamic tab title with smart truncation.

**Architecture:** Three layers — (1) shell integration scripts for cmd.exe/PowerShell emit OSC 7, (2) OS API fallback queries CWD from process PID when shell doesn't emit OSC 7, (3) tab title rendering shows CWD with adaptive truncation. OSC 7 remains the primary mechanism; OS API is the fallback with 1-second polling for tabs without shell integration.

**Tech Stack:** Rust, platform-specific APIs (Linux `/proc`, macOS `libproc` FFI, Windows `NtQueryInformationProcess` via `windows-sys`), cmd.exe `PROMPT`, PowerShell `prompt` function.

---

### Task 1: cmd.exe OSC 7 Shell Integration

**Files:**
- Modify: `src/pty/windows-aliases/init.cmd`

**Step 1: Add OSC 7 PROMPT to init.cmd**

In `src/pty/windows-aliases/init.cmd`, add a PROMPT line after `chcp 65001 >nul` that emits OSC 7 with the current directory before the standard prompt:

```cmd
@echo off
chcp 65001 >nul
REM OSC 7: report CWD to terminal on every prompt
prompt $e]7;file://%COMPUTERNAME%/$p$e\$p$g
REM Unix-style command aliases for Windows
...rest of file...
```

Notes:
- `$e` = ESC character in cmd.exe PROMPT
- `$p` = current drive and path
- `$e\` = ST (string terminator) — ESC followed by backslash
- `%COMPUTERNAME%` = hostname
- `$p$g` = standard prompt (`C:\path>`)
- This runs on every prompt display, including after `cd`

**Step 2: Verify no build errors**

Run: `cargo build --no-default-features 2>&1 | head -5`
Expected: successful compilation (init.cmd is embedded via `include_str!`)

**Step 3: Commit**

```bash
git add src/pty/windows-aliases/init.cmd
git commit -m "feat: emit OSC 7 CWD from cmd.exe via PROMPT"
```

---

### Task 2: PowerShell OSC 7 Shell Integration

**Files:**
- Create: `src/pty/shell-integration/powershell/ferrum.ps1`
- Modify: `src/pty/mod.rs`

**Step 1: Create PowerShell integration script**

Create `src/pty/shell-integration/powershell/ferrum.ps1`:

```powershell
if ($env:FERRUM_SHELL_INTEGRATION -ne "1") { return }

# Save user's existing prompt function (if any) and chain it
$__ferrum_original_prompt = if (Test-Path Function:\prompt) { Get-Content Function:\prompt } else { $null }

function prompt {
    # Emit OSC 7 with current directory (forward slashes for URI)
    $path = $PWD.Path -replace '\\', '/'
    $host_name = [System.Net.Dns]::GetHostName()
    [Console]::Write("`e]7;file://$host_name/$path`e\")
    # Standard PS prompt
    "PS $($PWD.Path)> "
}
```

**Step 2: Embed script and add PowerShell detection to `src/pty/mod.rs`**

After the existing `SHELL_INTEGRATION_FISH` constant (line 38), add:

```rust
const SHELL_INTEGRATION_POWERSHELL: &str =
    include_str!("shell-integration/powershell/ferrum.ps1");
```

In `setup_shell_integration()` (after line 105 where fish dir is created), add:

```rust
let ps_dir = temp_dir.join("powershell");
std::fs::create_dir_all(&ps_dir).ok()?;
std::fs::write(ps_dir.join("ferrum.ps1"), SHELL_INTEGRATION_POWERSHELL).ok()?;
```

In the `match shell_name` block inside `Session::spawn()` (after the `"fish"` arm at line 234), add a PowerShell arm:

```rust
name if name == "powershell" || name == "pwsh" || name == "powershell.exe" || name == "pwsh.exe" => {
    let ps_script = integration_dir.join("powershell").join("ferrum.ps1");
    cmd.arg("-NoExit");
    cmd.arg("-File");
    cmd.arg(ps_script.to_string_lossy().as_ref());
}
```

**Step 3: Verify compilation**

Run: `cargo build --no-default-features 2>&1 | head -5`
Expected: successful compilation

**Step 4: Commit**

```bash
git add src/pty/shell-integration/powershell/ferrum.ps1 src/pty/mod.rs
git commit -m "feat: add PowerShell shell integration with OSC 7 CWD reporting"
```

---

### Task 3: OS API CWD Fallback Module

**Files:**
- Create: `src/pty/cwd.rs`
- Modify: `src/pty/mod.rs` (add `pub mod cwd;`)
- Modify: `Cargo.toml` (add Windows API feature for process info)

**Step 1: Write failing test**

Create `src/pty/cwd.rs` with a test that verifies we can get CWD of the current process:

```rust
//! Cross-platform fallback for querying a process's current working directory
//! via OS APIs. Used when the shell doesn't emit OSC 7.

/// Queries the OS for the current working directory of the given process.
///
/// Returns `None` if the PID is invalid, the process has exited, or
/// the platform API call fails.
pub fn get_process_cwd(_pid: u32) -> Option<String> {
    None // stub
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_cwd_matches_env() {
        let pid = std::process::id();
        let cwd = get_process_cwd(pid);
        assert!(cwd.is_some(), "should be able to query own CWD");
        let expected = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(cwd.unwrap(), expected);
    }

    #[test]
    fn invalid_pid_returns_none() {
        assert!(get_process_cwd(u32::MAX).is_none());
    }
}
```

**Step 2: Register module in `src/pty/mod.rs`**

Add at the top of `src/pty/mod.rs` (after the existing `use` statements, before the `#[cfg(windows)]` const block):

```rust
pub mod cwd;
```

**Step 3: Run test to verify it fails**

Run: `cargo test cwd::tests::current_process_cwd -- --nocapture`
Expected: FAIL — `get_process_cwd` returns `None`

**Step 4: Implement Linux version**

Replace the stub in `src/pty/cwd.rs`:

```rust
/// Queries the OS for the current working directory of the given process.
pub fn get_process_cwd(pid: u32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        return get_cwd_linux(pid);
    }
    #[cfg(target_os = "macos")]
    {
        return get_cwd_macos(pid);
    }
    #[cfg(target_os = "windows")]
    {
        return get_cwd_windows(pid);
    }
    #[allow(unreachable_code)]
    {
        let _ = pid;
        None
    }
}

#[cfg(target_os = "linux")]
fn get_cwd_linux(pid: u32) -> Option<String> {
    let link = format!("/proc/{}/cwd", pid);
    std::fs::read_link(link)
        .ok()
        .and_then(|p| p.to_str().map(String::from))
}
```

**Step 5: Implement macOS version**

Add to `src/pty/cwd.rs`:

```rust
#[cfg(target_os = "macos")]
fn get_cwd_macos(pid: u32) -> Option<String> {
    use std::mem;

    // PROC_PIDVNODEPATHINFO returns the vnode path info including CWD.
    const PROC_PIDVNODEPATHINFO: i32 = 9;
    const MAXPATHLEN: usize = 1024;

    #[repr(C)]
    struct VInfoPathInfo {
        cdir: VnodePathInfo,
        rdir: VnodePathInfo,
    }

    #[repr(C)]
    struct VnodePathInfo {
        _vip_vi: [u8; 152],       // vnode_info_path padding
        vip_path: [u8; MAXPATHLEN],
    }

    extern "C" {
        fn proc_pidinfo(
            pid: i32,
            flavor: i32,
            arg: u64,
            buffer: *mut libc::c_void,
            buffersize: i32,
        ) -> i32;
    }

    let mut info: VInfoPathInfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<VInfoPathInfo>() as i32;

    let ret = unsafe {
        proc_pidinfo(
            pid as i32,
            PROC_PIDVNODEPATHINFO,
            0,
            &mut info as *mut _ as *mut libc::c_void,
            size,
        )
    };

    if ret <= 0 {
        return None;
    }

    let path_bytes = &info.cdir.vip_path;
    let nul = path_bytes.iter().position(|&b| b == 0).unwrap_or(MAXPATHLEN);
    std::str::from_utf8(&path_bytes[..nul])
        .ok()
        .filter(|s| !s.is_empty())
        .map(String::from)
}
```

**Step 6: Implement Windows version**

Add `Win32_System_ProcessStatus` feature to `Cargo.toml` Windows dependencies:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { version = "0.59", features = [
    "Win32_System_Diagnostics_ToolHelp",
    "Win32_System_Console",
    "Win32_System_Threading",
    "Win32_Foundation",
] }
```

Note: The Windows implementation uses `NtQueryInformationProcess` from `ntdll.dll` via dynamic loading — no extra `windows-sys` features needed.

Add to `src/pty/cwd.rs`:

```rust
#[cfg(target_os = "windows")]
fn get_cwd_windows(pid: u32) -> Option<String> {
    use std::mem;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };

    // We use a simplified approach: spawn a helper that prints the CWD.
    // This avoids complex NtQueryInformationProcess / ReadProcessMemory dance.
    // For our use case (querying the shell we spawned), this is reliable.
    //
    // Alternative: read /proc-style info via wmic or PowerShell.
    // We use the wmic approach as it's available on all Windows versions.

    let output = std::process::Command::new("wmic")
        .args(["process", "where", &format!("ProcessId={}", pid), "get", "ExecutablePath"])
        .output()
        .ok()?;

    // wmic is deprecated; fallback to direct Windows API.
    // For now, we'll use a simpler cross-check approach via handle.
    // The actual implementation queries the PEB via NtQueryInformationProcess.

    // Simplified approach: try to query via PowerShell one-liner
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command",
               &format!("(Get-Process -Id {} -ErrorAction SilentlyContinue).Path | Split-Path", pid)])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}
```

**IMPORTANT NOTE:** The Windows implementation above is a placeholder. The proper approach uses native Win32 APIs (`NtQueryInformationProcess` + `ReadProcessMemory` to read the PEB's `ProcessParameters.CurrentDirectory`). The PowerShell subprocess approach is too slow for polling. During implementation, research and use the native API approach. Here's the correct strategy:

```rust
#[cfg(target_os = "windows")]
fn get_cwd_windows(pid: u32) -> Option<String> {
    use std::mem;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid);
        if handle.is_null() {
            return None;
        }
        // Use NtQueryInformationProcess to get PEB address,
        // then ReadProcessMemory to read ProcessParameters.CurrentDirectory.
        // This requires dynamic loading of ntdll.dll.
        let result = query_process_cwd_native(handle);
        CloseHandle(handle);
        result
    }
}
```

The detailed Windows native implementation should be researched during the implementation step — it involves `PROCESS_BASIC_INFORMATION`, PEB layout, and `RTL_USER_PROCESS_PARAMETERS`. For now, stub it with `None` and focus on Linux/macOS which are simpler.

**Step 7: Run tests**

Run: `cargo test cwd::tests -- --nocapture`
Expected: PASS on current platform (Linux or macOS)

**Step 8: Commit**

```bash
git add src/pty/cwd.rs src/pty/mod.rs Cargo.toml
git commit -m "feat: add cross-platform OS API fallback for querying process CWD"
```

---

### Task 4: Helper — Extract CWD from PaneLeaf with OS API fallback

**Files:**
- Modify: `src/gui/pane.rs` or create helper in `src/gui/mod.rs`

Currently, CWD is retrieved from multiple places with the same pattern: `leaf.terminal.cwd.clone()`. We need a DRY helper that adds the OS API fallback.

**Step 1: Add helper method to `PaneLeaf`**

In `src/gui/pane.rs`, add to `PaneLeaf`:

```rust
/// Returns the pane's current working directory.
///
/// Prefers the value reported by the shell via OSC 7. Falls back to
/// querying the OS for the shell process's CWD when OSC 7 is unavailable.
pub(super) fn cwd(&self) -> Option<String> {
    self.terminal.cwd.clone().or_else(|| {
        self.session
            .as_ref()
            .and_then(|s| s.process_id())
            .and_then(crate::pty::cwd::get_process_cwd)
    })
}
```

**Step 2: Replace all `leaf.terminal.cwd.clone()` callsites with `leaf.cwd()`**

Search for all occurrences and replace:

In `src/gui/events/keyboard/tab_shortcuts.rs`:
- Line 19: `let cwd = self.active_leaf_ref().and_then(|l| l.terminal.cwd.clone());` → `let cwd = self.active_leaf_ref().and_then(|l| l.cwd());`
- Line 24: same pattern → same replacement
- Line 46: same pattern → same replacement

In `src/gui/tabs/manage.rs`:
- Line 80-82 (duplicate_tab): `let cwd = self.tabs[index].focused_leaf().and_then(|l| l.terminal.cwd.clone());` → `let cwd = self.tabs[index].focused_leaf().and_then(|l| l.cwd());`
- Lines 226-229 (split_pane): `let cwd = tab.pane_tree.find_leaf(focused_pane).and_then(|leaf| leaf.terminal.cwd.clone());` → `let cwd = tab.pane_tree.find_leaf(focused_pane).and_then(|leaf| leaf.cwd());`

In `src/gui/lifecycle/mod.rs` line 154:
- `let cwd = win.active_leaf_ref().and_then(|l| l.terminal.cwd.clone());` → `let cwd = win.active_leaf_ref().and_then(|l| l.cwd());`

In `src/gui/events/mouse/tab_bar.rs` (if any CWD references exist there).

**Step 3: Verify compilation**

Run: `cargo build --no-default-features 2>&1 | head -5`
Expected: successful compilation

**Step 4: Run all tests**

Run: `cargo test 2>&1 | tail -5`
Expected: all tests pass

**Step 5: Commit**

```bash
git add src/gui/pane.rs src/gui/events/keyboard/tab_shortcuts.rs src/gui/tabs/manage.rs src/gui/lifecycle/mod.rs
git commit -m "refactor: extract PaneLeaf::cwd() with OS API fallback — DRY"
```

---

### Task 5: Add `is_renamed` field to `TabState`

**Files:**
- Modify: `src/gui/state.rs`
- Modify: `src/gui/tabs/create.rs`
- Modify: `src/gui/tabs/manage.rs` (commit_rename)

**Step 1: Add field to `TabState`**

In `src/gui/state.rs`, add to `TabState` struct (after `next_pane_id` field, line 58):

```rust
/// `true` when the user has explicitly renamed this tab.
/// When `false`, the title auto-updates from the focused pane's CWD.
pub(super) is_renamed: bool,
```

**Step 2: Set default in `build_tab_state`**

In `src/gui/tabs/create.rs`, in the `TabState` construction (line 122-128), add the field:

```rust
Ok(TabState {
    id,
    title: title.unwrap_or_else(|| format!("bash #{}", id + 1)),
    pane_tree: PaneNode::Leaf(Box::new(leaf)),
    focused_pane: pane_id,
    next_pane_id: 1,
    is_renamed: title.is_some(), // custom title = user-provided = renamed
})
```

**Step 3: Set `is_renamed = true` on commit_rename**

In `src/gui/tabs/manage.rs`, `commit_rename()` (line 146-156), after setting `tab.title`:

```rust
tab.title = trimmed;
tab.is_renamed = true;
```

**Step 4: Verify compilation**

Run: `cargo build --no-default-features 2>&1 | head -5`
Expected: successful compilation

**Step 5: Commit**

```bash
git add src/gui/state.rs src/gui/tabs/create.rs src/gui/tabs/manage.rs
git commit -m "feat: add is_renamed field to TabState for dynamic tab naming"
```

---

### Task 6: Path Truncation Utility

**Files:**
- Create: `src/gui/renderer/shared/path_display.rs`
- Modify: `src/gui/renderer/shared/mod.rs` (register module)

**Step 1: Write failing tests**

Create `src/gui/renderer/shared/path_display.rs`:

```rust
//! Utilities for formatting filesystem paths for display in tab titles.
//!
//! Replaces the home directory prefix with `~`, then truncates middle
//! segments with `...` when the path exceeds the available character count.

use std::path::MAIN_SEPARATOR;

/// Formats a CWD path for display in a tab title.
///
/// - Replaces home directory prefix with `~`
/// - If the result exceeds `max_chars`, collapses middle segments to `...`
/// - If still too long, collapses beginning segments
/// - If even the last segment doesn't fit, returns `fallback`
pub fn format_tab_path(path: &str, max_chars: usize, fallback: &str) -> String {
    todo!()
}

/// Replaces the user's home directory prefix with `~`.
fn replace_home_prefix(path: &str) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_prefix_replaced_with_tilde() {
        let home = dirs_home();
        let path = format!("{}/projects/ferrum", home);
        let result = replace_home_prefix(&path);
        assert!(result.starts_with("~/") || result.starts_with("~\\"));
        assert!(result.contains("projects"));
        assert!(result.contains("ferrum"));
    }

    #[test]
    fn path_outside_home_unchanged() {
        let result = replace_home_prefix("/etc/nginx");
        assert_eq!(result, "/etc/nginx");
    }

    #[test]
    fn short_path_no_truncation() {
        let result = format_tab_path("/tmp/foo", 30, "#1");
        assert_eq!(result, "/tmp/foo");
    }

    #[test]
    fn long_path_middle_collapsed() {
        // ~/a/b/c/d/e/target -> should collapse middle
        let home = dirs_home();
        let path = format!("{}/aaa/bbb/ccc/ddd/eee/target", home);
        let result = format_tab_path(&path, 20, "#1");
        assert!(result.contains("..."));
        assert!(result.ends_with("target"));
        assert!(result.len() <= 20);
    }

    #[test]
    fn very_narrow_returns_last_segment() {
        let result = format_tab_path("/very/long/path/to/mydir", 5, "#1");
        assert_eq!(result, "mydir");
    }

    #[test]
    fn extremely_narrow_returns_fallback() {
        let result = format_tab_path("/very/long/path/to/extremely_long_dirname", 3, "#1");
        assert_eq!(result, "#1");
    }

    fn dirs_home() -> String {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| "/home/user".to_string())
    }
}
```

**Step 2: Register module**

In `src/gui/renderer/shared/mod.rs`, add:

```rust
pub mod path_display;
```

**Step 3: Run tests to verify they fail**

Run: `cargo test path_display::tests -- --nocapture`
Expected: FAIL — `todo!()` panics

**Step 4: Implement `replace_home_prefix`**

```rust
fn replace_home_prefix(path: &str) -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    if home.is_empty() {
        return path.to_string();
    }
    if let Some(rest) = path.strip_prefix(&home) {
        if rest.is_empty() {
            "~".to_string()
        } else {
            format!("~{rest}")
        }
    } else {
        path.to_string()
    }
}
```

**Step 5: Implement `format_tab_path`**

```rust
pub fn format_tab_path(path: &str, max_chars: usize, fallback: &str) -> String {
    if max_chars == 0 {
        return fallback.to_string();
    }

    let display = replace_home_prefix(path);

    // Fast path: fits as-is
    if display.chars().count() <= max_chars {
        return display;
    }

    let sep = MAIN_SEPARATOR;
    let segments: Vec<&str> = display.split(sep).filter(|s| !s.is_empty()).collect();
    let last = segments.last().copied().unwrap_or(&display);

    // If even the last segment doesn't fit, return fallback
    if last.chars().count() > max_chars {
        return fallback.to_string();
    }

    // If only the last segment fits, return it
    if last.chars().count() == max_chars {
        return last.to_string();
    }

    let prefix = if display.starts_with('~') { "~" } else if display.starts_with(sep) { &sep.to_string() } else { "" };

    // Try collapsing middle segments: keep prefix, ..., last N segments
    // Start with all segments, progressively remove from the middle
    let kept_start = if display.starts_with('~') { 1 } else { 0 }; // skip ~ segment
    let seg_start = if kept_start > 0 { &segments[1..] } else { &segments[..] };

    // Try keeping last N segments with ... prefix
    for keep_end in (1..=seg_start.len()).rev() {
        let end_segs = &seg_start[seg_start.len() - keep_end..];
        let candidate = if prefix.is_empty() {
            format!("...{sep}{}", end_segs.join(&sep.to_string()))
        } else {
            format!("{prefix}{sep}...{sep}{}", end_segs.join(&sep.to_string()))
        };
        if candidate.chars().count() <= max_chars {
            return candidate;
        }
    }

    // Just the last segment
    last.to_string()
}
```

**Step 6: Run tests**

Run: `cargo test path_display::tests -- --nocapture`
Expected: all PASS

**Step 7: Commit**

```bash
git add src/gui/renderer/shared/path_display.rs src/gui/renderer/shared/mod.rs
git commit -m "feat: add path truncation utility for tab title display"
```

---

### Task 7: Dynamic Tab Title Updates from CWD

**Files:**
- Modify: `src/gui/tabs/create.rs` — set initial title from CWD
- Modify: `src/gui/events/render_shared.rs` — use formatted CWD as title

This task makes tab titles dynamically reflect the current CWD.

**Step 1: Set initial tab title from CWD in `build_tab_state`**

In `src/gui/tabs/create.rs`, change the title logic in `build_tab_state()`:

```rust
// Replace the old default title:
// title: title.unwrap_or_else(|| format!("bash #{}", id + 1)),

// With CWD-based default:
let default_title = cwd.as_deref()
    .map(|dir| {
        use crate::gui::renderer::shared::path_display::format_tab_path;
        let last_seg = std::path::Path::new(dir)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(dir);
        last_seg.to_string()
    })
    .unwrap_or_else(|| {
        let shell_name = std::path::Path::new(&shell)
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("shell");
        shell_name.to_string()
    });

Ok(TabState {
    id,
    title: title.unwrap_or(default_title),
    ...
    is_renamed: title.is_some(),
})
```

**Step 2: Use formatted path in tab bar rendering**

In `src/gui/events/render_shared.rs`, where `TabBarFrameTabInfo` is built (around line 128-129), replace `title: t.title.clone()` with dynamic title based on CWD:

```rust
let display_title = if t.is_renamed {
    t.title.clone()
} else {
    t.focused_leaf()
        .and_then(|leaf| leaf.terminal.cwd.as_deref())
        .map(|cwd| cwd.to_string())
        .unwrap_or_else(|| t.title.clone())
};

TabBarFrameTabInfo {
    title: display_title,
    ...
}
```

The actual truncation happens in the renderer via `tab_title_max_chars()` and `format_tab_path()`. We need to wire up `format_tab_path` in the rendering code.

**Step 3: Update tab title rendering to use `format_tab_path`**

In `src/gui/renderer/tab_bar/tab_content.rs`, `draw_tab_title()` (line 108):

```rust
fn draw_tab_title(
    &mut self,
    target: &mut RenderTarget<'_>,
    tab: &super::super::TabInfo,
    text_x: u32,
    text_y: u32,
    fg: Color,
    max_chars: usize,
) {
    use crate::gui::renderer::shared::path_display::format_tab_path;
    let title = format_tab_path(tab.title, max_chars, &format!("#{}", tab.index + 1));
    // ... render chars
}
```

Wait — `TabInfo` doesn't have an `index` field. We need to pass the tab index for the `#N` fallback. Let's add it.

**Step 4: Add `index` field to `TabInfo`**

In `src/gui/renderer/types.rs`, add to `TabInfo`:

```rust
pub struct TabInfo<'a> {
    pub title: &'a str,
    pub index: usize,  // 0-based tab index
    // ... rest unchanged
}
```

Update all places that construct `TabInfo` to pass the index:
- `src/gui/events/render_shared.rs` (line 51): add `index: i` when building `TabBarFrameTabInfo`, and pipe it through `as_tab_info()`
- `src/gui/events/mouse/tab_bar.rs` (line 10-25): add `index: idx`

Add `index` field to `TabBarFrameTabInfo` as well.

**Step 5: Wire format_tab_path into both CPU and GPU renderers**

CPU renderer (`src/gui/renderer/tab_bar/tab_content.rs`, `draw_tab_title`):

```rust
fn draw_tab_title(&mut self, target: &mut RenderTarget<'_>, tab: &TabInfo, text_x: u32, text_y: u32, fg: Color, max_chars: usize) {
    use crate::gui::renderer::shared::path_display::format_tab_path;
    let fallback = format!("#{}", tab.index + 1);
    let title = format_tab_path(tab.title, max_chars, &fallback);
    for (ci, ch) in title.chars().enumerate() {
        let cx = text_x + ci as u32 * self.metrics.cell_width;
        self.draw_char(target, cx, text_y, ch, fg);
    }
}
```

GPU renderer (`src/gui/renderer/gpu/tab_rendering.rs`, `tab_title_commands`):

```rust
fn tab_title_commands(&mut self, tab: &TabInfo, tab_x: f32, tw: u32, text_y: u32, show_close: bool) {
    use crate::gui::renderer::shared::path_display::format_tab_path;
    let fg_color = if tab.is_active { TAB_TEXT_ACTIVE } else { TAB_TEXT_INACTIVE };
    let m = self.tab_layout_metrics();
    let max_chars = tab_math::tab_title_max_chars(&m, tw, show_close, tab.security_count);
    let tab_padding_h = self.metrics.scaled_px(tab_math::TAB_PADDING_H);
    let fallback = format!("#{}", tab.index + 1);
    let title = format_tab_path(tab.title, max_chars, &fallback);
    let tx = tab_x + tab_padding_h as f32;
    self.push_text(tx, text_y as f32, &title, fg_color, 1.0);
}
```

**Step 6: Verify compilation and tests**

Run: `cargo build --no-default-features && cargo test 2>&1 | tail -5`
Expected: all tests pass

**Step 7: Commit**

```bash
git add src/gui/tabs/create.rs src/gui/events/render_shared.rs src/gui/renderer/types.rs src/gui/renderer/tab_bar/tab_content.rs src/gui/renderer/gpu/tab_rendering.rs src/gui/events/mouse/tab_bar.rs
git commit -m "feat: dynamic tab titles showing CWD with smart path truncation"
```

---

### Task 8: CWD Polling for Tabs Without OSC 7

**Files:**
- Modify: `src/gui/lifecycle/mod.rs` — add CWD poll in `about_to_wait`
- Modify: `src/gui/state.rs` — add `last_cwd_poll` timestamp

**Step 1: Add CWD poll timestamp to per-window or per-app state**

In `src/gui/state.rs`, add to `FerrumWindow` (after `divider_drag` field):

```rust
/// Last time CWD was polled via OS API for tabs without OSC 7.
pub(super) last_cwd_poll: std::time::Instant,
```

Initialize in window creation code (wherever `FerrumWindow` is constructed) with `Instant::now()`.

**Step 2: Add CWD poll logic to `about_to_wait`**

In `src/gui/lifecycle/mod.rs`, inside `about_to_wait()`, after `drain_update_events()` and before the animation scheduling loop, add:

```rust
// Poll CWD via OS API for tabs that don't have OSC 7 data.
let cwd_poll_interval = std::time::Duration::from_secs(1);
for win in self.windows.values_mut() {
    if now.duration_since(win.last_cwd_poll) >= cwd_poll_interval {
        win.last_cwd_poll = now;
        win.poll_cwd_for_tabs();
    }
}
```

**Step 3: Implement `poll_cwd_for_tabs` on `FerrumWindow`**

Add in `src/gui/mod.rs` (or a suitable location):

```rust
/// Polls CWD via OS API for panes that haven't received OSC 7,
/// and updates the tab title if CWD changed.
pub(super) fn poll_cwd_for_tabs(&mut self) {
    for tab in &mut self.tabs {
        if tab.is_renamed {
            continue;
        }
        if let Some(leaf) = tab.focused_leaf_mut() {
            // Skip if we already have CWD from OSC 7
            if leaf.terminal.cwd.is_some() {
                continue;
            }
            // Query OS API
            if let Some(pid) = leaf.session.as_ref().and_then(|s| s.process_id()) {
                if let Some(cwd) = crate::pty::cwd::get_process_cwd(pid) {
                    // Update terminal.cwd so future tab/pane creation inherits it
                    leaf.terminal.cwd = Some(cwd.clone());
                    // Update tab title
                    let last_seg = std::path::Path::new(&cwd)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&cwd)
                        .to_string();
                    tab.title = last_seg;
                }
            }
        }
    }
}
```

Wait — `focused_leaf_mut()` takes `&mut self` but we're iterating `&mut self.tabs`. We need to work with the tab directly. Let me adjust:

```rust
pub(super) fn poll_cwd_for_tabs(&mut self) {
    for tab in &mut self.tabs {
        if tab.is_renamed {
            continue;
        }
        let focused = tab.focused_pane;
        let leaf = match tab.pane_tree.find_leaf_mut(focused) {
            Some(l) => l,
            None => continue,
        };
        if leaf.terminal.cwd.is_some() {
            continue;
        }
        let pid = match leaf.session.as_ref().and_then(|s| s.process_id()) {
            Some(p) => p,
            None => continue,
        };
        if let Some(cwd) = crate::pty::cwd::get_process_cwd(pid) {
            leaf.terminal.cwd = Some(cwd);
        }
    }
}
```

Note: We set `terminal.cwd` so the rendering code in Task 7 picks it up automatically. No need to set `tab.title` here since the render path now reads CWD directly.

**Step 4: Schedule wakeup for CWD polling**

In `about_to_wait()`, ensure the event loop wakes up for CWD polling. After the CWD poll block, compute the next CWD deadline:

```rust
// Ensure we wake up for next CWD poll
let next_cwd_poll = win.last_cwd_poll + cwd_poll_interval;
next_wakeup = Some(next_wakeup.map_or(next_cwd_poll, |current| current.min(next_cwd_poll)));
```

This should be inside the existing `for win in self.windows.values_mut()` animation scheduling loop.

**Step 5: Initialize `last_cwd_poll` in window creation**

Search for where `FerrumWindow` struct is instantiated and add `last_cwd_poll: std::time::Instant::now()`.

**Step 6: Verify compilation and tests**

Run: `cargo build --no-default-features && cargo test 2>&1 | tail -5`
Expected: all tests pass

**Step 7: Commit**

```bash
git add src/gui/state.rs src/gui/lifecycle/mod.rs src/gui/mod.rs
git commit -m "feat: add 1-second CWD polling for tabs without OSC 7 shell integration"
```

---

### Task 9: Final Integration Test & Polish

**Files:**
- All previously modified files

**Step 1: Run full test suite**

Run: `cargo test 2>&1 | tail -20`
Expected: all tests pass

**Step 2: Run clippy**

Run: `cargo clippy --all-targets 2>&1 | tail -20`
Expected: no warnings

**Step 3: Build with GPU feature**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build with default (GPU) features

**Step 4: Build CPU-only**

Run: `cargo build --no-default-features 2>&1 | tail -5`
Expected: successful build

**Step 5: Test on macOS (if available)**

Run: `cargo build` and `cargo test`
Expected: macOS `proc_pidinfo` implementation works

**Step 6: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix: resolve clippy warnings and integration issues"
```
