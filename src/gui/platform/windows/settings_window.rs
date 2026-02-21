use crate::config::{
    AppConfig, FontConfig, FontFamily, LayoutConfig, SecurityMode, SecuritySettings,
    TerminalConfig, ThemeChoice,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 Common Controls — manual FFI definitions ──────────────────
// Defined locally: windows-sys may not export every Controls symbol
// under the enabled feature flags.  Local items shadow glob imports,
// so any overlap with WindowsAndMessaging::* is harmless.

#[allow(non_camel_case_types, non_snake_case)]
#[repr(C)]
struct INITCOMMONCONTROLSEX {
    dwSize: u32,
    dwICC: u32,
}

#[allow(non_camel_case_types, non_snake_case)]
#[repr(C)]
struct TCITEMW {
    mask: u32,
    dwState: u32,
    dwStateMask: u32,
    pszText: *mut u16,
    cchTextMax: i32,
    iImage: i32,
    lParam: LPARAM,
}

#[allow(non_camel_case_types, non_snake_case)]
#[repr(C)]
struct NMHDR {
    hwndFrom: HWND,
    idFrom: usize,
    code: u32,
}

#[allow(non_snake_case)]
#[link(name = "comctl32")]
extern "system" {
    fn InitCommonControlsEx(picce: *const INITCOMMONCONTROLSEX) -> BOOL;
}

#[allow(non_snake_case)]
#[link(name = "user32")]
extern "system" {
    fn EnableWindow(hwnd: HWND, enable: BOOL) -> BOOL;
}

const ICC_TAB_CLASSES: u32 = 0x0008;
const ICC_BAR_CLASSES: u32 = 0x0004;
const ICC_STANDARD_CLASSES: u32 = 0x4000;

// Trackbar (slider) messages
const TBS_AUTOTICKS: u32 = 0x0001;
const TBM_GETPOS: u32 = 0x0400;
const TBM_SETPOS: u32 = 0x0405;
const TBM_SETRANGEMIN: u32 = 0x0407;
const TBM_SETRANGEMAX: u32 = 0x0408;
const TBM_SETTICFREQ: u32 = 0x0414;

// Tab control
const TCIF_TEXT: u32 = 0x0001;
const TCM_FIRST: u32 = 0x1300;
const TCM_INSERTITEMW: u32 = TCM_FIRST + 62;
const TCM_GETCURSEL: u32 = TCM_FIRST + 11;
const TCN_SELCHANGE: u32 = (-551i32) as u32;

// Control styles — u32 to combine with WS_* window style constants.
const SS_LEFT: u32 = 0x0000_0000;
const CBS_DROPDOWNLIST: u32 = 0x0003;
const CBS_HASSTRINGS: u32 = 0x0200;
const BS_AUTOCHECKBOX: u32 = 0x0003;
const BS_PUSHBUTTON: u32 = 0x0000_0000;

static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);
static JUST_CLOSED: AtomicBool = AtomicBool::new(false);
static SETTINGS_STATE: OnceLock<Mutex<Option<Win32State>>> = OnceLock::new();

/// Tracks whether we are in a programmatic update (e.g. reset, security sync).
static SUPPRESS: AtomicBool = AtomicBool::new(false);

pub fn is_settings_window_open() -> bool {
    WINDOW_OPEN.load(Ordering::Relaxed)
}

pub fn check_window_closed() -> bool {
    JUST_CLOSED.swap(false, Ordering::Relaxed)
}

pub fn close_settings_window() {
    let mutex = SETTINGS_STATE.get_or_init(|| Mutex::new(None));
    if let Some(state) = mutex.lock().unwrap().take() {
        let config = build_config(&state);
        crate::config::save_config(&config);
        unsafe { DestroyWindow(state.hwnd) };
    }
    WINDOW_OPEN.store(false, Ordering::Relaxed);
}

pub fn open_settings_window(config: &AppConfig, tx: mpsc::Sender<AppConfig>) {
    if WINDOW_OPEN.load(Ordering::Relaxed) {
        // Bring existing window to front.
        let mutex = SETTINGS_STATE.get_or_init(|| Mutex::new(None));
        if let Some(ref state) = *mutex.lock().unwrap() {
            unsafe {
                SetForegroundWindow(state.hwnd);
            }
        }
        return;
    }
    WINDOW_OPEN.store(true, Ordering::Relaxed);

    let config = config.clone();
    std::thread::spawn(move || {
        run_win32_window(config, tx);
    });
}

