# Working Directory Inheritance via OSC 7

**Date:** 2026-02-20
**Status:** Approved

## Goal

New tabs, splits, and windows inherit the working directory of the currently focused terminal pane, using the OSC 7 escape sequence standard.

## Approach

**OSC 7** — the industry standard used by Ghostty, kitty, iTerm2, WezTerm. Shell integration scripts automatically emit `\e]7;file://hostname/path\a` on every directory change. The terminal parses this and stores the CWD per-pane.

## Data Flow

```
Shell executes `cd /some/path`
    ↓
Shell integration hook (chpwd for zsh, PROMPT_COMMAND for bash, --on-variable PWD for fish)
    ↓
Emits: \e]7;file://hostname/some/path\a
    ↓
vte crate parses OSC sequence → calls osc_dispatch(&[b"7", b"file://hostname/path"])
    ↓
Terminal::osc_dispatch() parses URI → stores path in Terminal.cwd: Option<String>
    ↓
User presses Ctrl+T (new tab) / split shortcut / Ctrl+N (new window)
    ↓
Tab/split/window creation → reads cwd from active PaneLeaf.terminal.cwd
    ↓
pty::Session::spawn() receives cwd parameter → CommandBuilder.cwd(path)
    ↓
New terminal opens in /some/path
```

## Components & Changes

### 1. `src/core/terminal.rs` — CWD field + OSC 7 parsing

- Add field `cwd: Option<String>` to `Terminal`
- In `osc_dispatch()`: parse OSC 7 (`params[0] == b"7"`), extract path from `file://hostname/path` URI, validate hostname is local, store in `self.cwd`
- Empty OSC 7 (`\e]7;\a`) resets CWD to None

### 2. `src/pty/mod.rs` — CWD support in spawn

- `Session::spawn()` gets new parameter `cwd: Option<&str>`
- If CWD provided → `cmd.cwd(path)` + `cmd.env("PWD", path)`
- If not → behavior as now (home dir)
- Validate path exists before using, fallback to home if deleted

### 3. `src/gui/tabs/create.rs` — Pass CWD when creating tabs

- `build_tab_state()` gets `cwd: Option<String>` parameter
- Passes to `Session::spawn()`

### 4. `src/gui/tabs/manage.rs` — Pass CWD on split

- `split_pane()` reads `cwd` from current focused pane's terminal
- Passes to `Session::spawn()` for new pane

### 5. `src/gui/events/keyboard/` — CWD for new windows

- On Ctrl+N (new window) — read CWD from active pane and pass to new window

### 6. Shell integration scripts (new files)

- `src/shell-integration/zsh/ferrum-integration` — `chpwd` hook
- `src/shell-integration/bash/ferrum.bash` — `PROMPT_COMMAND` hook
- `src/shell-integration/fish/vendor_conf.d/ferrum-shell-integration.fish` — `--on-variable PWD` hook
- All emit `\e]7;file://hostname$PWD\a`

### 7. `src/pty/mod.rs` — Auto-inject shell integration

- Set `FERRUM_SHELL_INTEGRATION_DIR` env variable pointing to scripts directory
- For zsh: manipulate `ZDOTDIR` to source integration script
- For bash: use `BASH_ENV` or `--rcfile`
- For fish: add to `XDG_DATA_DIRS`

## Fallback Chain

1. **OSC 7 CWD** from focused pane — primary source
2. **Initial CWD** from spawn time (before first OSC 7) — stored on pane creation
3. **Home directory** — if nothing above is available

## Edge Cases

- **Deleted directory** — check `Path::exists()` before spawn, fallback to home
- **SSH session** — OSC 7 may contain remote hostname; validate hostname == local, otherwise ignore
- **Shell without integration** — remains home dir (expected behavior)
- **Multiple panes in tab** — read CWD from **focused** pane
- **Empty OSC 7** (`\e]7;\a`) — resets CWD to None

## Testing

### Unit tests
- OSC 7 parsing: various URI formats (`file://`, `file://hostname/`, empty)
- Hostname validation: local vs remote
- CWD fallback chain: OSC 7 → initial CWD → home
- Path validation: existing vs deleted directory

### Integration tests
- Spawn with CWD → verify shell starts in correct directory
- OSC 7 → new tab → verify CWD is passed
