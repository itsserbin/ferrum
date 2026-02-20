# CWD Preservation & Tab Naming Design

## Problem

When opening a new tab, window, or pane in Ferrum, the current working directory (CWD) is not preserved on Windows (cmd.exe, PowerShell) and any shell without OSC 7 support. New tabs always open in the home directory.

Additionally, tab names are generic (`bash #1`) and don't reflect what the user is working on.

## Solution Overview

Three components:

1. **Shell integration for cmd.exe and PowerShell** — emit OSC 7 on every prompt
2. **OS API fallback** — query CWD from process PID when OSC 7 is unavailable
3. **Dynamic tab naming** — show CWD as tab title with smart truncation

## Component 1: Shell Integration (cmd.exe + PowerShell)

### cmd.exe — OSC 7 via PROMPT

Modify `init.cmd` to set PROMPT that emits OSC 7 before the standard prompt:

```cmd
prompt $e]7;file://%COMPUTERNAME%/%CD%$e\$P$G
```

- `%CD%` expands to current directory on every prompt display
- `%COMPUTERNAME%` provides the hostname
- Updates automatically after every command (including `cd`)

### PowerShell — New integration script

New file: `src/pty/shell-integration/powershell/ferrum.ps1`

```powershell
if ($env:FERRUM_SHELL_INTEGRATION -ne "1") { return }
function prompt {
    $uri = "file://$([System.Net.Dns]::GetHostName())/$($PWD.Path -replace '\\','/')"
    Write-Host -NoNewline "`e]7;$uri`e\"
    "PS $($PWD.Path)> "
}
```

Injection: detect PowerShell in `src/pty/mod.rs`, launch with `-NoExit -File <path>`.

### Existing shell integration (no changes needed)

- bash: `PROMPT_COMMAND` hook emits OSC 7
- zsh: `chpwd` hook emits OSC 7
- fish: `--on-variable PWD` emits OSC 7

## Component 2: OS API Fallback

New module: `src/pty/cwd.rs`

Function: `get_process_cwd(pid: u32) -> Option<String>`

### Platform implementations

**Linux:**
```rust
std::fs::read_link(format!("/proc/{}/cwd", pid))
```

**macOS:**
```rust
// Use proc_pidinfo with PROC_PIDVNODEPATHINFO via FFI
```

**Windows:**
```rust
// NtQueryInformationProcess to get ProcessParameters.CurrentDirectory
```

### Usage

When creating a new tab/window/pane, if `terminal.cwd` is `None`:

```rust
let cwd = leaf.terminal.cwd.clone()
    .or_else(|| leaf.session.as_ref()
        .and_then(|s| s.process_id())
        .and_then(|pid| pty::cwd::get_process_cwd(pid)));
```

OSC 7 has priority. OS API is the fallback for shells without integration.

### Periodic polling for tab naming

For tabs where `terminal.cwd == None` (no OSC 7 support), poll CWD via OS API every ~1 second to keep tab title updated. Skip polling for tabs that already receive OSC 7 updates.

## Component 3: Dynamic Tab Naming

### Naming rules

1. If user renamed the tab (`is_renamed == true`) — show user's custom title
2. Otherwise — show CWD path with smart truncation
3. If CWD unknown — show shell name (`bash`, `cmd`, `pwsh`)

### Path display

- Paths under home directory: `~/relative/path`
- Paths outside home: absolute path (`/etc/nginx`)
- Windows: `~\relative\path` or `C:\absolute\path`

### Truncation algorithm (priority: last segment > first > middle)

Given path `~/PhpstormProjects/ferrum/src/gui`:

| Available width | Result |
|---|---|
| Plenty | `~/PhpstormProjects/ferrum/src/gui` |
| Medium | `~/.../ferrum/src/gui` |
| Small | `~/.../gui` |
| Very small | `gui` |
| Extreme | `#N` (tab index) |

**Algorithm:**
1. Replace home directory prefix with `~`
2. Measure text width against available tab width
3. If too long: collapse middle segments to `...`
4. If still too long: collapse beginning segments
5. Last segment (folder name) is preserved as long as possible
6. If even the last segment doesn't fit: show `#N` (1-indexed tab number)

### State changes

Add `is_renamed: bool` field to `TabState`:
- Default: `false`
- Set to `true` when user renames tab
- When `false`, title updates automatically when CWD changes (via OSC 7 or polling)

## Data Flow

```
Shell command (cd) → prompt displayed
    ├── Shell with integration → OSC 7 → terminal.cwd updated → tab title updated
    └── Shell without integration → OS API poll (1s) → terminal.cwd updated → tab title updated

New tab/window/pane request:
    terminal.cwd (from OSC 7 or OS API) → passed to Session::spawn() → new shell starts in same directory
```

## Files to create/modify

### New files
- `src/pty/cwd.rs` — OS API CWD query (cross-platform)
- `src/pty/shell-integration/powershell/ferrum.ps1` — PowerShell integration

### Modified files
- `src/pty/mod.rs` — add PowerShell injection, `include_str!` for PS1, use `cwd.rs`
- `src/pty/windows-aliases/init.cmd` — add OSC 7 PROMPT
- `src/gui/state.rs` — add `is_renamed` to `TabState`
- `src/gui/tabs/create.rs` — set initial tab title from CWD
- `src/gui/events/keyboard/tab_shortcuts.rs` — use OS API fallback when getting CWD
- `src/gui/events/menu_actions.rs` — use OS API fallback when getting CWD
- `src/gui/tabs/manage.rs` — use OS API fallback in split_pane, duplicate_tab
- Tab bar rendering code — implement truncation algorithm
- Event loop / lifecycle — add CWD polling timer for tabs without OSC 7
