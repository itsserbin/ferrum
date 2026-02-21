use crate::config::{
    AppConfig, FontConfig, FontFamily, LayoutConfig, SecurityMode, SecuritySettings,
    TerminalConfig, ThemeChoice,
};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicPtr, Ordering};
use std::sync::{mpsc, Mutex};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 Common Controls — manual FFI definitions ──────────────────
// Defined locally: windows-sys may not export every Controls symbol
// under the enabled feature flags.

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
unsafe extern "system" {
    fn InitCommonControlsEx(picce: *const INITCOMMONCONTROLSEX) -> i32;
}

#[allow(non_snake_case)]
#[link(name = "user32")]
unsafe extern "system" {
    fn EnableWindow(hwnd: HWND, enable: i32) -> i32;
    fn GetDpiForWindow(hwnd: HWND) -> u32;
    fn SystemParametersInfoW(uiAction: u32, uiParam: u32, pvParam: *mut core::ffi::c_void, fWinIni: u32) -> i32;
}

#[allow(non_snake_case)]
#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateFontIndirectW(lplf: *const LOGFONTW) -> HFONT;
}

const SPI_GETNONCLIENTMETRICS: u32 = 0x0029;

#[allow(non_camel_case_types, non_snake_case)]
#[repr(C)]
struct LOGFONTW {
    lfHeight: i32,
    lfWidth: i32,
    lfEscapement: i32,
    lfOrientation: i32,
    lfWeight: i32,
    lfItalic: u8,
    lfUnderline: u8,
    lfStrikeOut: u8,
    lfCharSet: u8,
    lfOutPrecision: u8,
    lfClipPrecision: u8,
    lfQuality: u8,
    lfPitchAndFamily: u8,
    lfFaceName: [u16; 32],
}

#[allow(non_camel_case_types, non_snake_case)]
#[repr(C)]
struct NONCLIENTMETRICSW {
    cbSize: u32,
    iBorderWidth: i32,
    iScrollWidth: i32,
    iScrollHeight: i32,
    iCaptionWidth: i32,
    iCaptionHeight: i32,
    lfCaptionFont: LOGFONTW,
    iSmCaptionWidth: i32,
    iSmCaptionHeight: i32,
    lfSmCaptionFont: LOGFONTW,
    iMenuWidth: i32,
    iMenuHeight: i32,
    lfMenuFont: LOGFONTW,
    lfStatusFont: LOGFONTW,
    lfMessageFont: LOGFONTW,
    iPaddedBorderWidth: i32,
}

type HFONT = *mut core::ffi::c_void;

const ICC_TAB_CLASSES: u32 = 0x0008;
const ICC_UPDOWN_CLASS: u32 = 0x0010;
const ICC_STANDARD_CLASSES: u32 = 0x4000;

// Tab control
const TCIF_TEXT: u32 = 0x0001;
const TCM_FIRST: u32 = 0x1300;
const TCM_INSERTITEMW: u32 = TCM_FIRST + 62;
const TCM_GETCURSEL: u32 = TCM_FIRST + 11;
const TCM_SETCURSEL: u32 = TCM_FIRST + 12;
const TCN_SELCHANGE: u32 = (-551i32) as u32;

// UpDown control
const UDS_ALIGNRIGHT: u32 = 0x0004;
const UDS_ARROWKEYS: u32 = 0x0020;
const UDS_NOTHOUSANDS: u32 = 0x0080;
const UDM_SETRANGE32: u32 = 0x046F;
const UDM_SETPOS32: u32 = 0x0471;
const UDM_GETPOS32: u32 = 0x0472;
const UDM_SETBUDDY: u32 = 0x0469;
const UDN_DELTAPOS: u32 = (-722i32) as u32;

// Control styles
const SS_LEFT: u32 = 0x0000_0000;
const CBS_DROPDOWNLIST: u32 = 0x0003;
const CBS_HASSTRINGS: u32 = 0x0200;
const BS_AUTOCHECKBOX: u32 = 0x0003;
const BS_PUSHBUTTON: u32 = 0x0000_0000;
const BST_CHECKED: usize = 0x0001;
const BST_UNCHECKED: usize = 0x0000;
const ES_READONLY: u32 = 0x0800;
const ES_RIGHT: u32 = 0x0002;

// ── Window layout — base values at 96 DPI, scaled at runtime ─────────
mod layout {
    pub const BASE_DPI: i32 = 96;
    pub const MARGIN: i32 = 5;
    pub const TAB_HEADER_H: i32 = 35;
    pub const ROW_SPACING: i32 = 38;
    pub const MAX_ROWS: i32 = 5;
    pub const CONTENT_X: i32 = 20;
    pub const CONTENT_Y: i32 = MARGIN + TAB_HEADER_H;

    pub const CLIENT_W: i32 = 470;
    pub const TAB_W: i32 = CLIENT_W - 2 * MARGIN;
    pub const TAB_H: i32 = TAB_HEADER_H + MAX_ROWS * ROW_SPACING + 15;

