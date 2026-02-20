# Working Directory Inheritance via OSC 7 — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** New tabs, splits, and windows inherit the working directory of the currently focused terminal pane via OSC 7.

**Architecture:** Shell integration scripts emit OSC 7 escape sequences on every `cd`. The terminal parses OSC 7 and stores the CWD per-pane. When creating new tabs/splits/windows, the CWD is read from the focused pane and passed to `Session::spawn()` which sets it on `CommandBuilder`. Shell integration is auto-injected via environment variable manipulation at PTY spawn time.

**Tech Stack:** Rust, vte 0.15 (Perform trait), portable_pty (CommandBuilder), zsh/bash/fish shell scripting.

---

### Task 1: Add `cwd` field to `Terminal` and implement OSC 7 parsing

**Files:**
- Modify: `src/core/terminal.rs:43-72` (Terminal struct)
- Modify: `src/core/terminal.rs:75-106` (Terminal::new)
- Modify: `src/core/terminal.rs:233-256` (full_reset)
- Modify: `src/core/terminal.rs:387` (osc_dispatch)
- Test: `tests/unit/core_terminal.rs`

**Step 1: Write failing tests for OSC 7 parsing**

Add at the end of `tests/unit/core_terminal.rs`:

```rust
// ── OSC 7: working directory reporting ──

#[test]
fn osc7_sets_cwd_from_file_uri() {
    let mut term = Terminal::new(4, 80);
    // OSC 7 ; file://hostname/some/path BEL
    term.process(b"\x1b]7;file://localhost/Users/test/project\x07");
    assert_eq!(term.cwd.as_deref(), Some("/Users/test/project"));
}

#[test]
fn osc7_sets_cwd_from_file_uri_without_host() {
    let mut term = Terminal::new(4, 80);
    // file:///path (empty hostname)
    term.process(b"\x1b]7;file:///home/user\x07");
    assert_eq!(term.cwd.as_deref(), Some("/home/user"));
}

#[test]
fn osc7_sets_cwd_from_kitty_scheme() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;kitty-shell-cwd:///tmp/work\x07");
    assert_eq!(term.cwd.as_deref(), Some("/tmp/work"));
}

#[test]
fn osc7_ignores_remote_hostname() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;file://remote-server/var/log\x07");
    // Remote hostname should be ignored — cwd stays None
    assert_eq!(term.cwd, None);
}

#[test]
fn osc7_empty_resets_cwd() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;file:///some/path\x07");
    assert_eq!(term.cwd.as_deref(), Some("/some/path"));
    // Empty OSC 7 resets
    term.process(b"\x1b]7;\x07");
    assert_eq!(term.cwd, None);
}

#[test]
fn osc7_full_reset_clears_cwd() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;file:///some/path\x07");
    assert_eq!(term.cwd.as_deref(), Some("/some/path"));
    // RIS (full reset)
    term.process(b"\x1bc");
    assert_eq!(term.cwd, None);
}

#[test]
fn osc7_decodes_percent_encoded_path() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;file:///home/user/my%20project\x07");
    assert_eq!(term.cwd.as_deref(), Some("/home/user/my project"));
}

#[test]
fn osc7_with_st_terminator() {
    let mut term = Terminal::new(4, 80);
    // ST terminator (\x1b\\) instead of BEL (\x07)
    term.process(b"\x1b]7;file:///tmp/test\x1b\\");
    assert_eq!(term.cwd.as_deref(), Some("/tmp/test"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test core_terminal -- osc7 -v`
Expected: FAIL — `cwd` field doesn't exist on Terminal

**Step 3: Add `cwd` field to Terminal struct and implement OSC 7 parsing**

In `src/core/terminal.rs`, add `cwd` field to the `Terminal` struct (after `parser`):

```rust
pub cwd: Option<String>,
```

In `Terminal::new()`, initialize it:

```rust
cwd: None,
```

In `full_reset()`, clear it:

```rust
self.cwd = None;
```

Implement `osc_dispatch`:

