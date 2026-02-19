//! Pure layout math for the tab bar.
//!
//! Every function in this module is a pure calculation: given dimensions,
//! counts, and font metrics it returns coordinates and sizes.  No rendering
//! code, no side effects.  Both CPU and GPU renderers can call these
//! functions to avoid duplicating layout logic.

// ── Constants ────────────────────────────────────────────────────────

/// Maximum tab width in logical pixels (before HiDPI scaling).
pub const MAX_TAB_WIDTH: u32 = 240;

/// Minimum tab width (number + close button).
pub const MIN_TAB_WIDTH: u32 = 36;

/// Minimum tab width before switching to number-only display.
pub const MIN_TAB_WIDTH_FOR_TITLE: u32 = 60;

/// Tab strip start offset for macOS (accounts for traffic light buttons).
#[cfg(target_os = "macos")]
pub const TAB_STRIP_START_X: u32 = 78;

/// Tab strip start offset for Windows.
#[cfg(target_os = "windows")]
pub const TAB_STRIP_START_X: u32 = 14;

/// Tab strip start offset for Linux and other platforms.
#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
pub const TAB_STRIP_START_X: u32 = 8;

/// Plus button extra margin for reservation calculation.
pub const PLUS_BUTTON_MARGIN: u32 = 20;

/// Close button size in logical pixels.
pub const CLOSE_BUTTON_SIZE: u32 = 20;

/// Close button margin from tab edge.
pub const CLOSE_BUTTON_MARGIN: u32 = 6;

/// Plus button size in logical pixels.
pub const PLUS_BUTTON_SIZE: u32 = 24;

/// Plus button gap from last tab.
pub const PLUS_BUTTON_GAP: u32 = 4;

/// Tab padding horizontal (left/right spacing for text).
pub const TAB_PADDING_H: u32 = 14;

/// Pin button size in logical pixels (non-macOS).
#[cfg(not(target_os = "macos"))]
pub const PIN_BUTTON_SIZE: u32 = 24;

/// Gap between pin button and first tab (non-macOS).
#[cfg(not(target_os = "macos"))]
pub const PIN_BUTTON_GAP: u32 = 8;

/// Window button width (non-macOS).
#[cfg(not(target_os = "macos"))]
pub const WIN_BTN_WIDTH: u32 = 46;

// ── Helper types ─────────────────────────────────────────────────────

/// A rectangle defined by origin + size, all in physical (scaled) pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    /// Converts to the `(x, y, w, h)` tuple used by existing renderer code.
    pub fn to_tuple(self) -> (u32, u32, u32, u32) {
        (self.x, self.y, self.w, self.h)
    }

    /// Returns `true` when `(px, py)` falls inside this rectangle.
    #[allow(dead_code)]
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x as f64
            && px < (self.x + self.w) as f64
            && py >= self.y as f64
            && py < (self.y + self.h) as f64
    }
}

/// Minimal set of font/display metrics needed by the layout functions.
///
/// Both `CpuRenderer` and `GpuRenderer` (via `FontMetrics`) can cheaply
/// construct this from their existing fields.
#[derive(Debug, Clone, Copy)]
pub struct TabLayoutMetrics {
    /// Scaled cell width (monospace character advance) in physical pixels.
    pub cell_width: u32,
    /// Scaled cell height in physical pixels.
    pub cell_height: u32,
    /// UI scale factor (e.g. 1.0, 2.0 for Retina).
    pub ui_scale: f64,
    /// Tab bar height in physical pixels (0 on macOS or when hidden).
    pub tab_bar_height: u32,
}

impl TabLayoutMetrics {
    /// Scales a base logical pixel value by the UI scale factor.
    /// Mirrors `CpuRenderer::scaled_px` / `FontMetrics::scaled_px`.
    pub fn scaled_px(&self, base: u32) -> u32 {
        if base == 0 {
            0
        } else {
            ((base as f64 * self.ui_scale).round() as u32).max(1)
        }
    }
}

// ── Layout functions ─────────────────────────────────────────────────

