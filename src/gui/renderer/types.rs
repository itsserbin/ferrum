use crate::core::Color;

/// Render-time tab metadata.
pub struct TabInfo<'a> {
    pub title: &'a str,
    pub is_active: bool,
    pub security_count: usize,
    pub hover_progress: f32,
    pub close_hover_progress: f32,
    pub is_renaming: bool,
    pub rename_text: Option<&'a str>,
    pub rename_cursor: usize,
    pub rename_selection: Option<(usize, usize)>, // Byte range within rename_text.
}

pub struct SecurityPopup {
    pub tab_index: usize,
    pub x: u32,
    pub y: u32,
    pub title: &'static str,
    pub lines: Vec<String>,
}

impl SecurityPopup {
    /// Computes the full visual layout for this security popup.
    ///
    /// The returned [`SecurityPopupLayout`] contains every rect and text span
    /// needed to draw the popup -- renderers just iterate and issue their
    /// backend-specific draw calls.
    pub fn layout(
        &self,
        cell_width: u32,
        cell_height: u32,
        ui_scale: f64,
        buf_width: u32,
        buf_height: u32,
    ) -> SecurityPopupLayout {
        let (rx, ry, rw, rh) = self.clamped_rect(cell_width, cell_height, buf_width, buf_height);
        let x = rx as f32;
        let y = ry as f32;
        let w = rw as f32;
        let h = rh as f32;
        let radius = scaled_px(6, ui_scale) as f32;
        let line_h = self.line_height(cell_height) as f32;
        let accent_pixel = super::SECURITY_ACCENT.to_pixel();

        let bg = RoundedRectCmd {
            x,
            y,
            w,
            h,
            radius,
            color: super::MENU_BG,
            opacity: 0.973,
        };
        let border = RoundedRectCmd {
            x,
            y,
            w,
            h,
            radius,
            color: 0xFFFFFF,
            opacity: 0.078,
        };

        let pad2 = scaled_px(2, ui_scale) as f32;
        let pad3 = scaled_px(3, ui_scale) as f32;
        let pad4 = scaled_px(4, ui_scale) as f32;
        let half_cw = cell_width as f32 / 2.0;

        let title = TextCmd {
            x: x + half_cw,
            y: y + pad2,
            text: self.title.to_string(),
            color: accent_pixel,
            opacity: 1.0,
        };

        let sep_y = y + line_h;
        let separator = FlatRectCmd {
            x: x + pad3,
            y: sep_y,
            w: w - pad3 * 2.0,
            h: 1.0,
            color: accent_pixel,
            opacity: 0.47,
        };

        let lines = self
            .lines
            .iter()
            .enumerate()
            .map(|(i, line)| TextCmd {
                x: x + half_cw,
                y: y + line_h + pad4 + i as f32 * line_h,
                text: format!("\u{2022} {}", line),
                color: Color::DEFAULT_FG.to_pixel(),
                opacity: 1.0,
            })
            .collect();

        SecurityPopupLayout {
            bg,
            border,
            title,
            separator,
            lines,
        }
    }
}

// ── Layout structs ──────────────────────────────────────────────────

/// Scales a base pixel value by the given UI scale factor.
///
/// This is the single source of truth for DPI-aware pixel scaling.
/// All other `scaled_px` helpers (on `FontMetrics`, `TabLayoutMetrics`, etc.)
/// delegate to this function.
pub(in crate::gui::renderer) fn scaled_px(base: u32, ui_scale: f64) -> u32 {
    if base == 0 {
        0
    } else {
        ((base as f64 * ui_scale).round() as u32).max(1)
    }
}

/// A rounded rectangle in the context-menu / security-popup overlay.
pub struct RoundedRectCmd {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
    pub color: u32,
    pub opacity: f32,
}

/// A flat (non-rounded) rectangle command.
pub struct FlatRectCmd {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: u32,
    pub opacity: f32,
}

/// A single text span to draw.
pub struct TextCmd {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: u32,
    /// Text opacity (used by the GPU renderer; the CPU renderer draws at full opacity).
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    pub opacity: f32,
}

/// Pre-computed layout for the security popup overlay.
pub struct SecurityPopupLayout {
    /// Background panel (fill).
    pub bg: RoundedRectCmd,
    /// Border overlay drawn on top of the background.
    pub border: RoundedRectCmd,
    /// Title text.
    pub title: TextCmd,
    /// Horizontal separator line below the title.
    pub separator: FlatRectCmd,
    /// Content lines (each prefixed with a bullet).
    pub lines: Vec<TextCmd>,
}

/// Result of tab-bar hit testing.
#[derive(Debug)]
pub enum TabBarHit {
    /// Clicked on a tab by index.
    Tab(usize),
    /// Clicked on a tab close button by index.
    CloseTab(usize),
    /// Clicked on the new-tab button.
    NewTab,
    /// Clicked on the pin button (non-macOS).
    #[cfg(not(target_os = "macos"))]
    PinButton,
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

/// Per-tab layout slot used during tab bar rendering.
///
/// Groups the index, metadata reference, position, width, and hover state
/// that are always passed together to tab drawing methods.
pub struct TabSlot<'a> {
    pub index: usize,
    pub tab: &'a TabInfo<'a>,
    pub x: u32,
    pub width: u32,
    pub is_hovered: bool,
}

/// Pin-button color triple for the three visual states.
pub struct PinColors {
    pub active: u32,
    pub hover: u32,
    pub inactive: u32,
}

