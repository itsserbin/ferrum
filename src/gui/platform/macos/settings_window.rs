use std::sync::Mutex;
use std::sync::mpsc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSBackingStoreType, NSButton, NSPopUpButton, NSStepper, NSTabView, NSTabViewItem, NSTextField,
    NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, ns_string};

use crate::config::{AppConfig, FontFamily, ThemeChoice};

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

    // SAFETY: Passing `None` for target and action is valid; the button
    // simply won't send any action until wired in a later task.
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

/// Closes the native settings window and cleans up state.
pub fn close_settings_window() {
    if let Some(state) = SETTINGS_STATE.lock().unwrap().take() {
        state.window.close();
    }
}
