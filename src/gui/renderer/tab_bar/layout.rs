#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::RenderTarget;
use super::super::shared::overlay_layout;
use super::super::traits::Renderer;
use super::{ACTIVE_TAB_BG, TAB_BORDER};
use crate::core::Color;

impl super::super::CpuRenderer {
    /// Draws a small tooltip with full tab title near the pointer.
    pub fn draw_tab_tooltip(
        &mut self,
        target: &mut RenderTarget<'_>,
        mouse_pos: (f64, f64),
        title: &str,
    ) {
        let (buffer, buf_width, buf_height) = (&mut *target.buffer, target.width, target.height);
        let m = self.tab_layout_metrics();
        let layout = match overlay_layout::compute_tooltip_layout(
            title,
            mouse_pos,
            &m,
            buf_width as u32,
            buf_height as u32,
        ) {
            Some(l) => l,
            None => return,
        };

        // Background fill.
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            layout.bg_x,
            layout.bg_y,
            layout.bg_w,
            layout.bg_h,
            layout.radius,
            ACTIVE_TAB_BG,
            245,
        );
        // Subtle border.
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            layout.bg_x,
            layout.bg_y,
            layout.bg_w,
            layout.bg_h,
            layout.radius,
            TAB_BORDER,
            80,
        );

        for (ci, ch) in layout.display_text.chars().enumerate() {
            let cx = layout.text_x + ci as u32 * self.metrics.cell_width;
            self.draw_char(
                buffer,
                buf_width,
                buf_height,
                cx,
                layout.text_y,
                ch,
                Color::DEFAULT_FG,
            );
        }
    }
}
