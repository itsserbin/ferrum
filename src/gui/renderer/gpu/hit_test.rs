//! Hit testing and popup drawing for the GPU renderer.

use crate::core::Color;

use super::super::{ContextAction, ContextMenu, SecurityPopup, TabBarHit, TabInfo};
#[cfg(not(target_os = "macos"))]
use super::super::WindowButton;

#[cfg(not(target_os = "macos"))]
use super::WIN_BTN_WIDTH;

impl super::GpuRenderer {
    // ── Hit testing ───────────────────────────────────────────────────

    pub(super) fn hit_test_tab_bar_impl(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> TabBarHit {
        if y >= self.metrics.tab_bar_height_px() as f64 {
            return TabBarHit::Empty;
        }

        #[cfg(not(target_os = "macos"))]
        if let Some(btn) = self.window_button_at_position_impl(x, y, buf_width) {
            return TabBarHit::WindowButton(btn);
        }

        let tw = self.tab_width_val(tab_count, buf_width);
        let tab_strip_start = self.tab_strip_start_x_val();

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
            if x >= cx as f64
                && x < (cx + cw) as f64
                && y >= cy as f64
                && y < (cy + ch) as f64
            {
                return TabBarHit::CloseTab(idx);
            }
            return TabBarHit::Tab(idx);
        }

        TabBarHit::Empty
    }

