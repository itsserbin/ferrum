# Native macOS Settings Window — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the custom settings overlay on macOS with a native AppKit settings window (NSWindow + NSToolbar + NSStepper/NSPopUpButton), keeping the custom overlay for Linux/Windows.

**Architecture:** Platform-conditional (`#[cfg(target_os = "macos")]`). Native window created via objc2-app-kit. Control values sent to main event loop via `mpsc::channel<AppConfig>`. Existing `apply_config_change()` and `save_config()` reused. Static `Mutex<Option<NativeSettingsState>>` holds control references for ObjC action callbacks.

**Tech Stack:** Rust, objc2, objc2-app-kit, objc2-foundation (all existing deps), mpsc channels.

---

### Task 1: Settings channel infrastructure

**Files:**
- Modify: `src/gui/state.rs`
- Modify: `src/gui/lifecycle/mod.rs`

**Step 1: Add channel to App struct**

In `src/gui/state.rs`, add to `App` struct after `config` field:

```rust
/// Channel for receiving config changes from native settings window (macOS).
#[cfg(target_os = "macos")]
pub(super) settings_tx: std::sync::mpsc::Sender<crate::config::AppConfig>,
#[cfg(target_os = "macos")]
pub(super) settings_rx: std::sync::mpsc::Receiver<crate::config::AppConfig>,
```

Initialize in `App::new()` (or wherever `App` is constructed — check the code):

```rust
#[cfg(target_os = "macos")]
let (settings_tx, settings_rx) = std::sync::mpsc::channel();
```

**Step 2: Poll channel in `about_to_wait()`**

In `src/gui/lifecycle/mod.rs`, inside the `about_to_wait()` method, after the existing macOS gear button polling block, add:

```rust
#[cfg(target_os = "macos")]
{
    // Apply config changes from native settings window.
    while let Ok(new_config) = self.settings_rx.try_recv() {
        // Find focused window to apply changes.
        if let Some((_id, win)) = self.windows.iter_mut().find(|(_, w)| w.window.has_focus()) {
            win.apply_config_change(&new_config);
            win.window.request_redraw();
        }
        self.config = new_config;
    }
}
```

**Step 3: Build and verify**

Run: `cargo build`
Expected: Compiles. No functional change yet.

**Step 4: Commit**

```bash
git add src/gui/state.rs src/gui/lifecycle/mod.rs
git commit -m "feat(settings): add mpsc channel for native settings window config updates"
```

---

### Task 2: Check objc2-app-kit feature flags

**Files:**
- Modify: `Cargo.toml`

**Step 1: Check which features are needed**

The following AppKit classes are needed. Check the [objc2-app-kit docs](https://docs.rs/objc2-app-kit) to see which feature flags enable them:
- `NSWindow` — likely already enabled
- `NSView`, `NSStackView` — layout
- `NSTextField` — labels and value display
- `NSStepper` — increment/decrement control
- `NSPopUpButton` — dropdown
- `NSButton` — already enabled (used for pin button)
- `NSToolbar`, `NSToolbarItem` — category tabs
- `NSTabView`, `NSTabViewItem` — content switching
- `NSFont` — for label sizing

Run: `cargo doc -p objc2-app-kit --no-deps 2>&1 | head -20` to see what compiles.

**Step 2: Add required features to Cargo.toml**

In `Cargo.toml` under `[target.'cfg(target_os = "macos")'.dependencies]`, update `objc2-app-kit` features. Current features are likely limited. Add needed ones:

```toml
objc2-app-kit = { version = "0.3", features = [
    "NSAlert",
    "NSButton",
    "NSControl",
    "NSImage",
    "NSLayoutConstraint",
    "NSMenu",
    "NSMenuItem",
    "NSPopUpButton",
    "NSResponder",
    "NSStackView",
    "NSStepper",
    "NSTabView",
    "NSTabViewItem",
    "NSTextField",
    "NSToolbar",
    "NSToolbarItem",
    "NSTitlebarAccessoryViewController",
    "NSView",
    "NSWindow",
    "NSFont",
    "NSRunningApplication",
    "NSApplication",
] }
```

Note: The exact feature names may differ. Check docs.rs/objc2-app-kit. Only add what's actually needed.

**Step 3: Build and verify all features resolve**

Run: `cargo build`
Expected: Compiles with new features available. If a feature name is wrong, the build will tell you.

**Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add objc2-app-kit features for native settings window controls"
```

---

### Task 3: Scaffold settings_window.rs with NativeSettingsState

**Files:**
- Create: `src/gui/platform/macos/settings_window.rs`
- Modify: `src/gui/platform/macos/mod.rs`

**Step 1: Create the module file**

Create `src/gui/platform/macos/settings_window.rs` with the state structure:

```rust
use std::sync::mpsc;
use std::sync::Mutex;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSButton, NSPopUpButton, NSStepper, NSTextField, NSView, NSWindow,
};

