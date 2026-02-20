use super::shared::scrollbar_math;
use super::*;
use super::{RenderTarget, ScrollbarState};
use crate::gui::pane::PaneRect;

// SCROLLBAR_BASE_ALPHA and SCROLLBAR_MIN_THUMB come from `use super::*`.

impl CpuRenderer {
    /// Renders an overlay scrollbar thumb with alpha blending over existing buffer content.
    ///
    /// `opacity` ranges from 0.0 (invisible) to 1.0 (fully shown).
    /// `hover` indicates whether the thumb should use the brighter hover color.
    pub fn render_scrollbar(
        &self,
        target: &mut RenderTarget<'_>,
        state: &ScrollbarState,
    ) {
        if state.scrollback_len == 0 || state.opacity <= 0.0 {
            return;
        }

        let (track_top, track_bottom, min_thumb) = scrollbar_math::scrollbar_track_params(
            self.tab_bar_height_px(),
            self.window_padding_px(),
            target.height,
            SCROLLBAR_MIN_THUMB,
            self.ui_scale(),
        );

        let (thumb_y, thumb_height) = match scrollbar_math::scrollbar_thumb_geometry(
            track_top,
            track_bottom,
            state.scroll_offset,
            state.scrollback_len,
            state.grid_rows,
            min_thumb,
        ) {
            Some(v) => v,
            None => return,
        };

        // Pixel bounds.
        let thumb_top = thumb_y.round() as usize;
        let thumb_bot = (thumb_y + thumb_height).round() as usize;
        let thumb_left = target
            .width
            .saturating_sub((self.scrollbar_width_px() + self.scrollbar_margin_px()) as usize);
        let thumb_right = target
            .width
            .saturating_sub(self.scrollbar_margin_px() as usize);

        // Color and alpha.
        let color = if state.hover {
            SCROLLBAR_HOVER_COLOR
        } else {
            SCROLLBAR_COLOR
        };
        let alpha = ((SCROLLBAR_BASE_ALPHA as f32 * state.opacity) as u32).min(255);
        if alpha == 0 {
            return;
        }
        let inv_alpha = 255 - alpha;

        // Draw with alpha blending over existing buffer content.
        for py in thumb_top..thumb_bot {
            if py >= target.height {
                break;
            }
            for px in thumb_left..thumb_right {
                if px >= target.width {
                    break;
                }
                let idx = py * target.width + px;
                let bg_pixel = target.buffer[idx];
                let bg_r = (bg_pixel >> 16) & 0xFF;
                let bg_g = (bg_pixel >> 8) & 0xFF;
                let bg_b = bg_pixel & 0xFF;
                let r = (color.r as u32 * alpha + bg_r * inv_alpha) / 255;
                let g = (color.g as u32 * alpha + bg_g * inv_alpha) / 255;
                let b = (color.b as u32 * alpha + bg_b * inv_alpha) / 255;
                target.buffer[idx] = (r << 16) | (g << 8) | b;
            }
        }
    }

    /// Renders the scrollbar within a pane sub-rectangle.
    pub fn render_scrollbar_in_rect(
        &self,
        target: &mut RenderTarget<'_>,
        state: &ScrollbarState,
        rect: PaneRect,
    ) {
        if state.scrollback_len == 0 || state.opacity <= 0.0 {
            return;
        }

        // Compute track within the pane rect.
        let track_top = rect.y as f32;
        let track_bottom = (rect.y + rect.height) as f32;
        let min_thumb = (SCROLLBAR_MIN_THUMB as f64 * self.ui_scale()) as f32;

        let (thumb_y, thumb_height) = match scrollbar_math::scrollbar_thumb_geometry(
            track_top,
            track_bottom,
            state.scroll_offset,
            state.scrollback_len,
            state.grid_rows,
            min_thumb,
        ) {
            Some(v) => v,
            None => return,
        };

        let thumb_top = thumb_y.round() as usize;
        let thumb_bot = (thumb_y + thumb_height).round() as usize;
        let rect_right = (rect.x + rect.width) as usize;
        let thumb_left = rect_right
            .saturating_sub((self.scrollbar_width_px() + self.scrollbar_margin_px()) as usize);
        let thumb_right = rect_right.saturating_sub(self.scrollbar_margin_px() as usize);

        let color = if state.hover {
            SCROLLBAR_HOVER_COLOR
        } else {
            SCROLLBAR_COLOR
        };
        let alpha = ((SCROLLBAR_BASE_ALPHA as f32 * state.opacity) as u32).min(255);
        if alpha == 0 {
            return;
        }
        let inv_alpha = 255 - alpha;

        for py in thumb_top..thumb_bot {
            if py >= target.height {
                break;
            }
            for px in thumb_left..thumb_right {
                if px >= target.width {
                    break;
                }
                let idx = py * target.width + px;
                if idx < target.buffer.len() {
                    let bg_pixel = target.buffer[idx];
                    let bg_r = (bg_pixel >> 16) & 0xFF;
                    let bg_g = (bg_pixel >> 8) & 0xFF;
                    let bg_b = bg_pixel & 0xFF;
                    let r = (color.r as u32 * alpha + bg_r * inv_alpha) / 255;
                    let g = (color.g as u32 * alpha + bg_g * inv_alpha) / 255;
                    let b = (color.b as u32 * alpha + bg_b * inv_alpha) / 255;
                    target.buffer[idx] = (r << 16) | (g << 8) | b;
                }
            }
        }
    }
}