    pub const BTN_W: i32 = 150;
    pub const BTN_H: i32 = 30;
    pub const RESET_X: i32 = (CLIENT_W - BTN_W) / 2;
    pub const RESET_Y: i32 = MARGIN + TAB_H + 10;
    pub const CLIENT_H: i32 = RESET_Y + BTN_H + MARGIN;
}

/// Scales a base-96-DPI value to the actual DPI.
fn dpi_scale(value: i32, dpi: u32) -> i32 {
    (value as i64 * dpi as i64 / layout::BASE_DPI as i64) as i32
}

/// Creates a DPI-aware font from NONCLIENTMETRICS, or falls back to DEFAULT_GUI_FONT.
unsafe fn create_dpi_font(dpi: u32) -> *mut core::ffi::c_void {
    unsafe {
        let mut ncm: NONCLIENTMETRICSW = std::mem::zeroed();
        ncm.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
        let ok = SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            ncm.cbSize,
            &mut ncm as *mut _ as *mut core::ffi::c_void,
            0,
        );
        if ok != 0 {
            // Scale the message font height for our DPI.
            let base_height = ncm.lfMessageFont.lfHeight;
            ncm.lfMessageFont.lfHeight = dpi_scale(base_height, dpi);
            let hfont = CreateFontIndirectW(&ncm.lfMessageFont);
            if !hfont.is_null() {
                return hfont;
            }
        }
        GetStockObject(DEFAULT_GUI_FONT) as *mut core::ffi::c_void
    }
}

static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);
static JUST_CLOSED: AtomicBool = AtomicBool::new(false);
/// HWND stored atomically for cross-thread access (close / bring-to-front).
static SETTINGS_HWND: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

/// Tracks whether we are in a programmatic update (e.g. reset, display refresh).
/// Prevents reentrant WM_COMMAND handlers from sending intermediate configs.
static SUPPRESS: AtomicBool = AtomicBool::new(false);
/// Tracks the currently selected tab (updated in on_tab_change).
static CURRENT_TAB: AtomicIsize = AtomicIsize::new(0);
/// When >= 0, the settings window should reopen at this tab after closing.
static REOPEN_WITH_TAB: AtomicIsize = AtomicIsize::new(-1);
/// Config + sender for the reopened window.
static REOPEN_DATA: Mutex<Option<(AppConfig, mpsc::Sender<AppConfig>)>> = Mutex::new(None);

pub fn is_settings_window_open() -> bool {
    WINDOW_OPEN.load(Ordering::Relaxed)
}

pub fn check_window_closed() -> bool {
    JUST_CLOSED.swap(false, Ordering::Relaxed)
}

pub fn close_settings_window() {
    let hwnd = SETTINGS_HWND.load(Ordering::Acquire);
    if !hwnd.is_null() {
        unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
    }
}

/// Returns the index of the currently selected settings tab.
pub fn selected_tab_index() -> usize {
    CURRENT_TAB.load(Ordering::Relaxed).max(0) as usize
}

/// Closes the settings window and reopens it at the given tab with fresh translations.
pub fn request_reopen(config: &AppConfig, tx: mpsc::Sender<AppConfig>, tab_index: usize) {
    *REOPEN_DATA.lock().unwrap() = Some((config.clone(), tx));
    REOPEN_WITH_TAB.store(tab_index as isize, Ordering::Relaxed);
    close_settings_window();
}

pub fn open_settings_window(config: &AppConfig, tx: mpsc::Sender<AppConfig>) {
    if WINDOW_OPEN
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        // Already open — bring to front.
        let hwnd = SETTINGS_HWND.load(Ordering::Acquire);
        if !hwnd.is_null() {
            unsafe { SetForegroundWindow(hwnd) };
        }
        return;
    }

    let config = config.clone();
    std::thread::spawn(move || {
        run_win32_window(config, tx);
    });
}

// ── Win32 state ──────────────────────────────────────────────────────

