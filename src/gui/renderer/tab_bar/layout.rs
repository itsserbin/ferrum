#![cfg_attr(target_os = "macos", allow(dead_code))]

#[cfg(not(target_os = "macos"))]
use super::super::WindowButton;
use super::super::shared::tab_math::{self, TabLayoutMetrics};
use super::super::{TabBarHit, TabInfo};
use super::{ACTIVE_TAB_BG, TAB_BORDER};
use crate::core::Color;

#[cfg(not(target_os = "macos"))]
use super::WIN_BTN_WIDTH;

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
        if y >= self.tab_bar_height_px() as f64 {
            return TabBarHit::Empty;
        }

        // Window buttons (non-macOS) have highest priority.
        #[cfg(not(target_os = "macos"))]
        if let Some(btn) = self.window_button_at_position(x, y, buf_width) {
            return TabBarHit::WindowButton(btn);
        }

        // Pin button (non-macOS).
        #[cfg(not(target_os = "macos"))]
        {
            let (pin_x, pin_y, pin_w, pin_h) = self.pin_button_rect();
            if x >= pin_x as f64 && x < (pin_x + pin_w) as f64 && y >= pin_y as f64 && y < (pin_y + pin_h) as f64 {
                return TabBarHit::PinButton;
            }
        }

        let tw = self.tab_width(tab_count, buf_width);
        let tab_strip_start = self.tab_strip_start_x();

        // New-tab button has priority over tab body hit-test.
        let (px, py, pw, ph) = self.plus_button_rect(tab_count, tw);
        if x >= px as f64 && x < (px + pw) as f64 && y >= py as f64 && y < (py + ph) as f64 {
            return TabBarHit::NewTab;
        }

        if x < tab_strip_start as f64 {
            return TabBarHit::Empty;
        }

        let rel_x = x as u32 - tab_strip_start;
        let tab_index = rel_x / tw;
        if (tab_index as usize) < tab_count {
            let idx = tab_index as usize;
            let (cx, cy, cw, ch) = self.close_button_rect(idx, tw);
            if x >= cx as f64 && x < (cx + cw) as f64 && y >= cy as f64 && y < (cy + ch) as f64 {
                return TabBarHit::CloseTab(idx);
            }
            return TabBarHit::Tab(idx);
        }

        TabBarHit::Empty
    }

    /// Hit-tests tab hover target (without button checks).
    pub fn hit_test_tab_hover(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> Option<usize> {
        if y >= self.tab_bar_height_px() as f64 || tab_count == 0 {
            return None;
        }
        let tw = self.tab_width(tab_count, buf_width);
        let tab_strip_start = self.tab_strip_start_x();
        if x < tab_strip_start as f64 {
            return None;
        }
        let rel_x = x as u32 - tab_strip_start;
        let idx = rel_x / tw;
        if (idx as usize) < tab_count {
            Some(idx as usize)
        } else {
            None
        }
    }

    /// Returns tab index when pointer is over a security badge.
    pub fn hit_test_tab_security_badge(
        &self,
        x: f64,
        y: f64,
        tabs: &[TabInfo],
        buf_width: u32,
    ) -> Option<usize> {
        for (idx, tab) in tabs.iter().enumerate() {
            if tab.security_count == 0 {
                continue;
            }
            let Some((sx, sy, sw, sh)) =
                self.security_badge_rect(idx, tabs.len(), buf_width, tab.security_count)
            else {
                continue;
            };
            if x >= sx as f64 && x < (sx + sw) as f64 && y >= sy as f64 && y < (sy + sh) as f64 {
                return Some(idx);
            }
        }
        None
    }

    /// Hit-test window control buttons (non-macOS only).
    #[cfg(not(target_os = "macos"))]
    pub fn window_button_at_position(
        &self,
        x: f64,
        y: f64,
        buf_width: u32,
    ) -> Option<WindowButton> {
        let bar_h = self.tab_bar_height_px();
        if y >= bar_h as f64 {
            return None;
        }
        let btn_w = self.scaled_px(WIN_BTN_WIDTH);
        let close_x = buf_width.saturating_sub(btn_w);
        let min_x = buf_width.saturating_sub(btn_w * 2);
        let minimize_x = buf_width.saturating_sub(btn_w * 3);

        if x >= close_x as f64 && x < buf_width as f64 {
            Some(WindowButton::Close)
        } else if x >= min_x as f64 && x < (min_x + btn_w) as f64 {
            Some(WindowButton::Maximize)
        } else if x >= minimize_x as f64 && x < (minimize_x + btn_w) as f64 {
            Some(WindowButton::Minimize)
        } else {
            None
        }
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
        let idx = hovered_tab?;
        let tab = tabs.get(idx)?;
        if tab.is_renaming || tab.title.is_empty() {
            return None;
        }

        let tw = self.tab_width(tabs.len(), buf_width);
        if self.should_show_number(tw) {
            return Some(tab.title);
        }

        let max_chars = self.title_max_chars(tab, tw, true);
        let title_chars = tab.title.chars().count();
        (title_chars > max_chars).then_some(tab.title)
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
