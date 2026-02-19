pub mod backend;
mod context_menu;
mod cpu;
mod cursor;
#[cfg(feature = "gpu")]
pub mod gpu;
pub mod metrics;
mod scrollbar;
mod security;
pub mod shared;
mod tab_bar;
mod terminal;
pub mod traits;
pub mod types;

use crate::core::{Color, CursorStyle, Grid, Selection};

pub use backend::RendererBackend;
pub use cpu::CpuRenderer;
pub use traits::Renderer;

pub(super) const FONT_SIZE: f32 = 15.0;
pub(super) const LINE_PADDING: u32 = 2;

/// Scrollbar thumb width in pixels.
pub const SCROLLBAR_WIDTH: u32 = 6;

/// Scrollbar hit zone width from right edge (wider than thumb for easier targeting).
pub const SCROLLBAR_HIT_ZONE: u32 = 14;

/// Margin between the thumb right edge and the window right edge.
pub const SCROLLBAR_MARGIN: u32 = 2;

/// Scrollbar thumb color — Catppuccin Mocha Overlay0 #6C7086.
pub(super) const SCROLLBAR_COLOR: Color = Color {
    r: 108,
    g: 112,
    b: 134,
};

/// Scrollbar thumb color when hovered/dragged — Catppuccin Mocha Overlay1 #7F849C.
pub(super) const SCROLLBAR_HOVER_COLOR: Color = Color {
    r: 127,
    g: 132,
    b: 156,
};

/// Tab bar height in pixels.
#[cfg(not(target_os = "macos"))]
pub const TAB_BAR_HEIGHT: u32 = 36;

/// Outer terminal padding inside the window.
#[cfg(target_os = "windows")]
pub const WINDOW_PADDING: u32 = 12;
/// Outer terminal padding inside the window.
#[cfg(not(target_os = "windows"))]
pub const WINDOW_PADDING: u32 = 8;

/// Active-tab accent (Catppuccin Mocha Lavender #B4BEFE) — used by rename selection.
pub(super) const ACTIVE_ACCENT: Color = Color {
    r: 180,
    g: 190,
    b: 254,
};

/// Pin button active color (Catppuccin Mocha Lavender #B4BEFE).
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const PIN_ACTIVE_COLOR: u32 = 0xB4BEFE;

// -- Context menu palette --

/// Context menu background (#1E2433).
pub(super) const MENU_BG: u32 = 0x1E2433;
/// Context menu hover highlight (#3A3F57).
pub(super) const MENU_HOVER_BG: u32 = 0x3A3F57;
/// Destructive action text color (Catppuccin Mocha Red #F38BA8).
pub(super) const DESTRUCTIVE_COLOR: Color = Color {
    r: 243,
    g: 139,
    b: 168,
};

/// Security indicator color (Catppuccin Mocha Yellow #F9E2AF).
pub(super) const SECURITY_ACCENT: Color = Color {
    r: 249,
    g: 226,
    b: 175,
};

/// Minimum scrollbar thumb height in base UI pixels.
pub(super) const SCROLLBAR_MIN_THUMB: u32 = 20;

// -- Tab bar palette (Catppuccin Mocha, flat Chrome-style) --

/// Mantle — bar background.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const BAR_BG: u32 = 0x181825;

/// Base — active-tab fill that merges with the terminal area.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const ACTIVE_TAB_BG: u32 = 0x1E1E2E;

/// Surface0 — inactive-tab hover highlight.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const INACTIVE_TAB_HOVER: u32 = 0x313244;

/// Text — active-tab text color.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const TAB_TEXT_ACTIVE: u32 = 0xCDD6F4;

/// Overlay0 — inactive-tab text color.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const TAB_TEXT_INACTIVE: u32 = 0x6C7086;

/// Surface0 — tab bottom separator / border.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const TAB_BORDER: u32 = 0x313244;

/// Surface2 — close-button hover background.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const CLOSE_HOVER_BG_COLOR: u32 = 0x585B70;

/// Distinct editable-field background for rename input.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const RENAME_FIELD_BG: u32 = 0x24273A;

/// Subtle field border for rename input.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const RENAME_FIELD_BORDER: u32 = 0x6C7086;

/// Rename selection highlight (Catppuccin Mocha Lavender #B4BEFE).
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const RENAME_SELECTION_BG: u32 = 0xB4BEFE;

/// Tab drag insertion indicator (Catppuccin Mocha Mauve #CBA6F7).
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const INSERTION_COLOR: u32 = 0xCBA6F7;

/// Window-button close hover background (Catppuccin Mocha Red #F38BA8).
#[cfg(not(target_os = "macos"))]
pub(super) const WIN_BTN_CLOSE_HOVER: u32 = 0xF38BA8;

/// Sanitizes a DPI scale factor to a safe, finite range.
///
/// Returns `1.0` for non-finite inputs, otherwise clamps to `[0.75, 4.0]`.
pub(super) fn sanitize_scale(scale_factor: f64) -> f64 {
    if scale_factor.is_finite() {
        scale_factor.clamp(0.75, 4.0)
    } else {
        1.0
    }
}

/// Returns `true` when the new scale differs meaningfully from the old scale.
pub(super) fn scale_changed(old: f64, new: f64) -> bool {
    (old - new).abs() >= 1e-6
}

/// Resolves the effective tab-bar visibility for the current platform.
///
/// On macOS the native tab bar is always used, so this returns `false`
/// regardless of the requested value.
pub(super) fn resolve_tab_bar_visible(visible: bool) -> bool {
    #[cfg(target_os = "macos")]
    {
        let _ = visible;
        false
    }
    #[cfg(not(target_os = "macos"))]
    {
        visible
    }
}

pub use types::*;