use crate::config::AppConfig;

/// Holds references to the native settings window and all its controls.
struct NativeSettingsState {
    window: Retained<NSWindow>,
    sender: mpsc::Sender<AppConfig>,
    // Font
    font_size_stepper: Retained<NSStepper>,
    font_size_field: Retained<NSTextField>,
    font_family_popup: Retained<NSPopUpButton>,
    line_padding_stepper: Retained<NSStepper>,
    line_padding_field: Retained<NSTextField>,
    // Theme
    theme_popup: Retained<NSPopUpButton>,
    // Terminal
    scrollback_stepper: Retained<NSStepper>,
    scrollback_field: Retained<NSTextField>,
    cursor_blink_stepper: Retained<NSStepper>,
    cursor_blink_field: Retained<NSTextField>,
    // Layout
    window_padding_stepper: Retained<NSStepper>,
    window_padding_field: Retained<NSTextField>,
    tab_bar_height_stepper: Retained<NSStepper>,
    tab_bar_height_field: Retained<NSTextField>,
    pane_padding_stepper: Retained<NSStepper>,
    pane_padding_field: Retained<NSTextField>,
    scrollbar_width_stepper: Retained<NSStepper>,
    scrollbar_width_field: Retained<NSTextField>,
    // Reset
    reset_button: Retained<NSButton>,
}

static SETTINGS_STATE: Mutex<Option<NativeSettingsState>> = Mutex::new(None);

/// Returns true if the native settings window is currently open.
pub fn is_settings_window_open() -> bool {
    SETTINGS_STATE.lock().unwrap().is_some()
}

/// Opens the native macOS settings window. No-op if already open.
pub fn open_settings_window(config: &AppConfig, sender: mpsc::Sender<AppConfig>) {
    if is_settings_window_open() {
        // Bring existing window to front.
        if let Some(ref state) = *SETTINGS_STATE.lock().unwrap() {
            state.window.makeKeyAndOrderFront(None);
        }
        return;
    }

    let Some(_mtm) = MainThreadMarker::new() else {
        eprintln!("[ferrum] Settings window must be created on the main thread");
        return;
    };

    // TODO: Create window and controls in subsequent tasks.
    todo!("Task 4+: create NSWindow with controls");
}

/// Closes the native settings window and cleans up state.
pub fn close_settings_window() {
    if let Some(state) = SETTINGS_STATE.lock().unwrap().take() {
        state.window.close();
    }
}
```

**Step 2: Register module in mod.rs**

In `src/gui/platform/macos/mod.rs`, add:

```rust
pub mod settings_window;
```

And add to the pub use exports:

```rust
pub use settings_window::{open_settings_window, close_settings_window, is_settings_window_open};
```

**Step 3: Build**

Run: `cargo build`
Expected: Compiles (the `todo!()` is never called yet).

**Step 4: Commit**

```bash
git add src/gui/platform/macos/settings_window.rs src/gui/platform/macos/mod.rs
git commit -m "feat(settings): scaffold native macOS settings window module"
```

---

### Task 4: Create NSWindow with NSToolbar

**Files:**
- Modify: `src/gui/platform/macos/settings_window.rs`

**Step 1: Implement window creation**

Replace the `todo!()` in `open_settings_window` with NSWindow creation:

```rust
use objc2_foundation::{ns_string, NSRect, NSPoint, NSSize, NSString};
use objc2_app_kit::{
    NSBackingStoreType, NSWindowStyleMask, NSToolbar, NSToolbarItem,
    NSTabView, NSTabViewItem,
};

// Inside open_settings_window:

let mtm = MainThreadMarker::new().unwrap();

// Create window.
let style = NSWindowStyleMask::Titled
    | NSWindowStyleMask::Closable
    | NSWindowStyleMask::Miniaturizable;