/// Control IDs — used in WM_COMMAND / WM_NOTIFY to identify controls.
mod id {
    pub const TAB_CONTROL: i32 = 100;
    // Font
    pub const FONT_SIZE_UPDOWN: i32 = 200;
    pub const FONT_SIZE_EDIT: i32 = 201;
    pub const FONT_FAMILY_COMBO: i32 = 202;
    pub const LINE_PADDING_UPDOWN: i32 = 203;
    pub const LINE_PADDING_EDIT: i32 = 204;
    // Theme
    pub const THEME_COMBO: i32 = 300;
    // Terminal
    pub const LANGUAGE_COMBO: i32 = 408;
    pub const SCROLLBACK_UPDOWN: i32 = 400;
    pub const SCROLLBACK_EDIT: i32 = 401;
    pub const CURSOR_BLINK_UPDOWN: i32 = 402;
    pub const CURSOR_BLINK_EDIT: i32 = 403;
    // Layout
    pub const WIN_PADDING_UPDOWN: i32 = 500;
    pub const WIN_PADDING_EDIT: i32 = 501;
    pub const PANE_PADDING_UPDOWN: i32 = 502;
    pub const PANE_PADDING_EDIT: i32 = 503;
    pub const SCROLLBAR_UPDOWN: i32 = 504;
    pub const SCROLLBAR_EDIT: i32 = 505;
    pub const TAB_BAR_UPDOWN: i32 = 506;
    pub const TAB_BAR_EDIT: i32 = 507;
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
    tx: mpsc::Sender<AppConfig>,
    // Tab control
    tab_ctrl: HWND,
    // Font tab
    font_size_updown: HWND,
    font_size_edit: HWND,
    font_family_combo: HWND,
    line_padding_updown: HWND,
    line_padding_edit: HWND,
    // Theme tab
    theme_combo: HWND,
    // Terminal tab
    language_combo: HWND,
    scrollback_updown: HWND,
    scrollback_edit: HWND,
    cursor_blink_updown: HWND,
    cursor_blink_edit: HWND,
    // Layout tab
    win_padding_updown: HWND,
    win_padding_edit: HWND,
    pane_padding_updown: HWND,
    pane_padding_edit: HWND,
    scrollbar_updown: HWND,
    scrollbar_edit: HWND,
    tab_bar_updown: HWND,
    tab_bar_edit: HWND,
    // Security tab
    security_mode_combo: HWND,
    paste_check: HWND,
    block_title_check: HWND,
    limit_cursor_check: HWND,
    clear_mouse_check: HWND,
    // Tab groups (for show/hide)
    tab_pages: [Vec<HWND>; 5],
}

// ── Win32 window ─────────────────────────────────────────────────────

fn run_win32_window(config: AppConfig, tx: mpsc::Sender<AppConfig>) {
    unsafe {
        // Initialize common controls (for tab control + updown).
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_TAB_CLASSES | ICC_UPDOWN_CLASS | ICC_STANDARD_CLASSES,
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
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_WINDOW + 1) as usize as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);

        let mut config = config;
        let mut tx = tx;
        let mut initial_tab: usize = 0;

        loop {
            let t = crate::i18n::t();
            let title = to_wide(t.settings_title);
            let style = WS_OVERLAPPEDWINDOW & !WS_MAXIMIZEBOX & !WS_THICKFRAME;

            // Create window first to get its DPI, then resize.
            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null(),
            );

            let dpi = if !hwnd.is_null() { GetDpiForWindow(hwnd) } else { 96 };
            let dpi = if dpi == 0 { 96 } else { dpi };

            // Resize to DPI-scaled client area.
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: dpi_scale(layout::CLIENT_W, dpi),
                bottom: dpi_scale(layout::CLIENT_H, dpi),
            };
            AdjustWindowRectEx(&mut rect, style, 0, 0);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0, 0,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOMOVE | SWP_NOZORDER,
            );

            if hwnd.is_null() {
                WINDOW_OPEN.store(false, Ordering::Relaxed);
                return;
            }

            // Apply dark title bar.
            let dark: i32 = 1;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                &dark as *const _ as *const _,
                std::mem::size_of::<i32>() as u32,
            );

            let state = create_controls(hwnd, hinstance, &config, tx.clone(), dpi);

            // Select the requested tab (non-zero after a language-change reopen).
            if initial_tab > 0 {
                SendMessageW(state.tab_ctrl, TCM_SETCURSEL, initial_tab, 0);
                on_tab_change(&state);
            }

            // Store state on the window — accessible from wndproc without any mutex.
            let state_ptr = Box::into_raw(Box::new(state));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
            SETTINGS_HWND.store(hwnd, Ordering::Release);

            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);

            // Message loop.
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            // Cleanup done in WM_NCDESTROY handler.

            // Check if a language change requested a reopen.
            let tab = REOPEN_WITH_TAB.swap(-1, Ordering::Relaxed);
            if tab < 0 {
                break; // Normal close — exit thread.
            }
            if let Some((new_config, new_tx)) = REOPEN_DATA.lock().unwrap().take() {
                config = new_config;
                tx = new_tx;
                initial_tab = tab as usize;
                WINDOW_OPEN.store(true, Ordering::Relaxed);
                JUST_CLOSED.store(false, Ordering::Relaxed);
                // Loop back to create a new window with fresh translations.
            } else {
                break;
            }
        }
    }
}