```rust
fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
    if params.is_empty() {
        return;
    }
    // OSC 7 — Working directory reporting
    if params[0] == b"7" {
        if params.len() < 2 || params[1].is_empty() {
            self.cwd = None;
            return;
        }
        let uri = String::from_utf8_lossy(params[1]);
        self.cwd = parse_osc7_uri(&uri);
    }
}
```

Add a helper function (private, outside the impl blocks, above `#[cfg(test)]`):

```rust
/// Parses an OSC 7 URI (file://hostname/path or kitty-shell-cwd://hostname/path)
/// and returns the path if the hostname is local.
fn parse_osc7_uri(uri: &str) -> Option<String> {
    // Strip scheme
    let after_scheme = uri
        .strip_prefix("file://")
        .or_else(|| uri.strip_prefix("kitty-shell-cwd://"))?;

    // Split into hostname and path at the first '/'
    let (hostname, path) = if let Some(idx) = after_scheme.find('/') {
        (&after_scheme[..idx], &after_scheme[idx..])
    } else {
        // No path component
        return None;
    };

    // Validate hostname is local (empty, "localhost", or matches gethostname)
    if !hostname.is_empty() && hostname != "localhost" {
        let local = gethostname::gethostname();
        if hostname != local.to_string_lossy().as_ref() {
            return None;
        }
    }

    // Percent-decode the path
    let decoded = percent_decode(path);
    if decoded.is_empty() {
        return None;
    }
    Some(decoded)
}

/// Simple percent-decoding for OSC 7 paths.
fn percent_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(h), Some(l)) = (hi, lo) {
                if let (Some(hv), Some(lv)) = (hex_val(h), hex_val(l)) {
                    result.push((hv << 4 | lv) as char);
                    continue;
                }
            }
            result.push('%');
        } else {
            result.push(b as char);
        }
    }
    result
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
```

**Step 4: Add `gethostname` dependency**

Run: `cargo add gethostname`

**Step 5: Run tests to verify they pass**

Run: `cargo test core_terminal -- osc7 -v`
Expected: All 8 OSC 7 tests PASS

**Step 6: Commit**

```bash
git add src/core/terminal.rs tests/unit/core_terminal.rs Cargo.toml Cargo.lock
git commit -m "feat: add OSC 7 parsing for working directory tracking"
```

---

### Task 2: Add CWD parameter to `Session::spawn()`

**Files:**
- Modify: `src/pty/mod.rs:96-146` (Session::spawn)

**Step 1: Write failing test**

Not practical to unit-test PTY spawn directly (requires spawning real processes). We'll verify via integration in Task 4.

**Step 2: Add `cwd` parameter to `Session::spawn()`**

In `src/pty/mod.rs`, change the signature and implementation:

```rust
pub fn spawn(shell: &str, rows: u16, cols: u16, cwd: Option<&str>) -> anyhow::Result<Self> {
```

After building `cmd` and before `cmd.env("TERM", ...)`, add:

```rust
// Set working directory if provided and the path exists.
if let Some(dir) = cwd {
    let path = std::path::Path::new(dir);
    if path.is_dir() {
        cmd.cwd(dir);
        cmd.env("PWD", dir);
    }
}
```

**Step 3: Fix all call sites (they will fail to compile)**

All existing calls pass 3 args — add `None` as the 4th:

In `src/gui/tabs/create.rs:51`:
```rust
let session = pty::Session::spawn(&shell, rows as u16, cols as u16, None)
```

In `src/gui/tabs/manage.rs:224`:
```rust
let session = match pty::Session::spawn(&shell, rows as u16, cols as u16, None)
```

**Step 4: Build to verify compilation**

Run: `cargo build 2>&1 | head -20`
Expected: Build succeeds

**Step 5: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 6: Commit**

```bash
git add src/pty/mod.rs src/gui/tabs/create.rs src/gui/tabs/manage.rs
git commit -m "feat: add cwd parameter to Session::spawn()"
```

---

### Task 3: Pass CWD from focused pane when creating new tabs

**Files:**
- Modify: `src/gui/tabs/create.rs:7-14` (new_tab, new_tab_with_title)
- Modify: `src/gui/tabs/create.rs:38-52` (build_tab_state)

**Step 1: Add `cwd` parameter to tab creation methods**