let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(500.0, 400.0));

let window = unsafe {
    NSWindow::initWithContentRect_styleMask_backing_defer(
        mtm.alloc(),
        frame,
        style,
        NSBackingStoreType::NSBackingStoreBuffered,
        false,
    )
};

window.setTitle(ns_string!("Ferrum Settings"));
window.center();
window.setReleasedWhenClosed(false);

// Create tab view for content switching.
let tab_view = unsafe { NSTabView::initWithFrame(mtm.alloc(), frame) };

// Create 4 tabs.
let tab_names = ["Font", "Theme", "Terminal", "Layout"];
for name in &tab_names {
    let item = unsafe { NSTabViewItem::new(mtm) };
    item.setLabel(&NSString::from_str(name));
    let view = unsafe { NSView::initWithFrame(mtm.alloc(), frame) };
    item.setView(Some(&view));
    tab_view.addTabViewItem(&item);
}

window.setContentView(Some(&tab_view));
window.makeKeyAndOrderFront(None);
```

Note: The exact API may differ slightly. Consult `objc2-app-kit` docs for the init method signatures. Some methods may need `unsafe` blocks. Adjust as needed based on compiler errors.

**Step 2: Build and test manually**

Run: `cargo build && cargo run`
Then press Cmd+, (after wiring in Task 8). For now, just verify compilation.

**Step 3: Commit**

```bash
git add src/gui/platform/macos/settings_window.rs
git commit -m "feat(settings): create native NSWindow with NSTabView for settings"
```

---

### Task 5: Helper functions for control rows

**Files:**
- Modify: `src/gui/platform/macos/settings_window.rs`

**Step 1: Create stepper row helper**

```rust
/// Creates a label + NSTextField (value) + NSStepper row.
/// Returns (label_field, value_field, stepper).
fn create_stepper_row(
    mtm: MainThreadMarker,
    label: &str,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    y_offset: f64,
) -> (Retained<NSTextField>, Retained<NSTextField>, Retained<NSStepper>) {
    // Label (read-only).
    let label_field = unsafe {
        NSTextField::labelWithString(mtm, &NSString::from_str(label))
    };
    label_field.setFrame(NSRect::new(
        NSPoint::new(20.0, y_offset),
        NSSize::new(160.0, 24.0),
    ));

    // Value field (editable, shows current value).
    let value_field = unsafe {
        NSTextField::textFieldWithString(mtm, &NSString::from_str(&format!("{}", value)))
    };
    value_field.setFrame(NSRect::new(
        NSPoint::new(200.0, y_offset),
        NSSize::new(80.0, 24.0),
    ));

    // Stepper ([-] [+] arrows).
    let stepper = unsafe { NSStepper::initWithFrame(mtm.alloc(), NSRect::new(
        NSPoint::new(290.0, y_offset),
        NSSize::new(20.0, 24.0),
    )) };
    stepper.setMinValue(min);
    stepper.setMaxValue(max);
    stepper.setIncrement(step);
    stepper.setDoubleValue(value);
    stepper.setValueWraps(false);

    (label_field, value_field, stepper)
}
```

**Step 2: Create popup row helper**

```rust
/// Creates a label + NSPopUpButton row.
/// Returns (label_field, popup).
fn create_popup_row(
    mtm: MainThreadMarker,
    label: &str,
    options: &[&str],
    selected: usize,
    y_offset: f64,
) -> (Retained<NSTextField>, Retained<NSPopUpButton>) {
    let label_field = unsafe {
        NSTextField::labelWithString(mtm, &NSString::from_str(label))
    };
    label_field.setFrame(NSRect::new(
        NSPoint::new(20.0, y_offset),
        NSSize::new(160.0, 24.0),
    ));

    let popup = unsafe {
        NSPopUpButton::initWithFrame_pullsDown(
            mtm.alloc(),
            NSRect::new(NSPoint::new(200.0, y_offset), NSSize::new(200.0, 26.0)),
            false,
        )
    };
    for opt in options {
        popup.addItemWithTitle(&NSString::from_str(opt));
    }
    popup.selectItemAtIndex(selected as isize);

    (label_field, popup)
}
```

**Step 3: Build**

Run: `cargo build`
Expected: Compiles. Helpers not called yet.

**Step 4: Commit**

```bash
git add src/gui/platform/macos/settings_window.rs
git commit -m "feat(settings): add helper functions for stepper and popup control rows"
```

---

### Task 6: Build all category views with controls

**Files:**
- Modify: `src/gui/platform/macos/settings_window.rs`

**Step 1: Build Font tab controls**

In `open_settings_window`, after creating the tab view, populate each tab's content view. Access the tab view items and their views:

```rust
let font = &config.font;

