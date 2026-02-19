#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::shared::tab_math;
use super::super::{
    ACTIVE_TAB_BG, BAR_BG, INACTIVE_TAB_HOVER, RENAME_FIELD_BG, RENAME_FIELD_BORDER,
    RENAME_SELECTION_BG, SECURITY_ACCENT, TAB_TEXT_ACTIVE, TAB_TEXT_INACTIVE, TabInfo,
};

impl super::GpuRenderer {
    /// Draws the tab bar background rectangle.
    pub(super) fn tab_bar_background_commands(&mut self, buf_width: u32) {
        let tab_bar_h = self.metrics.tab_bar_height_px() as f32;
        self.push_rounded_rect(
            0.0,
            0.0,
            buf_width as f32,
            tab_bar_h,
            self.metrics.scaled_px(10) as f32,
            BAR_BG,
            1.0,
        );
    }

    /// Draws the background for a single tab (active, hovered, or nothing).
    pub(super) fn tab_background_commands(&mut self, tab: &TabInfo, tab_x: f32, tw: u32) {
        let tab_bar_h = self.metrics.tab_bar_height_px() as f32;
        let hover_t = tab.hover_progress.clamp(0.0, 1.0);

        if tab.is_active {
            self.push_rect(tab_x, 0.0, tw as f32, tab_bar_h, ACTIVE_TAB_BG, 1.0);
        } else if hover_t > 0.01 {
            self.push_rect(
                tab_x,
                0.0,
                tw as f32,
                tab_bar_h,
                INACTIVE_TAB_HOVER,
                hover_t.min(1.0),
            );
        }
    }

    /// Draws the rename-mode UI for a tab: field background, border, text,
    /// selection highlight, and cursor.
    pub(super) fn tab_rename_commands(&mut self, tab: &TabInfo, tab_x: f32, tw: u32, text_y: u32) {
        let tab_padding_h = self.metrics.scaled_px(tab_math::TAB_PADDING_H);
        let rename_text = tab.rename_text.unwrap_or("");
        let text_x = tab_x + tab_padding_h as f32;
        let m = self.tab_layout_metrics();
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

        // Rename field background and border.
        let r = tab_math::rename_field_rect(&m, tab_x.round() as u32, tw);
        let field_x = tab_x + tab_padding_h.saturating_sub(self.metrics.scaled_px(3)) as f32;
        let field_y = r.y as f32;
        let field_w = r.w as f32;
        let field_h = r.h as f32;
        let radius = self.metrics.scaled_px(6) as f32;
        self.push_rounded_rect(
            field_x,
            field_y,
            field_w,
            field_h,
            radius,
            RENAME_FIELD_BG,
            0.96,
        );
        self.push_rounded_rect(
            field_x,
            field_y,
            field_w,
            field_h,
            radius,
            RENAME_FIELD_BORDER,
            0.35,
        );

        // Rename text characters with optional selection highlight.
        for (ci, ch) in rename_text.chars().take(max_chars).enumerate() {
            let cx = text_x + ci as f32 * self.metrics.cell_width as f32;
            let selected = selection_chars.is_some_and(|(start, end)| ci >= start && ci < end);
            if selected {
                self.push_rect(
                    cx,
                    text_y as f32,
                    self.metrics.cell_width as f32,
                    self.metrics.cell_height as f32,
                    RENAME_SELECTION_BG,
                    0.94,
                );
                self.push_text(cx, text_y as f32, &ch.to_string(), BAR_BG, 1.0);
            } else {
                self.push_text(cx, text_y as f32, &ch.to_string(), TAB_TEXT_ACTIVE, 1.0);
            }
        }

        // Rename cursor.
        let cursor_chars = rename_text
            .get(..tab.rename_cursor)
            .map_or(0, |prefix| prefix.chars().count())
            .min(max_chars);
        let cursor_x = text_x + cursor_chars as f32 * self.metrics.cell_width as f32;
        self.push_rect(
            cursor_x,
            (text_y + self.metrics.scaled_px(1)) as f32,
            self.metrics.scaled_px(2) as f32,
            self.metrics
                .cell_height
                .saturating_sub(self.metrics.scaled_px(2)) as f32,
            TAB_TEXT_ACTIVE,
            0.9,
        );
    }