In `src/gui/tabs/create.rs`, update signatures:

`new_tab`:
```rust
pub(in crate::gui) fn new_tab(
    &mut self,
    rows: usize,
    cols: usize,
    next_tab_id: &mut u64,
    tx: &mpsc::Sender<PtyEvent>,
    cwd: Option<String>,
) {
    self.new_tab_with_title(rows, cols, None, next_tab_id, tx, cwd);
}
```

`new_tab_with_title`:
```rust
pub(in crate::gui) fn new_tab_with_title(
    &mut self,
    rows: usize,
    cols: usize,
    title: Option<String>,
    next_tab_id: &mut u64,
    tx: &mpsc::Sender<PtyEvent>,
    cwd: Option<String>,
) {
    match Self::build_tab_state(rows, cols, title, next_tab_id, tx, cwd) {
```

`build_tab_state`:
```rust
fn build_tab_state(
    rows: usize,
    cols: usize,
    title: Option<String>,
    next_tab_id: &mut u64,
    tx: &mpsc::Sender<PtyEvent>,
    cwd: Option<String>,
) -> anyhow::Result<TabState> {
```

And update the spawn call:
```rust
let session = pty::Session::spawn(&shell, rows as u16, cols as u16, cwd.as_deref())
    .context("failed to spawn PTY session")?;
```

**Step 2: Fix all call sites**

Every call to `new_tab` and `new_tab_with_title` needs the `cwd` parameter.

In `src/gui/events/keyboard/tab_shortcuts.rs` (Ctrl+T on non-macOS):
```rust
self.new_tab(rows, cols, next_tab_id, tx, None);
```
Note: We pass `None` here for now — we'll wire up the CWD reading from focused pane in Task 5.

In `src/gui/lifecycle/window_requests.rs` (NewWindow handler, ~line 81):
```rust
new_win.new_tab_with_title(rows, cols, Some(tab_title), &mut self.next_tab_id, &self.tx, None);
```

In `src/gui/lifecycle/window_requests.rs` (macOS `open_tab_in_native_group`, ~line 24):
```rust
new_win.new_tab_with_title(rows, cols, Some(title), &mut self.next_tab_id, &self.tx, None);
```

Search for any remaining call sites with:
Run: `cargo build 2>&1 | grep "error"`
Fix all compilation errors by adding `None` as the last argument.

**Step 3: Build and test**

Run: `cargo build && cargo test`
Expected: Both succeed

**Step 4: Commit**

```bash
git add src/gui/tabs/create.rs src/gui/events/ src/gui/lifecycle/
git commit -m "feat: thread cwd parameter through tab creation"
```

---

### Task 4: Pass CWD from focused pane when splitting panes

**Files:**
- Modify: `src/gui/tabs/manage.rs:193-232` (split_pane)

**Step 1: Read CWD from focused pane and pass to spawn**

In `split_pane()`, after the block that calculates `(rows, cols)` (line ~220), add CWD reading:

```rust
// Read CWD from the focused pane's terminal (OSC 7 tracking).
let cwd = tab.pane_tree
    .find_leaf(focused_pane)
    .and_then(|leaf| leaf.terminal.cwd.clone());
```

Then update the spawn call:
```rust
let session = match pty::Session::spawn(&shell, rows as u16, cols as u16, cwd.as_deref())
```

**Step 2: Build and test**

Run: `cargo build && cargo test`
Expected: Both succeed

**Step 3: Commit**

```bash
git add src/gui/tabs/manage.rs
git commit -m "feat: inherit CWD from focused pane on split"
```

---

### Task 5: Wire CWD for new tabs, windows, and menu actions

**Files:**
- Modify: `src/gui/events/keyboard/tab_shortcuts.rs` (Ctrl+T)
- Modify: `src/gui/lifecycle/window_requests.rs` (NewWindow, NewTab)
- Modify: `src/gui/state.rs` (WindowRequest)
- Modify: `src/gui/events/menu_actions.rs` (context menu split/duplicate)
- Modify: `src/gui/tabs/manage.rs` (duplicate_tab)

**Step 1: Add CWD to WindowRequest::NewWindow and NewTab**