/// Internal rounded-rectangle parameters used by `draw_rounded_impl`.
///
/// Groups position, size, radius, color, and alpha into a single struct
/// so that the shared pixel-iteration code stays under the clippy argument limit.
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

/// Tab drag position data passed to overlay layout computation.
///
/// Replaces the raw `(f64, f32)` tuple, giving names to the cursor
/// position and the smoothed insertion indicator position.
pub struct DragPosition {
    pub current_x: f64,
    pub indicator_x: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SecurityPopup layout tests ──────────────────────────────────

    fn make_popup() -> SecurityPopup {
        SecurityPopup {
            tab_index: 0,
            x: 50,
            y: 40,
            title: "Security Warning",
            lines: vec!["Line one".to_string(), "Line two".to_string()],
        }
    }

    #[test]
    fn security_popup_layout_bg_position_clamped() {
        let popup = make_popup();
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        assert!(layout.bg.x >= 0.0);
        assert!(layout.bg.y >= 0.0);
        assert!(layout.bg.w > 0.0);
        assert!(layout.bg.h > 0.0);
    }

    #[test]
    fn security_popup_layout_border_matches_bg() {
        let popup = make_popup();
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        assert_eq!(layout.border.x, layout.bg.x);
        assert_eq!(layout.border.y, layout.bg.y);
        assert_eq!(layout.border.w, layout.bg.w);
        assert_eq!(layout.border.h, layout.bg.h);
    }

    #[test]
    fn security_popup_layout_title_uses_accent_color() {
        let popup = make_popup();
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        assert_eq!(layout.title.color, super::super::SECURITY_ACCENT.to_pixel());
        assert_eq!(layout.title.text, "Security Warning");
    }

    #[test]
    fn security_popup_layout_lines_match_content() {
        let popup = make_popup();
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        assert_eq!(layout.lines.len(), 2);
        assert!(layout.lines[0].text.starts_with('\u{2022}'));
        assert!(layout.lines[0].text.contains("Line one"));
        assert!(layout.lines[1].text.contains("Line two"));
    }

    #[test]
    fn security_popup_layout_lines_use_default_fg() {
        let popup = make_popup();
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        let default_fg = Color::DEFAULT_FG.to_pixel();
        for line in &layout.lines {
            assert_eq!(line.color, default_fg);
        }
    }

    #[test]
    fn security_popup_layout_separator_between_title_and_lines() {
        let popup = make_popup();
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        // Separator y should be below title y and above first line y.
        assert!(layout.separator.y > layout.title.y);
        if !layout.lines.is_empty() {
            assert!(layout.separator.y < layout.lines[0].y);
        }
    }

    #[test]
    fn security_popup_layout_hidpi_scales_radius() {
        let popup = make_popup();
        let layout_1x = popup.layout(8, 16, 1.0, 800, 600);
        let layout_2x = popup.layout(16, 32, 2.0, 800, 600);
        assert!(layout_2x.bg.radius > layout_1x.bg.radius);
    }

    #[test]
    fn security_popup_layout_clamped_to_buffer_bounds() {
        // Place popup near the bottom-right corner.
        let popup = SecurityPopup {
            tab_index: 0,
            x: 790,
            y: 590,
            title: "Test Title",
            lines: vec!["A line".to_string()],
        };
        let layout = popup.layout(8, 16, 1.0, 800, 600);
        // The popup should be fully within bounds.
        assert!((layout.bg.x + layout.bg.w) <= 800.0);
        assert!((layout.bg.y + layout.bg.h) <= 600.0);
    }

    // ── SecurityPopup::hit_test tests ─────────────────────────────

    #[test]
    fn security_popup_hit_test_inside_returns_true() {
        let popup = make_popup();
        // Point inside the popup's clamped rectangle.
        let (px, py, pw, ph) = popup.clamped_rect(8, 16, 800, 600);
        let cx = px as f64 + pw as f64 / 2.0;
        let cy = py as f64 + ph as f64 / 2.0;
        assert!(popup.hit_test(cx, cy, 8, 16, 800, 600));
    }

    #[test]
    fn security_popup_hit_test_outside_returns_false() {
        let popup = make_popup();
        // Point far outside the popup.
        assert!(!popup.hit_test(0.0, 0.0, 8, 16, 800, 600));
        assert!(!popup.hit_test(799.0, 599.0, 8, 16, 800, 600));
    }

    #[test]
    fn security_popup_hit_test_top_left_edge_returns_true() {
        let popup = make_popup();
        let (px, py, _, _) = popup.clamped_rect(8, 16, 800, 600);
        assert!(popup.hit_test(px as f64, py as f64, 8, 16, 800, 600));
    }

    #[test]
    fn security_popup_hit_test_bottom_right_edge_exclusive() {
        let popup = make_popup();
        let (px, py, pw, ph) = popup.clamped_rect(8, 16, 800, 600);
        // Exclusive end (just past the rect).
        assert!(!popup.hit_test((px + pw) as f64, (py + ph) as f64, 8, 16, 800, 600));
    }

    #[test]
    fn security_popup_hit_test_clamped_near_edge() {
        // Popup placed near bottom-right corner is clamped.
        let popup = SecurityPopup {
            tab_index: 0,
            x: 790,
            y: 590,
            title: "Test Title",
            lines: vec!["A line".to_string()],
        };
        let (px, py, pw, ph) = popup.clamped_rect(8, 16, 800, 600);
        let cx = px as f64 + pw as f64 / 2.0;
        let cy = py as f64 + ph as f64 / 2.0;
        assert!(popup.hit_test(cx, cy, 8, 16, 800, 600));
    }

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
