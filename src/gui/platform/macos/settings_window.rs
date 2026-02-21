use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::mpsc;

use objc2::MainThreadMarker;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2_app_kit::{
    NSBackingStoreType, NSButton, NSPopUpButton, NSStepper, NSTabView, NSTabViewItem,
    NSTextField, NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, ns_string};

use super::ffi::{class_addMethod, class_replaceMethod, object_getClass, sel_registerName};
use crate::config::{
    AppConfig, FontConfig, FontFamily, LayoutConfig, SecurityMode, SecuritySettings, TerminalConfig,
    ThemeChoice,
};

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
    // Layout (Tab Bar Height removed — macOS uses native tab bar)
    window_padding_stepper: Retained<NSStepper>,
    window_padding_field: Retained<NSTextField>,
    pane_padding_stepper: Retained<NSStepper>,
    pane_padding_field: Retained<NSTextField>,
    scrollbar_width_stepper: Retained<NSStepper>,
    scrollbar_width_field: Retained<NSTextField>,
    // Security
    security_mode_popup: Retained<NSPopUpButton>,
    paste_protection_check: Retained<NSButton>,
    block_title_query_check: Retained<NSButton>,
    limit_cursor_jumps_check: Retained<NSButton>,
    clear_mouse_on_reset_check: Retained<NSButton>,
    // Reset (kept alive so ObjC retains the button; never read from Rust).
    #[allow(dead_code)]
    reset_button: Retained<NSButton>,
}

// SAFETY: NativeSettingsState is only created and accessed on the main thread.
// The Mutex provides exclusive access. AppKit objects are safe to move between
// threads; they just must be used on the main thread, which we enforce via
// MainThreadMarker checks before any access.
unsafe impl Send for NativeSettingsState {}

static SETTINGS_STATE: Mutex<Option<NativeSettingsState>> = Mutex::new(None);

/// Atomic flag: a stepper or popup value changed.
static STEPPER_CHANGED: AtomicBool = AtomicBool::new(false);

/// Atomic flag: a text field value was committed (Enter or focus lost).
static TEXT_FIELD_CHANGED: AtomicBool = AtomicBool::new(false);

/// Atomic flag: Reset to Defaults was clicked.
static RESET_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Tracks the last known security mode popup index so `sync_security_controls`
/// can distinguish popup changes (apply presets) from checkbox changes (infer mode).
/// Initialised to 1 = Standard (the default).
static LAST_SECURITY_MODE_INDEX: AtomicIsize = AtomicIsize::new(1);

/// ObjC action handler for steppers and popups.
///
/// # Safety
///
/// Called by the Objective-C runtime as a method implementation.
/// Signature matches the type encoding "v@:@".
unsafe extern "C" fn handle_stepper_changed(
    _this: *mut core::ffi::c_void,
    _cmd: *const core::ffi::c_void,
    _sender: *mut core::ffi::c_void,
) {
    STEPPER_CHANGED.store(true, Ordering::SeqCst);
}

/// ObjC action handler for editable text fields.
///
/// # Safety
///
/// Called by the Objective-C runtime as a method implementation.
/// Signature matches the type encoding "v@:@".
unsafe extern "C" fn handle_text_field_changed(
    _this: *mut core::ffi::c_void,
    _cmd: *const core::ffi::c_void,
    _sender: *mut core::ffi::c_void,
) {
    TEXT_FIELD_CHANGED.store(true, Ordering::SeqCst);
}

/// ObjC action handler for the Reset to Defaults button.
///
/// # Safety
///
/// Called by the Objective-C runtime as a method implementation.
/// Signature matches the type encoding "v@:@".
unsafe extern "C" fn handle_reset_clicked(
    _this: *mut core::ffi::c_void,
    _cmd: *const core::ffi::c_void,
    _sender: *mut core::ffi::c_void,
) {
    RESET_REQUESTED.store(true, Ordering::SeqCst);
}

/// Returns and resets the stepper-changed flag.
pub fn take_stepper_changed() -> bool {
    STEPPER_CHANGED.swap(false, Ordering::SeqCst)
}