In `src/gui/state.rs`, update the WindowRequest enum:

```rust
NewWindow { cwd: Option<String> },
#[cfg(target_os = "macos")]
NewTab { cwd: Option<String> },
#[cfg(target_os = "macos")]
ReopenTab { title: String },
```

**Step 2: Update keyboard shortcuts to read CWD from focused pane**

In `src/gui/events/keyboard/tab_shortcuts.rs`, for Ctrl+N:
```rust
if Self::physical_key_is(physical, KeyCode::KeyN) {
    let cwd = self.active_leaf_ref().and_then(|l| l.terminal.cwd.clone());
    self.pending_requests.push(WindowRequest::NewWindow { cwd });
    return Some(true);
}
```

For Ctrl+T (macOS path):
```rust
#[cfg(target_os = "macos")]
{
    let cwd = self.active_leaf_ref().and_then(|l| l.terminal.cwd.clone());
    self.pending_requests.push(WindowRequest::NewTab { cwd });
}
```

For Ctrl+T (non-macOS path):
```rust
#[cfg(not(target_os = "macos"))]
{
    let cwd = self.active_leaf_ref().and_then(|l| l.terminal.cwd.clone());
    let size = self.window.inner_size();
    let (rows, cols) = self.calc_grid_size(size.width, size.height);
    self.new_tab(rows, cols, next_tab_id, tx, cwd);
}
```

**Step 3: Update window_requests.rs to use CWD**

In `process_window_requests`, update the pattern match:

```rust
WindowRequest::NewWindow { cwd } => {
    let tab_title = format!("bash #{}", self.windows.len() + 1);
    if let Some(new_id) = self.create_window(event_loop, None)
        && let Some(new_win) = self.windows.get_mut(&new_id)
    {
        let size = new_win.window.inner_size();
        let (rows, cols) = new_win.calc_grid_size(size.width, size.height);
        new_win.new_tab_with_title(
            rows, cols, Some(tab_title), &mut self.next_tab_id, &self.tx, cwd,
        );
        // ...
    }
}
#[cfg(target_os = "macos")]
WindowRequest::NewTab { cwd } => {
    let tab_title = format!("bash #{}", self.windows.len() + 1);
    self.open_tab_in_native_group(event_loop, window_id, tab_title, cwd);
}
```

Update `open_tab_in_native_group` signature to accept CWD:
```rust
fn open_tab_in_native_group(
    &mut self,
    event_loop: &ActiveEventLoop,
    source_window_id: WindowId,
    title: String,
    cwd: Option<String>,
) {
```

And pass it through:
```rust
new_win.new_tab_with_title(rows, cols, Some(title), &mut self.next_tab_id, &self.tx, cwd);
```

Update ReopenTab handler to pass `None` for CWD:
```rust
WindowRequest::ReopenTab { title } => {
    self.open_tab_in_native_group(event_loop, window_id, title, None);
}
```

**Step 4: Update duplicate_tab in manage.rs to inherit CWD**

```rust
pub(in crate::gui) fn duplicate_tab(
    &mut self,
    index: usize,
    next_tab_id: &mut u64,
    tx: &mpsc::Sender<PtyEvent>,
) {
    if index >= self.tabs.len() {
        return;
    }
    let title = format!("{} (copy)", self.tabs[index].title);
    let cwd = self.tabs[index]
        .focused_leaf()
        .and_then(|l| l.terminal.cwd.clone());
    let size = self.window.inner_size();
    let (rows, cols) = self.calc_grid_size(size.width, size.height);
    self.new_tab_with_title(rows, cols, Some(title), next_tab_id, tx, cwd);
}
```

**Step 5: Build and test**

Run: `cargo build && cargo test`
Expected: Both succeed

**Step 6: Commit**

```bash
git add src/gui/state.rs src/gui/events/ src/gui/lifecycle/ src/gui/tabs/manage.rs
git commit -m "feat: wire CWD inheritance for new tabs, windows, splits, and duplicates"
```

---

### Task 6: Create shell integration scripts

**Files:**
- Create: `src/shell-integration/zsh/ferrum-integration`
- Create: `src/shell-integration/bash/ferrum.bash`
- Create: `src/shell-integration/fish/vendor_conf.d/ferrum-shell-integration.fish`

