use crate::config::{
    AppConfig, FontConfig, FontFamily, LayoutConfig, SecurityMode, SecuritySettings,
    TerminalConfig, ThemeChoice,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Align, DropDown, Label, Notebook, Orientation, SpinButton, Switch, Window,
};

static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);
/// Set to true when the GTK window closes; cleared by `check_window_closed()`.
static JUST_CLOSED: AtomicBool = AtomicBool::new(false);
/// Set by `close_settings_window()` from the main thread; polled by GTK timer.
static CLOSE_REQUESTED: AtomicBool = AtomicBool::new(false);
/// Set by the main thread to request the GTK timer to present (raise) the window.
static PRESENT_REQUESTED: AtomicBool = AtomicBool::new(false);
/// Set to true only if GTK4 initialised successfully.
static GTK_READY: AtomicBool = AtomicBool::new(false);

pub fn is_settings_window_open() -> bool {
    WINDOW_OPEN.load(Ordering::Relaxed)
}

/// Returns `true` once after the settings window was closed, then resets.
pub fn check_window_closed() -> bool {
    JUST_CLOSED.swap(false, Ordering::Relaxed)
}

pub fn close_settings_window() {
    WINDOW_OPEN.store(false, Ordering::Relaxed);
    CLOSE_REQUESTED.store(true, Ordering::Relaxed);
}

/// Ensures GTK4 is initialized exactly once on a dedicated thread.
/// That thread runs a `glib::MainLoop` forever; subsequent windows are
/// dispatched onto it via `MainContext::default().invoke()`.
static GTK_INIT: std::sync::Once = std::sync::Once::new();

pub fn open_settings_window(config: &AppConfig, tx: mpsc::Sender<AppConfig>) {
    if WINDOW_OPEN
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        // Already open — request the GTK timer to bring the window to front.
        PRESENT_REQUESTED.store(true, Ordering::Relaxed);
        return;
    }

    let config = config.clone();

    GTK_INIT.call_once(|| {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Err(e) = gtk4::init() {
                eprintln!("[ferrum] Failed to initialize GTK4: {e}");
                let _ = ready_tx.send(false);
                return;
            }
            let _ = ready_tx.send(true);
            gtk4::glib::MainLoop::new(None, false).run();
        });
        if ready_rx.recv() == Ok(true) {
            GTK_READY.store(true, Ordering::Release);
        } else {
            eprintln!("[ferrum] GTK4 initialization failed, settings window unavailable");
        }
    });

    if !GTK_READY.load(Ordering::Acquire) {
        WINDOW_OPEN.store(false, Ordering::Relaxed);
        return;
    }

    gtk4::glib::MainContext::default().invoke(move || {
        build_window(&config, tx);
    });
}

// ── GTK4 window ──────────────────────────────────────────────────────

