//! Pure scrollbar thumb geometry shared by CPU and GPU renderers.

/// Computes the scrollbar track top, track bottom, and minimum thumb height
/// from raw layout parameters. This eliminates the duplicated 3-line preamble
/// that both CPU and GPU renderers repeat before calling
/// [`scrollbar_thumb_geometry`].
///
/// Returns `(track_top, track_bottom, min_thumb)` in physical pixels.
///
/// - `tab_bar_height` — tab bar height in physical pixels (0 on macOS).
/// - `window_padding` — window padding in physical pixels (0 on macOS).
/// - `buf_height` — buffer / surface height in physical pixels.
/// - `min_thumb_base` — unscaled minimum thumb constant (e.g. `SCROLLBAR_MIN_THUMB`).
/// - `ui_scale` — HiDPI scale factor (e.g. 1.0, 2.0).
pub(in crate::gui::renderer) fn scrollbar_track_params(
    tab_bar_height: u32,
    window_padding: u32,
    buf_height: usize,
    min_thumb_base: u32,
    ui_scale: f64,
) -> (f32, f32, f32) {
    let track_top = (tab_bar_height + window_padding) as f32;
    let track_bottom = buf_height as f32 - window_padding as f32;
    let min_thumb = scaled_px(min_thumb_base, ui_scale) as f32;
    (track_top, track_bottom, min_thumb)
}

/// Scales a base pixel value by `ui_scale`, rounding and clamping to at least 1
/// (or 0 when `base == 0`).
///
/// Delegates to [`crate::gui::renderer::types::scaled_px`] — the single source of truth.
fn scaled_px(base: u32, ui_scale: f64) -> u32 {
    crate::gui::renderer::types::scaled_px(base, ui_scale)
}

