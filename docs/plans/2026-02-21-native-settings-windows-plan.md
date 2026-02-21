# Native Settings Windows Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the custom-drawn settings overlay on Windows/Linux with native platform windows (Win32 API on Windows, GTK4 on Linux), delete all overlay code, and remove the `rfd` dependency.

**Architecture:** Three platform-specific settings window implementations sharing a common public API and communicating via `mpsc::Sender<AppConfig>`. macOS (existing AppKit) stays unchanged. The overlay rendering, mouse, and keyboard code is deleted entirely. Settings changes flow through a channel to the lifecycle loop which applies them to all windows.

**Tech Stack:** `windows-sys` crate (expanded features) for Win32 UI controls on Windows; `gtk4` crate for GTK4 widgets on Linux; existing `objc2`/`objc2-app-kit` for macOS.

**Design doc:** `docs/plans/2026-02-21-native-settings-windows-design.md`

---

## Task 1: Make settings channel unconditional (remove macOS-only guards)

Currently `settings_tx`/`settings_rx` are `#[cfg(target_os = "macos")]`. Since all platforms will now use the same channel pattern, make them unconditional.

**Files:**
- Modify: `src/gui/state.rs:238-239` (FerrumWindow), `src/gui/state.rs:252-255` (App)
- Modify: `src/gui/mod.rs:100-103` (FerrumWindow::new), `src/gui/mod.rs:234-248` (App::new), `src/gui/mod.rs:316-319` (create_window)
- Modify: `src/gui/lifecycle/mod.rs:227-235` (about_to_wait channel receive)

**Step 1: Update `src/gui/state.rs`**

Remove `#[cfg(target_os = "macos")]` from:
- Line 237-239: `settings_tx` field on `FerrumWindow`
- Line 252-255: `settings_tx` and `settings_rx` fields on `App`

**Step 2: Update `src/gui/mod.rs` — FerrumWindow::new**

Line 102-103: Remove `#[cfg(target_os = "macos")]` from `settings_tx` initialization.

**Step 3: Update `src/gui/mod.rs` — App::new**

Line 234-235: Remove `#[cfg(target_os = "macos")]` from channel creation.
Lines 245-248: Remove `#[cfg(target_os = "macos")]` from `settings_tx`/`settings_rx` in App struct init.

**Step 4: Update `src/gui/mod.rs` — create_window**

Lines 316-319: Remove `#[cfg(target_os = "macos")]` block, always clone `settings_tx` into new windows.

**Step 5: Update `src/gui/lifecycle/mod.rs` — about_to_wait**

Lines 227-235: Remove `#[cfg(target_os = "macos")]` from the `settings_rx.try_recv()` loop. This loop should run on all platforms now.

**Step 6: Verify compilation**

Run: `cargo build 2>&1 | head -30`
Expected: Compiles (possibly with warnings about unused `settings_tx` on non-macOS, which is fine for now).

**Step 7: Commit**

```bash
git add src/gui/state.rs src/gui/mod.rs src/gui/lifecycle/mod.rs
git commit -m "refactor: make settings channel unconditional across all platforms"
```

---

## Task 2: Remove settings overlay from FerrumWindow and all references

Remove the `settings_overlay: Option<SettingsOverlay>` field and `pending_config: Option<AppConfig>` field from `FerrumWindow`, and all code that uses them.

**Files:**
- Modify: `src/gui/state.rs:233-236` — remove fields
- Modify: `src/gui/mod.rs:100-101` — remove field initialization
- Modify: `src/gui/events/render_shared.rs:92` — remove from `FrameParams`
- Modify: `src/gui/events/render_cpu.rs:43` — remove from params construction
- Modify: `src/gui/events/render_gpu.rs:39` — remove from params construction
- Modify: `src/gui/events/render_shared.rs:433,454-457` — remove overlay rendering call
- Modify: `src/gui/lifecycle/mod.rs:133-138` — remove pending_config pickup
- Modify: `src/gui/events/keyboard/entry.rs:20-25` — remove overlay keyboard interception
- Modify: `src/gui/events/mouse/input.rs:18-21` — remove overlay mouse gate

**Step 1: Remove fields from `src/gui/state.rs`**

Delete lines 233-236:
```
    /// Settings overlay state (open when Some).
    pub(super) settings_overlay: Option<crate::gui::settings::SettingsOverlay>,
    /// Pending config update from settings overlay (picked up by App).
    pub(super) pending_config: Option<crate::config::AppConfig>,
```

