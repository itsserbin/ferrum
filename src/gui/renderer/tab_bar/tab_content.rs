#![cfg_attr(target_os = "macos", allow(dead_code))]

use crate::core::Color;

use super::super::shared::{tab_math, ui_layout};
use super::super::traits::Renderer;
use super::super::types::{RenderTarget, TabSlot};
use super::super::{CpuRenderer, SECURITY_ACCENT};
use super::{CLOSE_HOVER_BG_COLOR, TAB_TEXT_ACTIVE, TAB_TEXT_INACTIVE};

impl CpuRenderer {
    /// Renders a tab number (1-based) in overflow/compressed mode.
    pub(super) fn draw_tab_number(
        &mut self,
        target: &mut RenderTarget<'_>,
        slot: &TabSlot,
    ) {
        let m = self.tab_layout_metrics();
        let text_y = tab_math::tab_text_y(&m);
        let fg = if slot.tab.is_active {
            Color::from_pixel(TAB_TEXT_ACTIVE)
        } else {
            Color::from_pixel(TAB_TEXT_INACTIVE)
        };

        let number_str = (slot.index + 1).to_string();
        let show_close = tab_math::should_show_close_button(
            slot.tab.is_active,
            slot.is_hovered,
            slot.tab.hover_progress,
        );
        let close_reserved = if show_close {
            tab_math::close_button_reserved_width(&m)
        } else {
            0
        };
        let text_w = number_str.len() as u32 * self.metrics.cell_width;
        let text_x = slot.x + (slot.width.saturating_sub(text_w + close_reserved)) / 2;

        for (ci, ch) in number_str.chars().enumerate() {
            let cx = text_x + ci as u32 * self.metrics.cell_width;
            self.draw_char(target, cx, text_y, ch, fg);
        }

        if show_close {
            self.draw_close_button(
                target,
                slot.index,
                slot.width,
                slot.tab.close_hover_progress,
            );
        }
    }

    /// Renders normal tab content: title, security badge, and close button.
    pub(super) fn draw_tab_content(
        &mut self,
        target: &mut RenderTarget<'_>,
        slot: &TabSlot,
        tab_count: usize,
    ) {
        let m = self.tab_layout_metrics();
        let text_y = tab_math::tab_text_y(&m);
        let tab_padding_h = m.scaled_px(tab_math::TAB_PADDING_H);
        let fg = if slot.tab.is_active {
            Color::from_pixel(TAB_TEXT_ACTIVE)
        } else {
            Color::from_pixel(TAB_TEXT_INACTIVE)
        };

        let show_close = tab_math::should_show_close_button(
            slot.tab.is_active,
            slot.is_hovered,
            slot.tab.hover_progress,
        );
        let max_chars = tab_math::tab_title_max_chars(
            &m,
            slot.width,
            show_close,
            slot.tab.security_count,
        );

        let text_x = slot.x + tab_padding_h;
        self.draw_tab_title(target, slot.tab, text_x, text_y, fg, max_chars);

        self.draw_security_badge(target, slot.index, slot.tab, tab_count, text_y);

        if show_close {
            self.draw_close_button(
                target,
                slot.index,
                slot.width,
                slot.tab.close_hover_progress,
            );
        }
    }

    /// Renders the tab title text with truncation.
    fn draw_tab_title(
        &mut self,
        target: &mut RenderTarget<'_>,
        tab: &super::super::TabInfo,
        text_x: u32,
        text_y: u32,
        fg: Color,
        max_chars: usize,
    ) {
        let title: String = tab.title.chars().take(max_chars).collect();

        for (ci, ch) in title.chars().enumerate() {
            let cx = text_x + ci as u32 * self.metrics.cell_width;
            self.draw_char(target, cx, text_y, ch, fg);
        }
    }

    /// Renders the security badge icon and optional count text.
    fn draw_security_badge(
        &mut self,
        target: &mut RenderTarget<'_>,
        tab_index: usize,
        tab: &super::super::TabInfo,
        tab_count: usize,
        text_y: u32,
    ) {
        if let Some((sx, sy, sw, _)) =
            self.security_badge_rect(tab_index, tab_count, target.width as u32, tab.security_count)
        {
            self.draw_security_shield_icon(target, sx, sy, sw, SECURITY_ACCENT);
            if tab.security_count > 1 {
                let count_text = tab.security_count.min(99).to_string();
                let count_x = sx + sw + self.scaled_px(2);
                for (ci, ch) in count_text.chars().enumerate() {
                    let cx = count_x + ci as u32 * self.metrics.cell_width;
                    self.draw_char(target, cx, text_y, ch, SECURITY_ACCENT);
                }
            }
        }
    }

    /// Draws the close button with a circular hover effect.
    fn draw_close_button(
        &mut self,
        target: &mut RenderTarget<'_>,
        tab_index: usize,
        tw: u32,
        hover_progress: f32,
    ) {
        let rect = self.close_button_rect(tab_index, tw);
        let layout = ui_layout::compute_close_button_layout(
            rect,
            hover_progress,
            self.ui_scale(),
            CLOSE_HOVER_BG_COLOR,
            TAB_TEXT_INACTIVE,
            TAB_TEXT_ACTIVE,
        );

        if layout.show_hover_circle {
            let alpha = (layout.circle_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
            Self::draw_filled_circle(
                target,
                layout.circle_cx as i32,
                layout.circle_cy as i32,
                layout.circle_radius as u32,
                layout.circle_bg_color,
                alpha,
            );
        }

        let close_fg = Color::from_pixel(layout.icon_color);
        let pixel = close_fg.to_pixel();
        Self::draw_stroked_line(
            target,
            (layout.line_a.0, layout.line_a.1),
            (layout.line_a.2, layout.line_a.3),
            layout.icon_thickness,
            pixel,
        );
        Self::draw_stroked_line(
            target,
            (layout.line_b.0, layout.line_b.1),
            (layout.line_b.2, layout.line_b.3),
            layout.icon_thickness,
            pixel,
        );
    }
}