/// Returns the x-offset where the tab strip begins, in physical pixels.
///
/// On macOS this accounts for the traffic-light buttons.  On Windows/Linux
/// it includes the window padding, pin button, and pin-button gap.
pub fn tab_strip_start_x(m: &TabLayoutMetrics) -> u32 {
    #[cfg(target_os = "macos")]
    {
        m.scaled_px(TAB_STRIP_START_X)
    }
    #[cfg(not(target_os = "macos"))]
    {
        m.scaled_px(TAB_STRIP_START_X)
            + m.scaled_px(PIN_BUTTON_SIZE)
            + m.scaled_px(PIN_BUTTON_GAP)
    }
}

/// Width reserved for the plus (+) button area.
pub fn plus_button_reserved_width(m: &TabLayoutMetrics) -> u32 {
    m.cell_width + m.scaled_px(PLUS_BUTTON_MARGIN)
}

/// Total width reserved for window control buttons (non-macOS: 3 buttons).
pub fn window_buttons_reserved_width(m: &TabLayoutMetrics) -> u32 {
    #[cfg(not(target_os = "macos"))]
    {
        m.scaled_px(WIN_BTN_WIDTH) * 3
    }
    #[cfg(target_os = "macos")]
    {
        let _ = m;
        0
    }
}

/// Computes the adaptive tab width with overflow compression.
///
/// Tabs shrink from `MAX_TAB_WIDTH` down to `MIN_TAB_WIDTH` when many
/// tabs are open.  Returns the width in physical pixels.
pub fn calculate_tab_width(m: &TabLayoutMetrics, tab_count: usize, buf_width: u32) -> u32 {
    let reserved = tab_strip_start_x(m)
        + plus_button_reserved_width(m)
        + m.scaled_px(PLUS_BUTTON_GAP * 2)
        + window_buttons_reserved_width(m);
    let available = buf_width.saturating_sub(reserved);
    let min_tw = m.scaled_px(MIN_TAB_WIDTH);
    let max_tw = m.scaled_px(MAX_TAB_WIDTH);
    (available / tab_count.max(1) as u32).clamp(min_tw, max_tw)
}

/// Returns the x-origin of the tab at `tab_index`.
pub fn tab_origin_x(m: &TabLayoutMetrics, tab_index: usize, tab_width: u32) -> u32 {
    tab_strip_start_x(m) + tab_index as u32 * tab_width
}

/// Returns the rectangle for a per-tab close button.
pub fn close_button_rect(m: &TabLayoutMetrics, tab_index: usize, tab_width: u32) -> Rect {
    let btn_size = m.scaled_px(CLOSE_BUTTON_SIZE);
    let x = tab_origin_x(m, tab_index, tab_width) + tab_width - btn_size - m.scaled_px(CLOSE_BUTTON_MARGIN);
    let y = (m.tab_bar_height.saturating_sub(btn_size)) / 2;
    Rect { x, y, w: btn_size, h: btn_size }
}

/// Returns the rectangle for the new-tab (+) button.
pub fn plus_button_rect(m: &TabLayoutMetrics, tab_count: usize, tab_width: u32) -> Rect {
    let btn_size = m.scaled_px(PLUS_BUTTON_SIZE);
    let x = tab_strip_start_x(m) + tab_count as u32 * tab_width + m.scaled_px(PLUS_BUTTON_GAP);
    let y = (m.tab_bar_height.saturating_sub(btn_size)) / 2;
    Rect { x, y, w: btn_size, h: btn_size }
}

/// Returns the rectangle for the pin button (non-macOS only).
#[cfg(not(target_os = "macos"))]
pub fn pin_button_rect(m: &TabLayoutMetrics) -> Rect {
    let btn_size = m.scaled_px(PIN_BUTTON_SIZE);
    let x = m.scaled_px(TAB_STRIP_START_X);
    let y = (m.tab_bar_height.saturating_sub(btn_size)) / 2;
    Rect { x, y, w: btn_size, h: btn_size }
}

/// Returns the width (in physical pixels) reserved by the security badge
/// for a tab with the given `security_count`.
///
/// Returns 0 when `security_count` is 0.
pub fn security_badge_reserved_width(m: &TabLayoutMetrics, security_count: usize) -> u32 {
    if security_count == 0 {
        return 0;
    }
    let count_chars = security_count.min(99).to_string().len() as u32;
    let count_width = if security_count > 1 {
        count_chars * m.cell_width + m.scaled_px(2)
    } else {
        0
    };
    let badge_min = m.scaled_px(10);
    let badge_max = m.scaled_px(15);
    let badge_icon_size = m.cell_height
        .saturating_sub(m.scaled_px(10))
        .clamp(badge_min, badge_max);
    badge_icon_size + count_width + m.scaled_px(6)
}

