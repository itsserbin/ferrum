
use crate::core::Color;

use super::super::shared::{tab_math, ui_layout};
use super::super::traits::Renderer;
use super::super::types::{RenderTarget, TabSlot};
use super::super::CpuRenderer;

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
            self.palette.tab_text_active
        } else {
            self.palette.tab_text_inactive
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

    /// Renders normal tab content: title and close button.
    pub(super) fn draw_tab_content(
        &mut self,
        target: &mut RenderTarget<'_>,
        slot: &TabSlot,
    ) {
        let m = self.tab_layout_metrics();
        let text_y = tab_math::tab_text_y(&m);
        let tab_padding_h = m.scaled_px(tab_math::TAB_PADDING_H);
        let fg = if slot.tab.is_active {
            self.palette.tab_text_active
        } else {
            self.palette.tab_text_inactive
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
        );

        let text_x = slot.x + tab_padding_h;
        self.draw_tab_title(target, slot.tab, text_x, text_y, fg, max_chars);


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
        use crate::gui::renderer::shared::path_display::format_tab_path;
        let fallback = format!("#{}", tab.index + 1);
        let title = format_tab_path(tab.title, max_chars, &fallback);

        for (ci, ch) in title.chars().enumerate() {
            let cx = text_x + ci as u32 * self.metrics.cell_width;
            self.draw_char(target, cx, text_y, ch, fg);
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
            self.palette.close_hover_bg.to_pixel(),
            self.palette.tab_text_inactive.to_pixel(),
            self.palette.tab_text_active.to_pixel(),
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
