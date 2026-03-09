mod banner;
pub(super) mod primitives;
mod trait_impl;

use std::collections::HashMap;

use crate::config::{AppConfig, ThemePalette, load_fonts};
use crate::core::Color;
use super::rasterizer::{GlyphRasterizer, RasterMode, RasterizedGlyph};
use super::metrics::FontMetrics;

/// CPU-based software renderer using softbuffer pixel buffers.
pub struct CpuRenderer {
    pub(super) rasterizer:     GlyphRasterizer,
    pub(super) metrics:        FontMetrics,
    pub(super) glyph_cache:    HashMap<char, RasterizedGlyph>,
    pub(super) srgb_to_linear: [f32; 256],
    /// sRGB encode LUT: index is `(linear * 255 + 0.5) as u8`, value is the sRGB byte.
    /// Avoids a `powf` call per pixel in the glyph blend inner loop.
    pub(super) linear_to_srgb: [u8; 256],
    pub(super) palette:        ThemePalette,
}

fn build_srgb_lut() -> [f32; 256] {
    let mut lut = [0f32; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        *v = Color::channel_to_linear(i as u8);
    }
    lut
}

fn build_linear_to_srgb_lut() -> [u8; 256] {
    let mut lut = [0u8; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        *v = Color::channel_to_srgb(i as f32 / 255.0);
    }
    lut
}

impl CpuRenderer {
    pub fn new(config: &AppConfig) -> Self {
        let (font_data, fallback_data) = load_fonts(config.font.family);
        let scale_factor = 1.0_f64; // CPU renderer initialises without a window; scale set later via set_scale
        let mode = RasterMode::from_scale_factor(scale_factor);
        let mut rasterizer = GlyphRasterizer::new(font_data, fallback_data, config.font.size, mode);

        let mut metrics = FontMetrics::from_config(config);
        metrics.recompute(&mut rasterizer);

        let palette = config.theme.resolve();

        CpuRenderer {
            rasterizer,
            metrics,
            glyph_cache: HashMap::new(),
            srgb_to_linear: build_srgb_lut(),
            linear_to_srgb: build_linear_to_srgb_lut(),
            palette,
        }
    }

    pub(super) fn apply_config(&mut self, config: &AppConfig) {
        let (font_data, fallback_data) = load_fonts(config.font.family);
        self.rasterizer = GlyphRasterizer::new(
            font_data, fallback_data, config.font.size, self.rasterizer.mode,
        );
        self.metrics.update_bases(config);
        self.recompute_metrics();
        self.palette = config.theme.resolve();
    }

    pub(super) fn recompute_metrics(&mut self) {
        self.metrics.recompute(&mut self.rasterizer);
        self.glyph_cache.clear();
    }

    pub fn set_scale(&mut self, scale_factor: f64) {
        let scale = super::sanitize_scale(scale_factor);
        if !super::scale_changed(self.metrics.ui_scale, scale) {
            return;
        }
        self.metrics.ui_scale = scale;
        let new_mode = RasterMode::from_scale_factor(scale);
        if self.rasterizer.mode != new_mode {
            self.rasterizer.rebuild(self.metrics.font_size, new_mode);
        }
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

    #[cfg(not(target_os = "macos"))]
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

    /// Draws a standard hover-highlight rounded rect for a tab-bar button.
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_button_hover_bg(
        &self,
        target: &mut super::types::RenderTarget<'_>,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    ) {
        use super::RoundedShape;
        self.draw_rounded_rect(
            target,
            &RoundedShape {
                x: x as i32,
                y: y as i32,
                w,
                h,
                radius: self.scaled_px(5),
                color: self.palette.inactive_tab_hover.to_pixel(),
                alpha: 255,
            },
        );
    }

    /// Draws a floating overlay box: a filled rounded rect with a subtle border on top.
    /// Used for tooltips, banners, and other overlay UI elements.
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_overlay_box(
        &self,
        target: &mut super::types::RenderTarget<'_>,
        bg_x: i32,
        bg_y: i32,
        bg_w: u32,
        bg_h: u32,
        radius: u32,
    ) {
        use super::RoundedShape;

        // Background fill.
        self.draw_rounded_rect(
            target,
            &RoundedShape {
                x: bg_x,
                y: bg_y,
                w: bg_w,
                h: bg_h,
                radius,
                color: self.palette.active_tab_bg.to_pixel(),
                alpha: 245,
            },
        );
        // Subtle border.
        self.draw_rounded_rect(
            target,
            &RoundedShape {
                x: bg_x,
                y: bg_y,
                w: bg_w,
                h: bg_h,
                radius,
                color: self.palette.tab_border.to_pixel(),
                alpha: 80,
            },
        );
    }
}