    pub(super) fn hit_test_tab_hover_impl(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> Option<usize> {
        if y >= self.metrics.tab_bar_height_px() as f64 || tab_count == 0 {
            return None;
        }
        let tw = self.tab_width_val(tab_count, buf_width);
        let tab_strip_start = self.tab_strip_start_x_val();
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

    pub(super) fn hit_test_tab_security_badge_impl(
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
                self.security_badge_rect_val(idx, tabs.len(), buf_width, tab.security_count)
            else {
                continue;
            };
            if x >= sx as f64
                && x < (sx + sw) as f64
                && y >= sy as f64
                && y < (sy + sh) as f64
            {
                return Some(idx);
            }
        }
        None
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn window_button_at_position_impl(
        &self,
        x: f64,
        y: f64,
        buf_width: u32,
    ) -> Option<WindowButton> {
        let bar_h = self.metrics.tab_bar_height_px();
        if y >= bar_h as f64 {
            return None;
        }
        let btn_w = self.metrics.scaled_px(WIN_BTN_WIDTH);
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

    // ── Context menu ──────────────────────────────────────────────────

    pub(super) fn draw_context_menu_impl(&mut self, menu: &ContextMenu) {
        let mw = menu.width(self.metrics.cell_width) as f32;
        let ih = menu.item_height(self.metrics.cell_height) as f32;
        let mh = menu.height(self.metrics.cell_height) as f32;
        let mx = menu.x as f32;
        let my = menu.y as f32;
        let radius = self.metrics.scaled_px(6) as f32;

        // Background.
        self.push_rounded_rect(mx, my, mw, mh, radius, 0x1E2433, 0.97);
        self.push_rounded_rect(mx, my, mw, mh, radius, 0xFFFFFF, 0.08);

        for (i, (action, label)) in menu.items.iter().enumerate() {
            let item_y = my + self.metrics.scaled_px(2) as f32 + i as f32 * ih;

            if menu.hover_index == Some(i) {
                let hover_x = mx + self.metrics.scaled_px(4) as f32;
                let hover_w = mw - self.metrics.scaled_px(8) as f32;
                let hover_h = ih - self.metrics.scaled_px(1) as f32;
                self.push_rounded_rect(hover_x, item_y, hover_w, hover_h, radius, 0x31394D, 0.86);
            }

            let fg = if *action == ContextAction::Close {
                0xF38BA8
            } else {
                Color::DEFAULT_FG.to_pixel()
            };

            let text_x = mx + self.metrics.cell_width as f32;
            let text_y = item_y + self.metrics.scaled_px(2) as f32;
            self.push_text(text_x, text_y, label, fg, 1.0);
        }
    }

    pub(super) fn hit_test_context_menu_impl(
        &self,
        menu: &ContextMenu,
        x: f64,
        y: f64,
    ) -> Option<usize> {
        let mw = menu.width(self.metrics.cell_width);
        let ih = menu.item_height(self.metrics.cell_height);
        let mh = menu.height(self.metrics.cell_height);

        if x < menu.x as f64
            || x >= (menu.x + mw) as f64
            || y < menu.y as f64
            || y >= (menu.y + mh) as f64
        {
            return None;
        }

        let rel_y = (y - menu.y as f64 - 2.0) as u32;
        let idx = rel_y / ih;
        if (idx as usize) < menu.items.len() {
            Some(idx as usize)
        } else {
            None
        }
    }

    // ── Security ──────────────────────────────────────────────────────

    pub(super) fn draw_security_popup_impl(
        &mut self,
        buf_width: usize,
        buf_height: usize,
        popup: &SecurityPopup,
    ) {
        let pw = popup.width(self.metrics.cell_width);
        let ph = popup.height(self.metrics.cell_height);
        let width = pw.min(buf_width as u32) as f32;
        let height = ph.min(buf_height as u32) as f32;
        let x = popup.x.min((buf_width as u32).saturating_sub(pw)) as f32;
        let y = popup.y.min((buf_height as u32).saturating_sub(ph)) as f32;
        let radius = self.metrics.scaled_px(6) as f32;

        self.push_rounded_rect(x, y, width, height, radius, 0x1E2433, 0.97);
        self.push_rounded_rect(x, y, width, height, radius, 0xFFFFFF, 0.08);

        // Title.
        let header_y = y + self.metrics.scaled_px(2) as f32;
        let header_x = x + self.metrics.cell_width as f32 / 2.0;
        self.push_text(
            header_x,
            header_y,
            popup.title,
            super::super::SECURITY_ACCENT.to_pixel(),
            1.0,
        );

        // Separator line.
        let line_h = popup.line_height(self.metrics.cell_height) as f32;
        let sep_y = y + line_h;
        self.push_rect(
            x + self.metrics.scaled_px(3) as f32,
            sep_y,
            width - self.metrics.scaled_px(6) as f32,
            1.0,
            super::super::SECURITY_ACCENT.to_pixel(),
            0.47,
        );

        // Content lines.
        for (line_idx, line) in popup.lines.iter().enumerate() {
            let text_y = y + line_h + self.metrics.scaled_px(4) as f32 + line_idx as f32 * line_h;
            let text_x = x + self.metrics.cell_width as f32 / 2.0;
            let full_line = format!("\u{2022} {}", line);
            self.push_text(text_x, text_y, &full_line, Color::DEFAULT_FG.to_pixel(), 1.0);
        }
    }

    pub(super) fn hit_test_security_popup_impl(
        &self,
        popup: &SecurityPopup,
        x: f64,
        y: f64,
        buf_width: usize,
        buf_height: usize,
    ) -> bool {
        let pw = popup.width(self.metrics.cell_width);
        let ph = popup.height(self.metrics.cell_height);
        let width = pw.min(buf_width as u32);
        let height = ph.min(buf_height as u32);
        let px = popup.x.min((buf_width as u32).saturating_sub(pw));
        let py = popup.y.min((buf_height as u32).saturating_sub(ph));
        x >= px as f64 && x < (px + width) as f64 && y >= py as f64 && y < (py + height) as f64
    }

    pub(super) fn security_badge_rect_impl(
        &self,
        tab_index: usize,
        tab_count: usize,
        buf_width: u32,
        security_count: usize,
    ) -> Option<(u32, u32, u32, u32)> {
        self.security_badge_rect_val(tab_index, tab_count, buf_width, security_count)
    }
}
