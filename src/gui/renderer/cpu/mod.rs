mod primitives;
mod trait_impl;

use fontdue::{Font, FontSettings};
use std::collections::HashMap;

use super::metrics::FontMetrics;
use super::types::GlyphBitmap;

/// CPU-based software renderer using softbuffer pixel buffers.
pub struct CpuRenderer {
    pub(in crate::gui::renderer) font: Font,
    pub(in crate::gui::renderer) metrics: FontMetrics,
    /// Convenience mirror of `metrics.cell_width` for code that accesses
    /// this field directly (e.g. tab_bar modules).
    pub(in crate::gui::renderer) cell_width: u32,
    /// Convenience mirror of `metrics.cell_height` for code that accesses
    /// this field directly (e.g. tab_bar modules).
    pub(in crate::gui::renderer) cell_height: u32,
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

        let mut metrics = FontMetrics {
            cell_width: 1,
            cell_height: 1,
            font_size: 1.0,
            ui_scale: 1.0,
            ascent: 0,
            tab_bar_visible: false,
        };
        metrics.recompute(&font);

        let cell_width = metrics.cell_width;
        let cell_height = metrics.cell_height;
        CpuRenderer {
            font,
            metrics,
            cell_width,
            cell_height,
            glyph_cache: HashMap::new(),
        }
    }

    pub(in crate::gui::renderer) fn recompute_metrics(&mut self) {
        self.metrics.recompute(&self.font);
        self.cell_width = self.metrics.cell_width;
        self.cell_height = self.metrics.cell_height;
        self.glyph_cache.clear();
    }

    pub fn set_scale(&mut self, scale_factor: f64) {
        let scale = super::sanitize_scale(scale_factor);
        if !super::scale_changed(self.metrics.ui_scale, scale) {
            return;
        }
        self.metrics.ui_scale = scale;
        self.recompute_metrics();
    }

    pub(crate) fn ui_scale(&self) -> f64 {
        self.metrics.ui_scale
    }

    pub(crate) fn scaled_px(&self, base: u32) -> u32 {
        self.metrics.scaled_px(base)
    }

    pub(crate) fn tab_bar_height_px(&self) -> u32 {
        self.metrics.tab_bar_height_px()
    }

    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub(crate) fn set_tab_bar_visible(&mut self, visible: bool) {
        self.metrics.tab_bar_visible = super::resolve_tab_bar_visible(visible);
    }

    pub(crate) fn window_padding_px(&self) -> u32 {
        self.metrics.window_padding_px()
    }

    pub(crate) fn scrollbar_width_px(&self) -> u32 {
        self.metrics.scrollbar_width_px()
    }

    pub(crate) fn scrollbar_hit_zone_px(&self) -> u32 {
        self.metrics.scrollbar_hit_zone_px()
    }

    pub(crate) fn scrollbar_margin_px(&self) -> u32 {
        self.metrics.scrollbar_margin_px()
    }
}
