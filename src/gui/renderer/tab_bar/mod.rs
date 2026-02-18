#![cfg_attr(target_os = "macos", allow(dead_code))]

mod drag_overlay;
mod layout;
mod primitives;

use layout::{CLOSE_BUTTON_MARGIN, CLOSE_BUTTON_SIZE, TAB_PADDING_H};

use super::*;

// Catppuccin Mocha palette — flat Chrome-style tab bar.
const BAR_BG: u32 = 0x181825; // Mantle — bar background
pub(self) const ACTIVE_TAB_BG: u32 = 0x1E1E2E; // Base — merges with terminal
const INACTIVE_TAB_HOVER: u32 = 0x313244; // Surface0
const TAB_TEXT_ACTIVE: u32 = 0xCDD6F4; // Text
const TAB_TEXT_INACTIVE: u32 = 0x6C7086; // Overlay0
pub(self) const TAB_BORDER: u32 = 0x313244; // Surface0
const CLOSE_HOVER_BG_COLOR: u32 = 0x585B70; // Surface2
const RENAME_FIELD_BG: u32 = 0x24273A; // Distinct editable-field background
const RENAME_FIELD_BORDER: u32 = 0x6C7086; // Subtle field border

// Window button colors (non-macOS).
#[cfg(not(target_os = "macos"))]
const WIN_BTN_ICON: u32 = 0x6C7086; // Overlay0
#[cfg(not(target_os = "macos"))]
const WIN_BTN_HOVER: u32 = 0x313244; // Surface0
#[cfg(not(target_os = "macos"))]
const WIN_BTN_CLOSE_HOVER: u32 = 0xF38BA8; // Red
#[cfg(not(target_os = "macos"))]
pub(self) const WIN_BTN_WIDTH: u32 = 46;

// Insertion indicator color (Catppuccin Mocha Mauve).
pub(self) const INSERTION_COLOR: u32 = 0xCBA6F7;

