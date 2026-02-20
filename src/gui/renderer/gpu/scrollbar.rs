//! Scrollbar rendering and hit-testing for the GPU renderer.

use super::super::shared::scrollbar_math;
use super::super::types::{RoundedRectCmd, ScrollbarState};
use super::super::{SCROLLBAR_COLOR, SCROLLBAR_HOVER_COLOR, SCROLLBAR_MIN_THUMB};
use crate::gui::pane::PaneRect;

impl super::GpuRenderer {
    pub(super) fn render_scrollbar_impl(
        &mut self,
        buf_height: usize,
        state: &ScrollbarState,
    ) {
        if state.scrollback_len == 0 || state.opacity <= 0.0 {
            return;
        }

        let (track_top, track_bottom, min_thumb) = scrollbar_math::scrollbar_track_params(
            self.metrics.tab_bar_height_px(),
            self.metrics.window_padding_px(),
            buf_height,
            SCROLLBAR_MIN_THUMB,
            self.metrics.ui_scale,
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

        let sb_width = self.metrics.scaled_px(super::super::SCROLLBAR_WIDTH) as f32;
        let sb_margin = self.metrics.scaled_px(super::super::SCROLLBAR_MARGIN) as f32;
        let thumb_x = self.width as f32 - sb_width - sb_margin;
        let radius = self.metrics.scaled_px(3) as f32;

        let color = if state.hover {
            SCROLLBAR_HOVER_COLOR.to_pixel()
        } else {
            SCROLLBAR_COLOR.to_pixel()
        };
        let base_alpha = super::super::SCROLLBAR_BASE_ALPHA as f32 / 255.0;
        let alpha = base_alpha * state.opacity;

        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x: thumb_x, y: thumb_y, w: sb_width, h: thumb_height, radius,
            color, opacity: alpha,
        });
    }

    pub(super) fn render_scrollbar_in_rect_impl(
        &mut self,
        state: &ScrollbarState,
        rect: PaneRect,
    ) {
        if state.scrollback_len == 0 || state.opacity <= 0.0 {
            return;
        }

        let track_top = rect.y as f32;
        let track_bottom = (rect.y + rect.height) as f32;
        let min_thumb = (SCROLLBAR_MIN_THUMB as f64 * self.metrics.ui_scale) as f32;

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

        let sb_width = self.metrics.scaled_px(super::super::SCROLLBAR_WIDTH) as f32;
        let sb_margin = self.metrics.scaled_px(super::super::SCROLLBAR_MARGIN) as f32;
        let rect_right = (rect.x + rect.width) as f32;
        let thumb_x = rect_right - sb_width - sb_margin;
        let radius = self.metrics.scaled_px(3) as f32;

        let color = if state.hover {
            SCROLLBAR_HOVER_COLOR.to_pixel()
        } else {
            SCROLLBAR_COLOR.to_pixel()
        };
        let base_alpha = super::super::SCROLLBAR_BASE_ALPHA as f32 / 255.0;
        let alpha = base_alpha * state.opacity;

        self.push_rounded_rect_cmd(&RoundedRectCmd {
            x: thumb_x, y: thumb_y, w: sb_width, h: thumb_height, radius,
            color, opacity: alpha,
        });
    }
}
