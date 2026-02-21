pub(in crate::gui::renderer) mod primitives;
mod trait_impl;

use fontdue::Font;
use std::collections::HashMap;

use crate::config::{AppConfig, ThemePalette};
use super::metrics::FontMetrics;
use super::types::GlyphBitmap;

/// CPU-based software renderer using softbuffer pixel buffers.
pub struct CpuRenderer {
    pub(in crate::gui::renderer) font: Font,
    pub(in crate::gui::renderer) fallback_fonts: Vec<Font>,
    pub(in crate::gui::renderer) metrics: FontMetrics,
    pub(in crate::gui::renderer) glyph_cache: HashMap<char, GlyphBitmap>,
    pub(in crate::gui::renderer) palette: ThemePalette,
}

impl CpuRenderer {
    pub fn new(config: &AppConfig) -> Self {
        let (font, fallback_fonts) = crate::config::load_fonts(config.font.family);

        let mut metrics = FontMetrics::from_config(config);
        metrics.recompute(&font);

        let palette = config.theme.resolve();

        CpuRenderer {
            font,
            fallback_fonts,
            metrics,
            glyph_cache: HashMap::new(),
            palette,
        }
    }

    pub(in crate::gui::renderer) fn apply_config(&mut self, config: &AppConfig) {
        let (font, fallback_fonts) = crate::config::load_fonts(config.font.family);
        self.font = font;
        self.fallback_fonts = fallback_fonts;
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
