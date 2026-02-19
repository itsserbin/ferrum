#![cfg_attr(target_os = "macos", allow(dead_code))]

impl super::super::super::CpuRenderer {
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
        alpha: u8,
    ) {
        if buf_w == 0 || buffer.is_empty() || radius == 0 || alpha == 0 {
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

                let aa_alpha = (coverage * alpha as f32).round().clamp(0.0, 255.0) as u8;
                buffer[idx] = Self::blend_rgb(buffer[idx], color, aa_alpha);
            }
        }
    }

    pub(in crate::gui::renderer) fn point_to_segment_distance(
        px: f32,
        py: f32,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
    ) -> f32 {
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
}