/// Computes the scrollbar thumb position and height given the track dimensions.
///
/// Returns `Some((thumb_y, thumb_height))` in pixels, or `None` if the
/// scrollbar is not visible (no scrollback or zero-height track).
///
/// - `track_top`: top of the scrollbar track in pixels.
/// - `track_bottom`: bottom of the scrollbar track in pixels.
/// - `scroll_offset`: current scroll offset (0 = bottom, scrollback_len = top).
/// - `scrollback_len`: number of scrollback lines.
/// - `grid_rows`: number of visible terminal rows.
/// - `min_thumb`: minimum thumb height in pixels.
pub(in crate::gui::renderer) fn scrollbar_thumb_geometry(
    track_top: f32,
    track_bottom: f32,
    scroll_offset: usize,
    scrollback_len: usize,
    grid_rows: usize,
    min_thumb: f32,
) -> Option<(f32, f32)> {
    if scrollback_len == 0 {
        return None;
    }

    let track_height = track_bottom - track_top;
    if track_height <= 0.0 {
        return None;
    }

    let total_lines = scrollback_len + grid_rows;
    let viewport_ratio = grid_rows as f32 / total_lines as f32;
    let thumb_height = (viewport_ratio * track_height)
        .max(min_thumb)
        .min(track_height);

    let max_offset = scrollback_len as f32;
    let scroll_ratio = (max_offset - scroll_offset as f32) / max_offset;
    let thumb_y = track_top + scroll_ratio * (track_height - thumb_height);

    Some((thumb_y, thumb_height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_scrollback_returns_none() {
        assert!(scrollbar_thumb_geometry(0.0, 100.0, 0, 0, 24, 20.0).is_none());
    }

    #[test]
    fn zero_track_returns_none() {
        assert!(scrollbar_thumb_geometry(50.0, 50.0, 0, 10, 24, 20.0).is_none());
    }

    #[test]
    fn negative_track_returns_none() {
        assert!(scrollbar_thumb_geometry(100.0, 50.0, 0, 10, 24, 20.0).is_none());
    }

    #[test]
    fn basic_geometry_returns_some() {
        let result = scrollbar_thumb_geometry(36.0, 500.0, 0, 100, 24, 20.0);
        assert!(result.is_some());
        let (thumb_y, thumb_height) = result.unwrap();
        assert!(thumb_y >= 36.0);
        assert!(thumb_height > 0.0);
        assert!(thumb_y + thumb_height <= 500.0 + 1.0); // allow floating point margin
    }

    #[test]
    fn thumb_at_bottom_when_offset_zero() {
        let track_top = 36.0;
        let track_bottom = 500.0;
        let track_height = track_bottom - track_top;
        let result = scrollbar_thumb_geometry(track_top, track_bottom, 0, 100, 24, 20.0).unwrap();
        let (thumb_y, thumb_height) = result;
        // scroll_offset=0 means bottom: scroll_ratio = 1.0, thumb_y = track_top + track_height - thumb_height
        let expected_y = track_top + 1.0 * (track_height - thumb_height);
        assert!((thumb_y - expected_y).abs() < 0.01);
    }

    #[test]
    fn thumb_at_top_when_offset_max() {
        let track_top = 36.0;
        let track_bottom = 500.0;
        let scrollback = 100;
        let result =
            scrollbar_thumb_geometry(track_top, track_bottom, scrollback, scrollback, 24, 20.0)
                .unwrap();
        let (thumb_y, _) = result;
        // scroll_offset = scrollback_len means top: scroll_ratio = 0.0, thumb_y = track_top
        assert!((thumb_y - track_top).abs() < 0.01);
    }

    #[test]
    fn min_thumb_respected() {
        // Large scrollback, small grid -> viewport_ratio is tiny -> thumb clamped to min
        let min_thumb = 50.0;
        let result = scrollbar_thumb_geometry(0.0, 100.0, 0, 10000, 1, min_thumb).unwrap();
        let (_, thumb_height) = result;
        assert!((thumb_height - min_thumb).abs() < 0.01);
    }

    // ── scrollbar_track_params ────────────────────────────────────────

    #[test]
    fn track_params_basic() {
        let (top, bottom, min_t) = scrollbar_track_params(36, 8, 500, 20, 1.0);
        assert!((top - 44.0).abs() < 0.01); // 36 + 8
        assert!((bottom - 492.0).abs() < 0.01); // 500 - 8
        assert!((min_t - 20.0).abs() < 0.01); // 20 * 1.0
    }

    #[test]
    fn track_params_hidpi() {
        let (top, bottom, min_t) = scrollbar_track_params(72, 16, 1000, 20, 2.0);
        assert!((top - 88.0).abs() < 0.01); // 72 + 16
        assert!((bottom - 984.0).abs() < 0.01); // 1000 - 16
        assert!((min_t - 40.0).abs() < 0.01); // 20 * 2.0
    }

    #[test]
    fn track_params_zero_padding() {
        // macOS-like: no tab bar, no padding.
        let (top, bottom, min_t) = scrollbar_track_params(0, 0, 800, 20, 1.0);
        assert!((top - 0.0).abs() < 0.01);
        assert!((bottom - 800.0).abs() < 0.01);
        assert!((min_t - 20.0).abs() < 0.01);
    }

    #[test]
    fn track_params_consistent_with_manual() {
        // Verify the helper produces the same values a manual computation would.
        let tab_bar = 36u32;
        let padding = 8u32;
        let buf_h = 600usize;
        let min_base = 20u32;
        let scale = 1.5;

        let (top, bottom, min_t) = scrollbar_track_params(tab_bar, padding, buf_h, min_base, scale);

        let expected_top = (tab_bar + padding) as f32;
        let expected_bottom = buf_h as f32 - padding as f32;
        let expected_min = scaled_px(min_base, scale) as f32;

        assert!((top - expected_top).abs() < 0.01);
        assert!((bottom - expected_bottom).abs() < 0.01);
        assert!((min_t - expected_min).abs() < 0.01);
    }
}
