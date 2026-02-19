#![cfg_attr(target_os = "macos", allow(dead_code))]

use crate::core::Color;

use super::super::shared::tab_math;
use super::super::{ACTIVE_ACCENT, CpuRenderer, TabInfo};
use super::{RENAME_FIELD_BG, RENAME_FIELD_BORDER, TAB_TEXT_ACTIVE};

impl CpuRenderer {
    /// Renders the inline rename field: background, border, text with selection, and cursor.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_tab_rename_field(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        tab: &TabInfo,
        tab_x: u32,
        tw: u32,
        _tab_bar_height: u32,
    ) {
        let m = self.tab_layout_metrics();
        let text_y = tab_math::tab_text_y(&m);
        let tab_padding_h = m.scaled_px(tab_math::TAB_PADDING_H);
        let rename_text = tab.rename_text.unwrap_or("");
        let text_x = tab_x + tab_padding_h;
        let max_chars = tab_math::rename_field_max_chars(&m, tw);
        let selection_chars = tab.rename_selection.and_then(|(start, end)| {
            if start >= end {
                return None;
            }
            let start_chars = rename_text
                .get(..start)
                .map_or(0, |prefix| prefix.chars().count());
            let end_chars = rename_text
                .get(..end)
                .map_or(start_chars, |prefix| prefix.chars().count());
            Some((start_chars.min(max_chars), end_chars.min(max_chars)))
        });

        let r = tab_math::rename_field_rect(&m, tab_x, tw);
        self.draw_rename_background(buffer, buf_width, bar_h, &r);
        self.draw_rename_text(
            buffer,
            buf_width,
            bar_h,
            rename_text,
            text_x,
            text_y,
            max_chars,
            selection_chars,
        );
        self.draw_rename_cursor(
            buffer,
            buf_width,
            bar_h,
            rename_text,
            tab.rename_cursor,
            text_x,
            text_y,
            max_chars,
        );
    }

    /// Draws the rename field background (fill + border).
    fn draw_rename_background(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        r: &tab_math::Rect,
    ) {
        self.draw_rounded_rect(
            buffer,
            buf_width,
            bar_h,
            r.x as i32,
            r.y as i32,
            r.w,
            r.h,
            self.scaled_px(6),
            RENAME_FIELD_BG,
            245,
        );
        self.draw_rounded_rect(
            buffer,
            buf_width,
            bar_h,
            r.x as i32,
            r.y as i32,
            r.w,
            r.h,
            self.scaled_px(6),
            RENAME_FIELD_BORDER,
            90,
        );
    }

    /// Renders rename text characters with optional selection highlight.
    #[allow(clippy::too_many_arguments)]
    fn draw_rename_text(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        rename_text: &str,
        text_x: u32,
        text_y: u32,
        max_chars: usize,
        selection_chars: Option<(usize, usize)>,
    ) {
        for (ci, ch) in rename_text.chars().take(max_chars).enumerate() {
            let cx = text_x + ci as u32 * self.cell_width;
            let selected = selection_chars.is_some_and(|(start, end)| ci >= start && ci < end);
            if selected {
                self.draw_bg(buffer, buf_width, bar_h, cx, text_y, ACTIVE_ACCENT);
                self.draw_char(buffer, buf_width, bar_h, cx, text_y, ch, Color::DEFAULT_BG);
            } else {
                self.draw_char(buffer, buf_width, bar_h, cx, text_y, ch, Color::DEFAULT_FG);
            }
        }
    }

    /// Draws the blinking cursor bar in the rename field.
    #[allow(clippy::too_many_arguments)]
    fn draw_rename_cursor(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
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
        let cursor_x = text_x + cursor_chars as u32 * self.cell_width;
        let cursor_w = self.scaled_px(2);
        let cursor_h = self.cell_height.saturating_sub(self.scaled_px(2));
        let cursor_y = text_y + self.scaled_px(1);
        for py in cursor_y as usize..(cursor_y + cursor_h) as usize {
            if py >= bar_h {
                break;
            }
            for px in cursor_x as usize..(cursor_x + cursor_w) as usize {
                if px < buf_width && py * buf_width + px < buffer.len() {
                    let idx = py * buf_width + px;
                    buffer[idx] = Self::blend_pixel(buffer[idx], TAB_TEXT_ACTIVE, 220);
                }
            }
        }
    }
}