// ── Window procedure ─────────────────────────────────────────────────

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Win32State;

        match msg {
            WM_NOTIFY if !state_ptr.is_null() => {
                let nmhdr = &*(lparam as *const NMHDR);
                if nmhdr.idFrom == id::TAB_CONTROL as usize && nmhdr.code == TCN_SELCHANGE {
                    on_tab_change(&*state_ptr);
                } else if nmhdr.code == UDN_DELTAPOS {
                    // UDN_DELTAPOS fires before position change — defer reading
                    // values until the UpDown has updated its position.
                    PostMessageW(hwnd, WM_APP, 0, 0);
                }
                0
            }
            WM_APP if !state_ptr.is_null() => {
                on_value_changed(&*state_ptr);
                0
            }
            WM_COMMAND if !state_ptr.is_null() => {
                on_command(&*state_ptr, wparam);
                0
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY if !state_ptr.is_null() => {
                // Save config while child controls still exist (they respond
                // to SendMessageW). In WM_NCDESTROY they're already gone.
                let config = build_config(&*state_ptr);
                crate::config::save_config(&config);
                0
            }
            WM_NCDESTROY => {
                // Last message — free state, clean up statics.
                if !state_ptr.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    drop(Box::from_raw(state_ptr));
                }
                SETTINGS_HWND.store(std::ptr::null_mut(), Ordering::Release);
                WINDOW_OPEN.store(false, Ordering::Relaxed);
                JUST_CLOSED.store(true, Ordering::Relaxed);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn on_value_changed(state: &Win32State) {
    if SUPPRESS.load(Ordering::Relaxed) {
        return;
    }
    // SUPPRESS prevents reentrant handlers from sending intermediate configs:
    // update_all_displays → SetWindowTextW → WM_COMMAND(EN_CHANGE) → on_command.
    SUPPRESS.store(true, Ordering::Relaxed);
    update_all_displays(state);
    SUPPRESS.store(false, Ordering::Relaxed);
    let config = build_config(state);
    let _ = state.tx.send(config);
}

fn on_command(state: &Win32State, wparam: WPARAM) {
    if SUPPRESS.load(Ordering::Relaxed) {
        return;
    }
    let notification = ((wparam >> 16) & 0xFFFF) as u32;
    let ctrl_id = (wparam & 0xFFFF) as i32;

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
        (id::FONT_FAMILY_COMBO | id::THEME_COMBO | id::LANGUAGE_COMBO, CBN_SELCHANGE) => {
            let config = build_config(state);
            let _ = state.tx.send(config);
        }
        _ => {}
    }
}

fn on_tab_change(state: &Win32State) {
    let active = unsafe { SendMessageW(state.tab_ctrl, TCM_GETCURSEL, 0, 0) } as usize;
    CURRENT_TAB.store(active as isize, Ordering::Relaxed);
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
    dpi: u32,
) -> Win32State {
    unsafe {
    let font = create_dpi_font(dpi);
    let s = |v: i32| dpi_scale(v, dpi);

    // Create tab control.
    let tab_ctrl_class = to_wide("SysTabControl32");
    let tab_ctrl = CreateWindowExW(
        0,
        tab_ctrl_class.as_ptr(),
        to_wide("").as_ptr(),
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
        s(layout::MARGIN), s(layout::MARGIN), s(layout::TAB_W), s(layout::TAB_H),
        hwnd,
        id::TAB_CONTROL as isize as HMENU,
        hinstance,
        std::ptr::null(),
    );
    SendMessageW(tab_ctrl, WM_SETFONT, font as usize, 0);

    // Add tabs.
    let t = crate::i18n::t();
    for (i, name) in [t.settings_tab_font, t.settings_tab_theme, t.settings_tab_terminal, t.settings_tab_layout, t.settings_tab_security].iter().enumerate() {
        let text = to_wide(name);
        let mut item: TCITEMW = std::mem::zeroed();
        item.mask = TCIF_TEXT;
        item.pszText = text.as_ptr() as *mut _;
        SendMessageW(tab_ctrl, TCM_INSERTITEMW, i, &item as *const _ as LPARAM);
    }

    let x0 = s(layout::CONTENT_X);
    let y0 = s(layout::CONTENT_Y);
    let sp = s(layout::ROW_SPACING);

    let ctx = RowContext { parent: hwnd, hinstance, font, dpi };

    // ── Font tab controls ────────────────────────────────────────────
    let mut font_page = Vec::new();

    // Font Size (updown: 0..48 → SIZE_MIN..SIZE_MAX, step SIZE_STEP)
    let font_size_initial = ((config.font.size - FontConfig::SIZE_MIN) / FontConfig::SIZE_STEP).round() as i32;
    let font_size_range_max = ((FontConfig::SIZE_MAX - FontConfig::SIZE_MIN) / FontConfig::SIZE_STEP).round() as i32;
    let (font_size_updown, font_size_edit, mut ctrls) = create_spin_row(&ctx, &SpinRowParams {
        label_text: t.font_size_label, x: x0, y: y0,
        range_min: 0, range_max: font_size_range_max, initial: font_size_initial,
        updown_id: id::FONT_SIZE_UPDOWN, edit_id: id::FONT_SIZE_EDIT,
    });
    font_page.append(&mut ctrls);

    // Font Family combo
    let (font_family_combo, mut ctrls) = create_combo_row(&ctx, &ComboRowParams {
        label_text: t.font_family_label, x: x0, y: y0 + sp,
        options: FontFamily::DISPLAY_NAMES, selected: config.font.family.index(),
        combo_id: id::FONT_FAMILY_COMBO,
    });
    font_page.append(&mut ctrls);

    // Line Padding (updown: 0..10)
    let (line_padding_updown, line_padding_edit, mut ctrls) = create_spin_row(&ctx, &SpinRowParams {
        label_text: t.font_line_padding_label, x: x0, y: y0 + sp * 2,
        range_min: 0, range_max: 10, initial: config.font.line_padding as i32,
        updown_id: id::LINE_PADDING_UPDOWN, edit_id: id::LINE_PADDING_EDIT,
    });
    font_page.append(&mut ctrls);

    // ── Theme tab controls ───────────────────────────────────────────
    let theme_selected = match config.theme {
        ThemeChoice::FerrumDark => 0,
        ThemeChoice::FerrumLight => 1,
    };
    let (theme_combo, theme_page) = create_combo_row(&ctx, &ComboRowParams {
        label_text: t.theme_label, x: x0, y: y0,
        options: &["Ferrum Dark", "Ferrum Light"], selected: theme_selected,
        combo_id: id::THEME_COMBO,
    });

    // ── Terminal tab controls ────────────────────────────────────────
    let mut terminal_page = Vec::new();

    // Language combo
    let (language_combo, mut ctrls) = create_combo_row(&ctx, &ComboRowParams {
        label_text: t.terminal_language_label,
        x: x0,
        y: y0,
        options: crate::i18n::Locale::DISPLAY_NAMES,
        selected: config.language.index(),
        combo_id: id::LANGUAGE_COMBO,
    });
    terminal_page.append(&mut ctrls);

    // Scrollback (updown: 0..500 → 0..50000, step 100)
    let scrollback_initial = config.terminal.max_scrollback as i32 / 100;
    let (scrollback_updown, scrollback_edit, mut ctrls) = create_spin_row(&ctx, &SpinRowParams {
        label_text: t.terminal_max_scrollback_label, x: x0, y: y0 + sp,
        range_min: 0, range_max: 500, initial: scrollback_initial,
        updown_id: id::SCROLLBACK_UPDOWN, edit_id: id::SCROLLBACK_EDIT,
    });
    terminal_page.append(&mut ctrls);

    // Cursor Blink (updown: 0..N → BLINK_MS_MIN..BLINK_MS_MAX, step BLINK_MS_STEP)
    let blink_initial = (config.terminal.cursor_blink_interval_ms as i64 - TerminalConfig::BLINK_MS_MIN as i64) / TerminalConfig::BLINK_MS_STEP as i64;
    let blink_range_max = ((TerminalConfig::BLINK_MS_MAX - TerminalConfig::BLINK_MS_MIN) / TerminalConfig::BLINK_MS_STEP) as i32;
    let (cursor_blink_updown, cursor_blink_edit, mut ctrls) = create_spin_row(&ctx, &SpinRowParams {
        label_text: t.terminal_cursor_blink_label, x: x0, y: y0 + sp * 2,
        range_min: 0, range_max: blink_range_max, initial: blink_initial as i32,
        updown_id: id::CURSOR_BLINK_UPDOWN, edit_id: id::CURSOR_BLINK_EDIT,
    });
    terminal_page.append(&mut ctrls);

    // ── Layout tab controls ──────────────────────────────────────────
    let mut layout_page = Vec::new();

    let (win_padding_updown, win_padding_edit, mut ctrls) = create_spin_row(&ctx, &SpinRowParams {
        label_text: t.layout_window_padding_label, x: x0, y: y0,
        range_min: 0, range_max: 32, initial: config.layout.window_padding as i32,
        updown_id: id::WIN_PADDING_UPDOWN, edit_id: id::WIN_PADDING_EDIT,
    });
    layout_page.append(&mut ctrls);

    let (pane_padding_updown, pane_padding_edit, mut ctrls) = create_spin_row(&ctx, &SpinRowParams {
        label_text: t.layout_pane_padding_label, x: x0, y: y0 + sp,
        range_min: 0, range_max: 16, initial: config.layout.pane_inner_padding as i32,
        updown_id: id::PANE_PADDING_UPDOWN, edit_id: id::PANE_PADDING_EDIT,
    });
    layout_page.append(&mut ctrls);

    let (scrollbar_updown, scrollbar_edit, mut ctrls) = create_spin_row(&ctx, &SpinRowParams {
        label_text: t.layout_scrollbar_width_label, x: x0, y: y0 + sp * 2,
        range_min: 2, range_max: 16, initial: config.layout.scrollbar_width as i32,
        updown_id: id::SCROLLBAR_UPDOWN, edit_id: id::SCROLLBAR_EDIT,
    });
    layout_page.append(&mut ctrls);

    let (tab_bar_updown, tab_bar_edit, mut ctrls) = create_spin_row(&ctx, &SpinRowParams {
        label_text: t.layout_tab_bar_height_label, x: x0, y: y0 + sp * 3,
        range_min: 24, range_max: 48, initial: config.layout.tab_bar_height as i32,
        updown_id: id::TAB_BAR_UPDOWN, edit_id: id::TAB_BAR_EDIT,
    });
    layout_page.append(&mut ctrls);

    // ── Security tab controls ────────────────────────────────────────
    let mut security_page = Vec::new();

    let mode_index = match config.security.mode {
        SecurityMode::Disabled => 0,
        SecurityMode::Standard => 1,
        SecurityMode::Custom => 2,
    };
    let (security_mode_combo, mut ctrls) = create_combo_row(&ctx, &ComboRowParams {
        label_text: t.security_mode_label, x: x0, y: y0,
        options: &[t.security_mode_disabled, t.security_mode_standard, t.security_mode_custom], selected: mode_index,
        combo_id: id::SECURITY_MODE_COMBO,
    });
    security_page.append(&mut ctrls);

    let enabled = !matches!(config.security.mode, SecurityMode::Disabled);
    let (paste_check, mut ctrls) = create_checkbox_row(&ctx, &CheckboxRowParams {
        label_text: t.security_paste_protection_label, x: x0, y: y0 + sp,
        checked: config.security.paste_protection, enabled, check_id: id::PASTE_CHECK,
    });
    security_page.append(&mut ctrls);

    let (block_title_check, mut ctrls) = create_checkbox_row(&ctx, &CheckboxRowParams {
        label_text: t.security_block_title_query_label, x: x0, y: y0 + sp * 2,
        checked: config.security.block_title_query, enabled, check_id: id::BLOCK_TITLE_CHECK,
    });
    security_page.append(&mut ctrls);

    let (limit_cursor_check, mut ctrls) = create_checkbox_row(&ctx, &CheckboxRowParams {
        label_text: t.security_limit_cursor_jumps_label, x: x0, y: y0 + sp * 3,
        checked: config.security.limit_cursor_jumps, enabled, check_id: id::LIMIT_CURSOR_CHECK,
    });
    security_page.append(&mut ctrls);

    let (clear_mouse_check, mut ctrls) = create_checkbox_row(&ctx, &CheckboxRowParams {
        label_text: t.security_clear_mouse_on_reset_label, x: x0, y: y0 + sp * 4,
        checked: config.security.clear_mouse_on_reset, enabled, check_id: id::CLEAR_MOUSE_CHECK,
    });
    security_page.append(&mut ctrls);

    // ── Reset button (always visible, below tab control) ─────────────
    let reset_text = to_wide(t.settings_reset_to_defaults);
    let reset_btn = CreateWindowExW(
        0,
        to_wide("BUTTON").as_ptr(),
        reset_text.as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        s(layout::RESET_X), s(layout::RESET_Y), s(layout::BTN_W), s(layout::BTN_H),
        hwnd,
        id::RESET_BUTTON as isize as HMENU,
        hinstance,
        std::ptr::null(),
    );
    SendMessageW(reset_btn, WM_SETFONT, font as usize, 0);

    // Initially show only Font tab.
    for page in [&theme_page, &terminal_page, &layout_page, &security_page] {
        for &h in page {
            ShowWindow(h, SW_HIDE);
        }
    }

    let state = Win32State {
        tx,
        tab_ctrl,
        font_size_updown,
        font_size_edit,
        font_family_combo,
        line_padding_updown,
        line_padding_edit,
        theme_combo,
        language_combo,
        scrollback_updown,
        scrollback_edit,
        cursor_blink_updown,
        cursor_blink_edit,
        win_padding_updown,
        win_padding_edit,
        pane_padding_updown,
        pane_padding_edit,
        scrollbar_updown,
        scrollbar_edit,
        tab_bar_updown,
        tab_bar_edit,
        security_mode_combo,
        paste_check,
        block_title_check,
        limit_cursor_check,
        clear_mouse_check,
        tab_pages: [font_page, theme_page, terminal_page, layout_page, security_page],
    };

        update_all_displays(&state);
        state
    }
}

// ── Control builder helpers ──────────────────────────────────────────

/// Shared context for all control row builders (parent window, instance, font, DPI).
struct RowContext {
    parent: HWND,
    hinstance: HINSTANCE,
    font: *mut core::ffi::c_void,
    dpi: u32,
}

struct SpinRowParams<'a> {
    label_text: &'a str,
    x: i32,
    y: i32,
    range_min: i32,
    range_max: i32,
    initial: i32,
    updown_id: i32,
    edit_id: i32,
}

