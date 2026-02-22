# Auto-Update Feature Design

## Goal

When a newer version is available, show a non-intrusive in-app toast notification that lets
the user open the release page or trigger a one-click update without leaving the terminal.

## Architecture

**Approach: in-process + platform-specific installer logic.**

The existing background update checker (`src/update.rs`) already polls GitHub Releases and
delivers an `AvailableRelease` via channel. We extend it with:

1. **Toast UI** — rendered inside the terminal window (same layer as tooltip overlay).
2. **Install dispatcher** — detects install method (Homebrew / installer / binary replace)
   and runs the appropriate logic in a background thread.
3. **Settings "Updates" tab** — shows version info, last-checked date, and a manual check
   button. Added to all three platform settings windows.

```
Background thread ──→ mpsc channel ──→ App::available_release
                                              │
                                              ▼
                              FerrumWindow: Option<UpdateToastState>
                                              │
                                    ┌─────────┴──────────┐
                                    ▼                    ▼
                              render toast          handle click
                            (CPU + GPU path)    (open browser / install)
```

## Toast Notification

Small rounded rect at top-center of the terminal window, visually consistent with the
existing tooltip/drag-overlay (`radius = scaled_px(6)`, `menu_bg` background color).

```
╭──────────────────────────────────────────────────────╮
│  Update v0.3.2 available     [ Details ]  [ Install ]  ✕  │
╰──────────────────────────────────────────────────────╯
```

- **Details** — opens `AvailableRelease::html_url` in the default browser.
- **Install** — starts the platform-specific installer; button label changes to "Installing…".
- **✕** — dismisses; does not suppress future toasts (re-shown on next launch if update still
  available).

Layout computed in `src/gui/renderer/shared/update_toast_layout.rs` (mirrors
`overlay_layout.rs`). Rendered in the existing overlay pass of both CPU and GPU renderers.

Hit testing in `src/gui/events/mouse/update_toast.rs`.

## Install Logic (`src/update/installer.rs`)

| Platform | Detected by | Action |
|----------|-------------|--------|
| macOS + Homebrew | `brew list --cask ferrum` exit 0 | `brew upgrade --cask ferrum` |
| macOS + direct | fallback | download `ferrum-{arch}-apple-darwin.zip`, replace binary, relaunch |
| Windows | always | download `Ferrum-Setup-x64.exe`, run with `/VERYSILENT /CLOSEAPPLICATIONS /RESTARTAPPLICATIONS` |
| Linux + system path | binary path starts with `/usr` or `/opt` | `pkexec cp <new-bin> <current-bin> && relaunch` |
| Linux + user path | fallback | direct replace + relaunch |

Download URLs derived from `AvailableRelease::tag_name`:
```
https://github.com/itsserbin/ferrum/releases/download/{tag}/ferrum-{arch}-{os}{ext}
```

Config at `~/.config/ferrum/` is never touched — only the binary is replaced.

### Install State Machine

```
Idle ──[click Install]──→ Downloading ──→ Replacing ──→ Relaunching
                                 │
                         [error]─┴──→ Failed(message)
```

`UpdateInstallState` enum stored in `FerrumWindow`. The toast re-renders on each state change.

## Settings "Updates" Tab

Added to all three platform settings windows
(`src/gui/platform/{macos,windows,linux}/settings_window.rs`).

Contents:
- Current version label: "Ferrum v0.2.3"
- Last checked label: "Checked: 2026-02-22 14:30"
- Auto-check toggle (maps to new `AppConfig::updates::auto_check: bool`, default `true`)
- "Check Now" button — forces a fresh API fetch, ignoring cache

The `AppConfig` change is backwards-compatible (new field with `#[serde(default)]`).

## Data Flow for Manual Check

```
[Check Now click] → send UpdateRequest::CheckNow via existing settings channel
→ App drains channel → spawns one-shot background thread (same as auto-check)
→ result delivered via update_tx → App::available_release set
→ window redraws → toast appears
```

## Files to Create / Modify

| Path | Change |
|------|--------|
| `src/update/mod.rs` | rename from `src/update.rs`; add re-exports |
| `src/update/installer.rs` | new: install logic per platform |
| `src/gui/renderer/shared/update_toast_layout.rs` | new: toast geometry |
| `src/gui/events/mouse/update_toast.rs` | new: click hit testing |
| `src/gui/state.rs` | add `UpdateInstallState`, `update_toast` field |
| `src/gui/events/render_shared.rs` | render toast in overlay pass |
| `src/gui/renderer/cpu/mod.rs` | draw toast rects + text |
| `src/gui/renderer/gpu/mod.rs` | draw toast rects + text |
| `src/config/mod.rs` | add `UpdatesConfig { auto_check }` |
| `src/gui/platform/macos/settings_window.rs` | add Updates tab |
| `src/gui/platform/windows/settings_window.rs` | add Updates tab |
| `src/gui/platform/linux/settings_window.rs` | add Updates tab |
| `src/gui/lifecycle/mod.rs` | drain `UpdateRequest::CheckNow` channel |

## Testing

- Unit tests for installer URL construction (all platforms).
- Unit tests for `compute_update_toast_layout` (same pattern as `overlay_layout.rs`).
- Manual smoke test: trigger toast by temporarily setting a lower `CARGO_PKG_VERSION`.