**Step 2: Remove from `src/gui/mod.rs` — FerrumWindow::new**

Delete lines 100-101:
```
            settings_overlay: None,
            pending_config: None,
```

**Step 3: Remove from `src/gui/events/render_shared.rs`**

- Line 92: Remove `settings_overlay` field from `FrameParams`
- Line 433: Remove `params.settings_overlay.is_some()` — replace with `false` (the gear button highlight state; the native settings window will handle this differently later, or we can check `is_settings_window_open()`)
- Lines 454-457: Delete the settings overlay draw block:
  ```
  if let Some(overlay) = params.settings_overlay {
      renderer.draw_settings_overlay(&mut target, overlay);
  }
  ```

**Step 4: Remove from render path params construction**

- `src/gui/events/render_cpu.rs:43`: Remove `settings_overlay: self.settings_overlay.as_ref(),`
- `src/gui/events/render_gpu.rs:39`: Remove `settings_overlay: self.settings_overlay.as_ref(),`

**Step 5: Remove pending_config pickup from lifecycle**

`src/gui/lifecycle/mod.rs:133-138`: Delete the block:
```rust
if let Some(win) = self.windows.get_mut(&window_id)
    && let Some(new_config) = win.pending_config.take()
{
    self.config = new_config;
}
```

**Step 6: Remove overlay keyboard interception from `src/gui/events/keyboard/entry.rs`**

Lines 19-25: Delete the block:
```rust
// Settings overlay intercepts all keyboard input when open.
if self.settings_overlay.is_some() {
    let key = Self::normalize_non_text_key(&event.logical_key, &event.physical_key);
    if self.handle_settings_keyboard(&key) {
        return;
    }
}
```

**Step 7: Remove overlay mouse gate from `src/gui/events/mouse/input.rs`**

Lines 18-21: Delete the block:
```rust
if self.settings_overlay.is_some() && button != winit::event::MouseButton::Left {
    return;
}
```

**Step 8: Verify compilation**

Run: `cargo build 2>&1 | head -50`
Expected: Errors about `handle_settings_keyboard`, `close_settings_overlay`, `draw_settings_overlay` — these are removed in next tasks.

**Step 9: Commit (even with errors — this is a checkpoint)**

Skip commit if it doesn't compile. Continue to next task to complete the cleanup.

---

## Task 3: Delete overlay event handlers (mouse + keyboard)

**Files:**
- Delete: `src/gui/events/mouse/settings.rs`
- Delete: `src/gui/events/keyboard/settings.rs`
- Modify: `src/gui/events/mouse/mod.rs` — remove `mod settings;`
- Modify: `src/gui/events/keyboard/mod.rs` — remove `mod settings;`

**Step 1: Delete files**

```bash
rm src/gui/events/mouse/settings.rs
rm src/gui/events/keyboard/settings.rs
```

**Step 2: Remove module declarations**

In `src/gui/events/mouse/mod.rs`: Remove `mod settings;` line.
In `src/gui/events/keyboard/mod.rs`: Remove `mod settings;` line.

**Step 3: Create new minimal `toggle_settings_overlay` in `src/gui/events/keyboard/shortcuts.rs` or a new small file**

The `toggle_settings_overlay` function is called from:
- `src/gui/events/keyboard/shortcuts.rs:55-58` (Cmd/Ctrl+Comma)
- `src/gui/events/mouse/tab_bar.rs:155` (gear button click)
- `src/gui/lifecycle/mod.rs:197` (macOS native gear button)

Create a small replacement that delegates to each platform's native settings window. Add this to a new file `src/gui/events/settings_toggle.rs`:

```rust
use crate::config::AppConfig;
use crate::gui::*;

impl FerrumWindow {
    pub(in crate::gui) fn toggle_settings_overlay(&mut self, config: &AppConfig) {
        #[cfg(target_os = "macos")]
        {
            use crate::gui::platform::macos::settings_window;
            if settings_window::is_settings_window_open() {
                settings_window::close_settings_window();
            } else {
                settings_window::open_settings_window(config, self.settings_tx.clone());
            }
        }

        #[cfg(target_os = "windows")]
        {
            use crate::gui::platform::windows::settings_window;
            if settings_window::is_settings_window_open() {
                settings_window::close_settings_window();
            } else {
                settings_window::open_settings_window(config, self.settings_tx.clone());
            }
        }

        #[cfg(target_os = "linux")]
        {
            use crate::gui::platform::linux::settings_window;
            if settings_window::is_settings_window_open() {
                settings_window::close_settings_window();
            } else {
                settings_window::open_settings_window(config, self.settings_tx.clone());
            }
        }
    }
}
```