// Font tab (index 0).
let font_view = tab_view.tabViewItemAtIndex(0).view().unwrap();

let (_, font_size_field, font_size_stepper) = create_stepper_row(
    mtm, "Font Size", font.size as f64, 8.0, 32.0, 0.5, 280.0,
);
font_view.addSubview(&font_size_field);
font_view.addSubview(&font_size_stepper);

let (_, font_family_popup) = create_popup_row(
    mtm, "Font Family", &["JetBrains Mono", "Fira Code"],
    match font.family { FontFamily::JetBrainsMono => 0, FontFamily::FiraCode => 1 },
    230.0,
);
font_view.addSubview(&font_family_popup);

let (_, line_padding_field, line_padding_stepper) = create_stepper_row(
    mtm, "Line Padding", font.line_padding as f64, 0.0, 10.0, 1.0, 180.0,
);
font_view.addSubview(&line_padding_field);
font_view.addSubview(&line_padding_stepper);
```

Repeat for all 4 tabs. Also add `addSubview` for the label fields returned by the helpers.

**Step 2: Build Theme tab (index 1)**

```rust
let (_, theme_popup) = create_popup_row(
    mtm, "Theme", &["Ferrum Dark", "Catppuccin Latte"],
    match config.theme { ThemeChoice::FerrumDark => 0, ThemeChoice::CatppuccinLatte => 1 },
    280.0,
);
let theme_view = tab_view.tabViewItemAtIndex(1).view().unwrap();
theme_view.addSubview(&theme_popup);
```

**Step 3: Build Terminal tab (index 2)**

```rust
let terminal = &config.terminal;
let term_view = tab_view.tabViewItemAtIndex(2).view().unwrap();

let (_, scrollback_field, scrollback_stepper) = create_stepper_row(
    mtm, "Max Scrollback", terminal.max_scrollback as f64, 0.0, 50000.0, 100.0, 280.0,
);
term_view.addSubview(&scrollback_field);
term_view.addSubview(&scrollback_stepper);

let (_, cursor_blink_field, cursor_blink_stepper) = create_stepper_row(
    mtm, "Cursor Blink (ms)", terminal.cursor_blink_interval_ms as f64, 100.0, 2000.0, 50.0, 230.0,
);
term_view.addSubview(&cursor_blink_field);
term_view.addSubview(&cursor_blink_stepper);
```

**Step 4: Build Layout tab (index 3)**

```rust
let layout = &config.layout;
let layout_view = tab_view.tabViewItemAtIndex(3).view().unwrap();

let (_, window_padding_field, window_padding_stepper) = create_stepper_row(
    mtm, "Window Padding", layout.window_padding as f64, 0.0, 32.0, 1.0, 280.0,
);
let (_, tab_bar_height_field, tab_bar_height_stepper) = create_stepper_row(
    mtm, "Tab Bar Height", layout.tab_bar_height as f64, 24.0, 60.0, 1.0, 230.0,
);
let (_, pane_padding_field, pane_padding_stepper) = create_stepper_row(
    mtm, "Pane Padding", layout.pane_inner_padding as f64, 0.0, 16.0, 1.0, 180.0,
);
let (_, scrollbar_width_field, scrollbar_width_stepper) = create_stepper_row(
    mtm, "Scrollbar Width", layout.scrollbar_width as f64, 2.0, 16.0, 1.0, 130.0,
);
// Add all to layout_view.
layout_view.addSubview(&window_padding_field);
layout_view.addSubview(&window_padding_stepper);
layout_view.addSubview(&tab_bar_height_field);
layout_view.addSubview(&tab_bar_height_stepper);
layout_view.addSubview(&pane_padding_field);
layout_view.addSubview(&pane_padding_stepper);
layout_view.addSubview(&scrollbar_width_field);
layout_view.addSubview(&scrollbar_width_stepper);
```

**Step 5: Store all controls in NativeSettingsState and save to static**

After creating all controls, build the state struct and store it:

```rust
let state = NativeSettingsState {
    window: window.clone(),
    sender,
    font_size_stepper, font_size_field,
    font_family_popup, line_padding_stepper, line_padding_field,
    theme_popup,
    scrollback_stepper, scrollback_field,
    cursor_blink_stepper, cursor_blink_field,
    window_padding_stepper, window_padding_field,
    tab_bar_height_stepper, tab_bar_height_field,
    pane_padding_stepper, pane_padding_field,
    scrollbar_width_stepper, scrollbar_width_field,
    reset_button: todo!("Task 7"),
};
*SETTINGS_STATE.lock().unwrap() = Some(state);
```

**Step 6: Build**

Run: `cargo build`
Expected: Compiles (reset_button is `todo!()` for now).

**Step 7: Commit**

```bash
git add src/gui/platform/macos/settings_window.rs
git commit -m "feat(settings): create all controls for 4 category tabs"
```

---

### Task 7: ObjC action callbacks for live config updates

**Files:**
- Modify: `src/gui/platform/macos/settings_window.rs`

**Step 1: Write the config rebuild + send function**

```rust
use crate::config::{FontConfig, FontFamily, TerminalConfig, LayoutConfig};

