//! Layout computation for the update-available banner.
//!
//! The banner is a small rounded rect centred horizontally at the top of the
//! terminal area (just below the tab bar on non-macOS; at the top edge on
//! macOS). It contains: an "Update vX.Y.Z" label, a [Details] button, an
//! [Install] button, and a [✕] dismiss button.

use crate::gui::renderer::shared::tab_math::TabLayoutMetrics;

/// Pre-computed geometry for the update-available banner.
#[derive(Debug, Clone)]
pub(in crate::gui) struct UpdateBannerLayout {
    /// Background rectangle.
    pub bg_x: i32,
    pub bg_y: i32,
    pub bg_w: u32,
    pub bg_h: u32,
    /// Corner radius.
    pub radius: u32,
    /// "Update vX.Y.Z available" label position.
    pub label_x: u32,
    pub label_y: u32,
    /// The full label text (e.g. "Update v0.3.2 available").
    pub label_text: String,
    /// [Details] button rect.
    pub details_x: u32,
    pub details_y: u32,
    pub details_w: u32,
    pub btn_h: u32,
    /// [Install] button rect (same height as details).
    pub install_x: u32,
    pub install_y: u32,
    pub install_w: u32,
    /// [✕] dismiss button rect.
    pub dismiss_x: u32,
    pub dismiss_y: u32,
    pub dismiss_w: u32,
    /// When true, only the label is shown (with "Installing…" text), no buttons.
    pub installing: bool,
}

/// Computes the banner layout.
///
/// Returns `None` when the buffer is too small to fit anything.
///
/// `tab_bar_h` is the pixel height of the tab bar (0 on macOS where the
/// native tab bar does not consume our buffer space).
pub(in crate::gui) fn compute_update_banner_layout(
    tag_name: &str,
    m: &TabLayoutMetrics,
    buf_width: u32,
    buf_height: u32,
    tab_bar_h: u32,
) -> Option<UpdateBannerLayout> {
    if buf_width == 0 || buf_height == 0 {
        return None;
    }

    let cell_height = m.cell_height;

    let pad_x = m.scaled_px(10);
    let pad_y = m.scaled_px(6);
    let btn_pad_x = m.scaled_px(8);
    let gap = m.scaled_px(6);
    let radius = m.scaled_px(6);

    let label_text = format!("Update {} available", tag_name);
    let label_chars = label_text.chars().count() as u32;
    let label_w = label_chars * m.cell_width;

    let details_text_w = "Details".chars().count() as u32 * m.cell_width;
    let install_text_w = "Install".chars().count() as u32 * m.cell_width;
    let dismiss_text_w = m.cell_width; // "✕"

    let btn_h = cell_height + pad_y;
    let details_w = details_text_w + btn_pad_x * 2;
    let install_w = install_text_w + btn_pad_x * 2;
    let dismiss_w = dismiss_text_w + btn_pad_x * 2;

    let total_w = pad_x * 2 + label_w + gap + details_w + gap + install_w + gap + dismiss_w;
    if total_w > buf_width {
        return None;
    }

    let bg_h = btn_h + pad_y * 2;
    let margin_top = m.scaled_px(6);
    let bg_y = (tab_bar_h + margin_top) as i32;
    let bg_x = ((buf_width - total_w) / 2) as i32;

    let label_x = bg_x as u32 + pad_x;
    let label_y = bg_y as u32 + pad_y + (btn_h - cell_height) / 2;

    let details_x = label_x + label_w + gap;
    let details_y = bg_y as u32 + pad_y;
    let install_x = details_x + details_w + gap;
    let install_y = details_y;
    let dismiss_x = install_x + install_w + gap;
    let dismiss_y = details_y;

    Some(UpdateBannerLayout {
        bg_x,
        bg_y,
        bg_w: total_w,
        bg_h,
        radius,
        label_x,
        label_y,
        label_text,
        details_x,
        details_y,
        details_w,
        btn_h,
        install_x,
        install_y,
        install_w,
        dismiss_x,
        dismiss_y,
        dismiss_w,
        installing: false,
    })
}