Register it in `src/gui/events/mod.rs`: Add `mod settings_toggle;`

**Step 4: Update `close_settings_overlay` in `src/gui/events/settings_apply.rs`**

Delete the `close_settings_overlay` method (lines 51-58) — it's overlay-specific. The native windows handle saving on their own close.

Keep only the `apply_config_change` method.

**Step 5: Verify compilation**

Run: `cargo build 2>&1 | head -50`
Expected: Errors about missing platform modules (windows, linux) and `draw_settings_overlay` trait method.

---

## Task 4: Remove overlay rendering from trait and implementations

**Files:**
- Delete: `src/gui/renderer/settings.rs` (CPU overlay rendering)
- Delete: `src/gui/renderer/gpu/settings.rs` (GPU overlay rendering)
- Modify: `src/gui/renderer/traits.rs:306-312` — remove `draw_settings_overlay` from trait
- Modify: `src/gui/renderer/gpu/trait_impl.rs:201-206` — remove impl
- Modify: `src/gui/renderer/cpu/trait_impl.rs:166-171` — remove impl
- Modify: `src/gui/renderer/mod.rs` — remove `mod settings;`
- Modify: `src/gui/renderer/gpu/mod.rs` — remove `mod settings;`

**Step 1: Delete renderer settings files**

```bash
rm src/gui/renderer/settings.rs
rm src/gui/renderer/gpu/settings.rs
```

**Step 2: Remove module declarations**

In `src/gui/renderer/mod.rs`: Remove `mod settings;` line (if present).
In `src/gui/renderer/gpu/mod.rs`: Remove `mod settings;` line.

**Step 3: Remove trait method from `src/gui/renderer/traits.rs`**

Delete lines 306-312:
```rust
// ── Settings overlay ────────────────────────────────────────────

fn draw_settings_overlay(
    &mut self,
    target: &mut RenderTarget<'_>,
    overlay: &crate::gui::settings::SettingsOverlay,
);
```

**Step 4: Remove implementations**

In `src/gui/renderer/gpu/trait_impl.rs`: Delete the `draw_settings_overlay` method (lines 201-206).
In `src/gui/renderer/cpu/trait_impl.rs`: Delete the `draw_settings_overlay` method (lines 166-171).

**Step 5: Verify**

Run: `cargo build 2>&1 | head -50`

---

## Task 5: Delete overlay data model and settings module

**Files:**
- Delete: `src/gui/settings/overlay.rs`
- Delete: `src/gui/settings/layout.rs`
- Modify: `src/gui/settings/mod.rs` — remove overlay imports, keep module if anything remains or delete entirely

**Step 1: Check if settings/mod.rs has anything besides overlay**

Current content re-exports overlay types only. If `layout.rs` was `pub(super)`, check if anything else uses it.

The layout module is used only by the renderer settings files (already deleted). So delete everything.

**Step 2: Delete files and directory**

```bash
rm src/gui/settings/overlay.rs
rm src/gui/settings/layout.rs
rm src/gui/settings/mod.rs
rmdir src/gui/settings/
```

**Step 3: Remove module declaration from `src/gui/mod.rs`**

Remove: `mod settings;`

**Step 4: Remove any remaining imports of `crate::gui::settings::*`**

Grep for `crate::gui::settings` and remove all remaining imports/uses.

**Step 5: Handle `settings_open` parameter in tab bar rendering**

In `src/gui/events/render_shared.rs:433`, the `settings_overlay.is_some()` was passed to `draw_tab_bar` as `settings_open`. Now that overlay is gone, pass `false` or check `is_settings_window_open()`:

```rust
// For the gear button highlight, check if native settings window is open.
#[cfg(target_os = "macos")]
let settings_open = crate::gui::platform::macos::settings_window::is_settings_window_open();
#[cfg(target_os = "windows")]
let settings_open = crate::gui::platform::windows::settings_window::is_settings_window_open();
#[cfg(target_os = "linux")]
let settings_open = crate::gui::platform::linux::settings_window::is_settings_window_open();
```

Or simpler: just pass `false` for now and revisit after platform windows are implemented.

