#![cfg_attr(target_os = "macos", allow(dead_code))]

#[cfg(not(target_os = "macos"))]
use super::super::WindowButton;
use super::super::{MIN_TAB_WIDTH, MIN_TAB_WIDTH_FOR_TITLE};
use super::super::{TabBarHit, TabInfo};
use super::{ACTIVE_TAB_BG, TAB_BORDER};
use crate::core::Color;

#[cfg(not(target_os = "macos"))]
use super::WIN_BTN_WIDTH;

/// Maximum tab width in pixels (before HiDPI scaling).
const MAX_TAB_WIDTH: u32 = 240;

/// Tab strip start offset for macOS (accounts for traffic light buttons).
#[cfg(target_os = "macos")]
const TAB_STRIP_START_X: u32 = 78;

/// Tab strip start offset for Windows.
#[cfg(target_os = "windows")]
const TAB_STRIP_START_X: u32 = 14;

/// Tab strip start offset for Linux and other platforms.
#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
const TAB_STRIP_START_X: u32 = 8;

/// Plus button extra margin for reservation calculation.
const PLUS_BUTTON_MARGIN: u32 = 20;

/// Close button size in pixels.
pub(super) const CLOSE_BUTTON_SIZE: u32 = 20;

/// Close button margin from tab edge.
pub(super) const CLOSE_BUTTON_MARGIN: u32 = 6;

/// Plus button size in pixels.
const PLUS_BUTTON_SIZE: u32 = 24;

/// Plus button gap from last tab.
const PLUS_BUTTON_GAP: u32 = 4;

/// Tab padding horizontal (left/right spacing for text).
pub(super) const TAB_PADDING_H: u32 = 14;

/// Pin button size in pixels (non-macOS).
#[cfg(not(target_os = "macos"))]
const PIN_BUTTON_SIZE: u32 = 24;

/// Gap between pin button and first tab (non-macOS).
#[cfg(not(target_os = "macos"))]
const PIN_BUTTON_GAP: u32 = 8;

impl super::super::CpuRenderer {
    /// Computes adaptive tab width with overflow compression.
    /// Tabs shrink from max (MAX_TAB_WIDTH) down to MIN_TAB_WIDTH when many tabs are open.
    pub fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32 {
        let reserved = self.tab_strip_start_x()
            + self.plus_button_reserved_width()
            + self.scaled_px(PLUS_BUTTON_GAP * 2)
            + self.window_buttons_reserved_width();
        let available = buf_width.saturating_sub(reserved);
        let min_tab_width = self.scaled_px(MIN_TAB_WIDTH);
        let max_tab_width = self.scaled_px(MAX_TAB_WIDTH);
        (available / tab_count.max(1) as u32).clamp(min_tab_width, max_tab_width)
    }

    pub(crate) fn tab_strip_start_x(&self) -> u32 {
        #[cfg(not(target_os = "macos"))]
        {
            // On Windows/Linux: WINDOW_PADDING + pin button + gap
            self.scaled_px(TAB_STRIP_START_X) + self.scaled_px(PIN_BUTTON_SIZE) + self.scaled_px(PIN_BUTTON_GAP)
        }
        #[cfg(target_os = "macos")]
        {
            self.scaled_px(TAB_STRIP_START_X)
        }
    }

    pub(in crate::gui::renderer) fn plus_button_reserved_width(&self) -> u32 {
        self.cell_width + self.scaled_px(PLUS_BUTTON_MARGIN)
    }

    /// Returns total width reserved for window control buttons.
    pub(in crate::gui::renderer) fn window_buttons_reserved_width(&self) -> u32 {
        #[cfg(not(target_os = "macos"))]
        {
            self.scaled_px(WIN_BTN_WIDTH) * 3
        }
        #[cfg(target_os = "macos")]
        {
            0
        }
    }

    pub(crate) fn tab_origin_x(&self, tab_index: usize, tw: u32) -> u32 {
        self.tab_strip_start_x() + tab_index as u32 * tw
    }

    pub(crate) fn tab_insert_index_from_x(
        &self,
        x: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> usize {
        let tw = self.tab_width(tab_count, buf_width);
        let start = self.tab_strip_start_x() as f64;
        let mut idx = tab_count;
        for i in 0..tab_count {
            let center = start + i as f64 * tw as f64 + tw as f64 / 2.0;
            if x < center {
                idx = i;
                break;
            }
        }
        idx
    }

    /// Returns rectangle for per-tab close button.
    pub(in crate::gui::renderer) fn close_button_rect(
        &self,
        tab_index: usize,
        tw: u32,
    ) -> (u32, u32, u32, u32) {
        let btn_size = self.scaled_px(CLOSE_BUTTON_SIZE);
        let x = self.tab_origin_x(tab_index, tw) + tw - btn_size - self.scaled_px(CLOSE_BUTTON_MARGIN);
        let y = (self.tab_bar_height_px().saturating_sub(btn_size)) / 2;
        (x, y, btn_size, btn_size)
    }

    /// Returns rectangle for new-tab button.
    pub(in crate::gui::renderer) fn plus_button_rect(
        &self,
        tab_count: usize,
        tw: u32,
    ) -> (u32, u32, u32, u32) {
        let btn_size = self.scaled_px(PLUS_BUTTON_SIZE);
        let x = self.tab_strip_start_x() + tab_count as u32 * tw + self.scaled_px(PLUS_BUTTON_GAP);
        let y = (self.tab_bar_height_px().saturating_sub(btn_size)) / 2;
        (x, y, btn_size, btn_size)
    }

    /// Returns rectangle for pin button (non-macOS only).
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::renderer) fn pin_button_rect(&self) -> (u32, u32, u32, u32) {
        let btn_size = self.scaled_px(PIN_BUTTON_SIZE);
        let x = self.scaled_px(TAB_STRIP_START_X);
        let y = (self.tab_bar_height_px().saturating_sub(btn_size)) / 2;
        (x, y, btn_size, btn_size)
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
        tw < self.scaled_px(MIN_TAB_WIDTH_FOR_TITLE)
    }

    pub(in crate::gui::renderer) fn title_max_chars(
        &self,
        tab: &TabInfo,
        tw: u32,
        is_hovered: bool,
    ) -> usize {
        let tab_padding_h = self.scaled_px(TAB_PADDING_H);
        let show_close = tab.is_active || is_hovered;
        let close_reserved = if show_close {
            self.scaled_px(CLOSE_BUTTON_SIZE) + self.scaled_px(CLOSE_BUTTON_MARGIN)
        } else {
            0
        };
        let security_reserved = if tab.security_count > 0 {
            let count_chars = tab.security_count.min(99).to_string().len() as u32;
            let count_width = if tab.security_count > 1 {
                count_chars * self.cell_width + self.scaled_px(2)
            } else {
                0
            };
            let badge_min = self.scaled_px(10);
            let badge_max = self.scaled_px(15);
            self.cell_height
                .saturating_sub(self.scaled_px(10))
                .clamp(badge_min, badge_max)
                + count_width
                + self.scaled_px(6)
        } else {
            0
        };
        (tw.saturating_sub(tab_padding_h * 2 + close_reserved + security_reserved)
            / self.cell_width) as usize
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
