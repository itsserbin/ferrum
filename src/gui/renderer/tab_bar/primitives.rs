#![cfg_attr(target_os = "macos", allow(dead_code))]

use crate::core::Color;

impl super::super::CpuRenderer {
    pub(in crate::gui::renderer) fn blend_rgb(dst: u32, src: u32, alpha: u8) -> u32 {
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

    /// Draws a filled circle at a given center with a given radius.
    pub(in crate::gui::renderer) fn draw_filled_circle(
        buffer: &mut [u32],
        buf_w: usize,
        cx: i32,
        cy: i32,
        radius: u32,
        color: u32,
    ) {
        if buf_w == 0 || buffer.is_empty() || radius == 0 {
            return;
        }

        let buf_h = buffer.len() / buf_w;
        if buf_h == 0 {
            return;
        }

        let r = radius as f32;
        let min_x = (cx - radius as i32 - 1).max(0);
        let max_x = (cx + radius as i32 + 1).min(buf_w as i32 - 1);
        let min_y = (cy - radius as i32 - 1).max(0);
        let max_y = (cy + radius as i32 + 1).min(buf_h as i32 - 1);

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px as f32 + 0.5 - cx as f32;
                let dy = py as f32 + 0.5 - cy as f32;
                let dist = (dx * dx + dy * dy).sqrt();

                let coverage = (r + 0.5 - dist).clamp(0.0, 1.0);
                if coverage <= 0.0 {
                    continue;
                }

                let idx = py as usize * buf_w + px as usize;
                if idx >= buffer.len() {
                    continue;
                }

                let alpha = (coverage * 255.0).round() as u8;
                buffer[idx] = Self::blend_rgb(buffer[idx], color, alpha);
            }
        }
    }

    pub(in crate::gui::renderer) fn point_in_rect(x: f64, y: f64, rect: (u32, u32, u32, u32)) -> bool {
        let (rx, ry, rw, rh) = rect;
        x >= rx as f64 && x < (rx + rw) as f64 && y >= ry as f64 && y < (ry + rh) as f64
    }

    pub(in crate::gui::renderer) fn point_to_segment_distance(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
        let vx = x1 - x0;
        let vy = y1 - y0;
        let len_sq = vx * vx + vy * vy;
        if len_sq <= f32::EPSILON {
            return ((px - x0) * (px - x0) + (py - y0) * (py - y0)).sqrt();
        }

        let t = (((px - x0) * vx + (py - y0) * vy) / len_sq).clamp(0.0, 1.0);
        let proj_x = x0 + t * vx;
        let proj_y = y0 + t * vy;
        ((px - proj_x) * (px - proj_x) + (py - proj_y) * (py - proj_y)).sqrt()
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::gui::renderer) fn draw_stroked_line(
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        thickness: f32,
        color: u32,
    ) {
        if thickness <= 0.0 || buf_width == 0 || buf_height == 0 {
            return;
        }

        let half = thickness * 0.5;
        let min_x = ((x0.min(x1) - half - 1.0).floor() as i32).max(0);
        let max_x = ((x0.max(x1) + half + 1.0).ceil() as i32).min(buf_width as i32 - 1);
        let min_y = ((y0.min(y1) - half - 1.0).floor() as i32).max(0);
        let max_y = ((y0.max(y1) + half + 1.0).ceil() as i32).min(buf_height as i32 - 1);

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let pcx = px as f32 + 0.5;
                let pcy = py as f32 + 0.5;
                let dist = Self::point_to_segment_distance(pcx, pcy, x0, y0, x1, y1);

                let coverage = (half + 0.5 - dist).clamp(0.0, 1.0);
                if coverage <= 0.0 {
                    continue;
                }

                let idx = py as usize * buf_width + px as usize;
                if idx >= buffer.len() {
                    continue;
                }
                let alpha = (coverage * 255.0).round() as u8;
                buffer[idx] = Self::blend_rgb(buffer[idx], color, alpha);
            }
        }
    }

    pub(in crate::gui::renderer) fn draw_tab_plus_icon(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        rect: (u32, u32, u32, u32),
        color: Color,
    ) {
        let (x, _y, w, h) = rect;
        let center_x = x as f32 + w as f32 * 0.5;
        let center_y = rect.1 as f32 + h as f32 * 0.5;
        let half = (w.min(h) as f32 * 0.25).clamp(2.5, 5.0);
        let thickness = (1.25_f32 * self.ui_scale() as f32).clamp(1.15, 2.2);
        let pixel = color.to_pixel();

        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x - half,
            center_y,
            center_x + half,
            center_y,
            thickness,
            pixel,
        );
        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x,
            center_y - half,
            center_x,
            center_y + half,
            thickness,
            pixel,
        );
    }

    pub(in crate::gui::renderer) fn draw_tab_close_icon(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        rect: (u32, u32, u32, u32),
        color: Color,
    ) {
        let (x, y, w, h) = rect;
        let center_x = x as f32 + w as f32 * 0.5;
        let center_y = y as f32 + h as f32 * 0.5;
        let half = (w.min(h) as f32 * 0.22).clamp(2.5, 4.5);
        let thickness = (1.25_f32 * self.ui_scale() as f32).clamp(1.15, 2.2);
        let pixel = color.to_pixel();

        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x - half,
            center_y - half,
            center_x + half,
            center_y + half,
            thickness,
            pixel,
        );
        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x + half,
            center_y - half,
            center_x - half,
            center_y + half,
            thickness,
            pixel,
        );
    }

    /// Draws a rounded rect with only the top corners rounded (bottom corners square).
    /// Used for active/hovered tab shapes that merge with the terminal below.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gui::renderer) fn draw_top_rounded_rect(
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

                // Only round the top corners; bottom corners are square.
                let coverage = Self::top_rounded_coverage(px, py, w as i32, h as i32, r);
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

    /// Coverage function for a rect with only the top two corners rounded.
    pub(in crate::gui::renderer) fn top_rounded_coverage(px: i32, py: i32, w: i32, h: i32, r: i32) -> f32 {
        if px < 0 || py < 0 || px >= w || py >= h {
            return 0.0;
        }
        if r <= 0 {
            return 1.0;
        }

        let in_tl = px < r && py < r;
        let in_tr = px >= w - r && py < r;
        // Bottom corners are NOT rounded.
        if !(in_tl || in_tr) {
            return 1.0;
        }

        let cx = if in_tl {
            r as f32 - 0.5
        } else {
            (w - r) as f32 - 0.5
        };
        let cy = r as f32 - 0.5;

        let dx = px as f32 + 0.5 - cx;
        let dy = py as f32 + 0.5 - cy;
        let rr = r as f32;
        let dist = (dx * dx + dy * dy).sqrt();
        (rr + 0.5 - dist).clamp(0.0, 1.0)
    }

    /// Draws a 1px border on the top and sides of a top-rounded rect (no bottom border).
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(in crate::gui::renderer) fn draw_top_rounded_border(
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
        if w < 2 || h < 2 || alpha == 0 || buf_width == 0 || buf_height == 0 {
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

                // Determine if this pixel is on the border (top or sides, not bottom).
                let on_top = py == 0;
                let on_left = px == 0;
                let on_right = px == w as i32 - 1;

                if !on_top && !on_left && !on_right {
                    continue;
                }

                // Skip bottom row entirely (no bottom border).
                if py >= h as i32 - 1 {
                    continue;
                }

                // Check that the pixel is inside the top-rounded shape.
                let coverage = Self::top_rounded_coverage(px, py, w as i32, h as i32, r);
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
}
