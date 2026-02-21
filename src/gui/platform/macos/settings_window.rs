use std::sync::Mutex;
use std::sync::mpsc;

use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSPopUpButton, NSStepper, NSTextField, NSWindow};

use crate::config::AppConfig;

/// Holds references to the native settings window and all its controls.
///
/// All `Retained<NS*>` fields are ObjC objects that are not `Send` by default.
/// This struct is only ever created and accessed on the main thread (enforced by
/// `MainThreadMarker` checks in `open_settings_window`), and the `Mutex` ensures
/// exclusive access. The `unsafe impl Send` is sound because AppKit objects are
/// safe to transfer between threads — they just must be *used* on the main thread,
/// which we guarantee.
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

// SAFETY: NativeSettingsState is only created and accessed on the main thread.
// The Mutex provides exclusive access. AppKit objects are safe to move between
// threads; they just must be used on the main thread, which we enforce via
// MainThreadMarker checks before any access.
unsafe impl Send for NativeSettingsState {}

static SETTINGS_STATE: Mutex<Option<NativeSettingsState>> = Mutex::new(None);

/// Returns true if the native settings window is currently open.
pub fn is_settings_window_open() -> bool {
    SETTINGS_STATE.lock().unwrap().is_some()
}

/// Opens the native macOS settings window. No-op if already open.
pub fn open_settings_window(_config: &AppConfig, _sender: mpsc::Sender<AppConfig>) {
    if is_settings_window_open() {
        // Bring existing window to front.
        if let Some(ref state) = *SETTINGS_STATE.lock().unwrap() {
            state.window.makeKeyAndOrderFront(None);
        }
        return;
    }

    let Some(_mtm) = objc2::MainThreadMarker::new() else {
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
