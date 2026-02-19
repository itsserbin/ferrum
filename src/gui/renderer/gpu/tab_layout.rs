#![cfg_attr(target_os = "macos", allow(dead_code))]

//! Tab bar layout math and drawing helpers for the GPU renderer.

use super::super::shared::tab_math::{self, TabLayoutMetrics};
use super::{
    ACTIVE_TAB_BG, BAR_BG, INACTIVE_TAB_HOVER, INSERTION_COLOR, RENAME_FIELD_BG,
    RENAME_FIELD_BORDER, RENAME_SELECTION_BG, TAB_BORDER, TAB_TEXT_ACTIVE, TAB_TEXT_INACTIVE,
};

// Pin button active color (Catppuccin Mocha Lavender).
#[cfg(not(target_os = "macos"))]
const PIN_ACTIVE_COLOR: u32 = 0xB4BEFE;

use super::super::TabInfo;

#[cfg(not(target_os = "macos"))]
use super::WIN_BTN_WIDTH;

impl super::GpuRenderer {
    // ── Tab bar math (delegates to shared tab_math) ──────────────────────

    /// Builds a `TabLayoutMetrics` from the current GPU renderer state.
    fn tab_layout_metrics(&self) -> TabLayoutMetrics {
        TabLayoutMetrics {
            cell_width: self.metrics.cell_width,
            cell_height: self.metrics.cell_height,
            ui_scale: self.metrics.ui_scale,
            tab_bar_height: self.metrics.tab_bar_height_px(),
        }
    }

    pub(super) fn tab_strip_start_x_val(&self) -> u32 {
        let m = self.tab_layout_metrics();
        tab_math::tab_strip_start_x(&m)
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

    pub(super) fn point_in_rect(x: f64, y: f64, rect: (u32, u32, u32, u32)) -> bool {
        let (rx, ry, rw, rh) = rect;
        x >= rx as f64 && x < (rx + rw) as f64 && y >= ry as f64 && y < (ry + rh) as f64
    }

    // ── Tab bar rendering: orchestrator ─────────────────────────────────

    pub(super) fn draw_tab_bar_impl(
        &mut self,
        buf_width: usize,
        tabs: &[TabInfo],
        _hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
        tab_offsets: Option<&[f32]>,
        _pinned: bool,
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

            self.tab_background_commands(tab, tab_x, tw);

            if tab.is_renaming {
                self.tab_rename_commands(tab, tab_x, tw, text_y);
            } else if use_numbers {
                self.tab_number_commands(i, tab, tab_x, tw, text_y);
            } else {
                self.tab_content_commands(i, tab, tab_x, tw, text_y);
            }
        }

        self.plus_button_commands(tabs.len(), tw, mouse_pos);

        #[cfg(not(target_os = "macos"))]
        self.draw_pin_button_commands(mouse_pos, _pinned);

        #[cfg(not(target_os = "macos"))]
        self.draw_window_buttons_commands(bw, mouse_pos);

        // Bottom separator line.
        let tab_bar_h = self.metrics.tab_bar_height_px() as f32;
        let sep_y = tab_bar_h - 1.0;
        self.push_rect(0.0, sep_y, bw as f32, 1.0, TAB_BORDER, 0.7);
    }

    // ── Tab bar rendering: sub-functions ────────────────────────────────

    /// Draws the tab bar background rectangle.
    fn tab_bar_background_commands(&mut self, buf_width: u32) {
        let tab_bar_h = self.metrics.tab_bar_height_px() as f32;
        self.push_rounded_rect(
            0.0,
            0.0,
            buf_width as f32,
            tab_bar_h,
            self.metrics.scaled_px(10) as f32,
            BAR_BG,
            1.0,
        );
    }

    /// Draws the background for a single tab (active, hovered, or nothing).
    fn tab_background_commands(&mut self, tab: &TabInfo, tab_x: f32, tw: u32) {
        let tab_bar_h = self.metrics.tab_bar_height_px() as f32;
        let hover_t = tab.hover_progress.clamp(0.0, 1.0);

        if tab.is_active {
            self.push_rect(tab_x, 0.0, tw as f32, tab_bar_h, ACTIVE_TAB_BG, 1.0);
        } else if hover_t > 0.01 {
            self.push_rect(
                tab_x,
                0.0,
                tw as f32,
                tab_bar_h,
                INACTIVE_TAB_HOVER,
                hover_t.min(1.0),
            );
        }
    }