struct ComboRowParams<'a> {
    label_text: &'a str,
    x: i32,
    y: i32,
    options: &'a [&'a str],
    selected: usize,
    combo_id: i32,
}

struct CheckboxRowParams<'a> {
    label_text: &'a str,
    x: i32,
    y: i32,
    checked: bool,
    enabled: bool,
    check_id: i32,
}

/// Creates a row with: static label | read-only edit | updown (spin box).
/// Returns (updown_hwnd, edit_hwnd, vec_of_all_hwnds_for_page).
unsafe fn create_spin_row(ctx: &RowContext, p: &SpinRowParams) -> (HWND, HWND, Vec<HWND>) {
    unsafe {
    let s = |v: i32| dpi_scale(v, ctx.dpi);
    let label_wide = to_wide(p.label_text);
    let lbl = CreateWindowExW(
        0,
        to_wide("STATIC").as_ptr(),
        label_wide.as_ptr(),
        WS_CHILD | WS_VISIBLE | SS_LEFT,
        p.x, p.y + s(3), s(150), s(20),
        ctx.parent,
        std::ptr::null_mut(),
        ctx.hinstance,
        std::ptr::null(),
    );
    SendMessageW(lbl, WM_SETFONT, ctx.font as usize, 0);

    let edit = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        to_wide("EDIT").as_ptr(),
        to_wide("").as_ptr(),
        WS_CHILD | WS_VISIBLE | ES_READONLY | ES_RIGHT,
        p.x + s(160), p.y, s(100), s(24),
        ctx.parent,
        p.edit_id as isize as HMENU,
        ctx.hinstance,
        std::ptr::null(),
    );
    SendMessageW(edit, WM_SETFONT, ctx.font as usize, 0);

    let updown_class = to_wide("msctls_updown32");
    let updown = CreateWindowExW(
        0,
        updown_class.as_ptr(),
        std::ptr::null(),
        WS_CHILD | WS_VISIBLE | UDS_ALIGNRIGHT | UDS_ARROWKEYS | UDS_NOTHOUSANDS,
        0, 0, 0, 0,
        ctx.parent,
        p.updown_id as isize as HMENU,
        ctx.hinstance,
        std::ptr::null(),
    );
    SendMessageW(updown, UDM_SETBUDDY, edit as usize, 0);
    SendMessageW(updown, UDM_SETRANGE32, p.range_min as WPARAM, p.range_max as LPARAM);
    SendMessageW(updown, UDM_SETPOS32, 0, p.initial as LPARAM);

        (updown, edit, vec![lbl, edit, updown])
    }
}

