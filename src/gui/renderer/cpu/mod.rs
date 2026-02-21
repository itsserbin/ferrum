pub(in crate::gui::renderer) mod primitives;
mod trait_impl;

use fontdue::{Font, FontSettings};
use std::collections::HashMap;

use crate::config::{AppConfig, ThemePalette};
use super::metrics::FontMetrics;
use super::types::GlyphBitmap;

/// CPU-based software renderer using softbuffer pixel buffers.
pub struct CpuRenderer {
    pub(in crate::gui::renderer) font: Font,
    pub(in crate::gui::renderer) fallback_font: Font,
    pub(in crate::gui::renderer) metrics: FontMetrics,
    pub(in crate::gui::renderer) glyph_cache: HashMap<char, GlyphBitmap>,
    pub(in crate::gui::renderer) palette: ThemePalette,
}

impl CpuRenderer {
    pub fn new(config: &AppConfig) -> Self {
        let font_data = crate::config::font_data(config.font.family);
        let font = Font::from_bytes(font_data, FontSettings::default())
            .expect("font load failed");

        let fallback_data = crate::config::fallback_font_data();
        let fallback_font = Font::from_bytes(fallback_data, FontSettings::default())
            .expect("fallback font load failed");

        let mut metrics = FontMetrics {
            cell_width: 1,
            cell_height: 1,
            font_size: 1.0,
            ui_scale: 1.0,
            ascent: 0,
            tab_bar_visible: false,
            base_font_size: config.font.size,
            base_line_padding: config.font.line_padding,
            base_tab_bar_height: config.layout.tab_bar_height,
            base_window_padding: config.layout.window_padding,
            base_scrollbar_width: config.layout.scrollbar_width,
            base_pane_inner_padding: config.layout.pane_inner_padding,
        };
        metrics.recompute(&font);

        let palette = config.theme.resolve();

        CpuRenderer {
            font,
            fallback_font,
            metrics,
            glyph_cache: HashMap::new(),
            palette,
        }
    }

    pub(in crate::gui::renderer) fn apply_config(&mut self, config: &AppConfig) {
        let font_data = crate::config::font_data(config.font.family);
        self.font = Font::from_bytes(font_data, FontSettings::default()).expect("font load failed");
        self.metrics.update_bases(config);
        self.recompute_metrics();
        self.palette = config.theme.resolve();
    }

    pub(in crate::gui::renderer) fn recompute_metrics(&mut self) {
        self.metrics.recompute(&self.font);
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
