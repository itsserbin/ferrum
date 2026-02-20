#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::RenderTarget;
use super::super::TabInfo;
use super::super::cpu::primitives::RoundedShape;
use super::super::shared::overlay_layout;
use super::super::traits::Renderer;
use super::{ACTIVE_TAB_BG, INSERTION_COLOR, TAB_BORDER};
use crate::core::Color;

impl super::super::CpuRenderer {
    /// Draws the drag overlay: ghost tab at cursor X + insertion indicator.
    pub fn draw_tab_drag_overlay(
        &mut self,
        target: &mut RenderTarget<'_>,
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
            (current_x, indicator_x),
            target.width as u32,
        ) {
            Some(l) => l,
            None => return,
        };

        // Shadow (offset +2, +2, dark).
        self.draw_rounded_rect(
            target,
            &RoundedShape {
                x: layout.shadow_x,
                y: layout.shadow_y,
                w: layout.rect_w,
                h: layout.rect_h,
                radius: layout.radius,
                color: 0x000000,
                alpha: 60,
            },
        );

        // Ghost body.
        self.draw_rounded_rect(
            target,
            &RoundedShape {
                x: layout.body_x,
                y: layout.body_y,
                w: layout.rect_w,
                h: layout.rect_h,
                radius: layout.radius,
                color: ACTIVE_TAB_BG,
                alpha: 220,
            },
        );

        // Subtle border.
        self.draw_rounded_rect(
            target,
            &RoundedShape {
                x: layout.body_x,
                y: layout.body_y,
                w: layout.rect_w,
                h: layout.rect_h,
                radius: layout.radius,
                color: TAB_BORDER,
                alpha: 100,
            },
        );

        // Ghost title text.
        let buf_width = target.width;
        for (ci, ch) in layout.title_text.chars().enumerate() {
            let cx = layout.title_x + ci as i32 * self.metrics.cell_width as i32;
            if cx >= 0 && (cx as usize) < buf_width {
                self.draw_char(target, cx as u32, layout.title_y, ch, Color::DEFAULT_FG);
            }
        }

        // Smooth insertion indicator.
        let iy = layout.indicator_y as usize;
        let ih = layout.indicator_h as usize;
        for py in iy..iy + ih {
            for dx in 0..layout.indicator_w {
                let px = layout.indicator_x + dx;
                if (px as usize) < target.width && py < target.height {
                    let idx = py * target.width + px as usize;
                    if idx < target.buffer.len() {
                        target.buffer[idx] = INSERTION_COLOR;
                    }
                }
            }
        }
    }
}