**Step 6: Verify**

Run: `cargo build 2>&1 | head -50`
Expected: Errors about missing platform::windows and platform::linux modules. Overlay code should be fully gone.

**Step 7: Commit checkpoint**

```bash
git add -A
git commit -m "refactor: remove custom settings overlay code (rendering, events, data model)"
```

---

## Task 6: Add platform module stubs for Windows and Linux

Create empty stub modules so the project compiles. The actual implementations come in later tasks.

**Files:**
- Create: `src/gui/platform/windows/mod.rs`
- Create: `src/gui/platform/windows/settings_window.rs`
- Create: `src/gui/platform/linux/mod.rs`
- Create: `src/gui/platform/linux/settings_window.rs`
- Modify: `src/gui/platform/mod.rs` — add module declarations

**Step 1: Create directory structure**

```bash
mkdir -p src/gui/platform/windows
mkdir -p src/gui/platform/linux
```

**Step 2: Create `src/gui/platform/windows/settings_window.rs`**

```rust
use crate::config::AppConfig;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);

pub fn is_settings_window_open() -> bool {
    WINDOW_OPEN.load(Ordering::Relaxed)
}

pub fn open_settings_window(config: &AppConfig, tx: mpsc::Sender<AppConfig>) {
    if WINDOW_OPEN.load(Ordering::Relaxed) {
        // TODO: bring window to front
        return;
    }
    WINDOW_OPEN.store(true, Ordering::Relaxed);
    // TODO: implement Win32 settings window
    let _ = (config, tx);
    eprintln!("[ferrum] Win32 settings window not yet implemented");
    WINDOW_OPEN.store(false, Ordering::Relaxed);
}

pub fn close_settings_window() {
    WINDOW_OPEN.store(false, Ordering::Relaxed);
}
```

**Step 3: Create `src/gui/platform/windows/mod.rs`**

```rust
pub mod settings_window;
```

**Step 4: Create `src/gui/platform/linux/settings_window.rs`**

```rust
use crate::config::AppConfig;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);

pub fn is_settings_window_open() -> bool {
    WINDOW_OPEN.load(Ordering::Relaxed)
}

pub fn open_settings_window(config: &AppConfig, tx: mpsc::Sender<AppConfig>) {
    if WINDOW_OPEN.load(Ordering::Relaxed) {
        // TODO: bring window to front
        return;
    }
    WINDOW_OPEN.store(true, Ordering::Relaxed);
    // TODO: implement GTK4 settings window
    let _ = (config, tx);
    eprintln!("[ferrum] GTK4 settings window not yet implemented");
    WINDOW_OPEN.store(false, Ordering::Relaxed);
}

pub fn close_settings_window() {
    WINDOW_OPEN.store(false, Ordering::Relaxed);
}
```

**Step 5: Create `src/gui/platform/linux/mod.rs`**

```rust
pub mod settings_window;
```

**Step 6: Update `src/gui/platform/mod.rs`**

```rust
#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

mod close_dialog;

pub use close_dialog::confirm_window_close;
```

**Step 7: Verify compilation**

Run: `cargo build 2>&1 | head -30`
Expected: Should compile on macOS (Windows/Linux modules gated by cfg). Run `cargo clippy` too.

**Step 8: Commit**

```bash
git add -A
git commit -m "refactor: add platform stubs for Windows/Linux settings windows"
```

---

## Task 7: Implement Linux GTK4 settings window

**Files:**
- Modify: `Cargo.toml` — add `gtk4` dependency for Linux
- Modify: `src/gui/platform/linux/settings_window.rs` — full implementation

**Step 1: Add GTK4 dependency to Cargo.toml**

Under `[target.'cfg(target_os = "linux")'.dependencies]`:
```toml
gtk4 = "0.9"
```

**Step 2: Implement `src/gui/platform/linux/settings_window.rs`**

Full implementation following the macOS pattern. Key structure:

```rust
use crate::config::{AppConfig, FontFamily, SecurityMode, ThemeChoice};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};

static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);
static SETTINGS_STATE: Mutex<Option<GtkSettingsState>> = Mutex::new(None);

struct GtkSettingsState {
    // References to GTK widgets for reading values
}
```

