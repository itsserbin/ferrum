use crate::core::{CursorStyle, Grid, Selection};
use crate::gui::pane::PaneRect;

use super::super::traits;
use super::super::{SecurityPopup, TabInfo};
use super::GpuRenderer;

impl traits::Renderer for GpuRenderer {
    fn set_scale(&mut self, scale_factor: f64) {
        let scale = super::super::sanitize_scale(scale_factor);
        if !super::super::scale_changed(self.metrics.ui_scale, scale) {
            return;
        }
        self.metrics.ui_scale = scale;
        self.metrics.recompute(&self.font);
        self.rebuild_atlas();
    }

    fn set_tab_bar_visible(&mut self, visible: bool) {
        self.metrics.tab_bar_visible = super::super::resolve_tab_bar_visible(visible);
    }

    fn cell_width(&self) -> u32 {
        self.metrics.cell_width
    }

    fn cell_height(&self) -> u32 {
        self.metrics.cell_height
    }

    fn tab_bar_height_px(&self) -> u32 {
        self.metrics.tab_bar_height_px()
    }

    fn window_padding_px(&self) -> u32 {
        self.metrics.window_padding_px()
    }

    fn ui_scale(&self) -> f64 {
        self.metrics.ui_scale
    }

    fn scaled_px(&self, base: u32) -> u32 {
        self.metrics.scaled_px(base)
    }

    fn scrollbar_hit_zone_px(&self) -> u32 {
        self.metrics.scrollbar_hit_zone_px()
    }

    // ── Terminal rendering ────────────────────────────────────────────

    fn render(
        &mut self,
        _buffer: &mut [u32],
        _buf_width: usize,
        _buf_height: usize,
        grid: &Grid,
        selection: Option<&Selection>,
        viewport_start: usize,
    ) {
        let padding = self.metrics.window_padding_px();
        let max_width = self.width.saturating_sub(padding.saturating_mul(2));
        let max_height = self
            .height
            .saturating_sub(self.metrics.tab_bar_height_px() + padding.saturating_mul(2));
        self.queue_grid_batch(grid, selection, viewport_start, 0, 0, max_width, max_height, 0.0);
    }

    fn draw_cursor(
        &mut self,
        _buffer: &mut [u32],
        _buf_width: usize,
        _buf_height: usize,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
    ) {
        self.draw_cursor_impl(row, col, grid, style);
    }

    fn render_in_rect(
        &mut self,
        _buffer: &mut [u32],
        _buf_width: usize,
        _buf_height: usize,
        grid: &Grid,
        selection: Option<&Selection>,
        viewport_start: usize,
        rect: PaneRect,
        fg_dim: f32,
    ) {
        let padding = self.metrics.window_padding_px();
        let top = self.metrics.tab_bar_height_px().saturating_add(padding);
        let origin_x = rect.x.saturating_sub(padding);
        let origin_y = rect.y.saturating_sub(top);
        self.queue_grid_batch(
            grid,
            selection,
            viewport_start,
            origin_x,
            origin_y,
            rect.width,
            rect.height,
            fg_dim,
        );
    }

    fn draw_cursor_in_rect(
        &mut self,
        _buffer: &mut [u32],
        _buf_width: usize,
        _buf_height: usize,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
        rect: PaneRect,
    ) {
        self.draw_cursor_in_rect_impl(row, col, grid, style, rect);
    }

    fn render_scrollbar_in_rect(
        &mut self,
        _buffer: &mut [u32],
        _buf_width: usize,
        _buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
        opacity: f32,
        hover: bool,
        rect: PaneRect,
    ) {
        self.render_scrollbar_in_rect_impl(
            scroll_offset,
            scrollback_len,
            grid_rows,
            opacity,
            hover,
            rect,
        );
    }

    fn draw_pane_divider(&mut self, rect: PaneRect) {
        // Catppuccin Mocha Surface2 (same as CPU divider path).
        self.push_rect(
            rect.x as f32,
            rect.y as f32,
            rect.width as f32,
            rect.height as f32,
            0x585B70,
            1.0,
        );
    }

    // ── Scrollbar ─────────────────────────────────────────────────────

    fn render_scrollbar(
        &mut self,
        _buffer: &mut [u32],
        _buf_width: usize,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
        opacity: f32,
        hover: bool,
    ) {
        self.render_scrollbar_impl(
            buf_height,
            scroll_offset,
            scrollback_len,
            grid_rows,
            opacity,
            hover,
        );
    }

    // ── Tab bar (delegates to tab_layout) ─────────────────────────────

    fn draw_tab_bar(
        &mut self,
        _buffer: &mut [u32],
        buf_width: usize,
        _buf_height: usize,
        tabs: &[TabInfo],
        hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
        tab_offsets: Option<&[f32]>,
        pinned: bool,
    ) {
        self.draw_tab_bar_impl(buf_width, tabs, hovered_tab, mouse_pos, tab_offsets, pinned);
    }

    fn draw_tab_drag_overlay(
        &mut self,
        _buffer: &mut [u32],
        buf_width: usize,
        _buf_height: usize,
        tabs: &[TabInfo],
        source_index: usize,
        current_x: f64,
        indicator_x: f32,
    ) {
        self.draw_tab_drag_overlay_impl(buf_width, tabs, source_index, current_x, indicator_x);
    }

    fn draw_tab_tooltip(
        &mut self,
        _buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        mouse_pos: (f64, f64),
        title: &str,
    ) {
        self.draw_tab_tooltip_impl(buf_width, buf_height, mouse_pos, title);
    }

    // ── Security ──────────────────────────────────────────────────────

    fn draw_security_popup(
        &mut self,
        _buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        popup: &SecurityPopup,
    ) {
        self.draw_security_popup_impl(buf_width, buf_height, popup);
    }
}
