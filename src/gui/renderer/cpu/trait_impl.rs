use crate::core::{CursorStyle, Grid, Selection};

use super::super::traits;
use super::super::types::{ContextMenu, SecurityPopup, TabInfo};
use super::CpuRenderer;

impl traits::Renderer for CpuRenderer {
    fn set_scale(&mut self, scale_factor: f64) {
        CpuRenderer::set_scale(self, scale_factor);
    }

    fn set_tab_bar_visible(&mut self, visible: bool) {
        CpuRenderer::set_tab_bar_visible(self, visible);
    }

    fn cell_width(&self) -> u32 {
        self.metrics.cell_width
    }

    fn cell_height(&self) -> u32 {
        self.metrics.cell_height
    }

    fn tab_bar_height_px(&self) -> u32 {
        CpuRenderer::tab_bar_height_px(self)
    }

    fn window_padding_px(&self) -> u32 {
        CpuRenderer::window_padding_px(self)
    }

    fn ui_scale(&self) -> f64 {
        CpuRenderer::ui_scale(self)
    }

    fn scaled_px(&self, base: u32) -> u32 {
        CpuRenderer::scaled_px(self, base)
    }

    fn scrollbar_hit_zone_px(&self) -> u32 {
        CpuRenderer::scrollbar_hit_zone_px(self)
    }

    fn render(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        grid: &Grid,
        selection: Option<&Selection>,
        viewport_start: usize,
    ) {
        CpuRenderer::render(
            self,
            buffer,
            buf_width,
            buf_height,
            grid,
            selection,
            viewport_start,
        );
    }

    fn draw_cursor(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
    ) {
        CpuRenderer::draw_cursor(self, buffer, buf_width, buf_height, row, col, grid, style);
    }

    fn render_scrollbar(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
        opacity: f32,
        hover: bool,
    ) {
        CpuRenderer::render_scrollbar(
            self,
            buffer,
            buf_width,
            buf_height,
            scroll_offset,
            scrollback_len,
            grid_rows,
            opacity,
            hover,
        );
    }

    fn draw_tab_bar(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        tabs: &[TabInfo],
        hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
        tab_offsets: Option<&[f32]>,
        pinned: bool,
    ) {
        CpuRenderer::draw_tab_bar(
            self,
            buffer,
            buf_width,
            buf_height,
            tabs,
            hovered_tab,
            mouse_pos,
            tab_offsets,
            pinned,
        );
    }

    fn draw_tab_drag_overlay(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        tabs: &[TabInfo],
        source_index: usize,
        current_x: f64,
        indicator_x: f32,
    ) {
        CpuRenderer::draw_tab_drag_overlay(
            self,
            buffer,
            buf_width,
            buf_height,
            tabs,
            source_index,
            current_x,
            indicator_x,
        );
    }

    fn draw_tab_tooltip(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        mouse_pos: (f64, f64),
        title: &str,
    ) {
        CpuRenderer::draw_tab_tooltip(self, buffer, buf_width, buf_height, mouse_pos, title);
    }

    fn draw_context_menu(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        menu: &ContextMenu,
    ) {
        CpuRenderer::draw_context_menu(self, buffer, buf_width, buf_height, menu);
    }

    fn draw_security_popup(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        popup: &SecurityPopup,
    ) {
        CpuRenderer::draw_security_popup(self, buffer, buf_width, buf_height, popup);
    }
}