/// Returns and resets the text-field-changed flag.
pub fn take_text_field_changed() -> bool {
    TEXT_FIELD_CHANGED.swap(false, Ordering::SeqCst)
}

/// Returns and resets the reset-requested flag.
pub fn take_reset_requested() -> bool {
    RESET_REQUESTED.swap(false, Ordering::SeqCst)
}

/// Builds an `AppConfig` from the current control values.
fn build_config_from_controls(state: &NativeSettingsState) -> AppConfig {
    let security_mode = match state.security_mode_popup.indexOfSelectedItem() {
        0 => SecurityMode::Disabled,
        1 => SecurityMode::Standard,
        _ => SecurityMode::Custom,
    };
    AppConfig {
        font: FontConfig {
            size: state.font_size_stepper.doubleValue() as f32,
            family: FontFamily::from_index(
                state.font_family_popup.indexOfSelectedItem() as usize,
            ),
            line_padding: state.line_padding_stepper.integerValue() as u32,
        },
        theme: match state.theme_popup.indexOfSelectedItem() {
            0 => ThemeChoice::FerrumDark,
            _ => ThemeChoice::FerrumLight,
        },
        terminal: TerminalConfig {
            max_scrollback: state.scrollback_stepper.integerValue() as usize,
            cursor_blink_interval_ms: state.cursor_blink_stepper.integerValue() as u64,
        },
        layout: LayoutConfig {
            window_padding: state.window_padding_stepper.integerValue() as u32,
            tab_bar_height: AppConfig::default().layout.tab_bar_height,
            pane_inner_padding: state.pane_padding_stepper.integerValue() as u32,
            scrollbar_width: state.scrollbar_width_stepper.integerValue() as u32,
        },
        security: SecuritySettings {
            mode: security_mode,
            paste_protection: is_checkbox_on(&state.paste_protection_check),
            block_title_query: is_checkbox_on(&state.block_title_query_check),
            limit_cursor_jumps: is_checkbox_on(&state.limit_cursor_jumps_check),
            clear_mouse_on_reset: is_checkbox_on(&state.clear_mouse_on_reset_check),
        },
    }
}

/// Returns `true` if the checkbox (`NSButton` with switch style) is in the ON state.
fn is_checkbox_on(button: &NSButton) -> bool {
    // NSControlStateValueOn = 1
    button.state() == 1
}

/// Syncs security checkboxes and mode popup.
///
/// Uses `LAST_SECURITY_MODE_INDEX` to distinguish popup changes from checkbox changes:
/// - Popup changed → apply presets (force checkbox values for Standard/Disabled).
/// - Checkbox changed → infer mode from current values and update popup.
fn sync_security_controls(state: &NativeSettingsState) {
    let current_index = state.security_mode_popup.indexOfSelectedItem();
    let last_index = LAST_SECURITY_MODE_INDEX.swap(current_index, Ordering::SeqCst);

    let all_checks = [
        &state.paste_protection_check,
        &state.block_title_query_check,
        &state.limit_cursor_jumps_check,
        &state.clear_mouse_on_reset_check,
    ];

    if current_index != last_index {
        // Popup was changed by the user → apply presets.
        match current_index {
            0 => {
                // Disabled: force all off, disable checkboxes.
                for cb in &all_checks {
                    set_checkbox(cb, false);
                    cb.setEnabled(false);
                }
            }
            1 => {
                // Standard: force all on, enable checkboxes.
                for cb in &all_checks {
                    set_checkbox(cb, true);
                    cb.setEnabled(true);
                }
            }
            _ => {
                // Custom: just enable checkboxes, keep their current values.
                for cb in &all_checks {
                    cb.setEnabled(true);
                }
            }
        }
    } else {
        // Checkbox was changed → infer mode from current values.
        let settings = SecuritySettings {
            mode: SecurityMode::Custom,
            paste_protection: is_checkbox_on(&state.paste_protection_check),
            block_title_query: is_checkbox_on(&state.block_title_query_check),
            limit_cursor_jumps: is_checkbox_on(&state.limit_cursor_jumps_check),
            clear_mouse_on_reset: is_checkbox_on(&state.clear_mouse_on_reset_check),
        };
        let inferred = settings.inferred_mode();
        let new_index = match inferred {
            SecurityMode::Disabled => 0,
            SecurityMode::Standard => 1,
            SecurityMode::Custom => 2,
        };
        state.security_mode_popup.selectItemAtIndex(new_index);
        LAST_SECURITY_MODE_INDEX.store(new_index, Ordering::SeqCst);

        // If all toggled off → Disabled: disable checkboxes.
        if matches!(inferred, SecurityMode::Disabled) {
            for cb in &all_checks {
                cb.setEnabled(false);
            }
        }
    }
}

