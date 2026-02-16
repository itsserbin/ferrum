#![cfg_attr(target_os = "macos", allow(dead_code))]

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
        let tab_count = tabs.len();
        if source_index >= tab_count {
            return;
        }
        let tw = self.tab_width(tab_count, buf_width as u32);
        let tab_bar_height = self.tab_bar_height_px();
        let bar_h = tab_bar_height as usize;

        // Ghost tab: rounded rect with shadow + subtle border.
        let ghost_x = (current_x - tw as f64 / 2.0).round() as i32;
        let ghost_y = self.scaled_px(2) as i32;
        let ghost_h = tab_bar_height - self.scaled_px(4);
        let ghost_radius = self.scaled_px(6);

        // Shadow (offset +2, +2, slightly larger, dark).
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            ghost_x + 2,
            ghost_y + 2,
            tw,
            ghost_h,
            ghost_radius,
            0x000000,
            60,
        );

        // Ghost body.
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            ghost_x,
            ghost_y,
            tw,
            ghost_h,
            ghost_radius,
            ACTIVE_TAB_BG,
            220,
        );

        // Subtle border.
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            ghost_x,
            ghost_y,
            tw,
            ghost_h,
            ghost_radius,
            TAB_BORDER,
            100,
        );

        // Ghost title text.
        let tab = &tabs[source_index];
        let text_y = (tab_bar_height.saturating_sub(self.cell_height)) / 2 + self.scaled_px(1);
        let use_numbers = self.should_show_number(tw);
        let label: String = if use_numbers {
            (source_index + 1).to_string()
        } else {
            let pad = self.scaled_px(14);
            let max = (tw.saturating_sub(pad * 2) / self.cell_width) as usize;
            tab.title.chars().take(max).collect()
        };
        let lw = label.chars().count() as u32 * self.cell_width;
        let tx = ghost_x + ((tw as i32 - lw as i32) / 2).max(4);
        for (ci, ch) in label.chars().enumerate() {
            let cx = tx + ci as i32 * self.cell_width as i32;
            if cx >= 0 && (cx as usize) < buf_width {
                self.draw_char_at(
                    buffer,
                    buf_width,
                    buf_height,
                    cx as u32,
                    text_y,
                    ch,
                    Color::DEFAULT_FG,
                );
            }
        }

        // Smooth insertion indicator: 2px vertical line at lerped indicator_x.
        let ix = indicator_x.round() as u32;
        let indicator_y_pad = self.scaled_px(4) as usize;
        for py in indicator_y_pad..bar_h.saturating_sub(indicator_y_pad) {
            for dx in 0..self.scaled_px(2) {
                let px = ix + dx;
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
