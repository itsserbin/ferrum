# Native Settings Windows for Windows & Linux

**Date:** 2026-02-21
**Status:** Approved

## Problem

Settings UI on Windows and Linux uses a custom-drawn overlay panel rendered by Ferrum's GPU/CPU renderer. This overlay looks non-native, has custom controls (steppers, dropdowns, toggles) that don't match the OS, and is awkward to use. macOS already has a proper native `NSWindow` with AppKit controls.

## Decision

Replace the custom overlay with native platform windows:
- **macOS** — keep existing `NSWindow` + AppKit implementation (no changes)
- **Windows** — new `Win32 API` window via `windows` crate
- **Linux** — new `GTK4` window via `gtk4-rs` crate
- Remove `rfd` dependency (GTK4 covers Linux dialogs, macOS/Windows have native alternatives)
- Delete all custom overlay code

## Architecture

### Shared Interface

Each platform module exports the same public API:

```rust
pub fn open_settings_window(config: &AppConfig);
pub fn close_settings_window() -> Option<AppConfig>;
pub fn is_settings_window_open() -> bool;
pub fn check_window_closed() -> bool;
pub fn send_current_config() -> Option<AppConfig>;
```

### File Structure

```
src/gui/platform/
├── macos/settings_window.rs    # existing — NSWindow + AppKit
├── windows/settings_window.rs  # NEW — Win32 API
└── linux/settings_window.rs    # NEW — GTK4
```

### Shared Logic (unchanged)

- `src/config/model.rs` — `AppConfig` data model
- `src/gui/events/settings_apply.rs` — `apply_config_change()`
- `src/config/` — `save_config()` persistence
- `src/gui/lifecycle/mod.rs` — mpsc channel polling loop

### Code to Remove

- `src/gui/settings/overlay.rs` — overlay data model
- `src/gui/settings/layout.rs` — overlay layout computation
- `src/gui/renderer/settings.rs` — CPU overlay rendering
- `src/gui/renderer/gpu/settings.rs` — GPU overlay rendering
- `src/gui/events/mouse/settings.rs` — overlay mouse handling
- `src/gui/events/keyboard/settings.rs` — overlay-specific keyboard handling
- All `settings_overlay_open`, `SettingsOverlayLayout` references
- `rfd` dependency from `Cargo.toml`

## Windows Implementation (Win32 API)

**Crate:** `windows` (official Microsoft crate)

**Window structure:**
- `CreateWindowExW` — main dialog window
- `WC_TABCONTROL` — 5 tabs (Font, Theme, Terminal, Layout, Security)

**Controls per tab:**

| Setting | Win32 Control |
|---------|---------------|
| Font Size (float slider) | `TRACKBAR_CLASS` + `WC_STATIC` label |
| Font Family (enum) | `WC_COMBOBOX` |
| Line Padding (int slider) | `TRACKBAR_CLASS` + label |
| Theme (enum) | `WC_COMBOBOX` |
| Max Scrollback (large int) | `TRACKBAR_CLASS` + label |
| Cursor Blink ms (large int) | `TRACKBAR_CLASS` + label |
| Layout numbers (int) | `TRACKBAR_CLASS` + label |
| Security Mode (enum) | `WC_COMBOBOX` |
| Security toggles (bool) | `BS_AUTOCHECKBOX` |
| Reset to Defaults | `BS_PUSHBUTTON` |

**Event flow:**
1. `open_settings_window()` creates window in separate thread (Win32 message loop)
2. Controls generate `WM_COMMAND` / `WM_HSCROLL` / `WM_NOTIFY`
3. Window procedure processes messages, updates config via atomic flags
4. Lifecycle loop in main thread polls `send_current_config()`
5. `WM_CLOSE` → save config, destroy window

**Dark mode:** `DwmSetWindowAttribute` with `DWMWA_USE_IMMERSIVE_DARK_MODE` for title bar. `SetWindowTheme("DarkMode_Explorer")` for controls (Windows 10+).

## Linux Implementation (GTK4)

**Crate:** `gtk4` (gtk4-rs)

**Window structure:**
- `gtk::Window` — standalone window
- `gtk::Notebook` — 5 tabs

**Controls per tab:**

| Setting | GTK4 Widget |
|---------|-------------|
| Font Size (float) | `gtk::SpinButton` (step 0.5) |
| Font Family (enum) | `gtk::ComboBoxText` |
| Line Padding (int) | `gtk::SpinButton` |
| Theme (enum) | `gtk::ComboBoxText` |
| Max Scrollback (large int) | `gtk::SpinButton` (step 100) |
| Cursor Blink ms (large int) | `gtk::SpinButton` (step 50) |
| Layout numbers (int) | `gtk::SpinButton` |
| Security Mode (enum) | `gtk::ComboBoxText` |
| Security toggles (bool) | `gtk::Switch` |
| Reset to Defaults | `gtk::Button` |

**Layout:** `gtk::Grid` for label + control alignment within each tab.

**Event flow:**
1. `open_settings_window()` — GTK4 runs in separate thread with its own event loop
2. Signal handlers (`connect_value_changed`, `connect_changed`) on controls
3. Changes sent via mpsc channel to lifecycle loop
4. `connect_close_request` → save config

**Theming:** GTK4 automatically picks up GNOME system theme (including dark mode via Adwaita).

## Settings Categories & Controls (all platforms)

**Font:** Font Size (8.0-32.0, step 0.5), Font Family (5 options), Line Padding (0-10)
**Theme:** Theme choice (FerrumDark, FerrumLight)
**Terminal:** Max Scrollback (0-50000, step 100), Cursor Blink ms (100-2000, step 50)
**Layout:** Window Padding (0-32), Pane Padding (0-16), Scrollbar Width (2-16), Tab Bar Height (24-48, non-macOS only)
**Security:** Security Mode (Disabled/Standard/Custom), Paste Protection, Block Title Query, Limit Cursor Jumps, Clear Mouse on Reset

## Dependencies

### Add
- `gtk4` — Linux settings window (cfg target_os = "linux")
- `windows` crate features for Win32 UI controls (cfg target_os = "windows") — may already be partially present

### Remove
- `rfd` — replaced by native dialogs on each platform

## Live Preview & Persistence

No changes to existing flow:
1. Platform window sends `AppConfig` through mpsc channel
2. Lifecycle loop calls `apply_config_change()` for live preview
3. On window close: `save_config()` persists to disk
