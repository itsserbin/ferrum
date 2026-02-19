#![cfg_attr(target_os = "macos", allow(dead_code))]

//! Hit testing and popup drawing for the GPU renderer.

use super::super::shared::tab_hit_test;
use super::super::{ContextMenu, SecurityPopup, TabBarHit, TabInfo};

impl super::GpuRenderer {
    // ── Hit testing (delegates to shared tab_hit_test) ────────────────

    pub(super) fn hit_test_tab_bar_impl(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> TabBarHit {
        let m = self.tab_layout_metrics();
        tab_hit_test::hit_test_tab_bar(x, y, tab_count, buf_width, &m)
    }

    pub(super) fn hit_test_tab_hover_impl(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> Option<usize> {
        let m = self.tab_layout_metrics();
        tab_hit_test::hit_test_tab_hover(x, y, tab_count, buf_width, &m)
    }

    pub(super) fn hit_test_tab_security_badge_impl(
        &self,
        x: f64,
        y: f64,
        tabs: &[TabInfo],
        buf_width: u32,
    ) -> Option<usize> {
        let m = self.tab_layout_metrics();
        tab_hit_test::hit_test_tab_security_badge(x, y, tabs, buf_width, &m)
    }

    // ── Context menu ──────────────────────────────────────────────────

    /// Draws context menu overlay using a shared layout.
    pub(super) fn draw_context_menu_impl(&mut self, menu: &ContextMenu) {
        let layout = menu.layout(
            self.metrics.cell_width,
            self.metrics.cell_height,
            self.metrics.ui_scale,
        );

        self.push_rounded_rect(
            layout.bg.x,
            layout.bg.y,
            layout.bg.w,
            layout.bg.h,
            layout.bg.radius,
            layout.bg.color,
            layout.bg.opacity,
        );
        self.push_rounded_rect(
            layout.border.x,
            layout.border.y,
            layout.border.w,
            layout.border.h,
            layout.border.radius,
            layout.border.color,
            layout.border.opacity,
        );

        for item in &layout.items {
            if let Some(ref hover) = item.hover_rect {
                self.push_rounded_rect(
                    hover.x, hover.y, hover.w, hover.h, hover.radius, hover.color, hover.opacity,
                );
            }

            self.push_text(
                item.text.x,
                item.text.y,
                &item.text.text,
                item.text.color,
                item.text.opacity,
            );
        }
    }

    pub(super) fn hit_test_context_menu_impl(
        &self,
        menu: &ContextMenu,
        x: f64,
        y: f64,
    ) -> Option<usize> {
        menu.hit_test(x, y, self.metrics.cell_width, self.metrics.cell_height)
    }

    // ── Security ──────────────────────────────────────────────────────

    /// Draws security popup overlay using a shared layout.
    pub(super) fn draw_security_popup_impl(
        &mut self,
        buf_width: usize,
        buf_height: usize,
        popup: &SecurityPopup,
    ) {
        let layout = popup.layout(
            self.metrics.cell_width,
            self.metrics.cell_height,
            self.metrics.ui_scale,
            buf_width as u32,
            buf_height as u32,
        );

        self.push_rounded_rect(
            layout.bg.x,
            layout.bg.y,
            layout.bg.w,
            layout.bg.h,
            layout.bg.radius,
            layout.bg.color,
            layout.bg.opacity,
        );
        self.push_rounded_rect(
            layout.border.x,
            layout.border.y,
            layout.border.w,
            layout.border.h,
            layout.border.radius,
            layout.border.color,
            layout.border.opacity,
        );

        // Title.
        self.push_text(
            layout.title.x,
            layout.title.y,
            &layout.title.text,
            layout.title.color,
            layout.title.opacity,
        );

        // Separator line.
        self.push_rect(
            layout.separator.x,
            layout.separator.y,
            layout.separator.w,
            layout.separator.h,
            layout.separator.color,
            layout.separator.opacity,
        );

        // Content lines.
        for text_cmd in &layout.lines {
            self.push_text(text_cmd.x, text_cmd.y, &text_cmd.text, text_cmd.color, text_cmd.opacity);
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
        let (px, py, pw, ph) = popup.clamped_rect(
            self.metrics.cell_width,
            self.metrics.cell_height,
            buf_width as u32,
            buf_height as u32,
        );
        x >= px as f64 && x < (px + pw) as f64 && y >= py as f64 && y < (py + ph) as f64
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