/// Reads all control values and sends updated config via channel.
fn send_current_config() {
    let guard = SETTINGS_STATE.lock().unwrap();
    let Some(state) = guard.as_ref() else { return };

    let config = AppConfig {
        font: FontConfig {
            size: state.font_size_stepper.doubleValue() as f32,
            family: match state.font_family_popup.indexOfSelectedItem() {
                0 => FontFamily::JetBrainsMono,
                _ => FontFamily::FiraCode,
            },
            line_padding: state.line_padding_stepper.integerValue() as u32,
        },
        theme: match state.theme_popup.indexOfSelectedItem() {
            0 => crate::config::ThemeChoice::FerrumDark,
            _ => crate::config::ThemeChoice::CatppuccinLatte,
        },
        terminal: TerminalConfig {
            max_scrollback: state.scrollback_stepper.integerValue() as usize,
            cursor_blink_interval_ms: state.cursor_blink_stepper.integerValue() as u64,
        },
        layout: LayoutConfig {
            window_padding: state.window_padding_stepper.integerValue() as u32,
            tab_bar_height: state.tab_bar_height_stepper.integerValue() as u32,
            pane_inner_padding: state.pane_padding_stepper.integerValue() as u32,
            scrollbar_width: state.scrollbar_width_stepper.integerValue() as u32,
        },
    };

    let _ = state.sender.send(config);
}
```

**Step 2: Write the ObjC callback**

Following the existing `handle_gear_button_click` pattern in `pin.rs`:

```rust
/// Atomic flag: any control value changed.
static SETTINGS_CHANGED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

unsafe extern "C" fn handle_settings_control_changed(
    _this: *mut core::ffi::c_void,
    _cmd: *const core::ffi::c_void,
    _sender: *mut core::ffi::c_void,
) {
    SETTINGS_CHANGED.store(true, std::sync::atomic::Ordering::SeqCst);
}
```

Then in the polling section (Task 1's `about_to_wait`), check the flag:

```rust
// In about_to_wait(), inside #[cfg(target_os = "macos")]:
if platform::macos::settings_window::take_settings_changed() {
    platform::macos::settings_window::send_current_config();
}
```

Add the take function:
```rust
pub fn take_settings_changed() -> bool {
    SETTINGS_CHANGED.swap(false, std::sync::atomic::Ordering::SeqCst)
}
```

**Step 3: Wire action to all controls**

In `open_settings_window`, after creating each stepper and popup, set its target and action. Use the same method injection pattern as `pin.rs`:

```rust
use crate::gui::platform::macos::ffi::{class_addMethod, sel_registerName, object_getClass};

// Register the shared action selector once.
let sel = sel_registerName(c"ferrumSettingsChanged:".as_ptr());
let cls = object_getClass(window.as_ref() as *const _ as *const core::ffi::c_void);
class_addMethod(
    cls as *mut _,
    sel,
    std::mem::transmute::<
        unsafe extern "C" fn(*mut core::ffi::c_void, *const core::ffi::c_void, *mut core::ffi::c_void),
        unsafe extern "C" fn(),
    >(handle_settings_control_changed),
    c"v@:@".as_ptr(),
);