/// Returns the icon size of the security badge shield.
pub fn security_badge_icon_size(m: &TabLayoutMetrics) -> u32 {
    let badge_min = m.scaled_px(10);
    let badge_max = m.scaled_px(15);
    m.cell_height
        .saturating_sub(m.scaled_px(10))
        .clamp(badge_min, badge_max)
}

/// Returns the bounding rectangle of the security badge within a tab.
///
/// Returns `None` when `security_count` is 0 or `tab_index >= tab_count`.
pub fn security_badge_rect(
    m: &TabLayoutMetrics,
    tab_index: usize,
    tab_count: usize,
    buf_width: u32,
    security_count: usize,
) -> Option<Rect> {
    if security_count == 0 || tab_index >= tab_count {
        return None;
    }
    let tw = calculate_tab_width(m, tab_count, buf_width);
    let tab_x = tab_origin_x(m, tab_index, tw);
    let badge_size = security_badge_icon_size(m);
    let count_chars = if security_count > 1 {
        security_count.min(99).to_string().len() as u32
    } else {
        0
    };
    let count_width = if count_chars > 0 {
        count_chars * m.cell_width + m.scaled_px(2)
    } else {
        0
    };
    let indicator_width = badge_size + count_width;
    let right_gutter = m.cell_width + m.scaled_px(10);
    let indicator_right = tab_x + tw.saturating_sub(right_gutter);
    let x = indicator_right.saturating_sub(indicator_width + m.scaled_px(2));
    let y = (m.tab_bar_height.saturating_sub(badge_size)) / 2;
    Some(Rect { x, y, w: badge_size, h: badge_size })
}

/// Returns the width reserved by the close button when it is visible.
pub fn close_button_reserved_width(m: &TabLayoutMetrics) -> u32 {
    m.scaled_px(CLOSE_BUTTON_SIZE) + m.scaled_px(CLOSE_BUTTON_MARGIN)
}

/// Returns the maximum number of characters that fit in the tab title area.
///
/// Accounts for horizontal padding, optional close button, and optional
/// security badge.
pub fn tab_title_max_chars(
    m: &TabLayoutMetrics,
    tab_width: u32,
    show_close: bool,
    security_count: usize,
) -> usize {
    let tab_padding_h = m.scaled_px(TAB_PADDING_H);
    let close_reserved = if show_close {
        close_button_reserved_width(m)
    } else {
        0
    };
    let security_reserved = security_badge_reserved_width(m, security_count);
    (tab_width.saturating_sub(tab_padding_h * 2 + close_reserved + security_reserved)
        / m.cell_width) as usize
}

/// Returns `true` when the tab width is too narrow to display a title,
/// meaning the renderer should show a tab number instead.
pub fn should_show_number(m: &TabLayoutMetrics, tab_width: u32) -> bool {
    tab_width < m.scaled_px(MIN_TAB_WIDTH_FOR_TITLE)
}

/// Returns the rectangle for the inline rename text field within a tab.
pub fn rename_field_rect(m: &TabLayoutMetrics, tab_x: u32, tab_width: u32) -> Rect {
    let tab_padding_h = m.scaled_px(TAB_PADDING_H);
    let field_pad_x = m.scaled_px(3);
    let text_y = (m.tab_bar_height.saturating_sub(m.cell_height)) / 2 + m.scaled_px(1);

    let x = tab_x + tab_padding_h.saturating_sub(field_pad_x);
    let y = text_y.saturating_sub(m.scaled_px(2));
    let w = tab_width.saturating_sub(tab_padding_h * 2) + field_pad_x * 2;
    let h = m.cell_height + m.scaled_px(4);
    Rect { x, y, w, h }
}

/// Maximum number of characters that fit in the rename field.
pub fn rename_field_max_chars(m: &TabLayoutMetrics, tab_width: u32) -> usize {
    let tab_padding_h = m.scaled_px(TAB_PADDING_H);
    (tab_width.saturating_sub(tab_padding_h * 2) / m.cell_width) as usize
}

