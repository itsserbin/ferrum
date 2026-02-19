#![cfg_attr(target_os = "macos", allow(dead_code))]

//! Tab bar layout math and drawing helpers for the GPU renderer.

use super::super::shared::tab_math::{self, TabLayoutMetrics};
use super::super::{TAB_BORDER, TabInfo};

impl super::GpuRenderer {
    // ── Tab bar math (delegates to shared tab_math) ──────────────────────

    /// Builds a `TabLayoutMetrics` from the current GPU renderer state.
    pub(super) fn tab_layout_metrics(&self) -> TabLayoutMetrics {
        TabLayoutMetrics {
            cell_width: self.metrics.cell_width,
            cell_height: self.metrics.cell_height,
            ui_scale: self.metrics.ui_scale,
            tab_bar_height: self.metrics.tab_bar_height_px(),
        }
    }

    /// Returns rectangle for pin button (non-macOS only).
    #[cfg(not(target_os = "macos"))]
    pub(super) fn pin_button_rect(&self) -> (u32, u32, u32, u32) {
        let m = self.tab_layout_metrics();
        tab_math::pin_button_rect(&m).to_tuple()
    }

    pub(super) fn tab_width_val(&self, tab_count: usize, buf_width: u32) -> u32 {
        let m = self.tab_layout_metrics();
        tab_math::calculate_tab_width(&m, tab_count, buf_width)
    }

    pub(super) fn tab_origin_x_val(&self, tab_index: usize, tw: u32) -> u32 {
        let m = self.tab_layout_metrics();
        tab_math::tab_origin_x(&m, tab_index, tw)
    }

    pub(super) fn close_button_rect(&self, tab_index: usize, tw: u32) -> (u32, u32, u32, u32) {
        let m = self.tab_layout_metrics();
        tab_math::close_button_rect(&m, tab_index, tw).to_tuple()
    }

    pub(super) fn plus_button_rect(&self, tab_count: usize, tw: u32) -> (u32, u32, u32, u32) {
        let m = self.tab_layout_metrics();
        tab_math::plus_button_rect(&m, tab_count, tw).to_tuple()
    }

    pub(super) fn should_show_number(&self, tw: u32) -> bool {
        let m = self.tab_layout_metrics();
        tab_math::should_show_number(&m, tw)
    }

    pub(super) fn security_badge_rect_val(
        &self,
        tab_index: usize,
        tab_count: usize,
        buf_width: u32,
        security_count: usize,
    ) -> Option<(u32, u32, u32, u32)> {
        let m = self.tab_layout_metrics();
        tab_math::security_badge_rect(&m, tab_index, tab_count, buf_width, security_count)
            .map(|r| r.to_tuple())
    }

    // ── Tab bar rendering: orchestrator ─────────────────────────────────

    pub(super) fn draw_tab_bar_impl(
        &mut self,
        buf_width: usize,
        tabs: &[TabInfo],
        hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
        tab_offsets: Option<&[f32]>,
        pinned: bool,
    ) {
        let bw = buf_width as u32;
        let tw = self.tab_width_val(tabs.len(), bw);
        let m = self.tab_layout_metrics();
        let text_y = tab_math::tab_text_y(&m);
        let use_numbers = self.should_show_number(tw);

        self.tab_bar_background_commands(bw);

        for (i, tab) in tabs.iter().enumerate() {
            let anim_offset = tab_offsets.and_then(|o| o.get(i)).copied().unwrap_or(0.0);
            let tab_x = self.tab_origin_x_val(i, tw) as f32 + anim_offset;
            let is_hovered = hovered_tab == Some(i);

            self.tab_background_commands(tab, tab_x, tw);

            if tab.is_renaming {
                self.tab_rename_commands(tab, tab_x, tw, text_y);
            } else if use_numbers {
                self.tab_number_commands(i, tab, tab_x, tw, text_y, is_hovered);
            } else {
                self.tab_content_commands(i, tab, tabs.len(), bw, tab_x, tw, text_y, is_hovered);
            }
        }

        self.plus_button_commands(tabs.len(), tw, mouse_pos);

        #[cfg(not(target_os = "macos"))]
        self.draw_pin_button_commands(mouse_pos, pinned);

        #[cfg(target_os = "macos")]
        let _ = pinned;

        #[cfg(not(target_os = "macos"))]
        self.draw_window_buttons_commands(bw, mouse_pos);

        // Bottom separator line.
        let tab_bar_h = self.metrics.tab_bar_height_px() as f32;
        let sep_y = tab_bar_h - 1.0;
        self.push_rect(0.0, sep_y, bw as f32, 1.0, TAB_BORDER, 0.7);
    }
}