/// Reads all control values and sends the resulting config through the channel.
pub fn send_current_config() {
    let guard = SETTINGS_STATE.lock().unwrap();
    let Some(state) = guard.as_ref() else {
        return;
    };
    sync_security_controls(state);
    let config = build_config_from_controls(state);
    let _ = state.sender.send(config);
}

/// Parses editable text field values and updates the corresponding steppers.
/// Called before reading stepper values so manual text input is reflected.
pub fn sync_text_fields_to_steppers() {
    let guard = SETTINGS_STATE.lock().unwrap();
    let Some(state) = guard.as_ref() else {
        return;
    };

    // Helper: parse text field, clamp to stepper range, update stepper.
    fn sync_float(field: &NSTextField, stepper: &NSStepper) {
        let text = field.stringValue();
        if let Ok(val) = text.to_string().parse::<f64>() {
            let clamped = val.clamp(stepper.minValue(), stepper.maxValue());
            stepper.setDoubleValue(clamped);
        }
    }
    fn sync_int(field: &NSTextField, stepper: &NSStepper) {
        let text = field.stringValue();
        if let Ok(val) = text.to_string().parse::<f64>() {
            let clamped = val.clamp(stepper.minValue(), stepper.maxValue());
            stepper.setDoubleValue(clamped.round());
        }
    }

    sync_float(&state.font_size_field, &state.font_size_stepper);
    sync_int(&state.line_padding_field, &state.line_padding_stepper);
    sync_int(&state.scrollback_field, &state.scrollback_stepper);
    sync_int(&state.cursor_blink_field, &state.cursor_blink_stepper);
    sync_int(&state.window_padding_field, &state.window_padding_stepper);
    sync_int(&state.pane_padding_field, &state.pane_padding_stepper);
    sync_int(&state.scrollbar_width_field, &state.scrollbar_width_stepper);
}

/// Updates all text fields to match the current stepper values.
pub fn update_text_fields() {
    let guard = SETTINGS_STATE.lock().unwrap();
    let Some(state) = guard.as_ref() else {
        return;
    };

    state.font_size_field.setStringValue(&NSString::from_str(
        &format!("{:.1}", state.font_size_stepper.doubleValue()),
    ));
    state.line_padding_field.setStringValue(&NSString::from_str(
        &format!("{}", state.line_padding_stepper.integerValue()),
    ));
    state.scrollback_field.setStringValue(&NSString::from_str(
        &format!("{}", state.scrollback_stepper.integerValue()),
    ));
    state
        .cursor_blink_field
        .setStringValue(&NSString::from_str(&format!(
            "{}",
            state.cursor_blink_stepper.integerValue()
        )));
    state
        .window_padding_field
        .setStringValue(&NSString::from_str(&format!(
            "{}",
            state.window_padding_stepper.integerValue()
        )));
    state
        .pane_padding_field
        .setStringValue(&NSString::from_str(&format!(
            "{}",
            state.pane_padding_stepper.integerValue()
        )));
    state
        .scrollbar_width_field
        .setStringValue(&NSString::from_str(&format!(
            "{}",
            state.scrollbar_width_stepper.integerValue()
        )));
}

