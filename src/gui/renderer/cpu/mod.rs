mod primitives;
mod trait_impl;

use fontdue::{Font, FontSettings};
use std::collections::HashMap;

use super::{FONT_SIZE, LINE_PADDING};
use super::types::GlyphBitmap;

#[cfg(not(target_os = "macos"))]
use super::{SCROLLBAR_HIT_ZONE, SCROLLBAR_MARGIN, SCROLLBAR_WIDTH, TAB_BAR_HEIGHT, WINDOW_PADDING};
#[cfg(target_os = "macos")]
use super::{SCROLLBAR_HIT_ZONE, SCROLLBAR_MARGIN, SCROLLBAR_WIDTH};

/// CPU-based software renderer using softbuffer pixel buffers.
pub struct CpuRenderer {
    pub(in crate::gui::renderer) font: Font,
    pub(in crate::gui::renderer) font_size: f32,
    ui_scale: f64,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub(in crate::gui::renderer) tab_bar_visible: bool,
    pub cell_width: u32,
    pub cell_height: u32,
    pub(in crate::gui::renderer) ascent: i32,
    pub(in crate::gui::renderer) glyph_cache: HashMap<char, GlyphBitmap>,
}

impl CpuRenderer {
    pub fn new() -> Self {
        let font_data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/JetBrainsMono-Regular.ttf"
        ));
        let font = Font::from_bytes(font_data as &[u8], FontSettings::default())
            .expect("font load failed");

        let mut renderer = CpuRenderer {
            font,
            font_size: 1.0,
            ui_scale: 1.0,
            tab_bar_visible: false,
            cell_width: 1,
            cell_height: 1,
            ascent: 0,
            glyph_cache: HashMap::new(),
        };
        renderer.recompute_metrics();
        renderer
    }

    pub(in crate::gui::renderer) fn recompute_metrics(&mut self) {
        let scaled_font_size = (FONT_SIZE as f64 * self.ui_scale).max(1.0) as f32;
        let line_padding = self.scaled_px(LINE_PADDING);
        let line_metrics = self
            .font
            .horizontal_line_metrics(scaled_font_size)
            .expect("no horizontal line metrics");
        let asc = line_metrics.ascent.round() as i32;
        let desc = line_metrics.descent.round() as i32; // negative
        self.ascent = asc + line_padding as i32 / 2;
        self.cell_height = ((asc - desc).max(1) as u32) + line_padding;

        // Measure advance width from 'M'
        let (m_metrics, _) = self.font.rasterize('M', scaled_font_size);
        self.cell_width = m_metrics.advance_width.round().max(1.0) as u32;
        self.font_size = scaled_font_size;
        self.glyph_cache.clear();
    }

    pub fn set_scale(&mut self, scale_factor: f64) {
        let scale = if scale_factor.is_finite() {
            scale_factor.clamp(0.75, 4.0)
        } else {
            1.0
        };
        const SCALE_EPSILON: f64 = 1e-6;
        if (self.ui_scale - scale).abs() < SCALE_EPSILON {
            return;
        }
        self.ui_scale = scale;
        self.recompute_metrics();
    }

    pub(crate) fn ui_scale(&self) -> f64 {
        self.ui_scale
    }

    pub(crate) fn scaled_px(&self, base: u32) -> u32 {
        if base == 0 {
            0
        } else {
            ((base as f64 * self.ui_scale).round() as u32).max(1)
        }
    }

    pub(crate) fn tab_bar_height_px(&self) -> u32 {
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

    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub(crate) fn set_tab_bar_visible(&mut self, visible: bool) {
        #[cfg(target_os = "macos")]
        {
            let _ = visible;
            self.tab_bar_visible = false;
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.tab_bar_visible = visible;
        }
    }

    pub(crate) fn window_padding_px(&self) -> u32 {
        #[cfg(target_os = "macos")]
        {
            0
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.scaled_px(WINDOW_PADDING)
        }
    }

    pub(crate) fn scrollbar_width_px(&self) -> u32 {
        self.scaled_px(SCROLLBAR_WIDTH)
    }

    pub(crate) fn scrollbar_hit_zone_px(&self) -> u32 {
        self.scaled_px(SCROLLBAR_HIT_ZONE)
    }

    pub(crate) fn scrollbar_margin_px(&self) -> u32 {
        self.scaled_px(SCROLLBAR_MARGIN)
    }
}
