use crate::core::Color;

use super::CpuRenderer;

impl CpuRenderer {
    pub(in crate::gui::renderer) fn draw_bg(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x: u32,
        y: u32,
        color: Color,
    ) {
        let pixel = color.to_pixel();
        for dy in 0..self.cell_height as usize {
            let py = y as usize + dy;
            if py >= buf_height {
                break;
            }
            for dx in 0..self.cell_width as usize {
                let px = x as usize + dx;
                if px >= buf_width {
                    break;
                }
                buffer[py * buf_width + px] = pixel;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::gui::renderer) fn draw_char(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x: u32,
        y: u32,
        character: char,
        fg: Color,
    ) {
        if !self.glyph_cache.contains_key(&character) {
            let (metrics, bitmap) = self.font.rasterize(character, self.font_size);
            let cached = super::super::types::GlyphBitmap {
                data: bitmap,
                width: metrics.width,
                height: metrics.height,
                left: metrics.xmin,
                top: metrics.height as i32 + metrics.ymin,
            };
            self.glyph_cache.insert(character, cached);
        }
        let glyph = self.glyph_cache.get(&character).unwrap();

        for gy in 0..glyph.height {
            for gx in 0..glyph.width {
                let alpha = glyph.data[gy * glyph.width + gx];
                if alpha == 0 {
                    continue;
                }

                let sx = x as i32 + glyph.left + gx as i32;
                let sy = y as i32 + (self.ascent - glyph.top) + gy as i32;

                if sx >= 0 && sy >= 0 && (sx as usize) < buf_width && (sy as usize) < buf_height {
                    let idx = sy as usize * buf_width + sx as usize;
                    let a = alpha as u32;
                    let inv_a = 255 - a;
                    let bg_pixel = buffer[idx];
                    let bg_r = (bg_pixel >> 16) & 0xFF;
                    let bg_g = (bg_pixel >> 8) & 0xFF;
                    let bg_b = bg_pixel & 0xFF;
                    let r = (fg.r as u32 * a + bg_r * inv_a) / 255;
                    let g = (fg.g as u32 * a + bg_g * inv_a) / 255;
                    let b = (fg.b as u32 * a + bg_b * inv_a) / 255;
                    buffer[idx] = (r << 16) | (g << 8) | b;
                }
            }
        }
    }

    pub(in crate::gui::renderer) fn blend_pixel(dst: u32, src: u32, alpha: u8) -> u32 {
        if alpha == 255 {
            return src;
        }
        if alpha == 0 {
            return dst;
        }

        let a = alpha as u32;
        let inv = 255 - a;

        let dr = (dst >> 16) & 0xFF;
        let dg = (dst >> 8) & 0xFF;
        let db = dst & 0xFF;

        let sr = (src >> 16) & 0xFF;
        let sg = (src >> 8) & 0xFF;
        let sb = src & 0xFF;

        let r = (sr * a + dr * inv + 127) / 255;
        let g = (sg * a + dg * inv + 127) / 255;
        let b = (sb * a + db * inv + 127) / 255;

        (r << 16) | (g << 8) | b
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::gui::renderer) fn draw_rounded_rect(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        radius: u32,
        color: u32,
        alpha: u8,
    ) {
        if w == 0 || h == 0 || alpha == 0 || buf_width == 0 || buf_height == 0 {
            return;
        }

        let r = radius.min(w / 2).min(h / 2) as i32;
        let max_x = buf_width as i32 - 1;
        let max_y = buf_height as i32 - 1;

        for py in 0..h as i32 {
            let sy = y + py;
            if sy < 0 || sy > max_y {
                continue;
            }

            for px in 0..w as i32 {
                let sx = x + px;
                if sx < 0 || sx > max_x {
                    continue;
                }

                let coverage = Self::rounded_coverage(px, py, w as i32, h as i32, r);
                if coverage <= 0.0 {
                    continue;
                }

                let idx = sy as usize * buf_width + sx as usize;
                if idx >= buffer.len() {
                    continue;
                }
                let aa_alpha = ((alpha as f32) * coverage).round().clamp(0.0, 255.0) as u8;
                if aa_alpha == 0 {
                    continue;
                }
                buffer[idx] = Self::blend_pixel(buffer[idx], color, aa_alpha);
            }
        }
    }

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
