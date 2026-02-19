#![cfg_attr(target_os = "macos", allow(dead_code))]

use crate::core::Color;

use super::super::shared::tab_math::{self, TabLayoutMetrics};
use super::super::{CpuRenderer, SECURITY_ACCENT, TabInfo};
use super::{CLOSE_HOVER_BG_COLOR, TAB_TEXT_ACTIVE, TAB_TEXT_INACTIVE};

impl CpuRenderer {
    /// Renders a tab number (1-based) in overflow/compressed mode.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_tab_number(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        tab_index: usize,
        tab: &TabInfo,
        tab_x: u32,
        tw: u32,
        tab_bar_height: u32,
        is_hovered: bool,
    ) {
        let m = TabLayoutMetrics {
            cell_width: self.cell_width,
            cell_height: self.cell_height,
            ui_scale: self.ui_scale(),
            tab_bar_height,
        };
        let text_y = tab_math::tab_text_y(&m);
        let fg = if tab.is_active {
            Color::from_pixel(TAB_TEXT_ACTIVE)
        } else {
            Color::from_pixel(TAB_TEXT_INACTIVE)
        };

        let number_str = (tab_index + 1).to_string();
        let show_close =
            tab_math::should_show_close_button(tab.is_active, is_hovered, tab.hover_progress);
        let close_reserved = if show_close {
            tab_math::close_button_reserved_width(&m)
        } else {
            0
        };
        let text_w = number_str.len() as u32 * self.cell_width;
        let text_x = tab_x + (tw.saturating_sub(text_w + close_reserved)) / 2;

        for (ci, ch) in number_str.chars().enumerate() {
            let cx = text_x + ci as u32 * self.cell_width;
            self.draw_char_at(buffer, buf_width, bar_h, cx, text_y, ch, fg);
        }

        if show_close {
            self.draw_close_button(
                buffer,
                buf_width,
                bar_h,
                tab_index,
                tw,
                tab.close_hover_progress,
            );
        }
    }

    /// Renders normal tab content: title, security badge, and close button.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_tab_content(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        tab_index: usize,
        tab: &TabInfo,
        tab_count: usize,
        tab_x: u32,
        tw: u32,
        tab_bar_height: u32,
        is_hovered: bool,
    ) {
        let m = TabLayoutMetrics {
            cell_width: self.cell_width,
            cell_height: self.cell_height,
            ui_scale: self.ui_scale(),
            tab_bar_height,
        };
        let text_y = tab_math::tab_text_y(&m);
        let tab_padding_h = m.scaled_px(tab_math::TAB_PADDING_H);
        let fg = if tab.is_active {
            Color::from_pixel(TAB_TEXT_ACTIVE)
        } else {
            Color::from_pixel(TAB_TEXT_INACTIVE)
        };

        let show_close =
            tab_math::should_show_close_button(tab.is_active, is_hovered, tab.hover_progress);
        let max_chars = tab_math::tab_title_max_chars(&m, tw, show_close, tab.security_count);

        self.draw_tab_title(
            buffer,
            buf_width,
            bar_h,
            tab,
            tab_x,
            tab_padding_h,
            text_y,
            fg,
            max_chars,
        );

        self.draw_security_badge(buffer, buf_width, bar_h, tab_index, tab, tab_count, text_y);

        if show_close {
            self.draw_close_button(
                buffer,
                buf_width,
                bar_h,
                tab_index,
                tw,
                tab.close_hover_progress,
            );
        }
    }

    /// Renders the tab title text with truncation.
    #[allow(clippy::too_many_arguments)]
    fn draw_tab_title(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        tab: &TabInfo,
        tab_x: u32,
        tab_padding_h: u32,
        text_y: u32,
        fg: Color,
        max_chars: usize,
    ) {
        let title: String = tab.title.chars().take(max_chars).collect();
        let text_x = tab_x + tab_padding_h;

        for (ci, ch) in title.chars().enumerate() {
            let cx = text_x + ci as u32 * self.cell_width;
            self.draw_char_at(buffer, buf_width, bar_h, cx, text_y, ch, fg);
        }
    }

    /// Renders the security badge icon and optional count text.
    #[allow(clippy::too_many_arguments)]
    fn draw_security_badge(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        tab_index: usize,
        tab: &TabInfo,
        tab_count: usize,
        text_y: u32,
    ) {
        if let Some((sx, sy, sw, _)) =
            self.security_badge_rect(tab_index, tab_count, buf_width as u32, tab.security_count)
        {
            self.draw_security_shield_icon(buffer, buf_width, bar_h, sx, sy, sw, SECURITY_ACCENT);
            if tab.security_count > 1 {
                let count_text = tab.security_count.min(99).to_string();
                let count_x = sx + sw + self.scaled_px(2);
                for (ci, ch) in count_text.chars().enumerate() {
                    let cx = count_x + ci as u32 * self.cell_width;
                    self.draw_char_at(buffer, buf_width, bar_h, cx, text_y, ch, SECURITY_ACCENT);
                }
            }
        }
    }

    /// Draws the close button with a circular hover effect.
    #[allow(clippy::too_many_arguments)]
    fn draw_close_button(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        tab_index: usize,
        tw: u32,
        hover_progress: f32,
    ) {
        let (cx, cy, cw, ch) = self.close_button_rect(tab_index, tw);
        let hover_t = hover_progress.clamp(0.0, 1.0);
        if hover_t > 0.01 {
            let circle_r = cw.min(ch) / 2;
            let circle_cx = (cx + cw / 2) as i32;
            let circle_cy = (cy + ch / 2) as i32;
            let alpha = (90.0 + hover_t * 125.0).round().clamp(0.0, 255.0) as u8;
            Self::draw_filled_circle(
                buffer,
                buf_width,
                circle_cx,
                circle_cy,
                circle_r,
                CLOSE_HOVER_BG_COLOR,
                alpha,
            );
        }

        let active_mix = (hover_t * 175.0).round().clamp(0.0, 255.0) as u8;
        let close_fg = Color::from_pixel(Self::blend_rgb(
            TAB_TEXT_INACTIVE,
            TAB_TEXT_ACTIVE,
            active_mix,
        ));
        self.draw_tab_close_icon(buffer, buf_width, buf_height, (cx, cy, cw, ch), close_fg);
    }
}