impl UpdateBannerLayout {
    /// Returns the background rectangle as `(x, y, w, h)`.
    pub(in crate::gui) fn bg_rect(&self) -> (i32, i32, u32, u32) {
        (self.bg_x, self.bg_y, self.bg_w, self.bg_h)
    }

    /// Returns the corner radius for the background rounded rect.
    pub(in crate::gui) fn corner_radius(&self) -> u32 {
        self.radius
    }

    /// Returns the label text and its top-left position.
    pub(in crate::gui) fn label(&self) -> (&str, u32, u32) {
        (&self.label_text, self.label_x, self.label_y)
    }

    /// Returns the [Details] button rect as `(x, y, w, h)`.
    pub(in crate::gui) fn details_rect(&self) -> (u32, u32, u32, u32) {
        (self.details_x, self.details_y, self.details_w, self.btn_h)
    }

    /// Returns the [Install] button rect as `(x, y, w, h)`.
    pub(in crate::gui) fn install_rect(&self) -> (u32, u32, u32, u32) {
        (self.install_x, self.install_y, self.install_w, self.btn_h)
    }

    /// Returns the [✕] dismiss button rect as `(x, y, w, h)`.
    pub(in crate::gui) fn dismiss_rect(&self) -> (u32, u32, u32, u32) {
        (self.dismiss_x, self.dismiss_y, self.dismiss_w, self.btn_h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics_1x() -> TabLayoutMetrics {
        TabLayoutMetrics {
            cell_width: 9,
            cell_height: 20,
            ui_scale: 1.0,
            tab_bar_height: 36,
        }
    }

    fn metrics_2x() -> TabLayoutMetrics {
        TabLayoutMetrics {
            cell_width: 18,
            cell_height: 40,
            ui_scale: 2.0,
            tab_bar_height: 72,
        }
    }

    #[test]
    fn zero_buf_returns_none() {
        let m = metrics_1x();
        assert!(compute_update_banner_layout("v0.3.2", &m, 0, 600, 36).is_none());
        assert!(compute_update_banner_layout("v0.3.2", &m, 800, 0, 36).is_none());
    }

    #[test]
    fn basic_layout_has_positive_dimensions() {
        let m = metrics_1x();
        let l = compute_update_banner_layout("v0.3.2", &m, 800, 600, 36)
            .expect("should fit in 800px");
        assert!(l.bg_w > 0);
        assert!(l.bg_h > 0);
        assert!(l.radius > 0);
        assert!(!l.label_text.is_empty());
    }

    #[test]
    fn banner_positioned_below_tab_bar() {
        let m = metrics_1x();
        let l = compute_update_banner_layout("v0.3.2", &m, 800, 600, 36)
            .expect("layout");
        assert!(l.bg_y >= 36);
    }

    #[test]
    fn banner_horizontally_centred() {
        let m = metrics_1x();
        let l = compute_update_banner_layout("v0.3.2", &m, 800, 600, 36)
            .expect("layout");
        let right_space = 800i32 - (l.bg_x + l.bg_w as i32);
        assert!((l.bg_x - right_space).abs() <= 1);
    }

    #[test]
    fn hidpi_scales_dimensions() {
        let m1 = metrics_1x();
        let m2 = metrics_2x();
        let l1 = compute_update_banner_layout("v0.3.2", &m1, 800, 600, 36).unwrap();
        let l2 = compute_update_banner_layout("v0.3.2", &m2, 1600, 1200, 72).unwrap();
        assert!(l2.bg_h > l1.bg_h);
        assert!(l2.radius > l1.radius);
    }

    #[test]
    fn buttons_inside_background() {
        let m = metrics_1x();
        let l = compute_update_banner_layout("v0.3.2", &m, 800, 600, 36).unwrap();
        assert!(l.details_x >= l.bg_x as u32);
        assert!(l.install_x >= l.bg_x as u32);
        assert!(l.dismiss_x >= l.bg_x as u32);
        let bg_right = (l.bg_x as u32) + l.bg_w;
        assert!(l.dismiss_x + l.dismiss_w <= bg_right);
    }
}