**Step 1: Create zsh integration script**

Create `src/shell-integration/zsh/ferrum-integration`:

```zsh
# Ferrum terminal shell integration for zsh.
# Sends OSC 7 with the current working directory on every prompt.

# Guard: only run inside Ferrum.
[[ -n "$FERRUM_SHELL_INTEGRATION" ]] || return

_ferrum_report_cwd() {
  builtin printf '\e]7;file://%s%s\a' "$HOST" "$PWD"
}

# Report on every directory change.
autoload -Uz add-zsh-hook
add-zsh-hook chpwd _ferrum_report_cwd

# Also report once at shell startup (initial prompt).
_ferrum_report_cwd
```

**Step 2: Create bash integration script**

Create `src/shell-integration/bash/ferrum.bash`:

```bash
# Ferrum terminal shell integration for bash.
# Sends OSC 7 with the current working directory on every prompt.

# Guard: only run inside Ferrum.
[[ -n "$FERRUM_SHELL_INTEGRATION" ]] || return

_ferrum_last_reported_cwd=""

_ferrum_report_cwd() {
  if [[ "$_ferrum_last_reported_cwd" != "$PWD" ]]; then
    _ferrum_last_reported_cwd="$PWD"
    builtin printf '\e]7;file://%s%s\a' "$HOSTNAME" "$PWD"
  fi
}

PROMPT_COMMAND="_ferrum_report_cwd${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
```

**Step 3: Create fish integration script**

Create `src/shell-integration/fish/vendor_conf.d/ferrum-shell-integration.fish`:

```fish
# Ferrum terminal shell integration for fish.
# Sends OSC 7 with the current working directory on every directory change.

# Guard: only run inside Ferrum.
if not set -q FERRUM_SHELL_INTEGRATION
    exit
end

function __ferrum_report_cwd --on-variable PWD
    printf '\e]7;file://%s%s\a' (hostname) (string escape --style=url -- $PWD)
end

# Report initial CWD.
__ferrum_report_cwd
```

**Step 4: Commit**

```bash
git add src/shell-integration/
git commit -m "feat: add shell integration scripts for OSC 7 CWD reporting"
```

---

### Task 7: Auto-inject shell integration at PTY spawn

**Files:**
- Modify: `src/pty/mod.rs` (Session::spawn — env variable setup)

**Step 1: Embed shell integration scripts at compile time**

At the top of `src/pty/mod.rs`, add:

```rust
const SHELL_INTEGRATION_ZSH: &str = include_str!("../shell-integration/zsh/ferrum-integration");
const SHELL_INTEGRATION_BASH: &str = include_str!("../shell-integration/bash/ferrum.bash");
const SHELL_INTEGRATION_FISH: &str = include_str!("../shell-integration/fish/vendor_conf.d/ferrum-shell-integration.fish");
```

**Step 2: Write shell integration files to temp dir and set env variables**

Add a function to create the integration directory and return its path:

```rust
fn setup_shell_integration() -> Option<std::path::PathBuf> {
    let temp_dir = std::env::temp_dir().join("ferrum_shell_integration");

    // Create directory structure
    let zsh_dir = temp_dir.join("zsh");
    let bash_dir = temp_dir.join("bash");
    let fish_dir = temp_dir.join("fish").join("vendor_conf.d");

    std::fs::create_dir_all(&zsh_dir).ok()?;
    std::fs::create_dir_all(&bash_dir).ok()?;
    std::fs::create_dir_all(&fish_dir).ok()?;

    std::fs::write(zsh_dir.join("ferrum-integration"), SHELL_INTEGRATION_ZSH).ok()?;
    std::fs::write(bash_dir.join("ferrum.bash"), SHELL_INTEGRATION_BASH).ok()?;
    std::fs::write(
        fish_dir.join("ferrum-shell-integration.fish"),
        SHELL_INTEGRATION_FISH,
    ).ok()?;

    Some(temp_dir)
}
```

**Step 3: Inject environment variables in `Session::spawn()`**