fn build_window(config: &AppConfig, tx: mpsc::Sender<AppConfig>) {
    let t = crate::i18n::t();
    let window = Window::builder()
        .title(t.settings_title)
        .default_width(500)
        .default_height(420)
        .resizable(false)
        .build();

    let notebook = Notebook::new();

    // ── Font tab ─────────────────────────────────────────────────────
    let (font_box, font_size_spin, font_family_combo, line_padding_spin) =
        build_font_tab(config, t);
    notebook.append_page(&font_box, Some(&Label::new(Some(t.settings_tab_font))));

    // ── Theme tab ────────────────────────────────────────────────────
    let (theme_box, theme_combo) = build_theme_tab(config, t);
    notebook.append_page(&theme_box, Some(&Label::new(Some(t.settings_tab_theme))));

    // ── Terminal tab ─────────────────────────────────────────────────
    let (terminal_box, language_combo, scrollback_spin, cursor_blink_spin) = build_terminal_tab(config, t);
    notebook.append_page(&terminal_box, Some(&Label::new(Some(t.settings_tab_terminal))));

    // ── Layout tab ───────────────────────────────────────────────────
    let (layout_box, win_padding_spin, pane_padding_spin, scrollbar_spin, tab_bar_spin) =
        build_layout_tab(config, t);
    notebook.append_page(&layout_box, Some(&Label::new(Some(t.settings_tab_layout))));

    // ── Security tab ─────────────────────────────────────────────────
    let (
        security_box,
        security_mode_combo,
        paste_switch,
        block_title_switch,
        limit_cursor_switch,
        clear_mouse_switch,
    ) = build_security_tab(config, t);
    notebook.append_page(&security_box, Some(&Label::new(Some(t.settings_tab_security))));

    // ── Reset button ─────────────────────────────────────────────────
    let reset_btn = gtk4::Button::with_label(t.settings_reset_to_defaults);

    let main_box = gtk4::Box::new(Orientation::Vertical, 8);
    main_box.set_margin_top(8);
    main_box.set_margin_bottom(8);
    main_box.set_margin_start(8);
    main_box.set_margin_end(8);
    main_box.append(&notebook);
    main_box.append(&reset_btn);

    window.set_child(Some(&main_box));

    // ── Collect controls into a shared struct ────────────────────────
    use std::cell::RefCell;
    use std::rc::Rc;

    let controls = Rc::new(Controls {
        font_size: font_size_spin,
        font_family: font_family_combo,
        line_padding: line_padding_spin,
        theme: theme_combo,
        language: language_combo,
        scrollback: scrollback_spin,
        cursor_blink: cursor_blink_spin,
        win_padding: win_padding_spin,
        pane_padding: pane_padding_spin,
        scrollbar: scrollbar_spin,
        tab_bar: tab_bar_spin,
        security_mode: security_mode_combo,
        paste: paste_switch,
        block_title: block_title_switch,
        limit_cursor: limit_cursor_switch,
        clear_mouse: clear_mouse_switch,
    });

    // Tracks whether we're in a programmatic update (e.g. security sync or reset).
    let suppress = Rc::new(RefCell::new(false));

    // ── Wire change signals ──────────────────────────────────────────
    // Helper: build config from current control values and send.
    let build_and_send = {
        let controls = Rc::clone(&controls);
        let tx = tx.clone();
        let suppress = Rc::clone(&suppress);
        move || {
            if *suppress.borrow() {
                return;
            }
            let config = build_config(&controls);
            let _ = tx.send(config);
        }
    };

    // Connect SpinButton value-changed for all numeric controls.
    let spins: Vec<&SpinButton> = vec![
        &controls.font_size,
        &controls.line_padding,
        &controls.scrollback,
        &controls.cursor_blink,
        &controls.win_padding,
        &controls.pane_padding,
        &controls.scrollbar,
        &controls.tab_bar,
    ];
    for spin in spins {
        let send = build_and_send.clone();
        spin.connect_value_changed(move |_| send());
    }

    // Connect DropDown selection-changed for font family, theme, and language.
    {
        let send = build_and_send.clone();
        controls.font_family.connect_selected_notify(move |_| send());
    }
    {
        let send = build_and_send.clone();
        controls.theme.connect_selected_notify(move |_| send());
    }
    {
        let send = build_and_send.clone();
        controls.language.connect_selected_notify(move |_| send());
    }

    // Security mode combo — apply presets, then send.
    {
        let controls = Rc::clone(&controls);
        let suppress = Rc::clone(&suppress);
        let send = build_and_send.clone();
        let security_combo = controls.security_mode.clone();
        security_combo.connect_selected_notify(move |combo| {
            if *suppress.borrow() {
                return;
            }
            *suppress.borrow_mut() = true;
            let sel = combo.selected();
            let active = if sel == gtk4::INVALID_LIST_POSITION { None } else { Some(sel as usize) };
            apply_security_preset(&controls, active);
            *suppress.borrow_mut() = false;
            send();
        });
    }

    // Security switches — infer mode, then send.
    let switches: Vec<&Switch> = vec![
        &controls.paste,
        &controls.block_title,
        &controls.limit_cursor,
        &controls.clear_mouse,
    ];
    for sw in switches {
        let controls = Rc::clone(&controls);
        let suppress = Rc::clone(&suppress);
        let send = build_and_send.clone();
        sw.connect_state_set(move |_, _| {
            if *suppress.borrow() {
                return gtk4::glib::Propagation::Proceed;
            }
            // Defer to allow GTK to update the switch state first.
            let controls = Rc::clone(&controls);
            let suppress = Rc::clone(&suppress);
            let send = send.clone();
            gtk4::glib::idle_add_local_once(move || {
                *suppress.borrow_mut() = true;
                infer_security_mode(&controls);
                *suppress.borrow_mut() = false;
                send();
            });
            gtk4::glib::Propagation::Proceed
        });
    }

    // Reset button.
    {
        let controls = Rc::clone(&controls);
        let suppress = Rc::clone(&suppress);
        let send = build_and_send.clone();
        reset_btn.connect_clicked(move |_| {
            *suppress.borrow_mut() = true;
            reset_controls(&controls);
            *suppress.borrow_mut() = false;
            send();
        });
    }

    // On close request, save config and immediately mark window as closed
    // so the gear icon resets without waiting for the destroy signal.
    {
        let controls = Rc::clone(&controls);
        window.connect_close_request(move |_| {
            WINDOW_OPEN.store(false, Ordering::Relaxed);
            JUST_CLOSED.store(true, Ordering::Relaxed);
            let config = build_config(&controls);
            crate::config::save_config(&config);
            gtk4::glib::Propagation::Proceed
        });
    }

    // Poll CLOSE_REQUESTED and PRESENT_REQUESTED so the main thread can control us.
    {
        let w = window.clone();
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            if CLOSE_REQUESTED.swap(false, Ordering::Relaxed) {
                w.close();
                return gtk4::glib::ControlFlow::Break;
            }
            if PRESENT_REQUESTED.swap(false, Ordering::Relaxed) {
                w.present();
            }
            gtk4::glib::ControlFlow::Continue
        });
    }

    window.present();
}

