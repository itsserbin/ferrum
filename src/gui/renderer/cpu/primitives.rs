use crate::core::Color;

use super::super::types::{FlatRectCmd, RenderTarget, RoundedRectCmd, RoundedShape};
use super::CpuRenderer;

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
        use std::collections::hash_map::Entry;
        let glyph_entry = self.glyph_cache.entry(character);
        let glyph = match glyph_entry {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let is_primary = self.font.has_glyph(character);
                let font = if is_primary {
                    &self.font
                } else {
                    self.fallback_fonts
                        .iter()
                        .find(|f| f.has_glyph(character))
                        .unwrap_or(&self.font)
                };
                let font_size = if is_primary {
                    self.metrics.font_size
                } else {
                    // Scale down fallback glyphs that exceed cell width.
                    let m = font.metrics(character, self.metrics.font_size);
                    if m.advance_width > self.metrics.cell_width as f32 {
                        self.metrics.font_size * (self.metrics.cell_width as f32 / m.advance_width)
                    } else {
                        self.metrics.font_size
                    }
                };
                let (metrics, bitmap) = font.rasterize(character, font_size);
                entry.insert(super::super::types::GlyphBitmap {
                    data: bitmap,
                    width: metrics.width,
                    height: metrics.height,
                    left: metrics.xmin,
                    top: metrics.height as i32 + metrics.ymin,
                })
            }
        };

        for gy in 0..glyph.height {
            for gx in 0..glyph.width {
                let alpha = glyph.data[gy * glyph.width + gx];
                if alpha == 0 {
                    continue;
                }

                let sx = x as i32 + glyph.left + gx as i32;
                let sy = y as i32 + (self.metrics.ascent - glyph.top) + gy as i32;

                if sx >= 0
                    && sy >= 0
                    && (sx as usize) < target.width
                    && (sy as usize) < target.height
                {
                    let idx = sy as usize * target.width + sx as usize;
                    let a = alpha as u32;
                    let inv_a = 255 - a;
                    let bg_pixel = target.buffer[idx];
                    let bg_r = (bg_pixel >> 16) & 0xFF;
                    let bg_g = (bg_pixel >> 8) & 0xFF;
                    let bg_b = bg_pixel & 0xFF;
                    let r = (fg.r as u32 * a + bg_r * inv_a) / 255;
                    let g = (fg.g as u32 * a + bg_g * inv_a) / 255;
                    let b = (fg.b as u32 * a + bg_b * inv_a) / 255;
                    target.buffer[idx] = (r << 16) | (g << 8) | b;
                }
            }
        }
    }

    pub(in crate::gui::renderer) fn draw_rounded_rect(
        &self,
        target: &mut RenderTarget<'_>,
        shape: &RoundedShape,
    ) {
        Self::draw_rounded_impl(target, shape, Self::rounded_coverage);
    }

    /// Shared pixel iteration for rounded rectangle drawing.
    /// The `coverage_fn` determines which corners are rounded.
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

    /// Draws a rounded rectangle from a layout command.
    pub(in crate::gui::renderer) fn draw_rounded_rect_cmd(
        &self,
        target: &mut RenderTarget<'_>,
        cmd: &RoundedRectCmd,
    ) {
        let alpha = (cmd.opacity * 255.0).round().clamp(0.0, 255.0) as u8;
        let shape = RoundedShape {
            x: cmd.x as i32,
            y: cmd.y as i32,
            w: cmd.w as u32,
            h: cmd.h as u32,
            radius: cmd.radius as u32,
            color: cmd.color,
            alpha,
        };
        self.draw_rounded_rect(target, &shape);
    }

    /// Draws a flat (non-rounded) rectangle from a layout command using per-pixel blending.
    pub(in crate::gui::renderer) fn draw_flat_rect_cmd(
        &self,
        target: &mut RenderTarget<'_>,
        cmd: &FlatRectCmd,
    ) {
        let alpha = (cmd.opacity * 255.0).round().clamp(0.0, 255.0) as u8;
        if alpha == 0 {
            return;
        }
        let x = cmd.x as usize;
        let y = cmd.y as usize;
        let w = cmd.w as usize;
        let h = cmd.h as usize;
        for dy in 0..h {
            let py = y + dy;
            if py >= target.height {
                break;
            }
            for dx in 0..w {
                let px = x + dx;
                if px >= target.width {
                    break;
                }
                let idx = py * target.width + px;
                if idx < target.buffer.len() {
                    target.buffer[idx] = crate::gui::renderer::blend_rgb(target.buffer[idx], cmd.color, alpha);
                }
            }
        }
    }
}