/// Resets all controls to their default values and updates text fields.
pub fn reset_controls_to_defaults() {
    let guard = SETTINGS_STATE.lock().unwrap();
    let Some(state) = guard.as_ref() else {
        return;
    };
    let defaults = AppConfig::default();

    state
        .font_size_stepper
        .setDoubleValue(f64::from(defaults.font.size));
    state
        .line_padding_stepper
        .setIntegerValue(defaults.font.line_padding as isize);
    state.font_family_popup.selectItemAtIndex(0); // JetBrainsMono = default
    state.theme_popup.selectItemAtIndex(0); // FerrumDark = default
    state
        .scrollback_stepper
        .setIntegerValue(defaults.terminal.max_scrollback as isize);
    state
        .cursor_blink_stepper
        .setIntegerValue(defaults.terminal.cursor_blink_interval_ms as isize);
    state
        .window_padding_stepper
        .setIntegerValue(defaults.layout.window_padding as isize);
    state
        .pane_padding_stepper
        .setIntegerValue(defaults.layout.pane_inner_padding as isize);
    state
        .scrollbar_width_stepper
        .setIntegerValue(defaults.layout.scrollbar_width as isize);
    // Security: Standard mode, all toggles on.
    state.security_mode_popup.selectItemAtIndex(1); // Standard
    LAST_SECURITY_MODE_INDEX.store(1, Ordering::SeqCst);
    set_checkbox(&state.paste_protection_check, true);
    set_checkbox(&state.block_title_query_check, true);
    set_checkbox(&state.limit_cursor_jumps_check, true);
    set_checkbox(&state.clear_mouse_on_reset_check, true);
    state.paste_protection_check.setEnabled(true);
    state.block_title_query_check.setEnabled(true);
    state.limit_cursor_jumps_check.setEnabled(true);
    state.clear_mouse_on_reset_check.setEnabled(true);

    drop(guard);
    update_text_fields();
}

/// Sets a checkbox state. ON = 1, OFF = 0.
fn set_checkbox(button: &NSButton, on: bool) {
    button.setState(if on { 1 } else { 0 });
}

/// Returns true if the settings window exists but is no longer visible.
pub fn check_window_closed() -> bool {
    let guard = SETTINGS_STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        return !state.window.isVisible();
    }
    false
}

/// Creates a label + NSTextField (value display) + NSStepper row.
/// Returns (value_field, stepper). The label is added to the parent view.
#[allow(clippy::too_many_arguments)]
fn create_stepper_row(
    mtm: MainThreadMarker,
    parent: &NSView,
    label_text: &str,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    y_offset: f64,
) -> (Retained<NSTextField>, Retained<NSStepper>) {
    let label = NSTextField::labelWithString(&NSString::from_str(label_text), mtm);
    label.setFrame(NSRect::new(
        NSPoint::new(20.0, y_offset),
        NSSize::new(160.0, 24.0),
    ));
    parent.addSubview(&label);

    let value_str = if step < 1.0 {
        format!("{:.1}", value)
    } else {
        format!("{}", value as i64)
    };
    let value_field = NSTextField::textFieldWithString(&NSString::from_str(&value_str), mtm);
    value_field.setFrame(NSRect::new(
        NSPoint::new(200.0, y_offset),
        NSSize::new(80.0, 24.0),
    ));
    value_field.setEditable(true);
    value_field.setBezeled(true);
    parent.addSubview(&value_field);

    let stepper = NSStepper::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::new(290.0, y_offset), NSSize::new(20.0, 24.0)),
    );
    stepper.setMinValue(min);
    stepper.setMaxValue(max);
    stepper.setIncrement(step);
    stepper.setDoubleValue(value);
    stepper.setValueWraps(false);
    parent.addSubview(&stepper);

    (value_field, stepper)
}

/// Creates a label + NSPopUpButton row.
/// Returns the popup. The label is added to the parent view.
fn create_popup_row(
    mtm: MainThreadMarker,
    parent: &NSView,
    label_text: &str,
    options: &[&str],
    selected: usize,
    y_offset: f64,
) -> Retained<NSPopUpButton> {
    let label = NSTextField::labelWithString(&NSString::from_str(label_text), mtm);
    label.setFrame(NSRect::new(
        NSPoint::new(20.0, y_offset),
        NSSize::new(160.0, 24.0),
    ));
    parent.addSubview(&label);

    let popup = NSPopUpButton::initWithFrame_pullsDown(
        mtm.alloc(),
        NSRect::new(NSPoint::new(200.0, y_offset), NSSize::new(200.0, 26.0)),
        false,
    );
    for opt in options {
        popup.addItemWithTitle(&NSString::from_str(opt));
    }
    popup.selectItemAtIndex(selected as isize);
    parent.addSubview(&popup);

    popup
}

