use crate::core::{CursorStyle, Grid, Selection};

use super::super::traits;
use super::super::{ContextMenu, SecurityPopup, TabInfo};
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
        self.pack_grid(grid, selection, viewport_start);
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

    // ── Context menu ──────────────────────────────────────────────────

    fn draw_context_menu(
        &mut self,
        _buffer: &mut [u32],
        _buf_width: usize,
        _buf_height: usize,
        menu: &ContextMenu,
    ) {
        self.draw_context_menu_impl(menu);
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
