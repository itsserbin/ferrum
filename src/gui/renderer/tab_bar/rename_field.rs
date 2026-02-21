#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::RoundedShape;
use super::super::shared::{tab_math, ui_layout};
use super::super::traits::Renderer;
use super::super::types::{RenderTarget, TabSlot};
use super::super::CpuRenderer;

impl CpuRenderer {
    /// Renders the inline rename field: background, border, text with selection, and cursor.
    pub(super) fn draw_tab_rename_field(
        &mut self,
        target: &mut RenderTarget<'_>,
        slot: &TabSlot,
    ) {
        let m = self.tab_layout_metrics();
        let text_y = tab_math::tab_text_y(&m);
        let tab_padding_h = m.scaled_px(tab_math::TAB_PADDING_H);
        let rename_text = slot.tab.rename_text.unwrap_or("");
        let text_x = slot.x + tab_padding_h;
        let max_chars = tab_math::rename_field_max_chars(&m, slot.width);
        let selection_chars =
            ui_layout::rename_selection_chars(rename_text, slot.tab.rename_selection, max_chars);

        let r = tab_math::rename_field_rect(&m, slot.x, slot.width);
        self.draw_rename_background(target, &r);
        self.draw_rename_text(target, rename_text, text_x, text_y, max_chars, selection_chars);
        self.draw_rename_cursor(
            target,
            rename_text,
            slot.tab.rename_cursor,
            text_x,
            text_y,
            max_chars,
        );
    }

    /// Draws the rename field background (fill + border).
    fn draw_rename_background(
        &self,
        target: &mut RenderTarget<'_>,
        r: &tab_math::Rect,
    ) {
        let radius = self.scaled_px(6);
        self.draw_rounded_rect(
            target,
            &RoundedShape {
                x: r.x as i32,
                y: r.y as i32,
                w: r.w,
                h: r.h,
                radius,
                color: self.palette.rename_field_bg.to_pixel(),
                alpha: 245,
            },
        );
        self.draw_rounded_rect(
            target,
            &RoundedShape {
                x: r.x as i32,
                y: r.y as i32,
                w: r.w,
                h: r.h,
                radius,
                color: self.palette.rename_field_border.to_pixel(),
                alpha: 90,
            },
        );
    }

    /// Renders rename text characters with optional selection highlight.
    fn draw_rename_text(
        &mut self,
        target: &mut RenderTarget<'_>,
        rename_text: &str,
        text_x: u32,
        text_y: u32,
        max_chars: usize,
        selection_chars: Option<(usize, usize)>,
    ) {
        for (ci, ch) in rename_text.chars().take(max_chars).enumerate() {
            let cx = text_x + ci as u32 * self.metrics.cell_width;
            let selected = selection_chars.is_some_and(|(start, end)| ci >= start && ci < end);
            if selected {
                self.draw_bg(target, cx, text_y, self.palette.active_accent);
                self.draw_char(target, cx, text_y, ch, self.palette.default_bg);
            } else {
                self.draw_char(target, cx, text_y, ch, self.palette.default_fg);
            }
        }
    }

    /// Draws the blinking cursor bar in the rename field.
    fn draw_rename_cursor(
        &self,
        target: &mut RenderTarget<'_>,
        rename_text: &str,
        rename_cursor: usize,
        text_x: u32,
        text_y: u32,
        max_chars: usize,
    ) {
        let cursor_chars = rename_text
            .get(..rename_cursor)
            .map_or(0, |prefix| prefix.chars().count())
            .min(max_chars);
        let cursor_x = text_x + cursor_chars as u32 * self.metrics.cell_width;
        let cursor_w = self.scaled_px(2);
        let cursor_h = self.metrics.cell_height.saturating_sub(self.scaled_px(2));
        let cursor_y = text_y + self.scaled_px(1);
        for py in cursor_y as usize..(cursor_y + cursor_h) as usize {
            if py >= target.height {
                break;
            }
            for px in cursor_x as usize..(cursor_x + cursor_w) as usize {
                if px < target.width && py * target.width + px < target.buffer.len() {
                    let idx = py * target.width + px;
                    target.buffer[idx] = crate::gui::renderer::blend_rgb(target.buffer[idx], self.palette.tab_text_active.to_pixel(), 220);
                }
            }
        }
    }
}