// ── Win32 state ──────────────────────────────────────────────────────

/// Control IDs — used in WM_COMMAND / WM_HSCROLL to identify controls.
mod id {
    pub const TAB_CONTROL: i32 = 100;
    // Font
    pub const FONT_SIZE_TRACK: i32 = 200;
    pub const FONT_SIZE_LABEL: i32 = 201;
    pub const FONT_FAMILY_COMBO: i32 = 202;
    pub const LINE_PADDING_TRACK: i32 = 203;
    pub const LINE_PADDING_LABEL: i32 = 204;
    // Theme
    pub const THEME_COMBO: i32 = 300;
    // Terminal
    pub const SCROLLBACK_TRACK: i32 = 400;
    pub const SCROLLBACK_LABEL: i32 = 401;
    pub const CURSOR_BLINK_TRACK: i32 = 402;
    pub const CURSOR_BLINK_LABEL: i32 = 403;
    // Layout
    pub const WIN_PADDING_TRACK: i32 = 500;
    pub const WIN_PADDING_LABEL: i32 = 501;
    pub const PANE_PADDING_TRACK: i32 = 502;
    pub const PANE_PADDING_LABEL: i32 = 503;
    pub const SCROLLBAR_TRACK: i32 = 504;
    pub const SCROLLBAR_LABEL: i32 = 505;
    pub const TAB_BAR_TRACK: i32 = 506;
    pub const TAB_BAR_LABEL: i32 = 507;
    // Security
    pub const SECURITY_MODE_COMBO: i32 = 600;
    pub const PASTE_CHECK: i32 = 601;
    pub const BLOCK_TITLE_CHECK: i32 = 602;
    pub const LIMIT_CURSOR_CHECK: i32 = 603;
    pub const CLEAR_MOUSE_CHECK: i32 = 604;
    // Reset
    pub const RESET_BUTTON: i32 = 700;
}

struct Win32State {
    hwnd: HWND,
    tx: mpsc::Sender<AppConfig>,
    // Tab control
    tab_ctrl: HWND,
    // Font tab
    font_size_track: HWND,
    font_size_label: HWND,
    font_family_combo: HWND,
    line_padding_track: HWND,
    line_padding_label: HWND,
    // Theme tab
    theme_combo: HWND,
    // Terminal tab
    scrollback_track: HWND,
    scrollback_label: HWND,
    cursor_blink_track: HWND,
    cursor_blink_label: HWND,
    // Layout tab
    win_padding_track: HWND,
    win_padding_label: HWND,
    pane_padding_track: HWND,
    pane_padding_label: HWND,
    scrollbar_track: HWND,
    scrollbar_label: HWND,
    tab_bar_track: HWND,
    tab_bar_label: HWND,
    // Security tab
    security_mode_combo: HWND,
    paste_check: HWND,
    block_title_check: HWND,
    limit_cursor_check: HWND,
    clear_mouse_check: HWND,
    // Tab groups (for show/hide)
    tab_pages: [Vec<HWND>; 5],
}

// SAFETY: Win32 HWNDs are thread-safe integer handles.
unsafe impl Send for Win32State {}

// ── Win32 window ─────────────────────────────────────────────────────

fn run_win32_window(config: AppConfig, tx: mpsc::Sender<AppConfig>) {
    unsafe {
        // Initialize common controls (for tab control + trackbars).
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_TAB_CLASSES | ICC_BAR_CLASSES | ICC_STANDARD_CLASSES,
        };
        InitCommonControlsEx(&icc);

        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide("FerrumSettingsClass");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: 0,
            hCursor: LoadCursorW(0, IDC_ARROW),
            hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: 0,
        };
        RegisterClassExW(&wc);

        let title = to_wide("Ferrum Settings");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW & !WS_MAXIMIZEBOX & !WS_THICKFRAME,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            520,
            480,
            0,
            0,
            hinstance,
            std::ptr::null(),
        );

        // Apply dark title bar.
        let dark: i32 = 1;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );

        let state = create_controls(hwnd, hinstance, &config, tx);

        // Store state.
        let mutex = SETTINGS_STATE.get_or_init(|| Mutex::new(None));
        *mutex.lock().unwrap() = Some(state);

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        // Message loop.
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    // Cleanup after window closed.
    let mutex = SETTINGS_STATE.get_or_init(|| Mutex::new(None));
    if let Some(state) = mutex.lock().unwrap().take() {
        let config = build_config(&state);
        crate::config::save_config(&config);
    }
    WINDOW_OPEN.store(false, Ordering::Relaxed);
    JUST_CLOSED.store(true, Ordering::Relaxed);
}

