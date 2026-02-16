#![cfg_attr(target_os = "macos", allow(dead_code))]

//! Tab bar layout math and drawing helpers for the GPU renderer.

use super::{
    ACTIVE_TAB_BG, BAR_BG, INACTIVE_TAB_HOVER, INSERTION_COLOR, MIN_TAB_WIDTH,
    MIN_TAB_WIDTH_FOR_TITLE, TAB_BORDER, TAB_TEXT_ACTIVE, TAB_TEXT_INACTIVE,
};

use super::super::TabInfo;

#[cfg(not(target_os = "macos"))]
use super::WIN_BTN_WIDTH;

impl super::GpuRenderer {
    // ── Tab bar math (mirrors CpuRenderer) ────────────────────────────

    pub(super) fn tab_strip_start_x_val(&self) -> u32 {
        #[cfg(target_os = "macos")]
        {
            self.metrics.scaled_px(78)
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.metrics.scaled_px(8)
        }
    }

    pub(super) fn plus_button_reserved_width(&self) -> u32 {
        self.metrics.cell_width + self.metrics.scaled_px(20)
    }

    pub(super) fn window_buttons_reserved_width(&self) -> u32 {
        #[cfg(not(target_os = "macos"))]
        {
            self.metrics.scaled_px(WIN_BTN_WIDTH) * 3
        }
        #[cfg(target_os = "macos")]
        {
            0
        }
    }

    pub(super) fn tab_width_val(&self, tab_count: usize, buf_width: u32) -> u32 {
        let reserved = self.tab_strip_start_x_val()
            + self.plus_button_reserved_width()
            + self.metrics.scaled_px(8)
            + self.window_buttons_reserved_width();
        let available = buf_width.saturating_sub(reserved);
        let min_tw = self.metrics.scaled_px(MIN_TAB_WIDTH);
        let max_tw = self.metrics.scaled_px(240);
        (available / tab_count.max(1) as u32).clamp(min_tw, max_tw)
    }

    pub(super) fn tab_origin_x_val(&self, tab_index: usize, tw: u32) -> u32 {
        self.tab_strip_start_x_val() + tab_index as u32 * tw
    }

    pub(super) fn close_button_rect(&self, tab_index: usize, tw: u32) -> (u32, u32, u32, u32) {
        let btn_size = self.metrics.scaled_px(20);
        let x = self.tab_origin_x_val(tab_index, tw) + tw - btn_size - self.metrics.scaled_px(6);
        let y = (self.metrics.tab_bar_height_px().saturating_sub(btn_size)) / 2;
        (x, y, btn_size, btn_size)
    }

    pub(super) fn plus_button_rect(&self, tab_count: usize, tw: u32) -> (u32, u32, u32, u32) {
        let btn_size = self.metrics.scaled_px(24);
        let x = self.tab_strip_start_x_val() + tab_count as u32 * tw + self.metrics.scaled_px(4);
        let y = (self.metrics.tab_bar_height_px().saturating_sub(btn_size)) / 2;
        (x, y, btn_size, btn_size)
    }

    pub(super) fn should_show_number(&self, tw: u32) -> bool {
        tw < self.metrics.scaled_px(MIN_TAB_WIDTH_FOR_TITLE)
    }

    pub(super) fn security_badge_rect_val(
        &self,
        tab_index: usize,
        tab_count: usize,
        buf_width: u32,
        security_count: usize,
    ) -> Option<(u32, u32, u32, u32)> {
        if security_count == 0 || tab_index >= tab_count {
            return None;
        }
        let tw = self.tab_width_val(tab_count, buf_width);
        let tab_x = self.tab_origin_x_val(tab_index, tw);
        let badge_min = self.metrics.scaled_px(10);
        let badge_max = self.metrics.scaled_px(15);
        let badge_size = self
            .metrics
            .cell_height
            .saturating_sub(self.metrics.scaled_px(10))
            .clamp(badge_min, badge_max);
        let count_chars = if security_count > 1 {
            security_count.min(99).to_string().len() as u32
        } else {
            0
        };
        let count_width = if count_chars > 0 {
            count_chars * self.metrics.cell_width + self.metrics.scaled_px(2)
        } else {
            0
        };
        let indicator_width = badge_size + count_width;
        let right_gutter = self.metrics.cell_width + self.metrics.scaled_px(10);
        let indicator_right = tab_x + tw.saturating_sub(right_gutter);
        let x = indicator_right.saturating_sub(indicator_width + self.metrics.scaled_px(2));
        let y = (self.metrics.tab_bar_height_px().saturating_sub(badge_size)) / 2;
        Some((x, y, badge_size, badge_size))
    }

