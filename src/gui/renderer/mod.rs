pub mod backend;
mod cpu;
mod cursor;
#[cfg(feature = "gpu")]
pub mod gpu;
pub mod metrics;
mod scrollbar;
mod security;
mod settings;
pub mod shared;
mod tab_bar;
mod terminal;
pub mod traits;
pub mod types;

use crate::core::{Color, CursorStyle, Grid, Selection};

pub use backend::RendererBackend;
pub use cpu::CpuRenderer;
pub use traits::Renderer;

/// Scrollbar thumb width in pixels.
#[cfg_attr(not(feature = "gpu"), allow(dead_code))]
pub const SCROLLBAR_WIDTH: u32 = 6;

/// Scrollbar hit zone width from right edge (wider than thumb for easier targeting).
pub const SCROLLBAR_HIT_ZONE: u32 = 14;

/// Margin between the thumb right edge and the window right edge.
pub const SCROLLBAR_MARGIN: u32 = 2;

/// Tab bar height in pixels.
#[cfg(not(target_os = "macos"))]
pub const TAB_BAR_HEIGHT: u32 = 36;

/// Minimum scrollbar thumb height in base UI pixels.
pub(super) const SCROLLBAR_MIN_THUMB: u32 = 20;

// -- Tab bar palette (derived from terminal background #282C34) --
//
// All colors maintain the neutral gray tone of the terminal palette,
// avoiding the blue/purple tint of Catppuccin Mocha.

/// Bar background — darkened terminal bg, neutral gray.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const BAR_BG: u32 = 0x1E2127;

/// Active-tab fill — matches terminal background exactly.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const ACTIVE_TAB_BG: u32 = 0x282C34;

/// Inactive-tab hover highlight — between bar bg and active tab.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const INACTIVE_TAB_HOVER: u32 = 0x2E333C;

/// Active-tab text — matches terminal default fg.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const TAB_TEXT_ACTIVE: u32 = 0xD2DBEB;

/// Inactive-tab text — muted neutral gray.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const TAB_TEXT_INACTIVE: u32 = 0x6C7480;

/// Tab bottom separator / border.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const TAB_BORDER: u32 = 0x2E333C;

/// Close-button hover background — terminal ANSI black.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const CLOSE_HOVER_BG_COLOR: u32 = 0x454B59;

/// Editable-field background for rename input.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const RENAME_FIELD_BG: u32 = 0x1E2127;

/// Field border for rename input.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) const RENAME_FIELD_BORDER: u32 = 0x6C7480;

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

/// Blends `src` over `dst` with `alpha` in 0..=255 (both colors are 0xRRGGBB).
pub(super) fn blend_rgb(dst: u32, src: u32, alpha: u8) -> u32 {
    if alpha == 255 {
        return src;
    }
    if alpha == 0 {
        return dst;
    }

    let a = alpha as u32;
    let inv = 255 - a;

    let dr = (dst >> 16) & 0xFF;
    let dg = (dst >> 8) & 0xFF;
    let db = dst & 0xFF;

    let sr = (src >> 16) & 0xFF;
    let sg = (src >> 8) & 0xFF;
    let sb = src & 0xFF;

    let r = (sr * a + dr * inv + 127) / 255;
    let g = (sg * a + dg * inv + 127) / 255;
    let b = (sb * a + db * inv + 127) / 255;

    (r << 16) | (g << 8) | b
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
