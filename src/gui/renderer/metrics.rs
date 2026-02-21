use super::{SCROLLBAR_HIT_ZONE, SCROLLBAR_MARGIN};
use fontdue::Font;

/// Font metrics shared across renderers.
///
/// Encapsulates cell dimensions, font size, and DPI scale, providing
/// common metric calculations (scaled pixels, tab bar height, etc.).
pub struct FontMetrics {
    pub cell_width: u32,
    pub cell_height: u32,
    pub font_size: f32,
    pub ui_scale: f64,
    pub ascent: i32,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub tab_bar_visible: bool,
    // -- Configurable base values (from AppConfig) --
    pub base_font_size: f32,
    pub base_line_padding: u32,
    pub base_tab_bar_height: u32,
    pub base_window_padding: u32,
    pub base_scrollbar_width: u32,
    pub base_pane_inner_padding: u32,
}

impl FontMetrics {
    /// Recomputes all metrics from the given font and current ui_scale.
    pub fn recompute(&mut self, font: &Font) {
        let scaled_font_size = (self.base_font_size as f64 * self.ui_scale).max(1.0) as f32;
        let line_padding = self.scaled_px(self.base_line_padding);
        let line_metrics = font
            .horizontal_line_metrics(scaled_font_size)
            .expect("no horizontal line metrics");
        // Use ceil for ascent (round up to not clip tops of glyphs)
        // and ceil for descent magnitude (round towards zero to tighten cell).
        let asc = line_metrics.ascent.ceil() as i32;
        let desc = line_metrics.descent.ceil() as i32; // negative, ceil = towards zero
        self.ascent = asc + line_padding as i32 / 2;
        self.cell_height = ((asc - desc).max(1) as u32) + line_padding;

        // Measure advance width from 'M'
        let (m_metrics, _) = font.rasterize('M', scaled_font_size);
        self.cell_width = m_metrics.advance_width.round().max(1.0) as u32;
        self.font_size = scaled_font_size;
    }

    /// Scales a base pixel value by the current UI scale factor.
    ///
    /// Delegates to [`super::types::scaled_px`] — the single source of truth.
    pub fn scaled_px(&self, base: u32) -> u32 {
        super::types::scaled_px(base, self.ui_scale)
    }

    pub fn tab_bar_height_px(&self) -> u32 {
        #[cfg(target_os = "macos")]
        {
            0
        }
        #[cfg(not(target_os = "macos"))]
        {
            if self.tab_bar_visible {
                self.scaled_px(self.base_tab_bar_height)
            } else {
                0
            }
        }
    }

    pub fn window_padding_px(&self) -> u32 {
        self.scaled_px(self.base_window_padding)
    }

    pub fn scrollbar_hit_zone_px(&self) -> u32 {
        self.scaled_px(SCROLLBAR_HIT_ZONE)
    }

    pub fn scrollbar_width_px(&self) -> u32 {
        self.scaled_px(self.base_scrollbar_width)
    }

    pub fn scrollbar_margin_px(&self) -> u32 {
        self.scaled_px(SCROLLBAR_MARGIN)
    }

    pub fn pane_inner_padding_px(&self) -> u32 {
        self.scaled_px(self.base_pane_inner_padding)
    }

    /// Updates all configurable base values from config.
    pub fn update_bases(&mut self, config: &crate::config::AppConfig) {
        self.base_font_size = config.font.size;
        self.base_line_padding = config.font.line_padding;
        self.base_window_padding = config.layout.window_padding;
        self.base_tab_bar_height = config.layout.tab_bar_height;
        self.base_pane_inner_padding = config.layout.pane_inner_padding;
        self.base_scrollbar_width = config.layout.scrollbar_width;
    }
}