In `Session::spawn()`, after `cmd.env("TERM", "xterm-256color")` and before `let child = pair.slave.spawn_command(cmd)?`, add:

```rust
// Shell integration: set marker env var and configure per-shell injection.
cmd.env("FERRUM_SHELL_INTEGRATION", "1");

if let Some(integration_dir) = setup_shell_integration() {
    let shell_name = std::path::Path::new(shell)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(shell);

    match shell_name {
        "zsh" => {
            // For zsh: source the integration file via ZDOTDIR trick.
            // Create a .zshenv that sources both user config and our integration.
            let zdotdir = integration_dir.join("zsh");
            let user_zdotdir = std::env::var("ZDOTDIR")
                .unwrap_or_else(|_| {
                    std::env::var("HOME").unwrap_or_else(|_| String::from("/"))
                });
            let zshenv_content = format!(
                "ZDOTDIR=\"{user_zdotdir}\"\n\
                 [[ -f \"$ZDOTDIR/.zshenv\" ]] && source \"$ZDOTDIR/.zshenv\"\n\
                 source \"{}/ferrum-integration\"\n",
                zdotdir.display()
            );
            let _ = std::fs::write(zdotdir.join(".zshenv"), zshenv_content);
            cmd.env("ZDOTDIR", zdotdir.to_string_lossy().as_ref());
        }
        "bash" => {
            // For bash: use --rcfile or BASH_ENV is not reliable for interactive shells.
            // Instead, prepend our source to ENV if set, or use BASH_ENV for non-interactive
            // and rely on the FERRUM_SHELL_INTEGRATION env var for manual sourcing.
            // Best approach: XDG_CONFIG_HOME trick or --rcfile.
            // Simplest: set BASH_ENV for now — works for interactive login shells on macOS
            // because we pass -l. For robustness, we also wrap rcfile.
            let bash_integration = integration_dir.join("bash").join("ferrum.bash");
            let user_bashrc = std::env::var("HOME")
                .map(|h| format!("{h}/.bashrc"))
                .unwrap_or_default();
            let wrapper_content = format!(
                "[[ -f \"{user_bashrc}\" ]] && source \"{user_bashrc}\"\n\
                 source \"{}\"\n",
                bash_integration.display()
            );
            let wrapper_path = integration_dir.join("bash").join("ferrum-bashrc");
            let _ = std::fs::write(&wrapper_path, wrapper_content);
            cmd.env("BASH_ENV", bash_integration.to_string_lossy().as_ref());
        }
        "fish" => {
            // For fish: add our vendor_conf.d to XDG_DATA_DIRS.
            let fish_dir = integration_dir.join("fish");
            let existing = std::env::var("XDG_DATA_DIRS")
                .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
            let new_xdg = format!("{}:{}", fish_dir.display(), existing);
            cmd.env("XDG_DATA_DIRS", &new_xdg);
        }
        _ => {}
    }
}
```

**Step 4: Build and test**

Run: `cargo build && cargo test`
Expected: Both succeed

**Step 5: Commit**

```bash
git add src/pty/mod.rs
git commit -m "feat: auto-inject shell integration scripts via env variables"
```

---

### Task 8: Final integration test and cleanup

**Files:**
- Modify: `tests/unit/core_terminal.rs` (verify all tests pass)

**Step 1: Run full test suite**

Run: `cargo test -v`
Expected: All tests PASS

**Step 2: Build release to verify no warnings**

Run: `cargo build --release 2>&1 | grep -i warning`
Expected: No new warnings (or fix any that appear)

**Step 3: Manual testing checklist**

Run the app: `cargo run`

Verify:
1. Open terminal → `cd /tmp` → open new tab (Ctrl+T) → verify new tab starts in `/tmp`
2. Open terminal → `cd /tmp` → split pane (Ctrl+Shift+R) → verify new pane starts in `/tmp`
3. Open terminal → `cd /tmp` → new window (Ctrl+N) → verify new window starts in `/tmp`
4. Without shell integration → verify new tab/split starts in home dir (fallback)
5. Split a pane, `cd /var` in one, focus the other, `cd /etc` → split → verify it uses the focused pane's CWD

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete working directory inheritance via OSC 7"
```