    /// Draws a tab in number mode (narrow tabs): centered number + optional close button.
    pub(super) fn tab_number_commands(
        &mut self,
        tab_index: usize,
        tab: &TabInfo,
        tab_x: f32,
        tw: u32,
        text_y: u32,
        is_hovered: bool,
    ) {
        let fg_color = if tab.is_active {
            TAB_TEXT_ACTIVE
        } else {
            TAB_TEXT_INACTIVE
        };

        let m = self.tab_layout_metrics();
        let number_str = (tab_index + 1).to_string();
        let show_close =
            tab_math::should_show_close_button(tab.is_active, is_hovered, tab.hover_progress);
        let close_reserved = if show_close {
            tab_math::close_button_reserved_width(&m)
        } else {
            0
        };
        let text_w = number_str.len() as u32 * self.metrics.cell_width;
        let tx = tab_x + (tw.saturating_sub(text_w + close_reserved)) as f32 / 2.0;
        self.push_text(tx, text_y as f32, &number_str, fg_color, 1.0);

        if show_close {
            self.draw_close_button_commands(tab_index, tw, tab.close_hover_progress);
        }
    }

    /// Draws a tab in normal mode: title text + optional close button.
    /// Delegates to title, security badge, and close button helpers.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn tab_content_commands(
        &mut self,
        tab_index: usize,
        tab: &TabInfo,
        tab_count: usize,
        buf_width: u32,
        tab_x: f32,
        tw: u32,
        text_y: u32,
        is_hovered: bool,
    ) {
        let show_close =
            tab_math::should_show_close_button(tab.is_active, is_hovered, tab.hover_progress);

        self.tab_title_commands(tab, tab_x, tw, text_y, show_close);
        self.tab_security_badge_commands(tab_index, tab, tab_count, buf_width, text_y);

        if show_close {
            self.draw_close_button_commands(tab_index, tw, tab.close_hover_progress);
        }
    }

    /// Draws the tab title text, truncated to fit the available space.
    fn tab_title_commands(
        &mut self,
        tab: &TabInfo,
        tab_x: f32,
        tw: u32,
        text_y: u32,
        show_close: bool,
    ) {
        let fg_color = if tab.is_active {
            TAB_TEXT_ACTIVE
        } else {
            TAB_TEXT_INACTIVE
        };

        let m = self.tab_layout_metrics();
        let max_chars = tab_math::tab_title_max_chars(&m, tw, show_close, tab.security_count);
        let tab_padding_h = self.metrics.scaled_px(tab_math::TAB_PADDING_H);
        let title: String = tab.title.chars().take(max_chars).collect();
        let tx = tab_x + tab_padding_h as f32;
        self.push_text(tx, text_y as f32, &title, fg_color, 1.0);
    }

    /// Draws the security badge icon and optional numeric count.
    fn tab_security_badge_commands(
        &mut self,
        tab_index: usize,
        tab: &TabInfo,
        tab_count: usize,
        buf_width: u32,
        text_y: u32,
    ) {
        let Some((sx, sy, sw, _)) =
            self.security_badge_rect_val(tab_index, tab_count, buf_width, tab.security_count)
        else {
            return;
        };

        let color = SECURITY_ACCENT.to_pixel();
        let mid = sw / 2;
        let top_third = (sw / 3).max(1);
        let bottom_start = (sw * 2 / 3).max(top_third + 1);

        for dy in 0..sw {
            let half_span = if dy < top_third {
                1 + dy / 2
            } else if dy < bottom_start {
                mid.saturating_sub(1).max(1)
            } else {
                let progress = dy - bottom_start;
                let denom = (sw - bottom_start).max(1);
                let shrink = progress * mid.saturating_sub(1) / denom;
                mid.saturating_sub(shrink).max(1)
            };

            let left = mid.saturating_sub(half_span);
            let right = (mid + half_span).min(sw.saturating_sub(1));
            let row_x = sx + left;
            let row_w = right.saturating_sub(left) + 1;
            self.push_rect(
                row_x as f32,
                (sy + dy) as f32,
                row_w as f32,
                1.0,
                color,
                1.0,
            );
        }

        if tab.security_count > 1 {
            let count_text = tab.security_count.min(99).to_string();
            let count_x = sx + sw + self.metrics.scaled_px(2);
            self.push_text(count_x as f32, text_y as f32, &count_text, color, 1.0);
        }
    }
}
