#![cfg_attr(target_os = "macos", allow(dead_code))]

mod buttons;
mod drag_overlay;
mod layout;
mod primitives;

use super::shared::tab_math::{self, TabLayoutMetrics};

use super::*;

// Catppuccin Mocha palette — flat Chrome-style tab bar.
const BAR_BG: u32 = 0x181825; // Mantle — bar background
pub(self) const ACTIVE_TAB_BG: u32 = 0x1E1E2E; // Base — merges with terminal
pub(self) const INACTIVE_TAB_HOVER: u32 = 0x313244; // Surface0
pub(self) const TAB_TEXT_ACTIVE: u32 = 0xCDD6F4; // Text
pub(self) const TAB_TEXT_INACTIVE: u32 = 0x6C7086; // Overlay0
pub(self) const TAB_BORDER: u32 = 0x313244; // Surface0
const CLOSE_HOVER_BG_COLOR: u32 = 0x585B70; // Surface2
const RENAME_FIELD_BG: u32 = 0x24273A; // Distinct editable-field background
const RENAME_FIELD_BORDER: u32 = 0x6C7086; // Subtle field border

// Window button colors (non-macOS).
#[cfg(not(target_os = "macos"))]
pub(self) const WIN_BTN_ICON: u32 = 0x6C7086; // Overlay0
#[cfg(not(target_os = "macos"))]
pub(self) const WIN_BTN_HOVER: u32 = 0x313244; // Surface0
#[cfg(not(target_os = "macos"))]
pub(self) const WIN_BTN_CLOSE_HOVER: u32 = 0xF38BA8; // Red
#[cfg(not(target_os = "macos"))]
pub(self) const WIN_BTN_WIDTH: u32 = 46;

// Insertion indicator color (Catppuccin Mocha Mauve).
pub(self) const INSERTION_COLOR: u32 = 0xCBA6F7;

// Pin button active color (Catppuccin Mocha Lavender - same as active accent).
#[cfg(not(target_os = "macos"))]
pub(self) const PIN_ACTIVE_COLOR: u32 = 0xB4BEFE;

