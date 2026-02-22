pub mod backend;
mod cpu;
mod cursor;
#[cfg(feature = "gpu")]
pub mod gpu;
pub mod metrics;
mod scrollbar;
pub mod shared;
#[cfg(not(target_os = "macos"))]
mod tab_bar;
mod terminal;
pub mod traits;
pub mod types;

use crate::core::{CursorStyle, Grid, Selection};

pub use backend::RendererBackend;
pub use cpu::CpuRenderer;
pub use traits::Renderer;

/// Scrollbar thumb width in pixels.
#[cfg(feature = "gpu")]
pub const SCROLLBAR_WIDTH: u32 = 6;

/// Scrollbar hit zone width from right edge (wider than thumb for easier targeting).
pub const SCROLLBAR_HIT_ZONE: u32 = 14;

/// Margin between the thumb right edge and the window right edge.
pub const SCROLLBAR_MARGIN: u32 = 2;

/// Minimum scrollbar thumb height in base UI pixels.
pub(super) const SCROLLBAR_MIN_THUMB: u32 = 20;

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
pub(crate) fn blend_rgb(dst: u32, src: u32, alpha: u8) -> u32 {
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

#[cfg(not(target_os = "macos"))]
pub(super) fn resolve_tab_bar_visible(visible: bool) -> bool {
    visible
}

pub use types::*;
