#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::shared::overlay_layout;
use super::super::traits::Renderer;
use super::super::TabInfo;
use super::{ACTIVE_TAB_BG, INSERTION_COLOR, TAB_BORDER};
use crate::core::Color;

impl super::super::CpuRenderer {
    /// Draws the drag overlay: ghost tab at cursor X + insertion indicator.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_tab_drag_overlay(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        tabs: &[TabInfo],
        source_index: usize,
        current_x: f64,
        indicator_x: f32,
    ) {
        let m = self.tab_layout_metrics();
        let layout = match overlay_layout::compute_drag_overlay_layout(
            &m,
            tabs.len(),
            source_index,
            tabs[source_index].title,
            current_x,
            indicator_x,
            buf_width as u32,
        ) {
            Some(l) => l,
            None => return,
        };

        // Shadow (offset +2, +2, dark).
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            layout.shadow_x,
            layout.shadow_y,
            layout.rect_w,
            layout.rect_h,
            layout.radius,
            0x000000,
            60,
        );

        // Ghost body.
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            layout.body_x,
            layout.body_y,
            layout.rect_w,
            layout.rect_h,
            layout.radius,
            ACTIVE_TAB_BG,
            220,
        );

        // Subtle border.
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            layout.body_x,
            layout.body_y,
            layout.rect_w,
            layout.rect_h,
            layout.radius,
            TAB_BORDER,
            100,
        );

        // Ghost title text.
        for (ci, ch) in layout.title_text.chars().enumerate() {
            let cx = layout.title_x + ci as i32 * self.cell_width as i32;
            if cx >= 0 && (cx as usize) < buf_width {
                self.draw_char(
                    buffer,
                    buf_width,
                    buf_height,
                    cx as u32,
                    layout.title_y,
                    ch,
                    Color::DEFAULT_FG,
                );
            }
        }

        // Smooth insertion indicator.
        let iy = layout.indicator_y as usize;
        let ih = layout.indicator_h as usize;
        for py in iy..iy + ih {
            for dx in 0..layout.indicator_w {
                let px = layout.indicator_x + dx;
                if (px as usize) < buf_width && py < buf_height {
                    let idx = py * buf_width + px as usize;
                    if idx < buffer.len() {
                        buffer[idx] = INSERTION_COLOR;
                    }
                }
            }
        }
    }
}