**Architecture:**
1. `open_settings_window()`:
   - Spawns a new thread
   - In the thread: calls `gtk::init()`, creates `gtk::Window` with `gtk::Notebook` (5 tabs)
   - Each tab has a `gtk::Grid` with Label + Control pairs
   - Signal handlers on controls clone the `mpsc::Sender<AppConfig>` and send updated config on every change
   - The `close-request` signal saves config and sets `WINDOW_OPEN` to false
   - Runs `gtk::main()` (blocks the thread until window is closed)

2. **Controls per tab** (follow design doc exactly):
   - Font: SpinButton(float, 8.0-32.0, step 0.5), ComboBoxText(5 fonts), SpinButton(int, 0-10)
   - Theme: ComboBoxText(FerrumDark, FerrumLight)
   - Terminal: SpinButton(int, 0-50000, step 100), SpinButton(int, 100-2000, step 50)
   - Layout: SpinButton(int, 0-32), SpinButton(int, 0-16), SpinButton(int, 2-16), SpinButton(int, 24-48)
   - Security: ComboBoxText(Disabled/Standard/Custom), 4x Switch, with Security mode inference logic

3. **Config building**: Read all widget values, construct `AppConfig`, send through channel. Same `build_config_from_controls` pattern as macOS.

4. **Reset button**: Resets all controls to `AppConfig::default()` values.

5. **Window close detection**: `WINDOW_OPEN` atomic set to false when GTK window closes.

**Step 3: Verify compilation (Linux only — will need CI or Linux machine)**

On macOS this code is behind `#[cfg(target_os = "linux")]` so it won't compile locally.
Run: `cargo build` (should succeed on macOS — Linux code is gated).

**Step 4: Commit**

```bash
git add Cargo.toml src/gui/platform/linux/settings_window.rs
git commit -m "feat: implement native GTK4 settings window for Linux"
```

---

## Task 8: Implement Windows Win32 settings window

**Files:**
- Modify: `Cargo.toml` — expand `windows-sys` features for Win32 UI controls
- Modify: `src/gui/platform/windows/settings_window.rs` — full implementation

**Step 1: Expand `windows-sys` features in Cargo.toml**

Update the Windows dependencies section:
```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { version = "0.61", features = [
    "Win32_System_Diagnostics_ToolHelp",
    "Win32_System_Console",
    "Win32_System_Threading",
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Controls",
    "Win32_Graphics_Gdi",
    "Win32_System_LibraryLoader",
    "Win32_Graphics_Dwm",
] }
```

**Step 2: Implement `src/gui/platform/windows/settings_window.rs`**

Full implementation. Key structure:

```rust
use crate::config::{AppConfig, FontFamily, SecurityMode, ThemeChoice};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};

static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);
static SETTINGS_STATE: Mutex<Option<Win32SettingsState>> = Mutex::new(None);
```

**Architecture:**
1. `open_settings_window()`:
   - Spawns a new thread
   - Registers a window class (`RegisterClassExW`)
   - Creates the main window (`CreateWindowExW`) — titled, closable, 500x450
   - Creates a Tab Control (`WC_TABCONTROL`) — 5 tabs
   - For each tab, creates child controls (trackbars, comboboxes, checkboxes, labels)
   - Applies dark mode: `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE)`, `SetWindowTheme("DarkMode_Explorer")`
   - Runs the Win32 message loop (`GetMessageW`/`TranslateMessage`/`DispatchMessageW`)

2. **Window procedure** (`unsafe extern "system" fn wndproc`):
   - `WM_NOTIFY` + `TCN_SELCHANGE`: Switch visible tab controls
   - `WM_HSCROLL`: Trackbar value changed → send config
   - `WM_COMMAND` + `CBN_SELCHANGE`: Combobox changed → send config
   - `WM_COMMAND` + `BN_CLICKED`: Checkbox or reset button → send config
   - `WM_CLOSE`: Save config, destroy window, set `WINDOW_OPEN` false

3. **Controls per tab** (follow design doc):
   - Font: Trackbar(8-64, represents 8.0-32.0 in half-steps), ComboBox(5 fonts), Trackbar(0-10)
   - Theme: ComboBox(2 themes)
   - Terminal: Trackbar(0-500, represents 0-50000 in steps of 100), Trackbar(2-40, represents 100-2000 in steps of 50)
   - Layout: Trackbar + Label for each
   - Security: ComboBox + 4 Checkboxes

4. **Tab switching**: Show/hide groups of controls based on active tab. Store `HWND` for each control in the state.

