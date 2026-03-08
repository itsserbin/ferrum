use super::{SCROLLBAR_HIT_ZONE, SCROLLBAR_MARGIN};
use crate::gui::renderer::rasterizer::GlyphRasterizer;

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
    #[cfg(not(target_os = "macos"))]
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
    /// Creates a new `FontMetrics` initialized from config with default scale.
    pub fn from_config(config: &crate::config::AppConfig) -> Self {
        Self {
            cell_width: 1,
            cell_height: 1,
            font_size: 1.0,
            ui_scale: 1.0,
            ascent: 0,
            #[cfg(not(target_os = "macos"))]
            tab_bar_visible: false,
            base_font_size: config.font.size,
            base_line_padding: config.font.line_padding,
            base_tab_bar_height: config.layout.tab_bar_height,
            base_window_padding: config.layout.window_padding,
            base_scrollbar_width: config.layout.scrollbar_width,
            base_pane_inner_padding: config.layout.pane_inner_padding,
        }
    }

    /// Recomputes all metrics using the rasterizer and the current ui_scale.
    ///
    /// Updates the rasterizer's font size to `base_font_size * ui_scale` before
    /// querying cell metrics, so cell dimensions reflect DPI scaling.
    pub fn recompute(&mut self, rasterizer: &mut GlyphRasterizer) {
        let scaled_font_size = (self.base_font_size as f64 * self.ui_scale).max(1.0) as f32;
        let mode = rasterizer.mode;
        rasterizer.rebuild(scaled_font_size, mode);
        let line_padding = self.scaled_px(self.base_line_padding);
        let cell_metrics = rasterizer.metrics();
        self.ascent = cell_metrics.ascent + (line_padding / 2) as i32;
        self.cell_height = cell_metrics.cell_height + line_padding;
        self.cell_width = cell_metrics.cell_width;
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
