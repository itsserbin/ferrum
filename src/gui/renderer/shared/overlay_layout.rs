//! Pure layout computation for overlay elements (tooltip, drag overlay).
//!
//! Both the CPU and GPU renderers need identical geometry for these overlays.
//! This module computes positions and sizes without any rendering side effects,
//! so each backend only needs to iterate the resulting layout and issue its
//! own draw calls.

use super::tab_math::{self, TabLayoutMetrics};

// ── Tooltip ─────────────────────────────────────────────────────────

/// Pre-computed geometry for a tab tooltip.
#[derive(Debug, Clone)]
pub struct TooltipLayout {
    /// Background rectangle: (x, y, width, height).
    pub bg_x: i32,
    pub bg_y: i32,
    pub bg_w: u32,
    pub bg_h: u32,
    /// Corner radius for the background/border rounded rects.
    pub radius: u32,
    /// Top-left position where the display text should be drawn.
    pub text_x: u32,
    pub text_y: u32,
    /// The (potentially truncated) text to render.
    pub display_text: String,
}

/// Computes the tooltip layout from raw parameters.
///
/// Returns `None` when the title is empty or the buffer dimensions are zero
/// or too small to fit any content.
pub fn compute_tooltip_layout(
    title: &str,
    mouse_pos: (f64, f64),
    m: &TabLayoutMetrics,
    buf_width: u32,
    buf_height: u32,
) -> Option<TooltipLayout> {
    if title.is_empty() || buf_width == 0 || buf_height == 0 {
        return None;
    }

    let padding_x = m.scaled_px(6);
    let padding_y = m.scaled_px(4);
    let border_extra = m.scaled_px(2);
    let content_chars = title.chars().count() as u32;
    let width = (content_chars * m.cell_width + padding_x * 2 + border_extra)
        .min(buf_width.saturating_sub(4));
    let height = (m.cell_height + padding_y * 2 + border_extra).min(buf_height);
    if width <= border_extra || height <= border_extra {
        return None;
    }

    let margin = m.scaled_px(2);
    let mut x = mouse_pos.0.round() as i32 + m.scaled_px(10) as i32;
    let mut y = m.tab_bar_height as i32 + m.scaled_px(6) as i32;
    x = x
        .min(buf_width as i32 - width as i32 - margin as i32)
        .max(margin as i32);
    y = y
        .min(buf_height as i32 - height as i32 - margin as i32)
        .max(margin as i32);

    let radius = m.scaled_px(6);
    let text_x = x as u32 + m.scaled_px(1) + padding_x;
    let text_y = y as u32 + m.scaled_px(1) + padding_y;
    let max_chars = ((width - border_extra - padding_x * 2) / m.cell_width) as usize;
    let display_text: String = title.chars().take(max_chars).collect();

    Some(TooltipLayout {
        bg_x: x,
        bg_y: y,
        bg_w: width,
        bg_h: height,
        radius,
        text_x,
        text_y,
        display_text,
    })
}

// ── Drag overlay ────────────────────────────────────────────────────

/// Pre-computed geometry for a tab drag overlay (ghost tab + insertion indicator).
#[derive(Debug, Clone)]
pub struct DragOverlayLayout {
    /// Shadow rectangle offset (+2, +2 from ghost).
    pub shadow_x: i32,
    pub shadow_y: i32,
    /// Ghost body / border rectangle.
    pub body_x: i32,
    pub body_y: i32,
    /// Width and height shared by shadow, body, and border.
    pub rect_w: u32,
    pub rect_h: u32,
    /// Corner radius for all three rounded rects.
    pub radius: u32,
    /// The label text to render (number or truncated title).
    pub title_text: String,
    /// Top-left position of the title text.
    pub title_x: i32,
    pub title_y: u32,
    /// Insertion indicator: x position, y start, width, height.
    pub indicator_x: u32,
    pub indicator_y: u32,
    pub indicator_w: u32,
    pub indicator_h: u32,
}