5. **Config building**: Read all control values, construct `AppConfig`, send through `mpsc::Sender`.

**Step 3: Verify compilation (Windows only)**

On macOS this code is behind `#[cfg(target_os = "windows")]`.
Run: `cargo build` (should succeed on macOS).

**Step 4: Commit**

```bash
git add Cargo.toml src/gui/platform/windows/settings_window.rs
git commit -m "feat: implement native Win32 settings window for Windows"
```

---

## Task 9: Replace rfd with GTK4 for Linux close dialog

**Files:**
- Modify: `src/gui/platform/close_dialog.rs:110-123` — replace rfd with GTK4
- Modify: `Cargo.toml` — remove `rfd` dependency

**Step 1: Update Linux close dialog**

In `src/gui/platform/close_dialog.rs`, replace the `confirm_window_close_linux` function (lines 110-123):

```rust
#[cfg(target_os = "linux")]
fn confirm_window_close_linux(window: &Window) -> bool {
    let _ = window;

    use gtk4::prelude::*;
    use gtk4::{ButtonsType, DialogFlags, MessageDialog, MessageType, ResponseType};

    // Ensure GTK is initialized (safe to call multiple times).
    let _ = gtk4::init();

    let dialog = MessageDialog::new(
        None::<&gtk4::Window>,
        DialogFlags::MODAL,
        MessageType::Warning,
        ButtonsType::OkCancel,
        "Close Ferrum?",
    );
    dialog.set_secondary_text(Some(
        "Closing this terminal window will stop all running processes in its tabs.",
    ));
    dialog.set_title(Some("Close Ferrum"));

    let response = dialog.run();
    dialog.close();

    // Pump GTK events to ensure the dialog is fully closed.
    while gtk4::main_iteration_do(false) {}

    response == ResponseType::Ok
}
```

Note: GTK4 deprecated synchronous `run()` on dialogs. If `MessageDialog::run()` is not available in gtk4-rs 0.9, use an alternative approach:
- Use `gtk4::AlertDialog` (GTK 4.10+) with async pattern
- Or use a blocking `glib::MainContext` loop
- Or fall back to spawning `zenity` process as a simpler alternative

Test which approach works with the actual gtk4-rs API version.

**Step 2: Remove rfd from Cargo.toml**

Delete line 42: `rfd = "0.17"`

**Step 3: Verify no other rfd references remain**

```bash
grep -r "rfd" src/
```
Expected: No results.

**Step 4: Commit**

```bash
git add Cargo.toml src/gui/platform/close_dialog.rs
git commit -m "refactor: replace rfd with GTK4 for Linux close dialog"
```

---

## Task 10: Update gear button highlight for native settings

The gear button in the tab bar highlights when settings are open. Currently it checks `settings_overlay.is_some()`. Update it to check the native settings window state.

**Files:**
- Modify: `src/gui/events/render_shared.rs` — update `settings_open` parameter in `draw_tab_bar` call

**Step 1: Update render_shared.rs**

Where `settings_overlay.is_some()` was passed to `draw_tab_bar` (around the old line 433), replace with:

```rust
{
    #[cfg(target_os = "macos")]
    let settings_open = crate::gui::platform::macos::settings_window::is_settings_window_open();
    #[cfg(target_os = "windows")]
    let settings_open = crate::gui::platform::windows::settings_window::is_settings_window_open();
    #[cfg(target_os = "linux")]
    let settings_open = crate::gui::platform::linux::settings_window::is_settings_window_open();

    renderer.draw_tab_bar(
        &mut target,
        frame_tab_infos,
        params.hovered_tab,
        params.mouse_pos,
        tab_bar.tab_offsets.as_deref(),
        params.pinned,
        settings_open,
    );
}
```

Note: Consider extracting a helper `fn is_settings_window_open() -> bool` to avoid repeating the cfg blocks. Could go in `src/gui/platform/mod.rs`.

**Step 2: Verify compilation and run**

Run: `cargo build && cargo clippy`
Expected: Clean build, no warnings.

**Step 3: Commit**

```bash
git add src/gui/events/render_shared.rs
git commit -m "fix: update gear button highlight to check native settings window state"
```

---

## Task 11: Add lifecycle polling for Windows and Linux settings windows

The macOS implementation polls atomic flags in `about_to_wait`. Windows and Linux native windows send config directly through the channel, but we need to detect when the window is closed to save config.

**Files:**
- Modify: `src/gui/lifecycle/mod.rs` — add window close detection for Win/Linux