/// Returns the y-coordinate for tab text (vertically centered in the bar).
pub fn tab_text_y(m: &TabLayoutMetrics) -> u32 {
    (m.tab_bar_height.saturating_sub(m.cell_height)) / 2 + m.scaled_px(1)
}

/// Determines the insertion index when dragging a tab to position `x`.
pub fn tab_insert_index_from_x(
    m: &TabLayoutMetrics,
    x: f64,
    tab_count: usize,
    buf_width: u32,
) -> usize {
    let tw = calculate_tab_width(m, tab_count, buf_width);
    let start = tab_strip_start_x(m) as f64;
    let mut idx = tab_count;
    for i in 0..tab_count {
        let center = start + i as f64 * tw as f64 + tw as f64 / 2.0;
        if x < center {
            idx = i;
            break;
        }
    }
    idx
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: creates metrics with scale=1.0, cell_width=9, cell_height=20,
    /// tab_bar_height=36 (typical values for JetBrains Mono at 15px, 1x scale).
    fn default_metrics() -> TabLayoutMetrics {
        TabLayoutMetrics {
            cell_width: 9,
            cell_height: 20,
            ui_scale: 1.0,
            tab_bar_height: 36,
        }
    }

    /// Helper: creates 2x HiDPI metrics.
    fn hidpi_metrics() -> TabLayoutMetrics {
        TabLayoutMetrics {
            cell_width: 18,
            cell_height: 40,
            ui_scale: 2.0,
            tab_bar_height: 72,
        }
    }

    // ── scaled_px ────────────────────────────────────────────────────

    #[test]
    fn scaled_px_identity_at_1x() {
        let m = default_metrics();
        assert_eq!(m.scaled_px(10), 10);
        assert_eq!(m.scaled_px(1), 1);
    }

    #[test]
    fn scaled_px_doubles_at_2x() {
        let m = hidpi_metrics();
        assert_eq!(m.scaled_px(10), 20);
        assert_eq!(m.scaled_px(1), 2);
    }

    #[test]
    fn scaled_px_zero_returns_zero() {
        let m = default_metrics();
        assert_eq!(m.scaled_px(0), 0);
    }

    // ── calculate_tab_width ──────────────────────────────────────────

    #[test]
    fn single_tab_gets_max_width() {
        let m = default_metrics();
        let tw = calculate_tab_width(&m, 1, 1200);
        assert_eq!(tw, m.scaled_px(MAX_TAB_WIDTH));
    }

    #[test]
    fn many_tabs_clamp_to_min_width() {
        let m = default_metrics();
        let tw = calculate_tab_width(&m, 100, 800);
        assert_eq!(tw, m.scaled_px(MIN_TAB_WIDTH));
    }

    #[test]
    fn zero_tab_count_does_not_panic() {
        let m = default_metrics();
        // tab_count.max(1) prevents division by zero
        let tw = calculate_tab_width(&m, 0, 800);
        assert!(tw >= m.scaled_px(MIN_TAB_WIDTH));
    }

    #[test]
    fn tab_width_shrinks_with_more_tabs() {
        let m = default_metrics();
        let tw_2 = calculate_tab_width(&m, 2, 1200);
        let tw_10 = calculate_tab_width(&m, 10, 1200);
        assert!(tw_2 >= tw_10);
    }

    // ── tab_origin_x ────────────────────────────────────────────────

    #[test]
    fn tab_origin_x_first_tab_at_strip_start() {
        let m = default_metrics();
        assert_eq!(tab_origin_x(&m, 0, 200), tab_strip_start_x(&m));
    }

    #[test]
    fn tab_origin_x_second_tab_offset() {
        let m = default_metrics();
        let tw = 200;
        assert_eq!(tab_origin_x(&m, 1, tw), tab_strip_start_x(&m) + tw);
    }

    // ── close_button_rect ────────────────────────────────────────────

    #[test]
    fn close_button_within_tab_bounds() {
        let m = default_metrics();
        let tw = 200;
        let rect = close_button_rect(&m, 0, tw);
        let tab_end = tab_origin_x(&m, 0, tw) + tw;
        // Close button right edge must be within tab
        assert!(rect.x + rect.w <= tab_end);
        // Must be square
        assert_eq!(rect.w, rect.h);
    }

    #[test]
    fn close_button_size_matches_constant() {
        let m = default_metrics();
        let rect = close_button_rect(&m, 0, 200);
        assert_eq!(rect.w, m.scaled_px(CLOSE_BUTTON_SIZE));
    }

    // ── plus_button_rect ─────────────────────────────────────────────

    #[test]
    fn plus_button_after_last_tab() {
        let m = default_metrics();
        let tw = 200;
        let tab_count = 3;
        let rect = plus_button_rect(&m, tab_count, tw);
        let last_tab_end = tab_origin_x(&m, tab_count - 1, tw) + tw;
        assert!(rect.x > last_tab_end);
    }

    #[test]
    fn plus_button_size_matches_constant() {
        let m = default_metrics();
        let rect = plus_button_rect(&m, 1, 200);
        assert_eq!(rect.w, m.scaled_px(PLUS_BUTTON_SIZE));
        assert_eq!(rect.h, m.scaled_px(PLUS_BUTTON_SIZE));
    }

    // ── pin_button_rect (non-macOS) ──────────────────────────────────

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn pin_button_before_tab_strip() {
        let m = default_metrics();
        let rect = pin_button_rect(&m);
        assert!(rect.x + rect.w <= tab_strip_start_x(&m));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn pin_button_size_matches_constant() {
        let m = default_metrics();
        let rect = pin_button_rect(&m);
        assert_eq!(rect.w, m.scaled_px(PIN_BUTTON_SIZE));
    }

    // ── security_badge_reserved_width ────────────────────────────────

    #[test]
    fn security_badge_zero_count_returns_zero() {
        let m = default_metrics();
        assert_eq!(security_badge_reserved_width(&m, 0), 0);
    }

    #[test]
    fn security_badge_single_count_no_text() {
        let m = default_metrics();
        let w = security_badge_reserved_width(&m, 1);
        // badge_icon_size + 0 count_width + scaled_px(6)
        let icon = security_badge_icon_size(&m);
        assert_eq!(w, icon + m.scaled_px(6));
    }

    #[test]
    fn security_badge_multi_count_includes_text() {
        let m = default_metrics();
        let w1 = security_badge_reserved_width(&m, 1);
        let w5 = security_badge_reserved_width(&m, 5);
        assert!(w5 > w1);
    }

    // ── security_badge_rect ──────────────────────────────────────────

    #[test]
    fn security_badge_none_for_zero_count() {
        let m = default_metrics();
        assert!(security_badge_rect(&m, 0, 3, 1200, 0).is_none());
    }

    #[test]
    fn security_badge_none_for_invalid_index() {
        let m = default_metrics();
        assert!(security_badge_rect(&m, 5, 3, 1200, 1).is_none());
    }

    #[test]
    fn security_badge_some_for_valid() {
        let m = default_metrics();
        let rect = security_badge_rect(&m, 0, 3, 1200, 2);
        assert!(rect.is_some());
        let r = rect.unwrap();
        // Badge should be inside the tab area
        let tw = calculate_tab_width(&m, 3, 1200);
        let tab_end = tab_origin_x(&m, 0, tw) + tw;
        assert!(r.x + r.w <= tab_end);
    }

    // ── close_button_reserved_width ──────────────────────────────────

    #[test]
    fn close_reserved_matches_components() {
        let m = default_metrics();
        assert_eq!(
            close_button_reserved_width(&m),
            m.scaled_px(CLOSE_BUTTON_SIZE) + m.scaled_px(CLOSE_BUTTON_MARGIN)
        );
    }

    // ── tab_title_max_chars ──────────────────────────────────────────

    #[test]
    fn title_chars_decrease_with_close_button() {
        let m = default_metrics();
        let tw = 240;
        let without_close = tab_title_max_chars(&m, tw, false, 0);
        let with_close = tab_title_max_chars(&m, tw, true, 0);
        assert!(without_close >= with_close);
    }

    #[test]
    fn title_chars_decrease_with_security_badge() {
        let m = default_metrics();
        let tw = 240;
        let without_badge = tab_title_max_chars(&m, tw, true, 0);
        let with_badge = tab_title_max_chars(&m, tw, true, 5);
        assert!(without_badge >= with_badge);
    }

    #[test]
    fn title_chars_zero_for_very_narrow_tab() {
        let m = default_metrics();
        assert_eq!(tab_title_max_chars(&m, 0, true, 0), 0);
    }

    // ── should_show_number ───────────────────────────────────────────

    #[test]
    fn show_number_for_narrow_tab() {
        let m = default_metrics();
        let narrow = m.scaled_px(MIN_TAB_WIDTH_FOR_TITLE) - 1;
        assert!(should_show_number(&m, narrow));
    }

    #[test]
    fn no_show_number_for_wide_tab() {
        let m = default_metrics();
        let wide = m.scaled_px(MIN_TAB_WIDTH_FOR_TITLE);
        assert!(!should_show_number(&m, wide));
    }

    // ── rename_field_rect ────────────────────────────────────────────

    #[test]
    fn rename_field_within_tab() {
        let m = default_metrics();
        let tw = 200;
        let tab_x = tab_origin_x(&m, 0, tw);
        let rect = rename_field_rect(&m, tab_x, tw);
        // Field should start after tab_x
        assert!(rect.x >= tab_x);
        // Field should not extend past tab end
        assert!(rect.x + rect.w <= tab_x + tw + m.scaled_px(3) * 2);
    }

    #[test]
    fn rename_field_max_chars_positive() {
        let m = default_metrics();
        let chars = rename_field_max_chars(&m, 200);
        assert!(chars > 0);
    }

    // ── tab_text_y ───────────────────────────────────────────────────

    #[test]
    fn text_y_vertically_centered() {
        let m = default_metrics();
        let y = tab_text_y(&m);
        // Should be roughly centered: (36 - 20) / 2 + 1 = 9
        assert_eq!(y, (m.tab_bar_height - m.cell_height) / 2 + m.scaled_px(1));
    }

    // ── tab_insert_index_from_x ──────────────────────────────────────

    #[test]
    fn insert_index_before_first_tab() {
        let m = default_metrics();
        let idx = tab_insert_index_from_x(&m, 0.0, 3, 1200);
        assert_eq!(idx, 0);
    }

    #[test]
    fn insert_index_after_last_tab() {
        let m = default_metrics();
        let idx = tab_insert_index_from_x(&m, 10000.0, 3, 1200);
        assert_eq!(idx, 3);
    }

    // ── Rect::contains ───────────────────────────────────────────────

    #[test]
    fn rect_contains_center() {
        let r = Rect { x: 10, y: 10, w: 20, h: 20 };
        assert!(r.contains(20.0, 20.0));
    }

    #[test]
    fn rect_excludes_outside() {
        let r = Rect { x: 10, y: 10, w: 20, h: 20 };
        assert!(!r.contains(5.0, 15.0));
        assert!(!r.contains(31.0, 15.0));
    }

    #[test]
    fn rect_contains_top_left_edge() {
        let r = Rect { x: 10, y: 10, w: 20, h: 20 };
        assert!(r.contains(10.0, 10.0));
    }

    #[test]
    fn rect_excludes_bottom_right_edge() {
        let r = Rect { x: 10, y: 10, w: 20, h: 20 };
        // Exclusive end
        assert!(!r.contains(30.0, 30.0));
    }

    // ── to_tuple ─────────────────────────────────────────────────────

    #[test]
    fn rect_to_tuple_round_trip() {
        let r = Rect { x: 1, y: 2, w: 3, h: 4 };
        assert_eq!(r.to_tuple(), (1, 2, 3, 4));
    }

    // ── HiDPI consistency ────────────────────────────────────────────

    #[test]
    fn hidpi_close_button_scales() {
        let m1 = default_metrics();
        let m2 = hidpi_metrics();
        let r1 = close_button_rect(&m1, 0, 200);
        let r2 = close_button_rect(&m2, 0, 400);
        // 2x scale should produce 2x button size
        assert_eq!(r2.w, r1.w * 2);
    }

    #[test]
    fn hidpi_plus_button_scales() {
        let m1 = default_metrics();
        let m2 = hidpi_metrics();
        let r1 = plus_button_rect(&m1, 1, 200);
        let r2 = plus_button_rect(&m2, 1, 400);
        assert_eq!(r2.w, r1.w * 2);
    }
}