    pub(super) fn point_in_rect(x: f64, y: f64, rect: (u32, u32, u32, u32)) -> bool {
        let (rx, ry, rw, rh) = rect;
        x >= rx as f64 && x < (rx + rw) as f64 && y >= ry as f64 && y < (ry + rh) as f64
    }

    // ── Tab bar trait method implementations ──────────────────────────

    pub(super) fn draw_tab_bar_impl(
        &mut self,
        buf_width: usize,
        tabs: &[TabInfo],
        hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
        tab_offsets: Option<&[f32]>,
    ) {
        let tab_bar_h = self.metrics.tab_bar_height_px() as f32;
        let bw = buf_width as u32;

        // Bar background.
        self.push_rounded_rect(
            0.0,
            0.0,
            bw as f32,
            tab_bar_h,
            self.metrics.scaled_px(10) as f32,
            BAR_BG,
            1.0,
        );

        let tw = self.tab_width_val(tabs.len(), bw);
        let text_y = (self
            .metrics
            .tab_bar_height_px()
            .saturating_sub(self.metrics.cell_height))
            / 2
            + self.metrics.scaled_px(1);
        let tab_padding_h = self.metrics.scaled_px(14);
        let use_numbers = self.should_show_number(tw);

        for (i, tab) in tabs.iter().enumerate() {
            let anim_offset = tab_offsets.and_then(|o| o.get(i)).copied().unwrap_or(0.0);
            let tab_x = self.tab_origin_x_val(i, tw) as f32 + anim_offset;
            let is_hovered = hovered_tab == Some(i);

            // Tab background.
            if tab.is_active {
                self.push_rect(tab_x, 0.0, tw as f32, tab_bar_h, ACTIVE_TAB_BG, 1.0);
            } else if is_hovered {
                self.push_rect(tab_x, 0.0, tw as f32, tab_bar_h, INACTIVE_TAB_HOVER, 1.0);
            }

            let fg_color = if tab.is_active {
                TAB_TEXT_ACTIVE
            } else {
                TAB_TEXT_INACTIVE
            };

            if tab.is_renaming {
                // Simplified rename rendering -- just show the text.
                let rename_text = tab.rename_text.unwrap_or("");
                let text_x = tab_x + tab_padding_h as f32;
                let max_chars =
                    (tw.saturating_sub(tab_padding_h * 2) / self.metrics.cell_width) as usize;
                let display: String = rename_text.chars().take(max_chars).collect();
                self.push_text(text_x, text_y as f32, &display, TAB_TEXT_ACTIVE, 1.0);
            } else if use_numbers {
                let number_str = (i + 1).to_string();
                let show_close = tab.is_active || is_hovered;
                let close_reserved = if show_close {
                    self.metrics.scaled_px(20) + self.metrics.scaled_px(6)
                } else {
                    0
                };
                let text_w = number_str.len() as u32 * self.metrics.cell_width;
                let tx = tab_x + (tw.saturating_sub(text_w + close_reserved)) as f32 / 2.0;
                self.push_text(tx, text_y as f32, &number_str, fg_color, 1.0);

                if show_close {
                    self.draw_close_button_commands(i, tw, mouse_pos);
                }
            } else {
                // Normal mode: title + close button.
                let show_close = tab.is_active || is_hovered;
                let close_reserved = if show_close {
                    self.metrics.scaled_px(20) + self.metrics.scaled_px(6)
                } else {
                    0
                };
                let security_reserved = if tab.security_count > 0 {
                    let count_chars = tab.security_count.min(99).to_string().len() as u32;
                    let count_width = if tab.security_count > 1 {
                        count_chars * self.metrics.cell_width + self.metrics.scaled_px(2)
                    } else {
                        0
                    };
                    let badge_min = self.metrics.scaled_px(10);
                    let badge_max = self.metrics.scaled_px(15);
                    self.metrics
                        .cell_height
                        .saturating_sub(self.metrics.scaled_px(10))
                        .clamp(badge_min, badge_max)
                        + count_width
                        + self.metrics.scaled_px(6)
                } else {
                    0
                };
                let max_chars = (tw
                    .saturating_sub(tab_padding_h * 2 + close_reserved + security_reserved)
                    / self.metrics.cell_width) as usize;
                let title: String = tab.title.chars().take(max_chars).collect();
                let tx = tab_x + tab_padding_h as f32;
                self.push_text(tx, text_y as f32, &title, fg_color, 1.0);

                if show_close {
                    self.draw_close_button_commands(i, tw, mouse_pos);
                }
            }
        }

        // New-tab (+) button.
        let plus_rect = self.plus_button_rect(tabs.len(), tw);
        let plus_hover = Self::point_in_rect(mouse_pos.0, mouse_pos.1, plus_rect);
        if plus_hover {
            let (px, py, pw, ph) = plus_rect;
            self.push_rounded_rect(
                px as f32,
                py as f32,
                pw as f32,
                ph as f32,
                self.metrics.scaled_px(5) as f32,
                INACTIVE_TAB_HOVER,
                1.0,
            );
        }
        let plus_fg = if plus_hover {
            TAB_TEXT_ACTIVE
        } else {
            TAB_TEXT_INACTIVE
        };
        let (px, py, pw, ph) = plus_rect;
        let center_x = px as f32 + pw as f32 * 0.5;
        let center_y = py as f32 + ph as f32 * 0.5;
        let half = (pw.min(ph) as f32 * 0.25).clamp(2.5, 5.0);
        let thickness = (1.25 * self.metrics.ui_scale as f32).clamp(1.15, 2.2);
        self.push_line(
            center_x - half,
            center_y,
            center_x + half,
            center_y,
            thickness,
            plus_fg,
            1.0,
        );
        self.push_line(
            center_x,
            center_y - half,
            center_x,
            center_y + half,
            thickness,
            plus_fg,
            1.0,
        );

        // Window control buttons (non-macOS).
        #[cfg(not(target_os = "macos"))]
        self.draw_window_buttons_commands(bw, mouse_pos);

        // Bottom separator line.
        let sep_y = tab_bar_h - 1.0;
        self.push_rect(0.0, sep_y, bw as f32, 1.0, TAB_BORDER, 0.7);
    }

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
        let text_y = (self
            .metrics
            .tab_bar_height_px()
            .saturating_sub(self.metrics.cell_height))
            / 2
            + self.metrics.scaled_px(1);
        let use_numbers = self.should_show_number(tw);
        let label: String = if use_numbers {
            (source_index + 1).to_string()
        } else {
            let pad = self.metrics.scaled_px(14);
            let max = (tw.saturating_sub(pad * 2) / self.metrics.cell_width) as usize;
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
        let idx = hovered_tab?;
        let tab = tabs.get(idx)?;
        if tab.is_renaming || tab.title.is_empty() {
            return None;
        }

        let tw = self.tab_width_val(tabs.len(), buf_width);
        if self.should_show_number(tw) {
            return Some(tab.title);
        }

        let tab_padding_h = self.metrics.scaled_px(14);
        let show_close = tab.is_active || hovered_tab == Some(idx);
        let close_reserved = if show_close {
            self.metrics.scaled_px(20) + self.metrics.scaled_px(6)
        } else {
            0
        };
        let security_reserved = if tab.security_count > 0 {
            let count_chars = tab.security_count.min(99).to_string().len() as u32;
            let count_width = if tab.security_count > 1 {
                count_chars * self.metrics.cell_width + self.metrics.scaled_px(2)
            } else {
                0
            };
            let badge_min = self.metrics.scaled_px(10);
            let badge_max = self.metrics.scaled_px(15);
            self.metrics
                .cell_height
                .saturating_sub(self.metrics.scaled_px(10))
                .clamp(badge_min, badge_max)
                + count_width
                + self.metrics.scaled_px(6)
        } else {
            0
        };
        let max_chars = (tw.saturating_sub(tab_padding_h * 2 + close_reserved + security_reserved)
            / self.metrics.cell_width) as usize;
        let title_chars = tab.title.chars().count();
        (title_chars > max_chars).then_some(tab.title)
    }

    pub(super) fn tab_insert_index_from_x_impl(
        &self,
        x: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> usize {
        let tw = self.tab_width_val(tab_count, buf_width);
        let start = self.tab_strip_start_x_val() as f64;
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
}
