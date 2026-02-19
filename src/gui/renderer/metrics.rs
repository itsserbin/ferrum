use super::{FONT_SIZE, LINE_PADDING, SCROLLBAR_HIT_ZONE, SCROLLBAR_MARGIN, SCROLLBAR_WIDTH};
#[cfg(not(target_os = "macos"))]
use super::{TAB_BAR_HEIGHT, WINDOW_PADDING};
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
}

impl FontMetrics {
    /// Recomputes all metrics from the given font and current ui_scale.
    pub fn recompute(&mut self, font: &Font) {
        let scaled_font_size = (FONT_SIZE as f64 * self.ui_scale).max(1.0) as f32;
        let line_padding = self.scaled_px(LINE_PADDING);
        let line_metrics = font
            .horizontal_line_metrics(scaled_font_size)
            .expect("no horizontal line metrics");
        let asc = line_metrics.ascent.round() as i32;
        let desc = line_metrics.descent.round() as i32; // negative
        self.ascent = asc + line_padding as i32 / 2;
        self.cell_height = ((asc - desc).max(1) as u32) + line_padding;

        // Measure advance width from 'M'
        let (m_metrics, _) = font.rasterize('M', scaled_font_size);
        self.cell_width = m_metrics.advance_width.round().max(1.0) as u32;
        self.font_size = scaled_font_size;
    }

    /// Scales a base pixel value by the current UI scale factor.
    pub fn scaled_px(&self, base: u32) -> u32 {
        if base == 0 {
            0
        } else {
            ((base as f64 * self.ui_scale).round() as u32).max(1)
        }
    }

    pub fn tab_bar_height_px(&self) -> u32 {
        #[cfg(target_os = "macos")]
        {
            0
        }
        #[cfg(not(target_os = "macos"))]
        {
            if self.tab_bar_visible {
                self.scaled_px(TAB_BAR_HEIGHT)
            } else {
                0
            }
        }
    }

    pub fn window_padding_px(&self) -> u32 {
        #[cfg(target_os = "macos")]
        {
            0
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.scaled_px(WINDOW_PADDING)
        }
    }

    pub fn scrollbar_hit_zone_px(&self) -> u32 {
        self.scaled_px(SCROLLBAR_HIT_ZONE)
    }

    pub fn scrollbar_width_px(&self) -> u32 {
        self.scaled_px(SCROLLBAR_WIDTH)
    }

    pub fn scrollbar_margin_px(&self) -> u32 {
        self.scaled_px(SCROLLBAR_MARGIN)
    }
}