/// Computes the drag overlay layout from raw parameters.
///
/// `drag_pos` is `(current_x, indicator_x)`.
///
/// Returns `None` when `source_index` is out of range.
pub fn compute_drag_overlay_layout(
    m: &TabLayoutMetrics,
    tab_count: usize,
    source_index: usize,
    source_title: &str,
    drag_pos: (f64, f32),
    buf_width: u32,
) -> Option<DragOverlayLayout> {
    let (current_x, indicator_x) = drag_pos;
    if source_index >= tab_count {
        return None;
    }
    let tw = tab_math::calculate_tab_width(m, tab_count, buf_width);

    let ghost_x = (current_x - tw as f64 / 2.0).round() as i32;
    let ghost_y = m.scaled_px(2) as i32;
    let ghost_h = m.tab_bar_height - m.scaled_px(4);
    let ghost_radius = m.scaled_px(6);

    // Label: number when too narrow, otherwise truncated title.
    let use_numbers = tab_math::should_show_number(m, tw);
    let label: String = if use_numbers {
        (source_index + 1).to_string()
    } else {
        let max = tab_math::rename_field_max_chars(m, tw);
        source_title.chars().take(max).collect()
    };
    let lw = label.chars().count() as u32 * m.cell_width;
    let title_x = ghost_x + ((tw as i32 - lw as i32) / 2).max(4);
    let title_y = tab_math::tab_text_y(m);

    // Insertion indicator geometry.
    let ix = indicator_x.round() as u32;
    let ind_pad = m.scaled_px(4);
    let ind_w = m.scaled_px(2);
    let ind_h = m.tab_bar_height.saturating_sub(ind_pad * 2);

    Some(DragOverlayLayout {
        shadow_x: ghost_x + 2,
        shadow_y: ghost_y + 2,
        body_x: ghost_x,
        body_y: ghost_y,
        rect_w: tw,
        rect_h: ghost_h,
        radius: ghost_radius,
        title_text: label,
        title_x,
        title_y,
        indicator_x: ix,
        indicator_y: ind_pad,
        indicator_w: ind_w,
        indicator_h: ind_h,
    })
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn default_metrics() -> TabLayoutMetrics {
        TabLayoutMetrics {
            cell_width: 9,
            cell_height: 20,
            ui_scale: 1.0,
            tab_bar_height: 36,
        }
    }

    fn hidpi_metrics() -> TabLayoutMetrics {
        TabLayoutMetrics {
            cell_width: 18,
            cell_height: 40,
            ui_scale: 2.0,
            tab_bar_height: 72,
        }
    }

    // ── Tooltip layout ──────────────────────────────────────────────

    #[test]
    fn tooltip_empty_title_returns_none() {
        let m = default_metrics();
        assert!(compute_tooltip_layout("", (100.0, 10.0), &m, 800, 600).is_none());
    }

    #[test]
    fn tooltip_zero_width_returns_none() {
        let m = default_metrics();
        assert!(compute_tooltip_layout("hello", (100.0, 10.0), &m, 0, 600).is_none());
    }

    #[test]
    fn tooltip_zero_height_returns_none() {
        let m = default_metrics();
        assert!(compute_tooltip_layout("hello", (100.0, 10.0), &m, 800, 0).is_none());
    }

    #[test]
    fn tooltip_basic_layout_has_valid_dimensions() {
        let m = default_metrics();
        let layout = compute_tooltip_layout("Hello Tab", (100.0, 10.0), &m, 800, 600)
            .expect("layout should exist");
        assert!(layout.bg_w > 0);
        assert!(layout.bg_h > 0);
        assert!(layout.radius > 0);
        assert!(!layout.display_text.is_empty());
    }

    #[test]
    fn tooltip_text_position_inside_background() {
        let m = default_metrics();
        let layout = compute_tooltip_layout("Test", (50.0, 10.0), &m, 800, 600)
            .expect("layout should exist");
        assert!(layout.text_x >= layout.bg_x as u32);
        assert!(layout.text_y >= layout.bg_y as u32);
        assert!(layout.text_x < (layout.bg_x as u32 + layout.bg_w));
        assert!(layout.text_y < (layout.bg_y as u32 + layout.bg_h));
    }

    #[test]
    fn tooltip_clamps_to_right_edge() {
        let m = default_metrics();
        let layout = compute_tooltip_layout("Hello", (790.0, 10.0), &m, 800, 600)
            .expect("layout should exist");
        // Background right edge should not exceed buffer width minus margin.
        assert!((layout.bg_x + layout.bg_w as i32) <= 800);
    }

    #[test]
    fn tooltip_clamps_to_left_edge() {
        let m = default_metrics();
        let layout = compute_tooltip_layout("Hello", (-100.0, 10.0), &m, 800, 600)
            .expect("layout should exist");
        assert!(layout.bg_x >= 0);
    }

    #[test]
    fn tooltip_truncates_long_title() {
        let m = default_metrics();
        let long_title = "A".repeat(200);
        let layout = compute_tooltip_layout(&long_title, (100.0, 10.0), &m, 800, 600)
            .expect("layout should exist");
        assert!(layout.display_text.len() < 200);
    }

    #[test]
    fn tooltip_hidpi_scales_dimensions() {
        let m1 = default_metrics();
        let m2 = hidpi_metrics();
        let l1 = compute_tooltip_layout("Test", (100.0, 10.0), &m1, 800, 600).expect("layout 1x");
        let l2 = compute_tooltip_layout("Test", (100.0, 10.0), &m2, 1600, 1200).expect("layout 2x");
        // 2x layout should have roughly double the dimensions.
        assert!(l2.bg_w > l1.bg_w);
        assert!(l2.bg_h > l1.bg_h);
        assert!(l2.radius > l1.radius);
    }

    // ── Drag overlay layout ─────────────────────────────────────────

    #[test]
    fn drag_overlay_invalid_index_returns_none() {
        let m = default_metrics();
        assert!(compute_drag_overlay_layout(&m, 3, 5, "tab", (100.0, 50.0), 800).is_none());
    }

    #[test]
    fn drag_overlay_basic_layout_has_valid_geometry() {
        let m = default_metrics();
        let layout = compute_drag_overlay_layout(&m, 3, 1, "My Tab", (200.0, 50.0), 800)
            .expect("layout should exist");
        assert!(layout.rect_w > 0);
        assert!(layout.rect_h > 0);
        assert!(layout.radius > 0);
        assert!(!layout.title_text.is_empty());
    }

    #[test]
    fn drag_overlay_shadow_offset_from_body() {
        let m = default_metrics();
        let layout = compute_drag_overlay_layout(&m, 3, 0, "Tab", (200.0, 50.0), 800)
            .expect("layout should exist");
        assert_eq!(layout.shadow_x, layout.body_x + 2);
        assert_eq!(layout.shadow_y, layout.body_y + 2);
    }

    #[test]
    fn drag_overlay_title_uses_number_for_narrow_tabs() {
        let m = default_metrics();
        // With many tabs the width compresses below MIN_TAB_WIDTH_FOR_TITLE.
        let layout = compute_drag_overlay_layout(&m, 50, 2, "Long Title", (200.0, 50.0), 800)
            .expect("layout should exist");
        // When numbers are used, label is the 1-based index.
        assert_eq!(layout.title_text, "3");
    }

    #[test]
    fn drag_overlay_indicator_dimensions() {
        let m = default_metrics();
        let layout = compute_drag_overlay_layout(&m, 3, 0, "Tab", (200.0, 100.0), 800)
            .expect("layout should exist");
        assert_eq!(layout.indicator_x, 100);
        assert_eq!(layout.indicator_w, m.scaled_px(2));
        assert!(layout.indicator_h > 0);
    }

    #[test]
    fn drag_overlay_hidpi_scales_radius() {
        let m1 = default_metrics();
        let m2 = hidpi_metrics();
        let l1 =
            compute_drag_overlay_layout(&m1, 3, 0, "Tab", (200.0, 50.0), 800).expect("layout 1x");
        let l2 =
            compute_drag_overlay_layout(&m2, 3, 0, "Tab", (200.0, 50.0), 1600).expect("layout 2x");
        assert_eq!(l2.radius, l1.radius * 2);
    }
}