impl CpuRenderer {
    /// Draws top tab bar including tabs, controls, and separators.
    pub fn draw_tab_bar(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        tabs: &[TabInfo],
        _hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
        tab_offsets: Option<&[f32]>,
    ) {
        let tab_bar_height = self.tab_bar_height_px();
        let bar_h = tab_bar_height as usize;

        // Bar background with rounded top corners.
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

        let tw = self.tab_width(tabs.len(), buf_width as u32);
        let text_y = (tab_bar_height.saturating_sub(self.cell_height)) / 2 + self.scaled_px(1);
        let tab_padding_h = self.scaled_px(TAB_PADDING_H);
        let use_numbers = self.should_show_number(tw);
        let tab_inset_y = 0u32; // Tabs start from top of bar.
        let tab_h = tab_bar_height; // Full height so bottom merges with terminal.

        for (i, tab) in tabs.iter().enumerate() {
            let anim_offset = tab_offsets.and_then(|o| o.get(i)).copied().unwrap_or(0.0);
            let tab_x = (self.tab_origin_x(i, tw) as f32 + anim_offset).round() as u32;
            let hover_t = tab.hover_progress.clamp(0.0, 1.0);

            if tab.is_active {
                // Active tab: flat fill that merges with terminal.
                let fill_bg = ACTIVE_TAB_BG;
                for py in tab_inset_y as usize..(tab_inset_y + tab_h) as usize {
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
                for py in tab_inset_y as usize..(tab_inset_y + tab_h) as usize {
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

            // Text color based on state.
            let fg = if tab.is_active {
                Color::from_pixel(TAB_TEXT_ACTIVE)
            } else {
                Color::from_pixel(TAB_TEXT_INACTIVE)
            };

            if tab.is_renaming {
                // Inline rename rendering.
                let rename_text = tab.rename_text.unwrap_or("");
                let text_x = tab_x + tab_padding_h;
                let max_chars = (tw.saturating_sub(tab_padding_h * 2) / self.cell_width) as usize;
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

                let field_pad_x = self.scaled_px(3);
                let field_x = tab_x + tab_padding_h.saturating_sub(field_pad_x);
                let field_y = text_y.saturating_sub(self.scaled_px(2));
                let field_w = tw.saturating_sub(tab_padding_h * 2) + field_pad_x * 2;
                let field_h = self.cell_height + self.scaled_px(4);
                self.draw_rounded_rect(
                    buffer,
                    buf_width,
                    bar_h,
                    field_x as i32,
                    field_y as i32,
                    field_w,
                    field_h,
                    self.scaled_px(6),
                    RENAME_FIELD_BG,
                    245,
                );
                self.draw_rounded_rect(
                    buffer,
                    buf_width,
                    bar_h,
                    field_x as i32,
                    field_y as i32,
                    field_w,
                    field_h,
                    self.scaled_px(6),
                    RENAME_FIELD_BORDER,
                    90,
                );

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

                let cursor_chars = rename_text
                    .get(..tab.rename_cursor)
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
            } else if use_numbers {
                // Overflow mode: show tab number (1-based) instead of title.
                let number_str = (i + 1).to_string();
                let show_close = tab.is_active || hover_t > 0.05;
                let close_reserved = if show_close {
                    self.scaled_px(CLOSE_BUTTON_SIZE) + self.scaled_px(CLOSE_BUTTON_MARGIN)
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
                        i,
                        tab,
                        tw,
                        tab.close_hover_progress,
                    );
                }
            } else {
                // Normal mode: show title with close button and security badge.
                let show_close = tab.is_active || hover_t > 0.05;
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
                let max_chars = (tw
                    .saturating_sub(tab_padding_h * 2 + close_reserved + security_reserved)
                    / self.cell_width) as usize;
                let title: String = tab.title.chars().take(max_chars).collect();
                let text_x = tab_x + tab_padding_h;

                for (ci, ch) in title.chars().enumerate() {
                    let cx = text_x + ci as u32 * self.cell_width;
                    self.draw_char_at(buffer, buf_width, bar_h, cx, text_y, ch, fg);
                }

                // Security badge.
                if let Some((sx, sy, sw, _sh)) =
                    self.security_badge_rect(i, tabs.len(), buf_width as u32, tab.security_count)
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

                if show_close {
                    self.draw_close_button(
                        buffer,
                        buf_width,
                        bar_h,
                        i,
                        tab,
                        tw,
                        tab.close_hover_progress,
                    );
                }
            }
        }

        // New-tab (+) button after the last tab.
        let plus_rect = self.plus_button_rect(tabs.len(), tw);
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

        // Window control buttons (non-macOS).
        #[cfg(not(target_os = "macos"))]
        self.draw_window_buttons(buffer, buf_width, bar_h, mouse_pos);

        // 1px bottom separator between bar and terminal.
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

    /// Draws the 3 window control buttons at the right edge (non-macOS).
    #[cfg(not(target_os = "macos"))]
    fn draw_window_buttons(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        mouse_pos: (f64, f64),
    ) {
        let bar_h = self.tab_bar_height_px();
        let btn_w = self.scaled_px(WIN_BTN_WIDTH);
        let bw = buf_width as u32;

        // Button positions from right: Close, Maximize, Minimize.
        let buttons: [(u32, WindowButton); 3] = [
            (bw.saturating_sub(btn_w * 3), WindowButton::Minimize),
            (bw.saturating_sub(btn_w * 2), WindowButton::Maximize),
            (bw.saturating_sub(btn_w), WindowButton::Close),
        ];

        for &(btn_x, ref btn_type) in &buttons {
            let is_hovered = mouse_pos.0 >= btn_x as f64
                && mouse_pos.0 < (btn_x + btn_w) as f64
                && mouse_pos.1 >= 0.0
                && mouse_pos.1 < bar_h as f64;

            // Hover background.
            if is_hovered {
                let hover_bg = if *btn_type == WindowButton::Close {
                    WIN_BTN_CLOSE_HOVER
                } else {
                    WIN_BTN_HOVER
                };
                for py in 0..bar_h as usize {
                    for px in btn_x as usize..(btn_x + btn_w) as usize {
                        if px < buf_width && py < buf_height {
                            let idx = py * buf_width + px;
                            if idx < buffer.len() {
                                buffer[idx] = hover_bg;
                            }
                        }
                    }
                }
            }

            let icon_color = if is_hovered && *btn_type == WindowButton::Close {
                0xFFFFFF
            } else {
                WIN_BTN_ICON
            };

            let center_x = btn_x as f32 + btn_w as f32 / 2.0;
            let center_y = bar_h as f32 / 2.0;
            let thickness = (1.25_f32 * self.ui_scale() as f32).clamp(1.15, 2.2);

            match btn_type {
                WindowButton::Minimize => {
                    // Thin horizontal line.
                    let half_w = self.scaled_px(5) as f32;
                    Self::draw_stroked_line(
                        buffer,
                        buf_width,
                        buf_height,
                        center_x - half_w,
                        center_y,
                        center_x + half_w,
                        center_y,
                        thickness,
                        icon_color,
                    );
                }
                WindowButton::Maximize => {
                    // Small rectangle outline (10x10px scaled).
                    let half = self.scaled_px(5) as f32;
                    let x0 = center_x - half;
                    let y0 = center_y - half;
                    let x1 = center_x + half;
                    let y1 = center_y + half;
                    // Top.
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height, x0, y0, x1, y0, thickness, icon_color,
                    );
                    // Bottom.
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height, x0, y1, x1, y1, thickness, icon_color,
                    );
                    // Left.
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height, x0, y0, x0, y1, thickness, icon_color,
                    );
                    // Right.
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height, x1, y0, x1, y1, thickness, icon_color,
                    );
                }
                WindowButton::Close => {
                    // X shape.
                    let half = self.scaled_px(5) as f32 * 0.7;
                    Self::draw_stroked_line(
                        buffer,
                        buf_width,
                        buf_height,
                        center_x - half,
                        center_y - half,
                        center_x + half,
                        center_y + half,
                        thickness,
                        icon_color,
                    );
                    Self::draw_stroked_line(
                        buffer,
                        buf_width,
                        buf_height,
                        center_x + half,
                        center_y - half,
                        center_x - half,
                        center_y + half,
                        thickness,
                        icon_color,
                    );
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