// ── Window procedure ─────────────────────────────────────────────────

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_HSCROLL => {
            on_hscroll();
            0
        }
        WM_COMMAND => {
            on_command(wparam);
            0
        }
        WM_NOTIFY => {
            let nmhdr = &*(lparam as *const NMHDR);
            if nmhdr.idFrom == id::TAB_CONTROL as usize && nmhdr.code == TCN_SELCHANGE {
                on_tab_change();
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn on_hscroll() {
    if SUPPRESS.load(Ordering::Relaxed) {
        return;
    }
    let mutex = SETTINGS_STATE.get_or_init(|| Mutex::new(None));
    let guard = mutex.lock().unwrap();
    if let Some(ref state) = *guard {
        update_all_labels(state);
        let config = build_config(state);
        let _ = state.tx.send(config);
    }
}

fn on_command(wparam: WPARAM) {
    if SUPPRESS.load(Ordering::Relaxed) {
        return;
    }
    let notification = ((wparam >> 16) & 0xFFFF) as u32;
    let ctrl_id = (wparam & 0xFFFF) as i32;

    let mutex = SETTINGS_STATE.get_or_init(|| Mutex::new(None));
    let guard = mutex.lock().unwrap();
    let Some(ref state) = *guard else { return };

    match (ctrl_id, notification) {
        (id::RESET_BUTTON, BN_CLICKED) => {
            SUPPRESS.store(true, Ordering::Relaxed);
            reset_controls(state);
            SUPPRESS.store(false, Ordering::Relaxed);
            let config = build_config(state);
            let _ = state.tx.send(config);
        }
        (id::SECURITY_MODE_COMBO, CBN_SELCHANGE) => {
            SUPPRESS.store(true, Ordering::Relaxed);
            apply_security_preset(state);
            SUPPRESS.store(false, Ordering::Relaxed);
            let config = build_config(state);
            let _ = state.tx.send(config);
        }
        (id::PASTE_CHECK | id::BLOCK_TITLE_CHECK | id::LIMIT_CURSOR_CHECK | id::CLEAR_MOUSE_CHECK, BN_CLICKED) => {
            SUPPRESS.store(true, Ordering::Relaxed);
            infer_security_mode(state);
            SUPPRESS.store(false, Ordering::Relaxed);
            let config = build_config(state);
            let _ = state.tx.send(config);
        }
        (id::FONT_FAMILY_COMBO | id::THEME_COMBO, CBN_SELCHANGE) => {
            let config = build_config(state);
            let _ = state.tx.send(config);
        }
        _ => {}
    }
}

fn on_tab_change() {
    let mutex = SETTINGS_STATE.get_or_init(|| Mutex::new(None));
    let guard = mutex.lock().unwrap();
    let Some(ref state) = *guard else { return };
    let active = unsafe { SendMessageW(state.tab_ctrl, TCM_GETCURSEL, 0, 0) } as usize;
    for (i, page) in state.tab_pages.iter().enumerate() {
        let show = if i == active { SW_SHOW } else { SW_HIDE };
        for &hwnd in page {
            unsafe { ShowWindow(hwnd, show) };
        }
    }
}

// ── Control creation ─────────────────────────────────────────────────

unsafe fn create_controls(
    hwnd: HWND,
    hinstance: HINSTANCE,
    config: &AppConfig,
    tx: mpsc::Sender<AppConfig>,
) -> Win32State { unsafe {
    // Create tab control.
    let tab_ctrl_class = to_wide("SysTabControl32");
    let tab_ctrl = create_window(
        0,
        tab_ctrl_class.as_ptr(),
        &[],
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
        5,
        5,
        500,
        400,
        hwnd,
        id::TAB_CONTROL,
        hinstance,
    );

    // Add tabs.
    for (i, name) in ["Font", "Theme", "Terminal", "Layout", "Security"].iter().enumerate() {
        let text = to_wide(name);
        let mut item: TCITEMW = std::mem::zeroed();
        item.mask = TCIF_TEXT;
        item.pszText = text.as_ptr() as *mut _;
        SendMessageW(tab_ctrl, TCM_INSERTITEMW, i, &item as *const _ as LPARAM);
    }

    let x0 = 20; // Left margin inside tab control
    let y0 = 40; // Top margin (below tab headers)
    let spacing = 55; // Vertical spacing between rows

    // ── Font tab controls ────────────────────────────────────────────
    let mut font_page = Vec::new();

    // Font Size (trackbar: 0..48 → 8.0..32.0, step 0.5)
    let font_size_initial = ((config.font.size - 8.0) / 0.5).round() as isize;
    let (font_size_track, font_size_label, mut ctrls) = create_trackbar_row(
        hwnd, hinstance, "Font Size:", x0, y0, 0, 48, font_size_initial,
        id::FONT_SIZE_TRACK, id::FONT_SIZE_LABEL,
    );
    font_page.append(&mut ctrls);

    // Font Family combo
    let (font_family_combo, mut ctrls) = create_combo_row(
        hwnd, hinstance, "Font Family:", x0, y0 + spacing,
        FontFamily::DISPLAY_NAMES, config.font.family.index(),
        id::FONT_FAMILY_COMBO,
    );
    font_page.append(&mut ctrls);

    // Line Padding (trackbar: 0..10)
    let (line_padding_track, line_padding_label, mut ctrls) = create_trackbar_row(
        hwnd, hinstance, "Line Padding:", x0, y0 + spacing * 2, 0, 10,
        config.font.line_padding as isize,
        id::LINE_PADDING_TRACK, id::LINE_PADDING_LABEL,
    );
    font_page.append(&mut ctrls);

    // ── Theme tab controls ───────────────────────────────────────────
    let theme_selected = match config.theme {
        ThemeChoice::FerrumDark => 0,
        ThemeChoice::FerrumLight => 1,
    };
    let (theme_combo, theme_page) = create_combo_row(
        hwnd, hinstance, "Theme:", x0, y0,
        &["Ferrum Dark", "Ferrum Light"], theme_selected,
        id::THEME_COMBO,
    );

    // ── Terminal tab controls ────────────────────────────────────────
    let mut terminal_page = Vec::new();

    // Scrollback (trackbar: 0..500 → 0..50000, step 100)
    let scrollback_initial = config.terminal.max_scrollback as isize / 100;
    let (scrollback_track, scrollback_label, mut ctrls) = create_trackbar_row(
        hwnd, hinstance, "Max Scrollback:", x0, y0, 0, 500, scrollback_initial,
        id::SCROLLBACK_TRACK, id::SCROLLBACK_LABEL,
    );
    terminal_page.append(&mut ctrls);

    // Cursor Blink (trackbar: 2..40 → 100..2000, step 50)
    let blink_initial = (config.terminal.cursor_blink_interval_ms as isize - 100) / 50;
    let (cursor_blink_track, cursor_blink_label, mut ctrls) = create_trackbar_row(
        hwnd, hinstance, "Cursor Blink (ms):", x0, y0 + spacing, 0, 38, blink_initial,
        id::CURSOR_BLINK_TRACK, id::CURSOR_BLINK_LABEL,
    );
    terminal_page.append(&mut ctrls);

    // ── Layout tab controls ──────────────────────────────────────────
    let mut layout_page = Vec::new();

    let (win_padding_track, win_padding_label, mut ctrls) = create_trackbar_row(
        hwnd, hinstance, "Window Padding:", x0, y0, 0, 32,
        config.layout.window_padding as isize,
        id::WIN_PADDING_TRACK, id::WIN_PADDING_LABEL,
    );
    layout_page.append(&mut ctrls);

    let (pane_padding_track, pane_padding_label, mut ctrls) = create_trackbar_row(
        hwnd, hinstance, "Pane Padding:", x0, y0 + spacing, 0, 16,
        config.layout.pane_inner_padding as isize,
        id::PANE_PADDING_TRACK, id::PANE_PADDING_LABEL,
    );
    layout_page.append(&mut ctrls);

    let (scrollbar_track, scrollbar_label, mut ctrls) = create_trackbar_row(
        hwnd, hinstance, "Scrollbar Width:", x0, y0 + spacing * 2, 2, 16,
        config.layout.scrollbar_width as isize,
        id::SCROLLBAR_TRACK, id::SCROLLBAR_LABEL,
    );
    layout_page.append(&mut ctrls);

    let (tab_bar_track, tab_bar_label, mut ctrls) = create_trackbar_row(
        hwnd, hinstance, "Tab Bar Height:", x0, y0 + spacing * 3, 24, 48,
        config.layout.tab_bar_height as isize,
        id::TAB_BAR_TRACK, id::TAB_BAR_LABEL,
    );
    layout_page.append(&mut ctrls);

    // ── Security tab controls ────────────────────────────────────────
    let mut security_page = Vec::new();

    let mode_index = match config.security.mode {
        SecurityMode::Disabled => 0,
        SecurityMode::Standard => 1,
        SecurityMode::Custom => 2,
    };
    let (security_mode_combo, mut ctrls) = create_combo_row(
        hwnd, hinstance, "Security Mode:", x0, y0,
        &["Disabled", "Standard", "Custom"], mode_index,
        id::SECURITY_MODE_COMBO,
    );
    security_page.append(&mut ctrls);

    let enabled = !matches!(config.security.mode, SecurityMode::Disabled);
    let (paste_check, mut ctrls) = create_checkbox_row(
        hwnd, hinstance, "Paste Protection", x0, y0 + spacing,
        config.security.paste_protection, enabled, id::PASTE_CHECK,
    );
    security_page.append(&mut ctrls);

    let (block_title_check, mut ctrls) = create_checkbox_row(
        hwnd, hinstance, "Block Title Query", x0, y0 + spacing * 2,
        config.security.block_title_query, enabled, id::BLOCK_TITLE_CHECK,
    );
    security_page.append(&mut ctrls);

    let (limit_cursor_check, mut ctrls) = create_checkbox_row(
        hwnd, hinstance, "Limit Cursor Jumps", x0, y0 + spacing * 3,
        config.security.limit_cursor_jumps, enabled, id::LIMIT_CURSOR_CHECK,
    );
    security_page.append(&mut ctrls);

    let (clear_mouse_check, mut ctrls) = create_checkbox_row(
        hwnd, hinstance, "Clear Mouse on Reset", x0, y0 + spacing * 4,
        config.security.clear_mouse_on_reset, enabled, id::CLEAR_MOUSE_CHECK,
    );
    security_page.append(&mut ctrls);

    // ── Reset button (always visible, below tab control) ─────────────
    let reset_text = to_wide("Reset to Defaults");
    let reset_btn = CreateWindowExW(
        0,
        to_wide("BUTTON").as_ptr(),
        reset_text.as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        180,
        420,
        150,
        30,
        hwnd,
        id::RESET_BUTTON as HMENU,
        hinstance,
        std::ptr::null(),
    );
    let _ = reset_btn;

    // Initially show only Font tab.
    for page in [&theme_page, &terminal_page, &layout_page, &security_page] {
        for &h in page {
            ShowWindow(h, SW_HIDE);
        }
    }

    let state = Win32State {
        hwnd,
        tx,
        tab_ctrl,
        font_size_track,
        font_size_label,
        font_family_combo,
        line_padding_track,
        line_padding_label,
        theme_combo,
        scrollback_track,
        scrollback_label,
        cursor_blink_track,
        cursor_blink_label,
        win_padding_track,
        win_padding_label,
        pane_padding_track,
        pane_padding_label,
        scrollbar_track,
        scrollbar_label,
        tab_bar_track,
        tab_bar_label,
        security_mode_combo,
        paste_check,
        block_title_check,
        limit_cursor_check,
        clear_mouse_check,
        tab_pages: [font_page, theme_page, terminal_page, layout_page, security_page],
    };

    update_all_labels(&state);
    state
} }

// ── Control builder helpers ──────────────────────────────────────────

unsafe fn create_window(
    ex_style: u32,
    class: *const u16,
    text: &[u16],
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    parent: HWND,
    ctrl_id: i32,
    hinstance: HINSTANCE,
) -> HWND { unsafe {
    let text_ptr = if text.is_empty() {
        to_wide("").as_ptr()
    } else {
        text.as_ptr()
    };
    CreateWindowExW(
        ex_style,
        class,
        text_ptr,
        style,
        x,
        y,
        w,
        h,
        parent,
        ctrl_id as HMENU,
        hinstance,
        std::ptr::null(),
    )
} }

/// Creates a row with: static label | trackbar | value label.
/// Returns (trackbar_hwnd, value_label_hwnd, vec_of_all_hwnds_for_page).
unsafe fn create_trackbar_row(
    parent: HWND,
    hinstance: HINSTANCE,
    label_text: &str,
    x: i32,
    y: i32,
    range_min: isize,
    range_max: isize,
    initial: isize,
    track_id: i32,
    label_id: i32,
) -> (HWND, HWND, Vec<HWND>) { unsafe {
    let label_wide = to_wide(label_text);
    let lbl = CreateWindowExW(
        0,
        to_wide("STATIC").as_ptr(),
        label_wide.as_ptr(),
        WS_CHILD | WS_VISIBLE | SS_LEFT,
        x,
        y + 5,
        130,
        20,
        parent,
        0,
        hinstance,
        std::ptr::null(),
    );

    let trackbar_class = to_wide("msctls_trackbar32");
    let track = CreateWindowExW(
        0,
        trackbar_class.as_ptr(),
        std::ptr::null(),
        WS_CHILD | WS_VISIBLE | TBS_AUTOTICKS,
        x + 140,
        y,
        250,
        35,
        parent,
        track_id as HMENU,
        hinstance,
        std::ptr::null(),
    );
    SendMessageW(track, TBM_SETRANGEMIN, 0, range_min as LPARAM);
    SendMessageW(track, TBM_SETRANGEMAX, 0, range_max as LPARAM);
    SendMessageW(track, TBM_SETPOS, 1, initial as LPARAM);
    SendMessageW(track, TBM_SETTICFREQ, 1, 0);

    let val_label = CreateWindowExW(
        0,
        to_wide("STATIC").as_ptr(),
        to_wide("").as_ptr(),
        WS_CHILD | WS_VISIBLE | SS_LEFT,
        x + 400,
        y + 5,
        60,
        20,
        parent,
        label_id as HMENU,
        hinstance,
        std::ptr::null(),
    );

    (track, val_label, vec![lbl, track, val_label])
} }

/// Creates a row with: static label | combobox.
/// Returns (combo_hwnd, vec_of_all_hwnds_for_page).
unsafe fn create_combo_row(
    parent: HWND,
    hinstance: HINSTANCE,
    label_text: &str,
    x: i32,
    y: i32,
    options: &[&str],
    selected: usize,
    combo_id: i32,
) -> (HWND, Vec<HWND>) { unsafe {
    let label_wide = to_wide(label_text);
    let lbl = CreateWindowExW(
        0,
        to_wide("STATIC").as_ptr(),
        label_wide.as_ptr(),
        WS_CHILD | WS_VISIBLE | SS_LEFT,
        x,
        y + 5,
        130,
        20,
        parent,
        0,
        hinstance,
        std::ptr::null(),
    );

    let combo = CreateWindowExW(
        0,
        to_wide("COMBOBOX").as_ptr(),
        std::ptr::null(),
        WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST | CBS_HASSTRINGS,
        x + 140,
        y,
        250,
        200, // Drop-down height
        parent,
        combo_id as HMENU,
        hinstance,
        std::ptr::null(),
    );

    for opt in options {
        let wide = to_wide(opt);
        SendMessageW(combo, CB_ADDSTRING, 0, wide.as_ptr() as LPARAM);
    }
    SendMessageW(combo, CB_SETCURSEL, selected, 0);

    (combo, vec![lbl, combo])
} }

/// Creates a row with: checkbox.
/// Returns (checkbox_hwnd, vec_of_all_hwnds_for_page).
unsafe fn create_checkbox_row(
    parent: HWND,
    hinstance: HINSTANCE,
    label_text: &str,
    x: i32,
    y: i32,
    checked: bool,
    enabled: bool,
    check_id: i32,
) -> (HWND, Vec<HWND>) { unsafe {
    let text = to_wide(label_text);
    let check = CreateWindowExW(
        0,
        to_wide("BUTTON").as_ptr(),
        text.as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_AUTOCHECKBOX,
        x + 140,
        y + 5,
        250,
        20,
        parent,
        check_id as HMENU,
        hinstance,
        std::ptr::null(),
    );

    if checked {
        SendMessageW(check, BM_SETCHECK, BST_CHECKED as usize, 0);
    }
    if !enabled {
        EnableWindow(check, 0);
    }

    (check, vec![check])
} }

// ── Config building ──────────────────────────────────────────────────

fn build_config(state: &Win32State) -> AppConfig {
    unsafe {
        let font_size_pos = SendMessageW(state.font_size_track, TBM_GETPOS, 0, 0) as f32;
        let font_size = 8.0 + font_size_pos * 0.5;

        let font_family_idx = SendMessageW(state.font_family_combo, CB_GETCURSEL, 0, 0) as usize;
        let line_padding = SendMessageW(state.line_padding_track, TBM_GETPOS, 0, 0) as u32;

        let theme_idx = SendMessageW(state.theme_combo, CB_GETCURSEL, 0, 0);

        let scrollback_pos = SendMessageW(state.scrollback_track, TBM_GETPOS, 0, 0) as usize;
        let scrollback = scrollback_pos * 100;

        let blink_pos = SendMessageW(state.cursor_blink_track, TBM_GETPOS, 0, 0) as u64;
        let cursor_blink = 100 + blink_pos * 50;

        let win_padding = SendMessageW(state.win_padding_track, TBM_GETPOS, 0, 0) as u32;
        let pane_padding = SendMessageW(state.pane_padding_track, TBM_GETPOS, 0, 0) as u32;
        let scrollbar_width = SendMessageW(state.scrollbar_track, TBM_GETPOS, 0, 0) as u32;
        let tab_bar_height = SendMessageW(state.tab_bar_track, TBM_GETPOS, 0, 0) as u32;

        let security_mode_idx = SendMessageW(state.security_mode_combo, CB_GETCURSEL, 0, 0);
        let security_mode = match security_mode_idx {
            0 => SecurityMode::Disabled,
            1 => SecurityMode::Standard,
            _ => SecurityMode::Custom,
        };

        let paste = SendMessageW(state.paste_check, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
        let block_title = SendMessageW(state.block_title_check, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
        let limit_cursor = SendMessageW(state.limit_cursor_check, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
        let clear_mouse = SendMessageW(state.clear_mouse_check, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;

        AppConfig {
            font: FontConfig {
                size: font_size,
                family: FontFamily::from_index(font_family_idx),
                line_padding,
            },
            theme: match theme_idx {
                0 => ThemeChoice::FerrumDark,
                _ => ThemeChoice::FerrumLight,
            },
            terminal: TerminalConfig {
                max_scrollback: scrollback,
                cursor_blink_interval_ms: cursor_blink,
            },
            layout: LayoutConfig {
                window_padding: win_padding,
                tab_bar_height,
                pane_inner_padding: pane_padding,
                scrollbar_width,
            },
            security: SecuritySettings {
                mode: security_mode,
                paste_protection: paste,
                block_title_query: block_title,
                limit_cursor_jumps: limit_cursor,
                clear_mouse_on_reset: clear_mouse,
            },
        }
    }
}

// ── Label updates ────────────────────────────────────────────────────

fn update_all_labels(state: &Win32State) {
    unsafe {
        let font_size_pos = SendMessageW(state.font_size_track, TBM_GETPOS, 0, 0) as f32;
        set_label_text(state.font_size_label, &format!("{:.1}", 8.0 + font_size_pos * 0.5));

        let line_padding = SendMessageW(state.line_padding_track, TBM_GETPOS, 0, 0);
        set_label_text(state.line_padding_label, &line_padding.to_string());

        let scrollback = SendMessageW(state.scrollback_track, TBM_GETPOS, 0, 0) as usize * 100;
        set_label_text(state.scrollback_label, &scrollback.to_string());

        let blink = 100 + SendMessageW(state.cursor_blink_track, TBM_GETPOS, 0, 0) as u64 * 50;
        set_label_text(state.cursor_blink_label, &format!("{blink} ms"));

        let wp = SendMessageW(state.win_padding_track, TBM_GETPOS, 0, 0);
        set_label_text(state.win_padding_label, &wp.to_string());

        let pp = SendMessageW(state.pane_padding_track, TBM_GETPOS, 0, 0);
        set_label_text(state.pane_padding_label, &pp.to_string());

        let sw = SendMessageW(state.scrollbar_track, TBM_GETPOS, 0, 0);
        set_label_text(state.scrollbar_label, &sw.to_string());

        let tb = SendMessageW(state.tab_bar_track, TBM_GETPOS, 0, 0);
        set_label_text(state.tab_bar_label, &tb.to_string());
    }
}

unsafe fn set_label_text(hwnd: HWND, text: &str) { unsafe {
    let wide = to_wide(text);
    SetWindowTextW(hwnd, wide.as_ptr());
} }

// ── Security sync ────────────────────────────────────────────────────

fn apply_security_preset(state: &Win32State) {
    unsafe {
        let mode = SendMessageW(state.security_mode_combo, CB_GETCURSEL, 0, 0);
        let checks = [
            state.paste_check,
            state.block_title_check,
            state.limit_cursor_check,
            state.clear_mouse_check,
        ];
        match mode {
            0 => {
                // Disabled: uncheck all, disable.
                for &c in &checks {
                    SendMessageW(c, BM_SETCHECK, BST_UNCHECKED as usize, 0);
                    EnableWindow(c, 0);
                }
            }
            1 => {
                // Standard: check all, enable.
                for &c in &checks {
                    SendMessageW(c, BM_SETCHECK, BST_CHECKED as usize, 0);
                    EnableWindow(c, 1);
                }
            }
            _ => {
                // Custom: just enable.
                for &c in &checks {
                    EnableWindow(c, 1);
                }
            }
        }
    }
}

fn infer_security_mode(state: &Win32State) {
    unsafe {
        let settings = SecuritySettings {
            mode: SecurityMode::Custom,
            paste_protection: SendMessageW(state.paste_check, BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
            block_title_query: SendMessageW(state.block_title_check, BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
            limit_cursor_jumps: SendMessageW(state.limit_cursor_check, BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
            clear_mouse_on_reset: SendMessageW(state.clear_mouse_check, BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
        };
        let inferred = settings.inferred_mode();
        let new_index = match inferred {
            SecurityMode::Disabled => 0,
            SecurityMode::Standard => 1,
            SecurityMode::Custom => 2,
        };
        SendMessageW(state.security_mode_combo, CB_SETCURSEL, new_index, 0);

        if matches!(inferred, SecurityMode::Disabled) {
            let checks = [
                state.paste_check,
                state.block_title_check,
                state.limit_cursor_check,
                state.clear_mouse_check,
            ];
            for &c in &checks {
                EnableWindow(c, 0);
            }
        }
    }
}

fn reset_controls(state: &Win32State) {
    let d = AppConfig::default();
    unsafe {
        // Font
        let font_size_pos = ((d.font.size - 8.0) / 0.5).round() as isize;
        SendMessageW(state.font_size_track, TBM_SETPOS, 1, font_size_pos as LPARAM);
        SendMessageW(state.font_family_combo, CB_SETCURSEL, d.font.family.index(), 0);
        SendMessageW(state.line_padding_track, TBM_SETPOS, 1, d.font.line_padding as LPARAM);

        // Theme
        let theme_idx = match d.theme {
            ThemeChoice::FerrumDark => 0,
            ThemeChoice::FerrumLight => 1,
        };
        SendMessageW(state.theme_combo, CB_SETCURSEL, theme_idx, 0);

        // Terminal
        SendMessageW(state.scrollback_track, TBM_SETPOS, 1, (d.terminal.max_scrollback / 100) as LPARAM);
        let blink_pos = (d.terminal.cursor_blink_interval_ms as isize - 100) / 50;
        SendMessageW(state.cursor_blink_track, TBM_SETPOS, 1, blink_pos as LPARAM);

        // Layout
        SendMessageW(state.win_padding_track, TBM_SETPOS, 1, d.layout.window_padding as LPARAM);
        SendMessageW(state.pane_padding_track, TBM_SETPOS, 1, d.layout.pane_inner_padding as LPARAM);
        SendMessageW(state.scrollbar_track, TBM_SETPOS, 1, d.layout.scrollbar_width as LPARAM);
        SendMessageW(state.tab_bar_track, TBM_SETPOS, 1, d.layout.tab_bar_height as LPARAM);

        // Security
        let mode_idx = match d.security.mode {
            SecurityMode::Disabled => 0,
            SecurityMode::Standard => 1,
            SecurityMode::Custom => 2,
        };
        SendMessageW(state.security_mode_combo, CB_SETCURSEL, mode_idx, 0);
        apply_security_preset(state);
    }

    update_all_labels(state);
}

// ── Utility ──────────────────────────────────────────────────────────

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
