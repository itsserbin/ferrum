//! Scrollbar rendering and hit-testing for the GPU renderer.

use super::SCROLLBAR_MIN_THUMB;

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
        let track_height = track_bottom - track_top;
        if track_height <= 0.0 {
            return;
        }

        let total_lines = scrollback_len + grid_rows;
        let viewport_ratio = grid_rows as f32 / total_lines as f32;
        let min_thumb = self.metrics.scaled_px(SCROLLBAR_MIN_THUMB) as f32;
        let thumb_height = (viewport_ratio * track_height)
            .max(min_thumb)
            .min(track_height);

        let max_offset = scrollback_len as f32;
        let scroll_ratio = (max_offset - scroll_offset as f32) / max_offset;
        let thumb_y = track_top + scroll_ratio * (track_height - thumb_height);

        let sb_width = self.metrics.scaled_px(super::super::SCROLLBAR_WIDTH) as f32;
        let sb_margin = self.metrics.scaled_px(super::super::SCROLLBAR_MARGIN) as f32;
        let thumb_x = self.width as f32 - sb_width - sb_margin;
        let radius = self.metrics.scaled_px(3) as f32;

        let color = if hover { 0x7F849C } else { 0x6C7086 };
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
        if scrollback_len == 0 {
            return None;
        }

        let track_top =
            (self.metrics.tab_bar_height_px() + self.metrics.window_padding_px()) as f32;
        let track_bottom = buf_height as f32 - self.metrics.window_padding_px() as f32;
        let track_height = track_bottom - track_top;
        if track_height <= 0.0 {
            return None;
        }

        let total_lines = scrollback_len + grid_rows;
        let viewport_ratio = grid_rows as f32 / total_lines as f32;
        let min_thumb = self.metrics.scaled_px(SCROLLBAR_MIN_THUMB) as f32;
        let thumb_height = (viewport_ratio * track_height)
            .max(min_thumb)
            .min(track_height);

        let max_offset = scrollback_len as f32;
        let scroll_ratio = (max_offset - scroll_offset as f32) / max_offset;
        let thumb_y = track_top + scroll_ratio * (track_height - thumb_height);

        Some((thumb_y, thumb_height))
    }
}
