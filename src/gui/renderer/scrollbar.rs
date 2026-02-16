use super::*;

/// Base alpha for the scrollbar thumb even at full opacity (semi-transparent look).
const SCROLLBAR_BASE_ALPHA: u32 = 180;

/// Minimum thumb height in base UI pixels.
const SCROLLBAR_MIN_THUMB: u32 = 20;

impl CpuRenderer {
    /// Renders an overlay scrollbar thumb with alpha blending over existing buffer content.
    ///
    /// `opacity` ranges from 0.0 (invisible) to 1.0 (fully shown).
    /// `hover` indicates whether the thumb should use the brighter hover color.
    #[allow(clippy::too_many_arguments)]
    pub fn render_scrollbar(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
        opacity: f32,
        hover: bool,
    ) {
        if scrollback_len == 0 || opacity <= 0.0 {
            return;
        }

        let track_top = (self.tab_bar_height_px() + self.window_padding_px()) as f32;
        let track_bottom = buf_height as f32 - self.window_padding_px() as f32;
        let track_height = track_bottom - track_top;
        if track_height <= 0.0 {
            return;
        }

        // Thumb dimensions.
        let total_lines = scrollback_len + grid_rows;
        let viewport_ratio = grid_rows as f32 / total_lines as f32;
        let min_thumb = self.scaled_px(SCROLLBAR_MIN_THUMB) as f32;
        let thumb_height = (viewport_ratio * track_height)
            .max(min_thumb)
            .min(track_height);

        // Thumb position. scroll_offset=0 means bottom, scrollback_len means top.
        let max_offset = scrollback_len as f32;
        let scroll_ratio = (max_offset - scroll_offset as f32) / max_offset;
        let thumb_y = track_top + scroll_ratio * (track_height - thumb_height);

        // Pixel bounds.
        let thumb_top = thumb_y.round() as usize;
        let thumb_bot = (thumb_y + thumb_height).round() as usize;
        let thumb_left = buf_width
            .saturating_sub((self.scrollbar_width_px() + self.scrollbar_margin_px()) as usize);
        let thumb_right = buf_width.saturating_sub(self.scrollbar_margin_px() as usize);

        // Color and alpha.
        let color = if hover {
            SCROLLBAR_HOVER_COLOR
        } else {
            SCROLLBAR_COLOR
        };
        let alpha = ((SCROLLBAR_BASE_ALPHA as f32 * opacity) as u32).min(255);
        if alpha == 0 {
            return;
        }
        let inv_alpha = 255 - alpha;

        // Draw with alpha blending over existing buffer content.
        for py in thumb_top..thumb_bot {
            if py >= buf_height {
                break;
            }
            for px in thumb_left..thumb_right {
                if px >= buf_width {
                    break;
                }
                let idx = py * buf_width + px;
                let bg_pixel = buffer[idx];
                let bg_r = (bg_pixel >> 16) & 0xFF;
                let bg_g = (bg_pixel >> 8) & 0xFF;
                let bg_b = bg_pixel & 0xFF;
                let r = (color.r as u32 * alpha + bg_r * inv_alpha) / 255;
                let g = (color.g as u32 * alpha + bg_g * inv_alpha) / 255;
                let b = (color.b as u32 * alpha + bg_b * inv_alpha) / 255;
                buffer[idx] = (r << 16) | (g << 8) | b;
            }
        }
    }

    /// Returns (thumb_y, thumb_height) in pixels for scrollbar hit testing.
    /// Returns `None` if there is no scrollback (scrollbar not visible).
    pub fn scrollbar_thumb_bounds(
        &self,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
    ) -> Option<(f32, f32)> {
        if scrollback_len == 0 {
            return None;
        }

        let track_top = (self.tab_bar_height_px() + self.window_padding_px()) as f32;
        let track_bottom = buf_height as f32 - self.window_padding_px() as f32;
        let track_height = track_bottom - track_top;
        if track_height <= 0.0 {
            return None;
        }

        let total_lines = scrollback_len + grid_rows;
        let viewport_ratio = grid_rows as f32 / total_lines as f32;
        let min_thumb = self.scaled_px(SCROLLBAR_MIN_THUMB) as f32;
        let thumb_height = (viewport_ratio * track_height)
            .max(min_thumb)
            .min(track_height);

        let max_offset = scrollback_len as f32;
        let scroll_ratio = (max_offset - scroll_offset as f32) / max_offset;
        let thumb_y = track_top + scroll_ratio * (track_height - thumb_height);

        Some((thumb_y, thumb_height))
    }
}
