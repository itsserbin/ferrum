#![cfg_attr(target_os = "macos", allow(dead_code))]

mod buttons;
mod drag_overlay;
mod layout;
mod primitives;
mod rename_field;
mod tab_content;

use super::shared::tab_math;

use super::*;
use super::RenderTarget;

// Tab-bar palette constants (BAR_BG, TAB_TEXT_ACTIVE, INSERTION_COLOR, etc.)
// are centralized in the parent `renderer/mod.rs` and imported via `use super::*`.
// WIN_BTN_WIDTH comes from `shared::tab_math`.

impl CpuRenderer {
    /// Draws top tab bar including tabs, controls, and separators.
    pub fn draw_tab_bar(
        &mut self,
        target: &mut RenderTarget<'_>,
        tabs: &[TabInfo],
        hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
        tab_offsets: Option<&[f32]>,
        pinned: bool,
    ) {
        let (buffer, buf_width, buf_height) = (&mut *target.buffer, target.width, target.height);
        let tab_bar_height = self.tab_bar_height_px();
        let bar_h = tab_bar_height as usize;
        let tw = self.tab_width(tabs.len(), buf_width as u32);
        let use_numbers = self.should_show_number(tw);

        self.draw_tab_bar_background(buffer, buf_width, buf_height, tab_bar_height);

        for (i, tab) in tabs.iter().enumerate() {
            let anim_offset = tab_offsets.and_then(|o| o.get(i)).copied().unwrap_or(0.0);
            let tab_x = (self.tab_origin_x(i, tw) as f32 + anim_offset).round() as u32;
            let is_hovered = hovered_tab == Some(i);

            self.draw_tab_background(buffer, buf_width, bar_h, tab, tab_x, tw, tab_bar_height);

            if tab.is_renaming {
                self.draw_tab_rename_field(
                    buffer,
                    buf_width,
                    bar_h,
                    tab,
                    tab_x,
                    tw,
                    tab_bar_height,
                );
            } else if use_numbers {
                self.draw_tab_number(
                    buffer,
                    buf_width,
                    bar_h,
                    i,
                    tab,
                    tab_x,
                    tw,
                    tab_bar_height,
                    is_hovered,
                );
            } else {
                self.draw_tab_content(
                    buffer,
                    buf_width,
                    bar_h,
                    i,
                    tab,
                    tabs.len(),
                    tab_x,
                    tw,
                    tab_bar_height,
                    is_hovered,
                );
            }
        }

        self.draw_plus_button(buffer, buf_width, bar_h, tabs.len(), tw, mouse_pos);

        #[cfg(not(target_os = "macos"))]
        self.draw_pin_button(buffer, buf_width, bar_h, mouse_pos, pinned);

        #[cfg(target_os = "macos")]
        let _ = pinned;

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
    #[allow(clippy::too_many_arguments)]
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
        let plus_hover = tab_math::point_in_rect(mouse_pos.0, mouse_pos.1, plus_rect);
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
    fn draw_bottom_separator(&self, buffer: &mut [u32], buf_width: usize, bar_h: usize) {
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
}
