#![cfg_attr(target_os = "macos", allow(dead_code))]

#[cfg(not(target_os = "macos"))]
use super::super::WindowButton;
use super::super::shared::tab_hit_test;
use super::super::shared::tab_math::{self, TabLayoutMetrics};
use super::super::{TabBarHit, TabInfo};
use super::{ACTIVE_TAB_BG, TAB_BORDER};
use crate::core::Color;

impl super::super::CpuRenderer {
    /// Builds a `TabLayoutMetrics` from the current CPU renderer state.
    fn tab_layout_metrics(&self) -> TabLayoutMetrics {
        TabLayoutMetrics {
            cell_width: self.cell_width,
            cell_height: self.cell_height,
            ui_scale: self.ui_scale(),
            tab_bar_height: self.tab_bar_height_px(),
        }
    }

    /// Computes adaptive tab width with overflow compression.
    /// Tabs shrink from max (MAX_TAB_WIDTH) down to MIN_TAB_WIDTH when many tabs are open.
    pub fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32 {
        let m = self.tab_layout_metrics();
        tab_math::calculate_tab_width(&m, tab_count, buf_width)
    }

    pub(crate) fn tab_strip_start_x(&self) -> u32 {
        let m = self.tab_layout_metrics();
        tab_math::tab_strip_start_x(&m)
    }

    pub(crate) fn tab_origin_x(&self, tab_index: usize, tw: u32) -> u32 {
        let m = self.tab_layout_metrics();
        tab_math::tab_origin_x(&m, tab_index, tw)
    }

    pub(crate) fn tab_insert_index_from_x(
        &self,
        x: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> usize {
        let m = self.tab_layout_metrics();
        tab_math::tab_insert_index_from_x(&m, x, tab_count, buf_width)
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

    /// Hit-tests the tab bar and returns the clicked target.
    pub fn hit_test_tab_bar(&self, x: f64, y: f64, tab_count: usize, buf_width: u32) -> TabBarHit {
        let m = self.tab_layout_metrics();
        tab_hit_test::hit_test_tab_bar(x, y, tab_count, buf_width, &m)
    }

    /// Hit-tests tab hover target (without button checks).
    pub fn hit_test_tab_hover(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> Option<usize> {
        let m = self.tab_layout_metrics();
        tab_hit_test::hit_test_tab_hover(x, y, tab_count, buf_width, &m)
    }

    /// Returns tab index when pointer is over a security badge.
    pub fn hit_test_tab_security_badge(
        &self,
        x: f64,
        y: f64,
        tabs: &[TabInfo],
        buf_width: u32,
    ) -> Option<usize> {
        let m = self.tab_layout_metrics();
        tab_hit_test::hit_test_tab_security_badge(x, y, tabs, buf_width, &m)
    }

    /// Hit-test window control buttons (non-macOS only).
    #[cfg(not(target_os = "macos"))]
    pub fn window_button_at_position(
        &self,
        x: f64,
        y: f64,
        buf_width: u32,
    ) -> Option<WindowButton> {
        let m = self.tab_layout_metrics();
        tab_hit_test::window_button_at_position(x, y, buf_width, &m)
    }

    /// Returns true if the given tab width is too narrow to display the title.
    pub(in crate::gui::renderer) fn should_show_number(&self, tw: u32) -> bool {
        let m = self.tab_layout_metrics();
        tab_math::should_show_number(&m, tw)
    }

    pub(in crate::gui::renderer) fn title_max_chars(
        &self,
        tab: &TabInfo,
        tw: u32,
        is_hovered: bool,
    ) -> usize {
        let m = self.tab_layout_metrics();
        let show_close = tab.is_active || is_hovered;
        tab_math::tab_title_max_chars(&m, tw, show_close, tab.security_count)
    }

    /// Returns full tab title when hover should show a tooltip (compressed or truncated label).
    pub fn tab_hover_tooltip<'a>(
        &self,
        tabs: &'a [TabInfo<'a>],
        hovered_tab: Option<usize>,
        buf_width: u32,
    ) -> Option<&'a str> {
        let m = self.tab_layout_metrics();
        tab_hit_test::tab_hover_tooltip(tabs, hovered_tab, buf_width, &m)
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
        if title.is_empty() || buf_width == 0 || buf_height == 0 {
            return;
        }

        let padding_x = self.scaled_px(6);
        let padding_y = self.scaled_px(4);
        let content_chars = title.chars().count() as u32;
        let width = (content_chars * self.cell_width + padding_x * 2 + self.scaled_px(2))
            .min(buf_width.saturating_sub(4) as u32);
        let height = (self.cell_height + padding_y * 2 + self.scaled_px(2)).min(buf_height as u32);
        if width <= self.scaled_px(2) || height <= self.scaled_px(2) {
            return;
        }

        let mut x = mouse_pos.0.round() as i32 + self.scaled_px(10) as i32;
        let mut y = self.tab_bar_height_px() as i32 + self.scaled_px(6) as i32;
        x = x
            .min(buf_width as i32 - width as i32 - self.scaled_px(2) as i32)
            .max(self.scaled_px(2) as i32);
        y = y
            .min(buf_height as i32 - height as i32 - self.scaled_px(2) as i32)
            .max(self.scaled_px(2) as i32);

        let radius = self.scaled_px(6);
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            x,
            y,
            width,
            height,
            radius,
            ACTIVE_TAB_BG,
            245,
        );
        // Subtle border.
        self.draw_rounded_rect(
            buffer, buf_width, buf_height, x, y, width, height, radius, TAB_BORDER, 80,
        );

        let text_x = x as u32 + self.scaled_px(1) + padding_x;
        let text_y = y as u32 + self.scaled_px(1) + padding_y;
        let max_chars = ((width - self.scaled_px(2) - padding_x * 2) / self.cell_width) as usize;
        for (ci, ch) in title.chars().take(max_chars).enumerate() {
            let cx = text_x + ci as u32 * self.cell_width;
            self.draw_char_at(
                buffer,
                buf_width,
                buf_height,
                cx,
                text_y,
                ch,
                Color::DEFAULT_FG,
            );
        }
    }
}