/// Creates a row with: static label | combobox.
/// Returns (combo_hwnd, vec_of_all_hwnds_for_page).
unsafe fn create_combo_row(ctx: &RowContext, p: &ComboRowParams) -> (HWND, Vec<HWND>) {
    unsafe {
    let s = |v: i32| dpi_scale(v, ctx.dpi);
    let label_wide = to_wide(p.label_text);
    let lbl = CreateWindowExW(
        0,
        to_wide("STATIC").as_ptr(),
        label_wide.as_ptr(),
        WS_CHILD | WS_VISIBLE | SS_LEFT,
        p.x, p.y + s(5), s(150), s(20),
        ctx.parent,
        std::ptr::null_mut(),
        ctx.hinstance,
        std::ptr::null(),
    );
    SendMessageW(lbl, WM_SETFONT, ctx.font as usize, 0);

    let combo = CreateWindowExW(
        0,
        to_wide("COMBOBOX").as_ptr(),
        std::ptr::null(),
        WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST | CBS_HASSTRINGS,
        p.x + s(160), p.y, s(200), s(200),
        ctx.parent,
        p.combo_id as isize as HMENU,
        ctx.hinstance,
        std::ptr::null(),
    );
    SendMessageW(combo, WM_SETFONT, ctx.font as usize, 0);

    for opt in p.options {
        let wide = to_wide(opt);
        SendMessageW(combo, CB_ADDSTRING, 0, wide.as_ptr() as LPARAM);
    }
    SendMessageW(combo, CB_SETCURSEL, p.selected, 0);

        (combo, vec![lbl, combo])
    }
}