// Set target/action for each control.
// For NSStepper/NSPopUpButton (which are NSControl subclasses):
let action_sel = objc2::sel!(ferrumSettingsChanged:);
font_size_stepper.setTarget(Some(&window));
font_size_stepper.setAction(Some(action_sel));
// Repeat for all steppers and popups.
```

Also, update the NSTextField value when the stepper changes. This can be done in the polling callback by updating all text fields:

```rust
fn update_text_fields() {
    let guard = SETTINGS_STATE.lock().unwrap();
    let Some(state) = guard.as_ref() else { return };

    state.font_size_field.setStringValue(&NSString::from_str(
        &format!("{:.1}", state.font_size_stepper.doubleValue()),
    ));
    state.line_padding_field.setStringValue(&NSString::from_str(
        &format!("{}", state.line_padding_stepper.integerValue()),
    ));
    // ... repeat for all stepper fields.
}
```

Call `update_text_fields()` alongside `send_current_config()` in the polling.

**Step 4: Build**

Run: `cargo build`
Expected: Compiles.

**Step 5: Commit**

```bash
git add src/gui/platform/macos/settings_window.rs src/gui/lifecycle/mod.rs
git commit -m "feat(settings): wire ObjC action callbacks for live config updates"
```

---

### Task 8: Reset to Defaults button + window close

**Files:**
- Modify: `src/gui/platform/macos/settings_window.rs`

**Step 1: Create Reset button**

In `open_settings_window`, create the reset button and add it to the window's content view (below the tab view, or as a separate view):

```rust
static RESET_REQUESTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

unsafe extern "C" fn handle_reset_clicked(
    _this: *mut core::ffi::c_void,
    _cmd: *const core::ffi::c_void,
    _sender: *mut core::ffi::c_void,
) {
    RESET_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
}

pub fn take_reset_requested() -> bool {
    RESET_REQUESTED.swap(false, std::sync::atomic::Ordering::SeqCst)
}
```

Create the button:

```rust
let reset_button = unsafe {
    NSButton::buttonWithTitle_target_action(
        mtm,
        ns_string!("Reset to Defaults"),
        Some(&window),
        Some(reset_action_sel),  // Register similar to settings changed
    )
};
reset_button.setFrame(NSRect::new(
    NSPoint::new(170.0, 10.0),
    NSSize::new(160.0, 30.0),
));
// Add to content view or a wrapper view.
```

In the polling section, handle reset:

```rust
if platform::macos::settings_window::take_reset_requested() {
    platform::macos::settings_window::reset_controls_to_defaults();
    platform::macos::settings_window::send_current_config();
}
```

Implement `reset_controls_to_defaults`:

```rust
pub fn reset_controls_to_defaults() {
    let guard = SETTINGS_STATE.lock().unwrap();
    let Some(state) = guard.as_ref() else { return };
    let defaults = AppConfig::default();

    state.font_size_stepper.setDoubleValue(defaults.font.size as f64);
    state.line_padding_stepper.setIntegerValue(defaults.font.line_padding as isize);
    state.font_family_popup.selectItemAtIndex(0); // JetBrainsMono = 0
    state.theme_popup.selectItemAtIndex(0); // FerrumDark = 0
    state.scrollback_stepper.setIntegerValue(defaults.terminal.max_scrollback as isize);
    state.cursor_blink_stepper.setIntegerValue(defaults.terminal.cursor_blink_interval_ms as isize);
    state.window_padding_stepper.setIntegerValue(defaults.layout.window_padding as isize);
    state.tab_bar_height_stepper.setIntegerValue(defaults.layout.tab_bar_height as isize);
    state.pane_padding_stepper.setIntegerValue(defaults.layout.pane_inner_padding as isize);
    state.scrollbar_width_stepper.setIntegerValue(defaults.layout.scrollbar_width as isize);

    update_text_fields();
}
```

**Step 2: Handle window close → save config**

Use `NSNotificationCenter` to observe `NSWindowWillCloseNotification`, or check in polling if the window is no longer visible:

In the polling section:

```rust
if platform::macos::settings_window::is_settings_window_open() {
    // Check if window was closed by the user.
    let guard = SETTINGS_STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        if !state.window.isVisible() {
            drop(guard);
            // Save current config.
            let final_config = /* read from controls */;
            crate::config::save_config(&final_config);
            platform::macos::settings_window::close_settings_window();
        }
    }
}
```

**Step 3: Build**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/gui/platform/macos/settings_window.rs src/gui/lifecycle/mod.rs
git commit -m "feat(settings): add Reset to Defaults button and window close persistence"
```