// ── Tab builders ─────────────────────────────────────────────────────

fn build_font_tab(config: &AppConfig, t: &crate::i18n::Translations) -> (gtk4::Box, SpinButton, DropDown, SpinButton) {
    let vbox = gtk4::Box::new(Orientation::Vertical, 12);
    vbox.set_margin_top(16);
    vbox.set_margin_start(16);
    vbox.set_margin_end(16);

    let font_size = labeled_spin(
        &vbox,
        t.font_size_label,
        config.font.size as f64,
        f64::from(FontConfig::SIZE_MIN),
        f64::from(FontConfig::SIZE_MAX),
        f64::from(FontConfig::SIZE_STEP),
        1,
    );
    let font_family = labeled_combo(
        &vbox,
        t.font_family_label,
        FontFamily::DISPLAY_NAMES,
        config.font.family.index(),
    );
    let line_padding =
        labeled_spin(&vbox, t.font_line_padding_label, config.font.line_padding as f64, 0.0, 10.0, 1.0, 0);

    (vbox, font_size, font_family, line_padding)
}

fn build_theme_tab(config: &AppConfig, t: &crate::i18n::Translations) -> (gtk4::Box, DropDown) {
    let vbox = gtk4::Box::new(Orientation::Vertical, 12);
    vbox.set_margin_top(16);
    vbox.set_margin_start(16);
    vbox.set_margin_end(16);

    let selected = match config.theme {
        ThemeChoice::FerrumDark => 0,
        ThemeChoice::FerrumLight => 1,
    };
    let combo = labeled_combo(&vbox, t.theme_label, &["Ferrum Dark", "Ferrum Light"], selected);

    (vbox, combo)
}

fn build_terminal_tab(config: &AppConfig, t: &crate::i18n::Translations) -> (gtk4::Box, DropDown, SpinButton, SpinButton) {
    let vbox = gtk4::Box::new(Orientation::Vertical, 12);
    vbox.set_margin_top(16);
    vbox.set_margin_start(16);
    vbox.set_margin_end(16);

    let language = labeled_combo(
        &vbox,
        t.terminal_language_label,
        crate::i18n::Locale::DISPLAY_NAMES,
        config.language.index(),
    );
    let scrollback = labeled_spin(
        &vbox,
        t.terminal_max_scrollback_label,
        config.terminal.max_scrollback as f64,
        0.0,
        50000.0,
        100.0,
        0,
    );
    let cursor_blink = labeled_spin(
        &vbox,
        t.terminal_cursor_blink_label,
        config.terminal.cursor_blink_interval_ms as f64,
        TerminalConfig::BLINK_MS_MIN as f64,
        TerminalConfig::BLINK_MS_MAX as f64,
        TerminalConfig::BLINK_MS_STEP as f64,
        0,
    );

    (vbox, language, scrollback, cursor_blink)
}

