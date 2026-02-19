#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::shared::overlay_layout;
use super::super::shared::tab_math::{self, TabLayoutMetrics};
use super::{ACTIVE_TAB_BG, TAB_BORDER};
use crate::core::Color;

impl super::super::CpuRenderer {
    /// Builds a `TabLayoutMetrics` from the current CPU renderer state.
    pub(in crate::gui::renderer) fn tab_layout_metrics(&self) -> TabLayoutMetrics {
        TabLayoutMetrics {
            cell_width: self.metrics.cell_width,
            cell_height: self.metrics.cell_height,
            ui_scale: self.ui_scale(),
            tab_bar_height: self.tab_bar_height_px(),
        }
    }

    /// Returns rectangle for per-tab close button.
    pub(in crate::gui::renderer) fn close_button_rect(
        &self,
        tab_index: usize,
        tw: u32,
    ) -> (u32, u32, u32, u32) {
        let m = self.tab_layout_metrics();
        tab_math::close_button_rect(&m, tab_index, tw).to_tuple()
    }

    /// Returns rectangle for new-tab button.
    pub(in crate::gui::renderer) fn plus_button_rect(
        &self,
        tab_count: usize,
        tw: u32,
    ) -> (u32, u32, u32, u32) {
        let m = self.tab_layout_metrics();
        tab_math::plus_button_rect(&m, tab_count, tw).to_tuple()
    }

    /// Returns rectangle for pin button (non-macOS only).
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::renderer) fn pin_button_rect(&self) -> (u32, u32, u32, u32) {
        let m = self.tab_layout_metrics();
        tab_math::pin_button_rect(&m).to_tuple()
    }

    /// Returns true if the given tab width is too narrow to display the title.
    pub(in crate::gui::renderer) fn should_show_number(&self, tw: u32) -> bool {
        let m = self.tab_layout_metrics();
        tab_math::should_show_number(&m, tw)
    }

    /// Draws a small tooltip with full tab title near the pointer.
    pub fn draw_tab_tooltip(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        mouse_pos: (f64, f64),
        title: &str,
    ) {
        let m = self.tab_layout_metrics();
        let layout = match overlay_layout::compute_tooltip_layout(
            title,
            mouse_pos,
            &m,
            buf_width as u32,
            buf_height as u32,
        ) {
            Some(l) => l,
            None => return,
        };

        // Background fill.
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            layout.bg_x,
            layout.bg_y,
            layout.bg_w,
            layout.bg_h,
            layout.radius,
            ACTIVE_TAB_BG,
            245,
        );
        // Subtle border.
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            layout.bg_x,
            layout.bg_y,
            layout.bg_w,
            layout.bg_h,
            layout.radius,
            TAB_BORDER,
            80,
        );

        for (ci, ch) in layout.display_text.chars().enumerate() {
            let cx = layout.text_x + ci as u32 * self.metrics.cell_width;
            self.draw_char(
                buffer,
                buf_width,
                buf_height,
                cx,
                layout.text_y,
                ch,
                Color::DEFAULT_FG,
            );
        }
    }
}