/// Creates a row with: checkbox.
/// Returns (checkbox_hwnd, vec_of_all_hwnds_for_page).
unsafe fn create_checkbox_row(ctx: &RowContext, p: &CheckboxRowParams) -> (HWND, Vec<HWND>) {
    unsafe {
    let s = |v: i32| dpi_scale(v, ctx.dpi);
    let text = to_wide(p.label_text);
    let check = CreateWindowExW(
        0,
        to_wide("BUTTON").as_ptr(),
        text.as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_AUTOCHECKBOX,
        p.x + s(160), p.y + s(5), s(250), s(20),
        ctx.parent,
        p.check_id as isize as HMENU,
        ctx.hinstance,
        std::ptr::null(),
    );
    SendMessageW(check, WM_SETFONT, ctx.font as usize, 0);

    if p.checked {
        SendMessageW(check, BM_SETCHECK, BST_CHECKED as usize, 0);
    }
    if !p.enabled {
        EnableWindow(check, 0);
    }

        (check, vec![check])
    }
}

// ── Config building ──────────────────────────────────────────────────

fn build_config(state: &Win32State) -> AppConfig {
    unsafe {
        let font_size_pos = SendMessageW(state.font_size_updown, UDM_GETPOS32, 0, 0) as f32;
        let font_size = FontConfig::SIZE_MIN + font_size_pos * FontConfig::SIZE_STEP;

        let font_family_idx = SendMessageW(state.font_family_combo, CB_GETCURSEL, 0, 0) as usize;
        let line_padding = SendMessageW(state.line_padding_updown, UDM_GETPOS32, 0, 0) as u32;

        let theme_idx = SendMessageW(state.theme_combo, CB_GETCURSEL, 0, 0);

        let scrollback_pos = SendMessageW(state.scrollback_updown, UDM_GETPOS32, 0, 0) as usize;
        let scrollback = scrollback_pos * 100;

        let blink_pos = SendMessageW(state.cursor_blink_updown, UDM_GETPOS32, 0, 0) as u64;
        let cursor_blink = TerminalConfig::BLINK_MS_MIN + blink_pos * TerminalConfig::BLINK_MS_STEP;

        let win_padding = SendMessageW(state.win_padding_updown, UDM_GETPOS32, 0, 0) as u32;
        let pane_padding = SendMessageW(state.pane_padding_updown, UDM_GETPOS32, 0, 0) as u32;
        let scrollbar_width = SendMessageW(state.scrollbar_updown, UDM_GETPOS32, 0, 0) as u32;
        let tab_bar_height = SendMessageW(state.tab_bar_updown, UDM_GETPOS32, 0, 0) as u32;

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
            language: crate::i18n::Locale::from_index(
                SendMessageW(state.language_combo, CB_GETCURSEL, 0, 0) as usize,
            ),
        }
    }
}