/// Creates a checkbox with a small description label underneath.
/// Returns the checkbox button.
fn create_checkbox_row(
    mtm: MainThreadMarker,
    parent: &NSView,
    label_text: &str,
    description: &str,
    checked: bool,
    y_offset: f64,
) -> Retained<NSButton> {
    let checkbox = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str(label_text),
            None,
            None,
            mtm,
        )
    };
    checkbox.setFrame(NSRect::new(
        NSPoint::new(20.0, y_offset),
        NSSize::new(400.0, 18.0),
    ));
    set_checkbox(&checkbox, checked);
    parent.addSubview(&checkbox);

    // Small description label below the checkbox.
    let desc_label = NSTextField::wrappingLabelWithString(
        &NSString::from_str(description),
        mtm,
    );
    // x=40 aligns with the checkbox label text (checkbox indicator is ~18px wide).
    desc_label.setFrame(NSRect::new(
        NSPoint::new(40.0, y_offset - 16.0),
        NSSize::new(380.0, 14.0),
    ));
    use objc2_app_kit::{NSColor, NSFont};
    desc_label.setFont(Some(&NSFont::systemFontOfSize(11.0)));
    desc_label.setTextColor(Some(&NSColor::secondaryLabelColor()));
    parent.addSubview(&desc_label);

    checkbox
}

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

    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("[ferrum] Settings window must be created on the main thread");
        return;
    };

    // ── NSWindow ──────────────────────────────────────────────────────

    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable;
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(500.0, 400.0));

    // SAFETY: All arguments are valid: frame is a well-formed rect, style mask
    // is a valid combination, backing store type is NSBackingStoreBuffered, and
    // defer=false. The window is retained by the returned Retained<NSWindow>.
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            frame,
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    window.setTitle(&NSString::from_str("Ferrum Settings"));
    // SAFETY: We hold a Retained<NSWindow> (strong reference). Without this call,
    // macOS releases the window when the user closes it, leaving a dangling pointer.
    unsafe { window.setReleasedWhenClosed(false) };
    window.center();


    // ── NSTabView ─────────────────────────────────────────────────────

    let tab_view = NSTabView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::new(0.0, 40.0), NSSize::new(500.0, 360.0)),
    );

    // ── Font tab ──────────────────────────────────────────────────────

    let font_tab = NSTabViewItem::new();
    font_tab.setLabel(&NSString::from_str("Font"));
    let font_view = NSView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(500.0, 320.0)),
    );

    let (font_size_field, font_size_stepper) = create_stepper_row(
        mtm,
        &font_view,
        "Font Size:",
        f64::from(config.font.size),
        8.0,
        32.0,
        0.5,
        280.0,
    );

    let font_family_popup = create_popup_row(
        mtm,
        &font_view,
        "Font Family:",
        FontFamily::DISPLAY_NAMES,
        config.font.family.index(),
        230.0,
    );

    let (line_padding_field, line_padding_stepper) = create_stepper_row(
        mtm,
        &font_view,
        "Line Padding:",
        f64::from(config.font.line_padding),
        0.0,
        10.0,
        1.0,
        180.0,
    );

    font_tab.setView(Some(&font_view));
    tab_view.addTabViewItem(&font_tab);

    // ── Theme tab ─────────────────────────────────────────────────────

    let theme_tab = NSTabViewItem::new();
    theme_tab.setLabel(&NSString::from_str("Theme"));
    let theme_view = NSView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(500.0, 320.0)),
    );

    let theme_selected = match config.theme {
        ThemeChoice::FerrumDark => 0,
        ThemeChoice::FerrumLight => 1,
    };
    let theme_popup = create_popup_row(
        mtm,
        &theme_view,
        "Theme:",
        &["Ferrum Dark", "Ferrum Light"],
        theme_selected,
        280.0,
    );

    theme_tab.setView(Some(&theme_view));
    tab_view.addTabViewItem(&theme_tab);

    // ── Terminal tab ──────────────────────────────────────────────────

    let terminal_tab = NSTabViewItem::new();
    terminal_tab.setLabel(&NSString::from_str("Terminal"));
    let terminal_view = NSView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(500.0, 320.0)),
    );

    let (scrollback_field, scrollback_stepper) = create_stepper_row(
        mtm,
        &terminal_view,
        "Max Scrollback:",
        config.terminal.max_scrollback as f64,
        0.0,
        50000.0,
        100.0,
        280.0,
    );

    let (cursor_blink_field, cursor_blink_stepper) = create_stepper_row(
        mtm,
        &terminal_view,
        "Cursor Blink (ms):",
        config.terminal.cursor_blink_interval_ms as f64,
        100.0,
        2000.0,
        50.0,
        230.0,
    );

    terminal_tab.setView(Some(&terminal_view));
    tab_view.addTabViewItem(&terminal_tab);

    // ── Layout tab ────────────────────────────────────────────────────
    // Note: Tab Bar Height is omitted — macOS uses the native tab bar.

    let layout_tab = NSTabViewItem::new();
    layout_tab.setLabel(&NSString::from_str("Layout"));
    let layout_view = NSView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(500.0, 320.0)),
    );

    let (window_padding_field, window_padding_stepper) = create_stepper_row(
        mtm,
        &layout_view,
        "Window Padding:",
        f64::from(config.layout.window_padding),
        0.0,
        32.0,
        1.0,
        280.0,
    );

    let (pane_padding_field, pane_padding_stepper) = create_stepper_row(
        mtm,
        &layout_view,
        "Pane Padding:",
        f64::from(config.layout.pane_inner_padding),
        0.0,
        16.0,
        1.0,
        230.0,
    );

    let (scrollbar_width_field, scrollbar_width_stepper) = create_stepper_row(
        mtm,
        &layout_view,
        "Scrollbar Width:",
        f64::from(config.layout.scrollbar_width),
        2.0,
        16.0,
        1.0,
        180.0,
    );

    layout_tab.setView(Some(&layout_view));
    tab_view.addTabViewItem(&layout_tab);

    // ── Security tab ─────────────────────────────────────────────────

    let security_tab = NSTabViewItem::new();
    security_tab.setLabel(&NSString::from_str("Security"));
    let security_view = NSView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(500.0, 320.0)),
    );

    let security_mode_selected = match config.security.mode {
        SecurityMode::Disabled => 0,
        SecurityMode::Standard => 1,
        SecurityMode::Custom => 2,
    };
    let security_mode_popup = create_popup_row(
        mtm,
        &security_view,
        "Security Mode:",
        &["Disabled", "Standard", "Custom"],
        security_mode_selected,
        280.0,
    );

    let paste_protection_check = create_checkbox_row(
        mtm,
        &security_view,
        "Paste Protection",
        "Warn before pasting text with suspicious control characters",
        config.security.paste_protection,
        240.0,
    );
    let block_title_query_check = create_checkbox_row(
        mtm,
        &security_view,
        "Block Title Query",
        "Block programs from reading the terminal window title",
        config.security.block_title_query,
        194.0,
    );
    let limit_cursor_jumps_check = create_checkbox_row(
        mtm,
        &security_view,
        "Limit Cursor Jumps",
        "Restrict how far escape sequences can move the cursor",
        config.security.limit_cursor_jumps,
        148.0,
    );
    let clear_mouse_on_reset_check = create_checkbox_row(
        mtm,
        &security_view,
        "Clear Mouse on Reset",
        "Disable mouse tracking modes when the terminal resets",
        config.security.clear_mouse_on_reset,
        102.0,
    );

    security_tab.setView(Some(&security_view));
    tab_view.addTabViewItem(&security_tab);

    // ── Reset button ──────────────────────────────────────────────────

    // SAFETY: Passing `None` for target and action is valid; we wire the
    // action below via class_addMethod + msg_send.
    let reset_button = unsafe {
        NSButton::buttonWithTitle_target_action(
            ns_string!("Reset to Defaults"),
            None,
            None,
            mtm,
        )
    };
    reset_button.setFrame(NSRect::new(
        NSPoint::new(20.0, 8.0),
        NSSize::new(150.0, 28.0),
    ));

    // ── Register ObjC action methods on the window class ──────────────

    // SAFETY: We register two custom selectors on the NSWindow's runtime class
    // and wire them as target/action pairs. This follows the same pattern used
    // in pin.rs for the pin and gear buttons. The imp transmutes are required
    // because class_addMethod expects `unsafe extern "C" fn()` but our handlers
    // have the standard ObjC action signature (self, cmd, sender).
    unsafe {
        let types = c"v@:@".as_ptr();
        let win_cls = object_getClass(Retained::as_ptr(&window).cast());

        // Stepper/popup changed action.
        let sel_stepper_ptr = sel_registerName(c"ferrumStepperChanged:".as_ptr());
        let sel_stepper = Sel::register(c"ferrumStepperChanged:");
        let imp_stepper: unsafe extern "C" fn() = core::mem::transmute(
            handle_stepper_changed as unsafe extern "C" fn(_, _, _),
        );
        if !win_cls.is_null()
            && !class_addMethod(win_cls, sel_stepper_ptr, imp_stepper, types)
        {
            class_replaceMethod(win_cls, sel_stepper_ptr, imp_stepper, types);
        }

        // Text field changed action.
        let sel_text_ptr = sel_registerName(c"ferrumTextFieldChanged:".as_ptr());
        let sel_text = Sel::register(c"ferrumTextFieldChanged:");
        let imp_text: unsafe extern "C" fn() = core::mem::transmute(
            handle_text_field_changed as unsafe extern "C" fn(_, _, _),
        );
        if !win_cls.is_null()
            && !class_addMethod(win_cls, sel_text_ptr, imp_text, types)
        {
            class_replaceMethod(win_cls, sel_text_ptr, imp_text, types);
        }

        // Reset action (for the reset button).
        let sel_reset_ptr = sel_registerName(c"ferrumResetClicked:".as_ptr());
        let sel_reset = Sel::register(c"ferrumResetClicked:");
        let imp_reset: unsafe extern "C" fn() =
            core::mem::transmute(handle_reset_clicked as unsafe extern "C" fn(_, _, _));
        if !win_cls.is_null() && !class_addMethod(win_cls, sel_reset_ptr, imp_reset, types) {
            class_replaceMethod(win_cls, sel_reset_ptr, imp_reset, types);
        }

        // Wire all steppers and popups to the stepper-changed action.
        let _: () = msg_send![&font_size_stepper, setTarget: &*window];
        let _: () = msg_send![&font_size_stepper, setAction: sel_stepper];
        let _: () = msg_send![&font_family_popup, setTarget: &*window];
        let _: () = msg_send![&font_family_popup, setAction: sel_stepper];
        let _: () = msg_send![&line_padding_stepper, setTarget: &*window];
        let _: () = msg_send![&line_padding_stepper, setAction: sel_stepper];
        let _: () = msg_send![&theme_popup, setTarget: &*window];
        let _: () = msg_send![&theme_popup, setAction: sel_stepper];
        let _: () = msg_send![&scrollback_stepper, setTarget: &*window];
        let _: () = msg_send![&scrollback_stepper, setAction: sel_stepper];
        let _: () = msg_send![&cursor_blink_stepper, setTarget: &*window];
        let _: () = msg_send![&cursor_blink_stepper, setAction: sel_stepper];
        let _: () = msg_send![&window_padding_stepper, setTarget: &*window];
        let _: () = msg_send![&window_padding_stepper, setAction: sel_stepper];
        let _: () = msg_send![&pane_padding_stepper, setTarget: &*window];
        let _: () = msg_send![&pane_padding_stepper, setAction: sel_stepper];
        let _: () = msg_send![&scrollbar_width_stepper, setTarget: &*window];
        let _: () = msg_send![&scrollbar_width_stepper, setAction: sel_stepper];
        // Security popup and checkboxes also trigger stepper-changed.
        let _: () = msg_send![&security_mode_popup, setTarget: &*window];
        let _: () = msg_send![&security_mode_popup, setAction: sel_stepper];
        let _: () = msg_send![&paste_protection_check, setTarget: &*window];
        let _: () = msg_send![&paste_protection_check, setAction: sel_stepper];
        let _: () = msg_send![&block_title_query_check, setTarget: &*window];
        let _: () = msg_send![&block_title_query_check, setAction: sel_stepper];
        let _: () = msg_send![&limit_cursor_jumps_check, setTarget: &*window];
        let _: () = msg_send![&limit_cursor_jumps_check, setAction: sel_stepper];
        let _: () = msg_send![&clear_mouse_on_reset_check, setTarget: &*window];
        let _: () = msg_send![&clear_mouse_on_reset_check, setAction: sel_stepper];

        // Wire all editable text fields to the text-field-changed action.
        let _: () = msg_send![&font_size_field, setTarget: &*window];
        let _: () = msg_send![&font_size_field, setAction: sel_text];
        let _: () = msg_send![&line_padding_field, setTarget: &*window];
        let _: () = msg_send![&line_padding_field, setAction: sel_text];
        let _: () = msg_send![&scrollback_field, setTarget: &*window];
        let _: () = msg_send![&scrollback_field, setAction: sel_text];
        let _: () = msg_send![&cursor_blink_field, setTarget: &*window];
        let _: () = msg_send![&cursor_blink_field, setAction: sel_text];
        let _: () = msg_send![&window_padding_field, setTarget: &*window];
        let _: () = msg_send![&window_padding_field, setAction: sel_text];
        let _: () = msg_send![&pane_padding_field, setTarget: &*window];
        let _: () = msg_send![&pane_padding_field, setAction: sel_text];
        let _: () = msg_send![&scrollbar_width_field, setTarget: &*window];
        let _: () = msg_send![&scrollbar_width_field, setAction: sel_text];

        // Wire reset button.
        let _: () = msg_send![&reset_button, setTarget: &*window];
        let _: () = msg_send![&reset_button, setAction: sel_reset];
    }

    // ── Assemble content view ─────────────────────────────────────────

    let content_view = NSView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(500.0, 400.0)),
    );
    content_view.addSubview(&tab_view);
    content_view.addSubview(&reset_button);
    window.setContentView(Some(&content_view));

    // ── Show window ───────────────────────────────────────────────────

    window.makeKeyAndOrderFront(None::<&AnyObject>);

    // ── Store state ───────────────────────────────────────────────────

    // Initialise security mode tracking to match the current config.
    LAST_SECURITY_MODE_INDEX.store(security_mode_selected as isize, Ordering::SeqCst);

    let state = NativeSettingsState {
        window: window.clone(),
        sender,
        font_size_stepper,
        font_size_field,
        font_family_popup,
        line_padding_stepper,
        line_padding_field,
        theme_popup,
        scrollback_stepper,
        scrollback_field,
        cursor_blink_stepper,
        cursor_blink_field,
        window_padding_stepper,
        window_padding_field,
        pane_padding_stepper,
        pane_padding_field,
        scrollbar_width_stepper,
        scrollbar_width_field,
        security_mode_popup,
        paste_protection_check,
        block_title_query_check,
        limit_cursor_jumps_check,
        clear_mouse_on_reset_check,
        reset_button,
    };
    *SETTINGS_STATE.lock().unwrap() = Some(state);
}

/// Closes the native settings window, saves the final config, and cleans up state.
pub fn close_settings_window() {
    if let Some(state) = SETTINGS_STATE.lock().unwrap().take() {
        let config = build_config_from_controls(&state);
        crate::config::save_config(&config);
        state.window.close();
    }
}