fn build_layout_tab(
    config: &AppConfig,
    t: &crate::i18n::Translations,
) -> (gtk4::Box, SpinButton, SpinButton, SpinButton, SpinButton) {
    let vbox = gtk4::Box::new(Orientation::Vertical, 12);
    vbox.set_margin_top(16);
    vbox.set_margin_start(16);
    vbox.set_margin_end(16);

    let win_padding = labeled_spin(
        &vbox,
        t.layout_window_padding_label,
        config.layout.window_padding as f64,
        0.0,
        32.0,
        1.0,
        0,
    );
    let pane_padding = labeled_spin(
        &vbox,
        t.layout_pane_padding_label,
        config.layout.pane_inner_padding as f64,
        0.0,
        16.0,
        1.0,
        0,
    );
    let scrollbar = labeled_spin(
        &vbox,
        t.layout_scrollbar_width_label,
        config.layout.scrollbar_width as f64,
        2.0,
        16.0,
        1.0,
        0,
    );
    let tab_bar = labeled_spin(
        &vbox,
        t.layout_tab_bar_height_label,
        config.layout.tab_bar_height as f64,
        24.0,
        48.0,
        1.0,
        0,
    );

    (vbox, win_padding, pane_padding, scrollbar, tab_bar)
}

fn build_security_tab(
    config: &AppConfig,
    t: &crate::i18n::Translations,
) -> (gtk4::Box, DropDown, Switch, Switch, Switch, Switch) {
    let vbox = gtk4::Box::new(Orientation::Vertical, 12);
    vbox.set_margin_top(16);
    vbox.set_margin_start(16);
    vbox.set_margin_end(16);

    let mode_index = match config.security.mode {
        SecurityMode::Disabled => 0,
        SecurityMode::Standard => 1,
        SecurityMode::Custom => 2,
    };
    let mode_combo = labeled_combo(
        &vbox,
        t.security_mode_label,
        &[t.security_mode_disabled, t.security_mode_standard, t.security_mode_custom],
        mode_index,
    );

    let enabled = !matches!(config.security.mode, SecurityMode::Disabled);
    let paste = labeled_switch(&vbox, t.security_paste_protection_label, config.security.paste_protection, enabled);
    let block_title =
        labeled_switch(&vbox, t.security_block_title_query_label, config.security.block_title_query, enabled);
    let limit_cursor =
        labeled_switch(&vbox, t.security_limit_cursor_jumps_label, config.security.limit_cursor_jumps, enabled);
    let clear_mouse = labeled_switch(
        &vbox,
        t.security_clear_mouse_on_reset_label,
        config.security.clear_mouse_on_reset,
        enabled,
    );

    (vbox, mode_combo, paste, block_title, limit_cursor, clear_mouse)
}

// ── Widget helpers ───────────────────────────────────────────────────

fn labeled_spin(
    parent: &gtk4::Box,
    label: &str,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    digits: u32,
) -> SpinButton {
    let row = gtk4::Box::new(Orientation::Horizontal, 12);
    let lbl = Label::new(Some(label));
    lbl.set_halign(Align::Start);
    lbl.set_width_chars(20);

    let adj = Adjustment::new(value, min, max, step, step * 10.0, 0.0);
    let spin = SpinButton::new(Some(&adj), step, digits);
    spin.set_halign(Align::End);
    spin.set_hexpand(true);

    row.append(&lbl);
    row.append(&spin);
    parent.append(&row);

    spin
}

fn labeled_combo(
    parent: &gtk4::Box,
    label: &str,
    options: &[&str],
    selected: usize,
) -> DropDown {
    let row = gtk4::Box::new(Orientation::Horizontal, 12);
    let lbl = Label::new(Some(label));
    lbl.set_halign(Align::Start);
    lbl.set_width_chars(20);

    let dropdown = DropDown::from_strings(options);
    dropdown.set_selected(selected as u32);
    dropdown.set_halign(Align::End);
    dropdown.set_hexpand(true);

    row.append(&lbl);
    row.append(&dropdown);
    parent.append(&row);

    dropdown
}

fn labeled_switch(
    parent: &gtk4::Box,
    label: &str,
    active: bool,
    sensitive: bool,
) -> Switch {
    let row = gtk4::Box::new(Orientation::Horizontal, 12);
    let lbl = Label::new(Some(label));
    lbl.set_halign(Align::Start);
    lbl.set_width_chars(20);
    lbl.set_hexpand(true);

    let sw = Switch::new();
    sw.set_active(active);
    sw.set_sensitive(sensitive);
    sw.set_halign(Align::End);

    row.append(&lbl);
    row.append(&sw);
    parent.append(&row);

    sw
}

// ── Config building ──────────────────────────────────────────────────

struct Controls {
    font_size: SpinButton,
    font_family: DropDown,
    line_padding: SpinButton,
    theme: DropDown,
    language: DropDown,
    scrollback: SpinButton,
    cursor_blink: SpinButton,
    win_padding: SpinButton,
    pane_padding: SpinButton,
    scrollbar: SpinButton,
    tab_bar: SpinButton,
    security_mode: DropDown,
    paste: Switch,
    block_title: Switch,
    limit_cursor: Switch,
    clear_mouse: Switch,
}

