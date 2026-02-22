/// Render-time tab metadata.
#[cfg(not(target_os = "macos"))]
pub struct TabInfo<'a> {
    pub title: &'a str,
    #[cfg(not(target_os = "macos"))]
    pub index: usize,
    #[cfg(not(target_os = "macos"))]
    pub is_active: bool,
    #[cfg(not(target_os = "macos"))]
    pub hover_progress: f32,
    #[cfg(not(target_os = "macos"))]
    pub close_hover_progress: f32,
    #[cfg(not(target_os = "macos"))]
    pub is_renaming: bool,
    #[cfg(not(target_os = "macos"))]
    pub rename_text: Option<&'a str>,
    #[cfg(not(target_os = "macos"))]
    pub rename_cursor: usize,
    #[cfg(not(target_os = "macos"))]
    pub rename_selection: Option<(usize, usize)>, // Byte range within rename_text.
}

// ── Layout structs ──────────────────────────────────────────────────

/// Scales a base pixel value by the given UI scale factor.
///
/// This is the single source of truth for DPI-aware pixel scaling.
/// All other `scaled_px` helpers (on `FontMetrics`, `TabLayoutMetrics`, etc.)
/// delegate to this function.
pub(in crate::gui) fn scaled_px(base: u32, ui_scale: f64) -> u32 {
    if base == 0 {
        0
    } else {
        ((base as f64 * ui_scale).round() as u32).max(1)
    }
}

/// A rounded rectangle in the context-menu overlay.
pub struct RoundedRectCmd {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
    pub color: u32,
    pub opacity: f32,
}



/// Result of tab-bar hit testing.
#[derive(Debug)]
pub enum TabBarHit {
    /// Clicked on a tab by index.
    Tab(usize),
    /// Clicked on a tab close button by index.
    #[cfg(not(target_os = "macos"))]
    CloseTab(usize),
    /// Clicked on the new-tab button.
    #[cfg(not(target_os = "macos"))]
    NewTab,
    /// Clicked on the pin button (non-macOS).
    #[cfg(not(target_os = "macos"))]
    PinButton,
    /// Clicked on the settings gear button (non-macOS).
    #[cfg(not(target_os = "macos"))]
    SettingsButton,
    /// Clicked on a window control button (non-macOS).
    #[cfg(not(target_os = "macos"))]
    WindowButton(WindowButton),
    /// Clicked empty bar area (window drag).
    Empty,
}

/// Window control button type (non-macOS).
#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowButton {
    Minimize,
    Maximize,
    Close,
}

pub(super) struct GlyphBitmap {
    pub(super) data: Vec<u8>,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) left: i32,
    pub(super) top: i32,
}

/// Pixel buffer surface passed to rendering methods.
///
/// Groups the `(buffer, width, height)` triple that appears in every
/// renderer method, eliminating repeated parameters.
pub struct RenderTarget<'a> {
    pub buffer: &'a mut [u32],
    pub width: usize,
    pub height: usize,
}

#[cfg(not(target_os = "macos"))]
/// Per-tab layout slot used during tab bar rendering.
///
/// Groups the index, metadata reference, position, width, and hover state
/// that are always passed together to tab drawing methods.
pub struct TabSlot<'a> {
    pub index: usize,
    pub tab: &'a TabInfo<'a>,
    #[cfg(not(target_os = "macos"))]
    pub x: u32,
    #[cfg(not(target_os = "macos"))]
    pub width: u32,
    pub is_hovered: bool,
}

/// Pin-button color triple for the three visual states.
#[cfg(not(target_os = "macos"))]
pub struct PinColors {
    pub active: u32,
    pub hover: u32,
    pub inactive: u32,
}

/// Internal rounded-rectangle parameters used by `draw_rounded_impl`.
///
/// Groups position, size, radius, color, and alpha into a single struct
/// so that the shared pixel-iteration code stays under the clippy argument limit.
#[cfg(not(target_os = "macos"))]
pub(in crate::gui::renderer) struct RoundedShape {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub radius: u32,
    pub color: u32,
    pub alpha: u8,
}

/// Scrollbar rendering parameters.
///
/// Groups scroll state, opacity, and hover flag that are always passed
/// together to scrollbar drawing methods.
pub struct ScrollbarState {
    pub scroll_offset: usize,
    pub scrollback_len: usize,
    pub grid_rows: usize,
    pub opacity: f32,
    pub hover: bool,
}
#[cfg(not(target_os = "macos"))]
/// Bundled parameters for tab bar drawing.
///
/// Groups the arguments that `draw_tab_bar` / `draw_tab_bar_impl` need
/// beyond `&mut self` and `target`, keeping both renderers under the
/// clippy argument limit.
pub struct TabBarDrawParams<'a> {
    pub tabs: &'a [TabInfo<'a>],
    pub hovered_tab: Option<usize>,
    pub mouse_pos: (f64, f64),
    pub tab_offsets: Option<&'a [f32]>,
    pub pinned: bool,
    pub settings_open: bool,
}

/// Tab drag position data passed to overlay layout computation.
///
/// Replaces the raw `(f64, f32)` tuple, giving names to the cursor
/// position and the smoothed insertion indicator position.
#[cfg(not(target_os = "macos"))]
pub struct DragPosition {
    pub current_x: f64,
    pub indicator_x: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── scaled_px helper tests ──────────────────────────────────────

    #[test]
    fn scaled_px_identity_at_1x() {
        assert_eq!(scaled_px(6, 1.0), 6);
    }

    #[test]
    fn scaled_px_doubles_at_2x() {
        assert_eq!(scaled_px(6, 2.0), 12);
    }

    #[test]
    fn scaled_px_zero_returns_zero() {
        assert_eq!(scaled_px(0, 2.0), 0);
    }

    #[test]
    fn scaled_px_never_returns_zero_for_nonzero_input() {
        assert!(scaled_px(1, 0.01) >= 1);
    }
}