    /// Draws the rename-mode UI for a tab: field background, border, text,
    /// selection highlight, and cursor.
    fn tab_rename_commands(&mut self, tab: &TabInfo, tab_x: f32, tw: u32, text_y: u32) {
        let tab_padding_h = self.metrics.scaled_px(tab_math::TAB_PADDING_H);
        let rename_text = tab.rename_text.unwrap_or("");
        let text_x = tab_x + tab_padding_h as f32;
        let m = self.tab_layout_metrics();
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

        // Rename field background and border.
        let r = tab_math::rename_field_rect(&m, tab_x.round() as u32, tw);
        let field_x = tab_x + tab_padding_h.saturating_sub(self.metrics.scaled_px(3)) as f32;
        let field_y = r.y as f32;
        let field_w = r.w as f32;
        let field_h = r.h as f32;
        let radius = self.metrics.scaled_px(6) as f32;
        self.push_rounded_rect(field_x, field_y, field_w, field_h, radius, RENAME_FIELD_BG, 0.96);
        self.push_rounded_rect(
            field_x,
            field_y,
            field_w,
            field_h,
            radius,
            RENAME_FIELD_BORDER,
            0.35,
        );

        // Rename text characters with optional selection highlight.
        for (ci, ch) in rename_text.chars().take(max_chars).enumerate() {
            let cx = text_x + ci as f32 * self.metrics.cell_width as f32;
            let selected =
                selection_chars.is_some_and(|(start, end)| ci >= start && ci < end);
            if selected {
                self.push_rect(
                    cx,
                    text_y as f32,
                    self.metrics.cell_width as f32,
                    self.metrics.cell_height as f32,
                    RENAME_SELECTION_BG,
                    0.94,
                );
                self.push_text(cx, text_y as f32, &ch.to_string(), BAR_BG, 1.0);
            } else {
                self.push_text(cx, text_y as f32, &ch.to_string(), TAB_TEXT_ACTIVE, 1.0);
            }
        }

        // Rename cursor.
        let cursor_chars = rename_text
            .get(..tab.rename_cursor)
            .map_or(0, |prefix| prefix.chars().count())
            .min(max_chars);
        let cursor_x = text_x + cursor_chars as f32 * self.metrics.cell_width as f32;
        self.push_rect(
            cursor_x,
            (text_y + self.metrics.scaled_px(1)) as f32,
            self.metrics.scaled_px(2) as f32,
            self.metrics
                .cell_height
                .saturating_sub(self.metrics.scaled_px(2)) as f32,
            TAB_TEXT_ACTIVE,
            0.9,
        );
    }

    /// Draws a tab in number mode (narrow tabs): centered number + optional close button.
    fn tab_number_commands(
        &mut self,
        tab_index: usize,
        tab: &TabInfo,
        tab_x: f32,
        tw: u32,
        text_y: u32,
    ) {
        let hover_t = tab.hover_progress.clamp(0.0, 1.0);
        let fg_color = if tab.is_active {
            TAB_TEXT_ACTIVE
        } else {
            TAB_TEXT_INACTIVE
        };

        let m = self.tab_layout_metrics();
        let number_str = (tab_index + 1).to_string();
        let show_close = tab.is_active || hover_t > 0.05;
        let close_reserved = if show_close {
            tab_math::close_button_reserved_width(&m)
        } else {
            0
        };
        let text_w = number_str.len() as u32 * self.metrics.cell_width;
        let tx = tab_x + (tw.saturating_sub(text_w + close_reserved)) as f32 / 2.0;
        self.push_text(tx, text_y as f32, &number_str, fg_color, 1.0);

        if show_close {
            self.draw_close_button_commands(tab_index, tw, tab.close_hover_progress);
        }
    }

    /// Draws a tab in normal mode: title text + optional close button.
    /// Delegates to `tab_title_commands` and `tab_close_button_commands`.
    fn tab_content_commands(
        &mut self,
        tab_index: usize,
        tab: &TabInfo,
        tab_x: f32,
        tw: u32,
        text_y: u32,
    ) {
        let hover_t = tab.hover_progress.clamp(0.0, 1.0);
        let show_close = tab.is_active || hover_t > 0.05;

        self.tab_title_commands(tab, tab_x, tw, text_y, show_close);

        if show_close {
            self.draw_close_button_commands(tab_index, tw, tab.close_hover_progress);
        }
    }

