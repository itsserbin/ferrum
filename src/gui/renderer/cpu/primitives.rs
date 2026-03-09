use crate::core::Color;

#[cfg(not(target_os = "macos"))]
use super::super::types::RoundedShape;
use super::super::types::RenderTarget;
use super::CpuRenderer;
use crate::gui::renderer::rasterizer::GlyphCoverage;

/// Gamma-correct linear blend for one channel, encoded back to sRGB via the LUT.
///
/// `fg` and `bg` are linear light values; `cov` is coverage (0 = transparent, 255 = opaque).
#[inline(always)]
fn blend_ch(fg: f32, bg: f32, cov: u8, lut: &[u8; 256]) -> u8 {
    let t = cov as f32 / 255.0;
    let linear = fg * t + bg * (1.0 - t);
    lut[(linear * 255.0 + 0.5).min(255.0) as usize]
}

impl CpuRenderer {
    /// Renders a string starting at `(x, y)` in physical pixels, one character per cell.
    pub(in crate::gui::renderer) fn draw_text_at(
        &mut self,
        target: &mut RenderTarget<'_>,
        x: u32,
        y: u32,
        text: &str,
        fg: Color,
    ) {
        for (ci, ch) in text.chars().enumerate() {
            let cx = x + ci as u32 * self.metrics.cell_width;
            self.draw_char(target, cx, y, ch, fg);
        }
    }

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
        // Cache hit: 1 HashMap lookup. Cache miss: rasterize, then a second lookup via entry().
        let glyph = if let Some(g) = self.glyph_cache.get(&character) {
            g
        } else {
            let Some(rasterized) = self.rasterizer.rasterize(character) else { return; };
            self.glyph_cache.entry(character).or_insert(rasterized)
        };

        let ascent = self.metrics.ascent;

        // Decode foreground to linear using the precomputed LUT.
        let fg_lr = self.srgb_to_linear[fg.r as usize];
        let fg_lg = self.srgb_to_linear[fg.g as usize];
        let fg_lb = self.srgb_to_linear[fg.b as usize];
        let lut = &self.linear_to_srgb;

        // Hoist the coverage-type dispatch above the pixel loops so the inner loop
        // is a tight, branchless arithmetic sequence on a single concrete data type.
        match &glyph.coverage {
            GlyphCoverage::Grayscale(data) => {
                for gy in 0..glyph.height as usize {
                    for gx in 0..glyph.width as usize {
                        let sx = x as i32 + glyph.left + gx as i32;
                        let sy = y as i32 + (ascent - glyph.top) + gy as i32;
                        if sx < 0 || sy < 0 || sx as usize >= target.width || sy as usize >= target.height {
                            continue;
                        }
                        let idx = sy as usize * target.width + sx as usize;
                        let bg_pixel = target.buffer[idx];
                        let bg_lr = self.srgb_to_linear[((bg_pixel >> 16) & 0xFF) as usize];
                        let bg_lg = self.srgb_to_linear[((bg_pixel >>  8) & 0xFF) as usize];
                        let bg_lb = self.srgb_to_linear[ (bg_pixel        & 0xFF) as usize];
                        let cov = data[gy * glyph.width as usize + gx];
                        target.buffer[idx] = Color {
                            r: blend_ch(fg_lr, bg_lr, cov, lut),
                            g: blend_ch(fg_lg, bg_lg, cov, lut),
                            b: blend_ch(fg_lb, bg_lb, cov, lut),
                        }.to_pixel();
                    }
                }
            }
            GlyphCoverage::Lcd(data) => {
                for gy in 0..glyph.height as usize {
                    for gx in 0..glyph.width as usize {
                        let sx = x as i32 + glyph.left + gx as i32;
                        let sy = y as i32 + (ascent - glyph.top) + gy as i32;
                        if sx < 0 || sy < 0 || sx as usize >= target.width || sy as usize >= target.height {
                            continue;
                        }
                        let idx = sy as usize * target.width + sx as usize;
                        let bg_pixel = target.buffer[idx];
                        let bg_lr = self.srgb_to_linear[((bg_pixel >> 16) & 0xFF) as usize];
                        let bg_lg = self.srgb_to_linear[((bg_pixel >>  8) & 0xFF) as usize];
                        let bg_lb = self.srgb_to_linear[ (bg_pixel        & 0xFF) as usize];
                        let [rc, gc, bc] = data[gy * glyph.width as usize + gx];
                        target.buffer[idx] = Color {
                            r: blend_ch(fg_lr, bg_lr, rc, lut),
                            g: blend_ch(fg_lg, bg_lg, gc, lut),
                            b: blend_ch(fg_lb, bg_lb, bc, lut),
                        }.to_pixel();
                    }
                }
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
