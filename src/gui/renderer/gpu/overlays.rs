#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::shared::tab_math;
use super::super::{ACTIVE_TAB_BG, INSERTION_COLOR, TAB_BORDER, TAB_TEXT_ACTIVE, TabInfo};

impl super::GpuRenderer {
    pub(super) fn draw_tab_drag_overlay_impl(
        &mut self,
        buf_width: usize,
        tabs: &[TabInfo],
        source_index: usize,
        current_x: f64,
        indicator_x: f32,
    ) {
        let tab_count = tabs.len();
        if source_index >= tab_count {
            return;
        }
        let tw = self.tab_width_val(tab_count, buf_width as u32);
        let tab_bar_h = self.metrics.tab_bar_height_px() as f32;

        // Ghost tab: rounded rect + shadow + subtle border.
        let ghost_x = (current_x - tw as f64 / 2.0).round() as f32;
        let ghost_y = self.metrics.scaled_px(2) as f32;
        let ghost_h = tab_bar_h - self.metrics.scaled_px(4) as f32;
        let ghost_radius = self.metrics.scaled_px(6) as f32;

        // Shadow.
        self.push_rounded_rect(
            ghost_x + 2.0,
            ghost_y + 2.0,
            tw as f32,
            ghost_h,
            ghost_radius,
            0x000000,
            0.24,
        );
        // Body.
        self.push_rounded_rect(
            ghost_x,
            ghost_y,
            tw as f32,
            ghost_h,
            ghost_radius,
            ACTIVE_TAB_BG,
            0.86,
        );
        // Border.
        self.push_rounded_rect(
            ghost_x,
            ghost_y,
            tw as f32,
            ghost_h,
            ghost_radius,
            TAB_BORDER,
            0.39,
        );

        // Ghost title.
        let m = self.tab_layout_metrics();
        let text_y = tab_math::tab_text_y(&m);
        let use_numbers = self.should_show_number(tw);
        let label: String = if use_numbers {
            (source_index + 1).to_string()
        } else {
            let max = tab_math::rename_field_max_chars(&m, tw);
            tabs[source_index].title.chars().take(max).collect()
        };
        let lw = label.chars().count() as u32 * self.metrics.cell_width;
        let tx = ghost_x + ((tw as i32 - lw as i32) / 2).max(4) as f32;
        self.push_text(tx, text_y as f32, &label, TAB_TEXT_ACTIVE, 1.0);

        // Smooth insertion indicator at lerped position.
        let indicator_pad = self.metrics.scaled_px(4) as f32;
        self.push_rect(
            indicator_x,
            indicator_pad,
            self.metrics.scaled_px(2) as f32,
            tab_bar_h - indicator_pad * 2.0,
            INSERTION_COLOR,
            1.0,
        );
    }

    pub(super) fn draw_tab_tooltip_impl(
        &mut self,
        buf_width: usize,
        buf_height: usize,
        mouse_pos: (f64, f64),
        title: &str,
    ) {
        if title.is_empty() || buf_width == 0 || buf_height == 0 {
            return;
        }

        let padding_x = self.metrics.scaled_px(6) as f32;
        let padding_y = self.metrics.scaled_px(4) as f32;
        let content_chars = title.chars().count() as f32;
        let width = (content_chars * self.metrics.cell_width as f32
            + padding_x * 2.0
            + self.metrics.scaled_px(2) as f32)
            .min(buf_width as f32 - 4.0);
        let height =
            self.metrics.cell_height as f32 + padding_y * 2.0 + self.metrics.scaled_px(2) as f32;

        let mut x = mouse_pos.0 as f32 + self.metrics.scaled_px(10) as f32;
        let mut y = self.metrics.tab_bar_height_px() as f32 + self.metrics.scaled_px(6) as f32;
        x = x.min(buf_width as f32 - width - 2.0).max(2.0);
        y = y.min(buf_height as f32 - height - 2.0).max(2.0);

        let radius = self.metrics.scaled_px(6) as f32;
        self.push_rounded_rect(x, y, width, height, radius, ACTIVE_TAB_BG, 0.96);
        self.push_rounded_rect(x, y, width, height, radius, TAB_BORDER, 0.31);

        let text_x = x + self.metrics.scaled_px(1) as f32 + padding_x;
        let text_y = y + self.metrics.scaled_px(1) as f32 + padding_y;
        let max_chars = ((width - self.metrics.scaled_px(2) as f32 - padding_x * 2.0)
            / self.metrics.cell_width as f32) as usize;
        let display: String = title.chars().take(max_chars).collect();
        self.push_text(text_x, text_y, &display, TAB_TEXT_ACTIVE, 1.0);
    }

    pub(super) fn tab_hover_tooltip_impl<'a>(
        &self,
        tabs: &'a [TabInfo<'a>],
        hovered_tab: Option<usize>,
        buf_width: u32,
    ) -> Option<&'a str> {
        let m = self.tab_layout_metrics();
        super::super::shared::tab_hit_test::tab_hover_tooltip(tabs, hovered_tab, buf_width, &m)
    }

    pub(super) fn tab_insert_index_from_x_impl(
        &self,
        x: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> usize {
        let m = self.tab_layout_metrics();
        tab_math::tab_insert_index_from_x(&m, x, tab_count, buf_width)
    }
}