fn build_config(c: &Controls) -> AppConfig {
    let security_mode = match c.security_mode.selected() {
        0 => SecurityMode::Disabled,
        1 => SecurityMode::Standard,
        _ => SecurityMode::Custom,
    };

    AppConfig {
        font: FontConfig {
            size: c.font_size.value() as f32,
            family: FontFamily::from_index(c.font_family.selected() as usize),
            line_padding: c.line_padding.value() as u32,
        },
        theme: match c.theme.selected() {
            0 => ThemeChoice::FerrumDark,
            _ => ThemeChoice::FerrumLight,
        },
        terminal: TerminalConfig {
            max_scrollback: c.scrollback.value() as usize,
            cursor_blink_interval_ms: c.cursor_blink.value() as u64,
        },
        layout: LayoutConfig {
            window_padding: c.win_padding.value() as u32,
            tab_bar_height: c.tab_bar.value() as u32,
            pane_inner_padding: c.pane_padding.value() as u32,
            scrollbar_width: c.scrollbar.value() as u32,
        },
        security: SecuritySettings {
            mode: security_mode,
            paste_protection: c.paste.is_active(),
            block_title_query: c.block_title.is_active(),
            limit_cursor_jumps: c.limit_cursor.is_active(),
            clear_mouse_on_reset: c.clear_mouse.is_active(),
        },
        language: crate::i18n::Locale::from_index(c.language.selected() as usize),
    }
}

// ── Security sync ────────────────────────────────────────────────────

fn apply_security_preset(c: &Controls, active: Option<usize>) {
    let switches = [&c.paste, &c.block_title, &c.limit_cursor, &c.clear_mouse];
    match active {
        Some(0) => {
            // Disabled: all off, insensitive.
            for sw in &switches {
                sw.set_active(false);
                sw.set_sensitive(false);
            }
        }
        Some(1) => {
            // Standard: all on, sensitive.
            for sw in &switches {
                sw.set_active(true);
                sw.set_sensitive(true);
            }
        }
        _ => {
            // Custom: keep values, make sensitive.
            for sw in &switches {
                sw.set_sensitive(true);
            }
        }
    }
}

fn infer_security_mode(c: &Controls) {
    let settings = SecuritySettings {
        mode: SecurityMode::Custom,
        paste_protection: c.paste.is_active(),
        block_title_query: c.block_title.is_active(),
        limit_cursor_jumps: c.limit_cursor.is_active(),
        clear_mouse_on_reset: c.clear_mouse.is_active(),
    };
    let inferred = settings.inferred_mode();
    let new_index = match inferred {
        SecurityMode::Disabled => 0,
        SecurityMode::Standard => 1,
        SecurityMode::Custom => 2,
    };
    c.security_mode.set_selected(new_index as u32);

    if matches!(inferred, SecurityMode::Disabled) {
        let switches = [&c.paste, &c.block_title, &c.limit_cursor, &c.clear_mouse];
        for sw in &switches {
            sw.set_sensitive(false);
        }
    }
}

fn reset_controls(c: &Controls) {
    let d = AppConfig::default();
    c.font_size.set_value(d.font.size as f64);
    c.font_family.set_selected(d.font.family.index() as u32);
    c.line_padding.set_value(d.font.line_padding as f64);

    let theme_idx = match d.theme {
        ThemeChoice::FerrumDark => 0,
        ThemeChoice::FerrumLight => 1,
    };
    c.theme.set_selected(theme_idx);

    c.language.set_selected(crate::i18n::Locale::default().index() as u32);
    c.scrollback.set_value(d.terminal.max_scrollback as f64);
    c.cursor_blink.set_value(d.terminal.cursor_blink_interval_ms as f64);

    c.win_padding.set_value(d.layout.window_padding as f64);
    c.pane_padding.set_value(d.layout.pane_inner_padding as f64);
    c.scrollbar.set_value(d.layout.scrollbar_width as f64);
    c.tab_bar.set_value(d.layout.tab_bar_height as f64);

    let mode_idx = match d.security.mode {
        SecurityMode::Disabled => 0u32,
        SecurityMode::Standard => 1,
        SecurityMode::Custom => 2,
    };
    c.security_mode.set_selected(mode_idx);
    apply_security_preset(c, Some(mode_idx as usize));
}
