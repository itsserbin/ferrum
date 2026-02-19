//! Scrollbar rendering and hit-testing for the GPU renderer.

use super::super::shared::scrollbar_math;
use super::super::{SCROLLBAR_COLOR, SCROLLBAR_HOVER_COLOR, SCROLLBAR_MIN_THUMB};

impl super::GpuRenderer {
    pub(super) fn render_scrollbar_impl(
        &mut self,
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

        let track_top =
            (self.metrics.tab_bar_height_px() + self.metrics.window_padding_px()) as f32;
        let track_bottom = buf_height as f32 - self.metrics.window_padding_px() as f32;
        let min_thumb = self.metrics.scaled_px(SCROLLBAR_MIN_THUMB) as f32;

        let (thumb_y, thumb_height) = match scrollbar_math::scrollbar_thumb_geometry(
            track_top,
            track_bottom,
            scroll_offset,
            scrollback_len,
            grid_rows,
            min_thumb,
        ) {
            Some(v) => v,
            None => return,
        };

        let sb_width = self.metrics.scaled_px(super::super::SCROLLBAR_WIDTH) as f32;
        let sb_margin = self.metrics.scaled_px(super::super::SCROLLBAR_MARGIN) as f32;
        let thumb_x = self.width as f32 - sb_width - sb_margin;
        let radius = self.metrics.scaled_px(3) as f32;

        let color = if hover {
            SCROLLBAR_HOVER_COLOR.to_pixel()
        } else {
            SCROLLBAR_COLOR.to_pixel()
        };
        let base_alpha = 180.0 / 255.0;
        let alpha = base_alpha * opacity;

        self.push_rounded_rect(
            thumb_x,
            thumb_y,
            sb_width,
            thumb_height,
            radius,
            color,
            alpha,
        );
    }

    pub(super) fn scrollbar_thumb_bounds_impl(
        &self,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
    ) -> Option<(f32, f32)> {
        let track_top =
            (self.metrics.tab_bar_height_px() + self.metrics.window_padding_px()) as f32;
        let track_bottom = buf_height as f32 - self.metrics.window_padding_px() as f32;
        let min_thumb = self.metrics.scaled_px(SCROLLBAR_MIN_THUMB) as f32;

        scrollbar_math::scrollbar_thumb_geometry(
            track_top,
            track_bottom,
            scroll_offset,
            scrollback_len,
            grid_rows,
            min_thumb,
        )
    }
}