---

### Task 9: Wire Cmd+, to native window on macOS

**Files:**
- Modify: `src/gui/events/keyboard/settings.rs`
- Modify: `src/gui/events/keyboard/shortcuts.rs`

**Step 1: Platform-conditional toggle**

In `src/gui/events/keyboard/settings.rs`, change `toggle_settings_overlay`:

```rust
pub(in crate::gui) fn toggle_settings_overlay(
    &mut self,
    config: &AppConfig,
    #[cfg(target_os = "macos")]
    settings_tx: &std::sync::mpsc::Sender<AppConfig>,
) {
    #[cfg(target_os = "macos")]
    {
        use crate::gui::platform::macos::settings_window;
        if settings_window::is_settings_window_open() {
            settings_window::close_settings_window();
        } else {
            settings_window::open_settings_window(config, settings_tx.clone());
        }
        return;
    }

    #[cfg(not(target_os = "macos"))]
    {
        if self.settings_overlay.is_some() {
            self.close_settings_overlay();
        } else {
            self.settings_overlay = Some(crate::gui::settings::SettingsOverlay::new(config));
            self.window.request_redraw();
        }
    }
}
```

**Step 2: Update callers**

In `src/gui/events/keyboard/shortcuts.rs`, update the Cmd+, handler to pass `settings_tx`:

```rust
if Self::physical_key_is(physical, KeyCode::Comma) {
    self.toggle_settings_overlay(
        config,
        #[cfg(target_os = "macos")]
        settings_tx,
    );
    return true;
}
```

This requires threading `settings_tx` through to `handle_ctrl_shortcuts`. Check the call chain and add the parameter where needed.

Also update the gear button handler in `src/gui/lifecycle/mod.rs` to call the new platform-conditional toggle.

**Step 3: Build and test manually**

Run: `cargo build && cargo run`
Press Cmd+, — should open native settings window on macOS.

**Step 4: Commit**

```bash
git add src/gui/events/keyboard/settings.rs src/gui/events/keyboard/shortcuts.rs src/gui/lifecycle/mod.rs
git commit -m "feat(settings): wire Cmd+, to native settings window on macOS"
```

---

### Task 10: Final verification and cleanup

**Files:**
- All modified files

**Step 1: Run all tests**

Run: `cargo test -- --nocapture`
Expected: All existing tests pass.

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: Zero warnings. Fix any that appear.

**Step 3: Build CPU-only mode**

Run: `cargo build --no-default-features`
Expected: Compiles. The macOS settings code is independent of GPU/CPU.

**Step 4: Manual testing on macOS**

1. `cargo run` — run the terminal
2. Press Cmd+, — native settings window opens
3. Change Font Size via stepper — terminal updates live
4. Switch to Theme tab — change theme — terminal recolors live
5. Click Reset to Defaults — all values reset
6. Close settings window — config persisted
7. Reopen app — settings retained

**Step 5: Verify non-macOS compilation**

If you have access, verify that `cargo build` still works on Linux/Windows (the `#[cfg(not(target_os = "macos"))]` path should use the existing overlay).

**Step 6: Commit cleanup**

```bash
git add -A
git commit -m "chore(settings): cleanup and verify native macOS settings window"
```

---

## Notes

- **objc2-app-kit API may differ from examples above.** The exact method signatures, init patterns, and unsafe requirements depend on the crate version. Always check compiler errors and the [objc2-app-kit docs](https://docs.rs/objc2-app-kit).
- **Memory management:** `Retained<T>` handles ARC reference counting automatically. Keep references alive in `NativeSettingsState` as long as the window is open.
- **Thread safety:** All NSWindow creation and control manipulation must happen on the main thread (enforced by `MainThreadMarker`). The `send_current_config()` function locks the mutex briefly to read values.
- **The existing custom overlay** (`src/gui/settings/`) is NOT deleted — it continues to serve Linux/Windows.
