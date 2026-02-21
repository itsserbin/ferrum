use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use objc2::MainThreadMarker;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2_app_kit::{
    NSBackingStoreType, NSButton, NSPopUpButton, NSStepper, NSTabView, NSTabViewItem, NSTextField,
    NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, ns_string};

use super::ffi::{class_addMethod, class_replaceMethod, object_getClass, sel_registerName};
use crate::config::{
    AppConfig, FontConfig, FontFamily, LayoutConfig, TerminalConfig, ThemeChoice,
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

/// Atomic flag: a control value changed in the settings window.
static SETTINGS_CHANGED: AtomicBool = AtomicBool::new(false);

/// Atomic flag: Reset to Defaults was clicked.
static RESET_REQUESTED: AtomicBool = AtomicBool::new(false);

/// ObjC action handler for all settings controls (steppers, popups).
///
/// # Safety
///
/// Called by the Objective-C runtime as a method implementation.
/// Signature matches the type encoding "v@:@".
unsafe extern "C" fn handle_settings_control_changed(
    _this: *mut core::ffi::c_void,
    _cmd: *const core::ffi::c_void,
    _sender: *mut core::ffi::c_void,
) {
    SETTINGS_CHANGED.store(true, Ordering::SeqCst);
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

/// Returns and resets the settings-changed flag.
pub fn take_settings_changed() -> bool {
    SETTINGS_CHANGED.swap(false, Ordering::SeqCst)
}

/// Returns and resets the reset-requested flag.
pub fn take_reset_requested() -> bool {
    RESET_REQUESTED.swap(false, Ordering::SeqCst)
}

/// Builds an `AppConfig` from the current control values.
fn build_config_from_controls(state: &NativeSettingsState) -> AppConfig {
    AppConfig {
        font: FontConfig {
            size: state.font_size_stepper.doubleValue() as f32,
            family: match state.font_family_popup.indexOfSelectedItem() {
                0 => FontFamily::JetBrainsMono,
                _ => FontFamily::FiraCode,
            },
            line_padding: state.line_padding_stepper.integerValue() as u32,
        },
        theme: match state.theme_popup.indexOfSelectedItem() {
            0 => ThemeChoice::FerrumDark,
            _ => ThemeChoice::CatppuccinLatte,
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
    }
}

/// Reads all control values and sends the resulting config through the channel.
pub fn send_current_config() {
    let guard = SETTINGS_STATE.lock().unwrap();
    let Some(state) = guard.as_ref() else {
        return;
    };
    let config = build_config_from_controls(state);
    let _ = state.sender.send(config);
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
        .tab_bar_height_field
        .setStringValue(&NSString::from_str(&format!(
            "{}",
            state.tab_bar_height_stepper.integerValue()
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
        .tab_bar_height_stepper
        .setIntegerValue(defaults.layout.tab_bar_height as isize);
    state
        .pane_padding_stepper
        .setIntegerValue(defaults.layout.pane_inner_padding as isize);
    state
        .scrollbar_width_stepper
        .setIntegerValue(defaults.layout.scrollbar_width as isize);

    drop(guard);
    update_text_fields();
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
    let value_field = NSTextField::labelWithString(&NSString::from_str(&value_str), mtm);
    value_field.setFrame(NSRect::new(
        NSPoint::new(200.0, y_offset),
        NSSize::new(80.0, 24.0),
    ));
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

    let family_selected = match config.font.family {
        FontFamily::JetBrainsMono => 0,
        FontFamily::FiraCode => 1,
    };
    let font_family_popup = create_popup_row(
        mtm,
        &font_view,
        "Font Family:",
        &["JetBrains Mono", "Fira Code"],
        family_selected,
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
        ThemeChoice::CatppuccinLatte => 1,
    };
    let theme_popup = create_popup_row(
        mtm,
        &theme_view,
        "Theme:",
        &["Ferrum Dark", "Catppuccin Latte"],
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

    let (tab_bar_height_field, tab_bar_height_stepper) = create_stepper_row(
        mtm,
        &layout_view,
        "Tab Bar Height:",
        f64::from(config.layout.tab_bar_height),
        24.0,
        60.0,
        1.0,
        230.0,
    );

    let (pane_padding_field, pane_padding_stepper) = create_stepper_row(
        mtm,
        &layout_view,
        "Pane Padding:",
        f64::from(config.layout.pane_inner_padding),
        0.0,
        16.0,
        1.0,
        180.0,
    );

    let (scrollbar_width_field, scrollbar_width_stepper) = create_stepper_row(
        mtm,
        &layout_view,
        "Scrollbar Width:",
        f64::from(config.layout.scrollbar_width),
        2.0,
        16.0,
        1.0,
        130.0,
    );

    layout_tab.setView(Some(&layout_view));
    tab_view.addTabViewItem(&layout_tab);

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

        // Settings-changed action (for steppers and popups).
        let sel_changed_ptr = sel_registerName(c"ferrumSettingsChanged:".as_ptr());
        let sel_changed = Sel::register(c"ferrumSettingsChanged:");
        let imp_changed: unsafe extern "C" fn() = core::mem::transmute(
            handle_settings_control_changed as unsafe extern "C" fn(_, _, _),
        );
        if !win_cls.is_null()
            && !class_addMethod(win_cls, sel_changed_ptr, imp_changed, types)
        {
            class_replaceMethod(win_cls, sel_changed_ptr, imp_changed, types);
        }

        // Reset action (for the reset button).
        let sel_reset_ptr = sel_registerName(c"ferrumResetClicked:".as_ptr());
        let sel_reset = Sel::register(c"ferrumResetClicked:");
        let imp_reset: unsafe extern "C" fn() =
            core::mem::transmute(handle_reset_clicked as unsafe extern "C" fn(_, _, _));
        if !win_cls.is_null() && !class_addMethod(win_cls, sel_reset_ptr, imp_reset, types) {
            class_replaceMethod(win_cls, sel_reset_ptr, imp_reset, types);
        }

        // Wire all steppers and popups to the settings-changed action.
        let _: () = msg_send![&font_size_stepper, setTarget: &*window];
        let _: () = msg_send![&font_size_stepper, setAction: sel_changed];
        let _: () = msg_send![&font_family_popup, setTarget: &*window];
        let _: () = msg_send![&font_family_popup, setAction: sel_changed];
        let _: () = msg_send![&line_padding_stepper, setTarget: &*window];
        let _: () = msg_send![&line_padding_stepper, setAction: sel_changed];
        let _: () = msg_send![&theme_popup, setTarget: &*window];
        let _: () = msg_send![&theme_popup, setAction: sel_changed];
        let _: () = msg_send![&scrollback_stepper, setTarget: &*window];
        let _: () = msg_send![&scrollback_stepper, setAction: sel_changed];
        let _: () = msg_send![&cursor_blink_stepper, setTarget: &*window];
        let _: () = msg_send![&cursor_blink_stepper, setAction: sel_changed];
        let _: () = msg_send![&window_padding_stepper, setTarget: &*window];
        let _: () = msg_send![&window_padding_stepper, setAction: sel_changed];
        let _: () = msg_send![&tab_bar_height_stepper, setTarget: &*window];
        let _: () = msg_send![&tab_bar_height_stepper, setAction: sel_changed];
        let _: () = msg_send![&pane_padding_stepper, setTarget: &*window];
        let _: () = msg_send![&pane_padding_stepper, setAction: sel_changed];
        let _: () = msg_send![&scrollbar_width_stepper, setTarget: &*window];
        let _: () = msg_send![&scrollbar_width_stepper, setAction: sel_changed];

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
        tab_bar_height_stepper,
        tab_bar_height_field,
        pane_padding_stepper,
        pane_padding_field,
        scrollbar_width_stepper,
        scrollbar_width_field,
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