impl CpuRenderer {
    /// Draws top tab bar including tabs, controls, and separators.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_tab_bar(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        tabs: &[TabInfo],
        _hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
        tab_offsets: Option<&[f32]>,
        _pinned: bool,
    ) {
        let tab_bar_height = self.tab_bar_height_px();
        let bar_h = tab_bar_height as usize;
        let tw = self.tab_width(tabs.len(), buf_width as u32);
        let use_numbers = self.should_show_number(tw);

        self.draw_tab_bar_background(buffer, buf_width, buf_height, tab_bar_height);

        for (i, tab) in tabs.iter().enumerate() {
            let anim_offset = tab_offsets.and_then(|o| o.get(i)).copied().unwrap_or(0.0);
            let tab_x = (self.tab_origin_x(i, tw) as f32 + anim_offset).round() as u32;

            self.draw_tab_background(buffer, buf_width, bar_h, tab, tab_x, tw, tab_bar_height);

            if tab.is_renaming {
                self.draw_tab_rename_field(buffer, buf_width, bar_h, tab, tab_x, tw, tab_bar_height);
            } else if use_numbers {
                self.draw_tab_number(buffer, buf_width, bar_h, i, tab, tab_x, tw, tab_bar_height);
            } else {
                self.draw_tab_content(buffer, buf_width, bar_h, i, tab, tabs.len(), tab_x, tw, tab_bar_height);
            }
        }

        self.draw_plus_button(buffer, buf_width, bar_h, tabs.len(), tw, mouse_pos);

        #[cfg(not(target_os = "macos"))]
        self.draw_pin_button(buffer, buf_width, bar_h, mouse_pos, _pinned);

        #[cfg(not(target_os = "macos"))]
        self.draw_window_buttons(buffer, buf_width, bar_h, mouse_pos);

        self.draw_bottom_separator(buffer, buf_width, bar_h);
    }

    /// Fills the bar background with rounded top corners.
    fn draw_tab_bar_background(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        tab_bar_height: u32,
    ) {
        let bar_radius = self.scaled_px(10);
        self.draw_top_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            0,
            0,
            buf_width as u32,
            tab_bar_height,
            bar_radius,
            BAR_BG,
            255,
        );
    }

    /// Draws active/inactive/hover tab background fill.
    fn draw_tab_background(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        tab: &TabInfo,
        tab_x: u32,
        tw: u32,
        tab_bar_height: u32,
    ) {
        let hover_t = tab.hover_progress.clamp(0.0, 1.0);

        if tab.is_active {
            // Active tab: flat fill that merges with terminal.
            let fill_bg = ACTIVE_TAB_BG;
            for py in 0..tab_bar_height as usize {
                if py >= bar_h {
                    break;
                }
                for dx in 0..tw as usize {
                    let px = tab_x as usize + dx;
                    if px < buf_width {
                        let idx = py * buf_width + px;
                        if idx < buffer.len() {
                            buffer[idx] = fill_bg;
                        }
                    }
                }
            }
        } else if hover_t > 0.01 {
            // Inactive tab hover: flat fill highlight.
            let fill_bg = INACTIVE_TAB_HOVER;
            let alpha = (hover_t * 220.0).round().clamp(0.0, 255.0) as u8;
            for py in 0..tab_bar_height as usize {
                if py >= bar_h {
                    break;
                }
                for dx in 0..tw as usize {
                    let px = tab_x as usize + dx;
                    if px < buf_width {
                        let idx = py * buf_width + px;
                        if idx < buffer.len() {
                            buffer[idx] = Self::blend_pixel(buffer[idx], fill_bg, alpha);
                        }
                    }
                }
            }
        }
        // Inactive non-hovered: no background (BAR_BG shows through).
    }

    /// Renders the inline rename field: background, border, text with selection, and cursor.
    #[allow(clippy::too_many_arguments)]
    fn draw_tab_rename_field(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        tab: &TabInfo,
        tab_x: u32,
        tw: u32,
        tab_bar_height: u32,
    ) {
        let m = TabLayoutMetrics {
            cell_width: self.cell_width,
            cell_height: self.cell_height,
            ui_scale: self.ui_scale(),
            tab_bar_height,
        };
        let text_y = tab_math::tab_text_y(&m);
        let tab_padding_h = m.scaled_px(tab_math::TAB_PADDING_H);
        let rename_text = tab.rename_text.unwrap_or("");
        let text_x = tab_x + tab_padding_h;
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

        let r = tab_math::rename_field_rect(&m, tab_x, tw);
        self.draw_rename_background(buffer, buf_width, bar_h, &r);
        self.draw_rename_text(buffer, buf_width, bar_h, rename_text, text_x, text_y, max_chars, selection_chars);
        self.draw_rename_cursor(buffer, buf_width, bar_h, rename_text, tab.rename_cursor, text_x, text_y, max_chars);
    }

    /// Draws the rename field background (fill + border).
    fn draw_rename_background(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        r: &tab_math::Rect,
    ) {
        self.draw_rounded_rect(
            buffer,
            buf_width,
            bar_h,
            r.x as i32,
            r.y as i32,
            r.w,
            r.h,
            self.scaled_px(6),
            RENAME_FIELD_BG,
            245,
        );
        self.draw_rounded_rect(
            buffer,
            buf_width,
            bar_h,
            r.x as i32,
            r.y as i32,
            r.w,
            r.h,
            self.scaled_px(6),
            RENAME_FIELD_BORDER,
            90,
        );
    }

    /// Renders rename text characters with optional selection highlight.
    #[allow(clippy::too_many_arguments)]
    fn draw_rename_text(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        rename_text: &str,
        text_x: u32,
        text_y: u32,
        max_chars: usize,
        selection_chars: Option<(usize, usize)>,
    ) {
        for (ci, ch) in rename_text.chars().take(max_chars).enumerate() {
            let cx = text_x + ci as u32 * self.cell_width;
            let selected =
                selection_chars.is_some_and(|(start, end)| ci >= start && ci < end);
            if selected {
                self.draw_bg(buffer, buf_width, bar_h, cx, text_y, ACTIVE_ACCENT);
                self.draw_char_at(
                    buffer,
                    buf_width,
                    bar_h,
                    cx,
                    text_y,
                    ch,
                    Color::DEFAULT_BG,
                );
            } else {
                self.draw_char_at(
                    buffer,
                    buf_width,
                    bar_h,
                    cx,
                    text_y,
                    ch,
                    Color::DEFAULT_FG,
                );
            }
        }
    }

    /// Draws the blinking cursor bar in the rename field.
    #[allow(clippy::too_many_arguments)]
    fn draw_rename_cursor(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        rename_text: &str,
        rename_cursor: usize,
        text_x: u32,
        text_y: u32,
        max_chars: usize,
    ) {
        let cursor_chars = rename_text
            .get(..rename_cursor)
            .map_or(0, |prefix| prefix.chars().count())
            .min(max_chars);
        let cursor_x = text_x + cursor_chars as u32 * self.cell_width;
        let cursor_w = self.scaled_px(2);
        let cursor_h = self.cell_height.saturating_sub(self.scaled_px(2));
        let cursor_y = text_y + self.scaled_px(1);
        for py in cursor_y as usize..(cursor_y + cursor_h) as usize {
            if py >= bar_h {
                break;
            }
            for px in cursor_x as usize..(cursor_x + cursor_w) as usize {
                if px < buf_width && py * buf_width + px < buffer.len() {
                    let idx = py * buf_width + px;
                    buffer[idx] = Self::blend_pixel(buffer[idx], TAB_TEXT_ACTIVE, 220);
                }
            }
        }
    }

    /// Renders a tab number (1-based) in overflow/compressed mode.
    #[allow(clippy::too_many_arguments)]
    fn draw_tab_number(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        tab_index: usize,
        tab: &TabInfo,
        tab_x: u32,
        tw: u32,
        tab_bar_height: u32,
    ) {
        let m = TabLayoutMetrics {
            cell_width: self.cell_width,
            cell_height: self.cell_height,
            ui_scale: self.ui_scale(),
            tab_bar_height,
        };
        let text_y = tab_math::tab_text_y(&m);
        let hover_t = tab.hover_progress.clamp(0.0, 1.0);
        let fg = if tab.is_active {
            Color::from_pixel(TAB_TEXT_ACTIVE)
        } else {
            Color::from_pixel(TAB_TEXT_INACTIVE)
        };

        let number_str = (tab_index + 1).to_string();
        let show_close = tab.is_active || hover_t > 0.05;
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
                tab,
                tw,
                tab.close_hover_progress,
            );
        }
    }

    /// Renders normal tab content: title, security badge, and close button.
    #[allow(clippy::too_many_arguments)]
    fn draw_tab_content(
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
    ) {
        let m = TabLayoutMetrics {
            cell_width: self.cell_width,
            cell_height: self.cell_height,
            ui_scale: self.ui_scale(),
            tab_bar_height,
        };
        let text_y = tab_math::tab_text_y(&m);
        let tab_padding_h = m.scaled_px(tab_math::TAB_PADDING_H);
        let hover_t = tab.hover_progress.clamp(0.0, 1.0);
        let fg = if tab.is_active {
            Color::from_pixel(TAB_TEXT_ACTIVE)
        } else {
            Color::from_pixel(TAB_TEXT_INACTIVE)
        };

        let show_close = tab.is_active || hover_t > 0.05;
        let max_chars = tab_math::tab_title_max_chars(&m, tw, show_close, tab.security_count);

        self.draw_tab_title(buffer, buf_width, bar_h, tab, tab_x, tab_padding_h, text_y, fg, max_chars);

        self.draw_security_badge(buffer, buf_width, bar_h, tab_index, tab, tab_count, text_y);

        if show_close {
            self.draw_close_button(
                buffer,
                buf_width,
                bar_h,
                tab_index,
                tab,
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
        if let Some((sx, sy, sw, _sh)) =
            self.security_badge_rect(tab_index, tab_count, buf_width as u32, tab.security_count)
        {
            self.draw_security_shield_icon(
                buffer,
                buf_width,
                bar_h,
                sx,
                sy,
                sw,
                SECURITY_ACCENT,
            );
            if tab.security_count > 1 {
                let count_text = tab.security_count.min(99).to_string();
                let count_x = sx + sw + self.scaled_px(2);
                for (ci, ch) in count_text.chars().enumerate() {
                    let cx = count_x + ci as u32 * self.cell_width;
                    self.draw_char_at(
                        buffer,
                        buf_width,
                        bar_h,
                        cx,
                        text_y,
                        ch,
                        SECURITY_ACCENT,
                    );
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
        _tab: &TabInfo,
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

    /// Draws the new-tab (+) button after the last tab.
    fn draw_plus_button(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
        tab_count: usize,
        tw: u32,
        mouse_pos: (f64, f64),
    ) {
        let plus_rect = self.plus_button_rect(tab_count, tw);
        let plus_hover = Self::point_in_rect(mouse_pos.0, mouse_pos.1, plus_rect);
        if plus_hover {
            let (px, py, pw, ph) = plus_rect;
            self.draw_rounded_rect(
                buffer,
                buf_width,
                bar_h,
                px as i32,
                py as i32,
                pw,
                ph,
                self.scaled_px(5),
                INACTIVE_TAB_HOVER,
                255,
            );
        }
        let plus_fg = if plus_hover {
            Color::from_pixel(TAB_TEXT_ACTIVE)
        } else {
            Color::from_pixel(TAB_TEXT_INACTIVE)
        };
        self.draw_tab_plus_icon(buffer, buf_width, bar_h, plus_rect, plus_fg);
    }

    /// Draws the 1px bottom separator between the bar and the terminal area.
    fn draw_bottom_separator(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        bar_h: usize,
    ) {
        if bar_h > 0 {
            let py = bar_h - 1;
            for px in 0..buf_width {
                let idx = py * buf_width + px;
                if idx < buffer.len() {
                    buffer[idx] = Self::blend_pixel(buffer[idx], TAB_BORDER, 180);
                }
            }
        }
    }

    /// Returns true when pointer is over the custom window minimize button.
    #[allow(dead_code)]
    pub fn is_window_minimize_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        #[cfg(not(target_os = "macos"))]
        {
            self.window_button_at_position(x, y, buf_width as u32) == Some(WindowButton::Minimize)
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (x, y, buf_width);
            false
        }
    }

    /// Returns true when pointer is over the custom window maximize button.
    #[allow(dead_code)]
    pub fn is_window_maximize_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        #[cfg(not(target_os = "macos"))]
        {
            self.window_button_at_position(x, y, buf_width as u32) == Some(WindowButton::Maximize)
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (x, y, buf_width);
            false
        }
    }

    /// Returns true when pointer is over the custom window close button.
    #[allow(dead_code)]
    pub fn is_window_close_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        #[cfg(not(target_os = "macos"))]
        {
            self.window_button_at_position(x, y, buf_width as u32) == Some(WindowButton::Close)
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (x, y, buf_width);
            false
        }
    }
}