**Step 1: Add close detection in about_to_wait**

After the existing macOS settings polling block, add:

```rust
#[cfg(target_os = "windows")]
{
    if platform::windows::settings_window::check_window_closed() {
        // Config already sent through channel on close.
    }
}

#[cfg(target_os = "linux")]
{
    if platform::linux::settings_window::check_window_closed() {
        // Config already sent through channel on close.
    }
}
```

Add `check_window_closed() -> bool` to the Win32 and GTK4 stubs that checks and clears a "just closed" atomic flag.

**Step 2: Verify**

Run: `cargo build && cargo clippy`

**Step 3: Commit**

```bash
git add src/gui/lifecycle/mod.rs src/gui/platform/windows/settings_window.rs src/gui/platform/linux/settings_window.rs
git commit -m "feat: add lifecycle polling for Windows/Linux settings window close"
```

---

## Task 12: Run full test suite and clippy

**Step 1: Run tests**

Run: `cargo test`
Expected: All 353+ tests pass.

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: Zero warnings.

**Step 3: Fix any issues found**

Address any compilation errors, test failures, or clippy warnings.

**Step 4: Final commit**

```bash
git add -A
git commit -m "chore: fix clippy warnings and test issues after settings refactor"
```

---

## Task 13: Clean up unused imports and dead code

After all changes, grep for any remaining references to removed code.

**Step 1: Search for orphaned references**

```bash
grep -rn "SettingsOverlay\|SettingItem\|StepperHalf\|SettingsCategory\|settings_overlay\|pending_config\|draw_settings_overlay\|handle_settings_keyboard\|handle_settings_left_click\|handle_settings_mouse_move\|close_settings_overlay" src/
```

**Step 2: Clean up any remaining references**

Remove unused imports, dead code, orphaned comments.

**Step 3: Final clippy check**

Run: `cargo clippy`
Expected: Zero warnings.

**Step 4: Commit**

```bash
git add -A
git commit -m "chore: clean up dead references after overlay removal"
```

---

## Summary of Changes

| Action | Files |
|--------|-------|
| **Delete** | `src/gui/settings/overlay.rs`, `src/gui/settings/layout.rs`, `src/gui/settings/mod.rs`, `src/gui/renderer/settings.rs`, `src/gui/renderer/gpu/settings.rs`, `src/gui/events/mouse/settings.rs`, `src/gui/events/keyboard/settings.rs` |
| **Create** | `src/gui/platform/windows/mod.rs`, `src/gui/platform/windows/settings_window.rs`, `src/gui/platform/linux/mod.rs`, `src/gui/platform/linux/settings_window.rs`, `src/gui/events/settings_toggle.rs` |
| **Modify** | `Cargo.toml`, `src/gui/mod.rs`, `src/gui/state.rs`, `src/gui/platform/mod.rs`, `src/gui/platform/close_dialog.rs`, `src/gui/lifecycle/mod.rs`, `src/gui/events/mod.rs`, `src/gui/events/render_shared.rs`, `src/gui/events/render_cpu.rs`, `src/gui/events/render_gpu.rs`, `src/gui/events/settings_apply.rs`, `src/gui/events/keyboard/entry.rs`, `src/gui/events/mouse/input.rs`, `src/gui/events/mouse/mod.rs`, `src/gui/events/keyboard/mod.rs`, `src/gui/renderer/traits.rs`, `src/gui/renderer/gpu/trait_impl.rs`, `src/gui/renderer/cpu/trait_impl.rs`, `src/gui/renderer/gpu/mod.rs` |
| **Unchanged** | `src/gui/platform/macos/settings_window.rs`, `src/config/model.rs` |
| **Dependencies** | Add `gtk4` (Linux), expand `windows-sys` features (Windows), remove `rfd` |

## Execution Order

Tasks 1-6 can be done sequentially on macOS (all compile).
Tasks 7-8 (GTK4/Win32 implementations) need Linux/Windows respectively for testing — but compile on macOS behind cfg gates.
Task 9 (rfd → GTK4 close dialog) needs Linux for testing.
Tasks 10-13 are cleanup and verification.

**Critical path:** Tasks 1→2→3→4→5→6 must be done in order (each depends on the previous). Tasks 7 and 8 are independent of each other. Task 9 depends on Task 7 (GTK4 dependency). Tasks 10-13 depend on all previous.