// ── Display updates ──────────────────────────────────────────────────

fn update_all_displays(state: &Win32State) {
    unsafe {
        let font_size_pos = SendMessageW(state.font_size_updown, UDM_GETPOS32, 0, 0) as f32;
        set_edit_text(state.font_size_edit, &format!("{:.1}", FontConfig::SIZE_MIN + font_size_pos * FontConfig::SIZE_STEP));

        let line_padding = SendMessageW(state.line_padding_updown, UDM_GETPOS32, 0, 0);
        set_edit_text(state.line_padding_edit, &line_padding.to_string());

        let scrollback = SendMessageW(state.scrollback_updown, UDM_GETPOS32, 0, 0) as usize * 100;
        set_edit_text(state.scrollback_edit, &scrollback.to_string());

        let blink = TerminalConfig::BLINK_MS_MIN + SendMessageW(state.cursor_blink_updown, UDM_GETPOS32, 0, 0) as u64 * TerminalConfig::BLINK_MS_STEP;
        set_edit_text(state.cursor_blink_edit, &format!("{blink} ms"));

        let wp = SendMessageW(state.win_padding_updown, UDM_GETPOS32, 0, 0);
        set_edit_text(state.win_padding_edit, &wp.to_string());

        let pp = SendMessageW(state.pane_padding_updown, UDM_GETPOS32, 0, 0);
        set_edit_text(state.pane_padding_edit, &pp.to_string());

        let sw = SendMessageW(state.scrollbar_updown, UDM_GETPOS32, 0, 0);
        set_edit_text(state.scrollbar_edit, &sw.to_string());

        let tb = SendMessageW(state.tab_bar_updown, UDM_GETPOS32, 0, 0);
        set_edit_text(state.tab_bar_edit, &tb.to_string());
    }
}

unsafe fn set_edit_text(hwnd: HWND, text: &str) {
    unsafe {
        let wide = to_wide(text);
        SetWindowTextW(hwnd, wide.as_ptr());
    }
}

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
        let font_size_pos = ((d.font.size - FontConfig::SIZE_MIN) / FontConfig::SIZE_STEP).round() as i32;
        SendMessageW(state.font_size_updown, UDM_SETPOS32, 0, font_size_pos as LPARAM);
        SendMessageW(state.font_family_combo, CB_SETCURSEL, d.font.family.index(), 0);
        SendMessageW(state.line_padding_updown, UDM_SETPOS32, 0, d.font.line_padding as LPARAM);

        // Theme
        let theme_idx = match d.theme {
            ThemeChoice::FerrumDark => 0,
            ThemeChoice::FerrumLight => 1,
        };
        SendMessageW(state.theme_combo, CB_SETCURSEL, theme_idx, 0);

        // Terminal
        SendMessageW(state.language_combo, CB_SETCURSEL, crate::i18n::Locale::default().index(), 0);
        SendMessageW(state.scrollback_updown, UDM_SETPOS32, 0, (d.terminal.max_scrollback / 100) as LPARAM);
        let blink_pos = (d.terminal.cursor_blink_interval_ms as i64 - TerminalConfig::BLINK_MS_MIN as i64) / TerminalConfig::BLINK_MS_STEP as i64;
        SendMessageW(state.cursor_blink_updown, UDM_SETPOS32, 0, blink_pos as LPARAM);

        // Layout
        SendMessageW(state.win_padding_updown, UDM_SETPOS32, 0, d.layout.window_padding as LPARAM);
        SendMessageW(state.pane_padding_updown, UDM_SETPOS32, 0, d.layout.pane_inner_padding as LPARAM);
        SendMessageW(state.scrollbar_updown, UDM_SETPOS32, 0, d.layout.scrollbar_width as LPARAM);
        SendMessageW(state.tab_bar_updown, UDM_SETPOS32, 0, d.layout.tab_bar_height as LPARAM);

        // Security
        let mode_idx = match d.security.mode {
            SecurityMode::Disabled => 0,
            SecurityMode::Standard => 1,
            SecurityMode::Custom => 2,
        };
        SendMessageW(state.security_mode_combo, CB_SETCURSEL, mode_idx, 0);
        apply_security_preset(state);
    }

    update_all_displays(state);
}

// ── Utility ──────────────────────────────────────────────────────────

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
