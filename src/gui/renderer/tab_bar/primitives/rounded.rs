#![cfg_attr(target_os = "macos", allow(dead_code))]

impl super::super::super::CpuRenderer {
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
    pub(in crate::gui::renderer) fn top_rounded_coverage(
        px: i32,
        py: i32,
        w: i32,
        h: i32,
        r: i32,
    ) -> f32 {
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