    /// Draws the tab title text, truncated to fit the available space.
    fn tab_title_commands(
        &mut self,
        tab: &TabInfo,
        tab_x: f32,
        tw: u32,
        text_y: u32,
        show_close: bool,
    ) {
        let fg_color = if tab.is_active {
            TAB_TEXT_ACTIVE
        } else {
            TAB_TEXT_INACTIVE
        };

        let m = self.tab_layout_metrics();
        let max_chars = tab_math::tab_title_max_chars(&m, tw, show_close, tab.security_count);
        let tab_padding_h = self.metrics.scaled_px(tab_math::TAB_PADDING_H);
        let title: String = tab.title.chars().take(max_chars).collect();
        let tx = tab_x + tab_padding_h as f32;
        self.push_text(tx, text_y as f32, &title, fg_color, 1.0);
    }

    /// Draws the new-tab (+) button with hover highlight.
    fn plus_button_commands(&mut self, tab_count: usize, tw: u32, mouse_pos: (f64, f64)) {
        let plus_rect = self.plus_button_rect(tab_count, tw);
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
    }

    /// Draws the pin button at the left of the tab bar (non-macOS).
    #[cfg(not(target_os = "macos"))]
    fn draw_pin_button_commands(&mut self, mouse_pos: (f64, f64), pinned: bool) {
        let (pin_x, pin_y, pin_w, pin_h) = self.pin_button_rect();
        let is_hovered = Self::point_in_rect(mouse_pos.0, mouse_pos.1, (pin_x, pin_y, pin_w, pin_h));

        // Draw hover background.
        if is_hovered {
            self.push_rounded_rect(
                pin_x as f32,
                pin_y as f32,
                pin_w as f32,
                pin_h as f32,
                self.metrics.scaled_px(5) as f32,
                INACTIVE_TAB_HOVER,
                1.0,
            );
        }

        // Icon color: active (lavender) when pinned, inactive otherwise.
        let icon_color = if pinned {
            PIN_ACTIVE_COLOR
        } else if is_hovered {
            TAB_TEXT_ACTIVE
        } else {
            TAB_TEXT_INACTIVE
        };

        // Draw Bootstrap-style vertical pushpin icon
        let cx = pin_x as f32 + pin_w as f32 / 2.0;
        let cy = pin_y as f32 + pin_h as f32 / 2.0;
        let s = self.metrics.ui_scale as f32;
        let t = (1.2 * s).clamp(1.0, 2.0);

        // Dimensions (scaled)
        let head_w = 6.0 * s;      // width of top head
        let head_h = 2.0 * s;      // height of top head
        let body_w = 3.0 * s;      // width of body
        let body_h = 4.0 * s;      // height of body
        let platform_w = 7.0 * s;  // width of middle platform
        let platform_h = 1.5 * s;  // height of platform
        let needle_h = 4.0 * s;    // length of needle

        let top = cy - 6.0 * s;    // start from top

        // 1. Top head (wide rectangle)
        self.push_rect(cx - head_w / 2.0, top, head_w, head_h, icon_color, 1.0);

        // 2. Body (narrower rectangle below head)
        let body_top = top + head_h;
        self.push_rect(cx - body_w / 2.0, body_top, body_w, body_h, icon_color, 1.0);

        // 3. Platform/base (wider rectangle where pin enters surface)
        let platform_top = body_top + body_h;
        self.push_rect(cx - platform_w / 2.0, platform_top, platform_w, platform_h, icon_color, 1.0);

        // 4. Needle (thin line pointing down)
        let needle_top = platform_top + platform_h;
        self.push_line(cx, needle_top, cx, needle_top + needle_h, t, icon_color, 1.0);
    }

    // ── Other tab bar methods (unchanged) ───────────────────────────────

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
        let idx = hovered_tab?;
        let tab = tabs.get(idx)?;
        if tab.is_renaming || tab.title.is_empty() {
            return None;
        }

        let tw = self.tab_width_val(tabs.len(), buf_width);
        if self.should_show_number(tw) {
            return Some(tab.title);
        }

        let show_close = tab.is_active || hovered_tab == Some(idx);
        let m = self.tab_layout_metrics();
        let max_chars = tab_math::tab_title_max_chars(&m, tw, show_close, tab.security_count);
        let title_chars = tab.title.chars().count();
        (title_chars > max_chars).then_some(tab.title)
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
