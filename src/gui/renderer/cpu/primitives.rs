use crate::core::Color;

#[cfg(not(target_os = "macos"))]
use super::super::types::RoundedShape;
use super::super::types::RenderTarget;
use super::{CachedGlyph, CpuRenderer};
use crate::gui::renderer::rasterizer::GlyphCoverage;

impl CpuRenderer {
    pub(in crate::gui::renderer) fn draw_bg(
        &self,
        target: &mut RenderTarget<'_>,
        x: u32,
        y: u32,
        color: Color,
    ) {
        let pixel = color.to_pixel();
        for dy in 0..self.metrics.cell_height as usize {
            let py = y as usize + dy;
            if py >= target.height {
                break;
            }
            for dx in 0..self.metrics.cell_width as usize {
                let px = x as usize + dx;
                if px >= target.width {
                    break;
                }
                target.buffer[py * target.width + px] = pixel;
            }
        }
    }

    pub(in crate::gui::renderer) fn draw_char(
        &mut self,
        target: &mut RenderTarget<'_>,
        x: u32,
        y: u32,
        character: char,
        fg: Color,
    ) {
        // Rasterize and cache on first use; skip spaces / empty glyphs.
        if !self.glyph_cache.contains_key(&character) {
            if let Some(g) = self.rasterizer.rasterize(character) {
                self.glyph_cache.insert(character, CachedGlyph {
                    width:    g.width,
                    height:   g.height,
                    left:     g.left,
                    top:      g.top,
                    coverage: g.coverage,
                });
            } else {
                return;
            }
        }
        let glyph = match self.glyph_cache.get(&character) {
            Some(g) => g,
            None    => return,
        };

        let ascent = self.metrics.ascent;

        // Decode foreground to linear using the precomputed LUT.
        let fg_lr = self.srgb_to_linear[fg.r as usize];
        let fg_lg = self.srgb_to_linear[fg.g as usize];
        let fg_lb = self.srgb_to_linear[fg.b as usize];

        for gy in 0..glyph.height as usize {
            for gx in 0..glyph.width as usize {
                let sx = x as i32 + glyph.left + gx as i32;
                let sy = y as i32 + (ascent - glyph.top) + gy as i32;
                if sx < 0
                    || sy < 0
                    || sx as usize >= target.width
                    || sy as usize >= target.height
                {
                    continue;
                }
                let idx = sy as usize * target.width + sx as usize;
                let bg_pixel = target.buffer[idx];
                let bg_lr = self.srgb_to_linear[((bg_pixel >> 16) & 0xFF) as usize];
                let bg_lg = self.srgb_to_linear[((bg_pixel >>  8) & 0xFF) as usize];
                let bg_lb = self.srgb_to_linear[ (bg_pixel        & 0xFF) as usize];

                let pixel = match &glyph.coverage {
                    GlyphCoverage::Grayscale(data) => {
                        let t = data[gy * glyph.width as usize + gx] as f32 / 255.0;
                        let r = Color::channel_to_srgb(fg_lr * t + bg_lr * (1.0 - t));
                        let g = Color::channel_to_srgb(fg_lg * t + bg_lg * (1.0 - t));
                        let b = Color::channel_to_srgb(fg_lb * t + bg_lb * (1.0 - t));
                        (r as u32) << 16 | (g as u32) << 8 | b as u32
                    }
                    GlyphCoverage::Lcd(data) => {
                        let [rt, gt, bt] = data[gy * glyph.width as usize + gx];
                        let r = Color::channel_to_srgb(
                            fg_lr * rt as f32 / 255.0 + bg_lr * (1.0 - rt as f32 / 255.0));
                        let g = Color::channel_to_srgb(
                            fg_lg * gt as f32 / 255.0 + bg_lg * (1.0 - gt as f32 / 255.0));
                        let b = Color::channel_to_srgb(
                            fg_lb * bt as f32 / 255.0 + bg_lb * (1.0 - bt as f32 / 255.0));
                        (r as u32) << 16 | (g as u32) << 8 | b as u32
                    }
                };
                target.buffer[idx] = pixel;
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::renderer) fn draw_rounded_rect(
        &self,
        target: &mut RenderTarget<'_>,
        shape: &RoundedShape,
    ) {
        Self::draw_rounded_impl(target, shape, Self::rounded_coverage);
    }

    /// Shared pixel iteration for rounded rectangle drawing.
    /// The `coverage_fn` determines which corners are rounded.
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::renderer) fn draw_rounded_impl(
        target: &mut RenderTarget<'_>,
        shape: &RoundedShape,
        coverage_fn: fn(i32, i32, i32, i32, i32) -> f32,
    ) {
        if shape.w == 0
            || shape.h == 0
            || shape.alpha == 0
            || target.width == 0
            || target.height == 0
        {
            return;
        }

        let r = shape.radius.min(shape.w / 2).min(shape.h / 2) as i32;
        let max_x = target.width as i32 - 1;
        let max_y = target.height as i32 - 1;

        for py in 0..shape.h as i32 {
            let sy = shape.y + py;
            if sy < 0 || sy > max_y {
                continue;
            }

            for px in 0..shape.w as i32 {
                let sx = shape.x + px;
                if sx < 0 || sx > max_x {
                    continue;
                }

                let coverage = coverage_fn(px, py, shape.w as i32, shape.h as i32, r);
                if coverage <= 0.0 {
                    continue;
                }

                let idx = sy as usize * target.width + sx as usize;
                if idx >= target.buffer.len() {
                    continue;
                }
                let aa_alpha = ((shape.alpha as f32) * coverage)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                if aa_alpha == 0 {
                    continue;
                }
                target.buffer[idx] = crate::gui::renderer::blend_rgb(target.buffer[idx], shape.color, aa_alpha);
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn rounded_coverage(px: i32, py: i32, w: i32, h: i32, r: i32) -> f32 {
        if px < 0 || py < 0 || px >= w || py >= h {
            return 0.0;
        }
        if r <= 0 {
            return 1.0;
        }

        let in_tl = px < r && py < r;
        let in_tr = px >= w - r && py < r;
        let in_bl = px < r && py >= h - r;
        let in_br = px >= w - r && py >= h - r;
        if !(in_tl || in_tr || in_bl || in_br) {
            return 1.0;
        }

        let cx = if in_tl || in_bl {
            r as f32 - 0.5
        } else {
            (w - r) as f32 - 0.5
        };
        let cy = if in_tl || in_tr {
            r as f32 - 0.5
        } else {
            (h - r) as f32 - 0.5
        };

        let dx = px as f32 + 0.5 - cx;
        let dy = py as f32 + 0.5 - cy;
        let rr = r as f32;
        let dist = (dx * dx + dy * dy).sqrt();
        (rr + 0.5 - dist).clamp(0.0, 1.0)
    }


}
