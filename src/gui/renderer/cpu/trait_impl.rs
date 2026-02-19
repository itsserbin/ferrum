use crate::core::{CursorStyle, Grid, Selection};

use super::super::traits;
use super::super::types::{ContextMenu, SecurityPopup, TabBarHit, TabInfo};
#[cfg(not(target_os = "macos"))]
use super::super::types::WindowButton;
use super::CpuRenderer;

impl traits::Renderer for CpuRenderer {
    fn set_scale(&mut self, scale_factor: f64) {
        CpuRenderer::set_scale(self, scale_factor);
    }

    fn set_tab_bar_visible(&mut self, visible: bool) {
        CpuRenderer::set_tab_bar_visible(self, visible);
    }

    fn cell_width(&self) -> u32 {
        self.cell_width
    }

    fn cell_height(&self) -> u32 {
        self.cell_height
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

    fn scrollbar_thumb_bounds(
        &self,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
    ) -> Option<(f32, f32)> {
        CpuRenderer::scrollbar_thumb_bounds(
            self,
            buf_height,
            scroll_offset,
            scrollback_len,
            grid_rows,
        )
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

    fn tab_hover_tooltip<'a>(
        &self,
        tabs: &'a [TabInfo<'a>],
        hovered_tab: Option<usize>,
        buf_width: u32,
    ) -> Option<&'a str> {
        CpuRenderer::tab_hover_tooltip(self, tabs, hovered_tab, buf_width)
    }

    fn tab_insert_index_from_x(&self, x: f64, tab_count: usize, buf_width: u32) -> usize {
        CpuRenderer::tab_insert_index_from_x(self, x, tab_count, buf_width)
    }

    fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32 {
        CpuRenderer::tab_width(self, tab_count, buf_width)
    }

    fn tab_strip_start_x(&self) -> u32 {
        CpuRenderer::tab_strip_start_x(self)
    }

    fn tab_origin_x(&self, tab_index: usize, tw: u32) -> u32 {
        CpuRenderer::tab_origin_x(self, tab_index, tw)
    }

    fn hit_test_tab_bar(&self, x: f64, y: f64, tab_count: usize, buf_width: u32) -> TabBarHit {
        CpuRenderer::hit_test_tab_bar(self, x, y, tab_count, buf_width)
    }

    fn hit_test_tab_hover(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> Option<usize> {
        CpuRenderer::hit_test_tab_hover(self, x, y, tab_count, buf_width)
    }

    fn hit_test_tab_security_badge(
        &self,
        x: f64,
        y: f64,
        tabs: &[TabInfo],
        buf_width: u32,
    ) -> Option<usize> {
        CpuRenderer::hit_test_tab_security_badge(self, x, y, tabs, buf_width)
    }

    #[cfg(not(target_os = "macos"))]
    fn window_button_at_position(&self, x: f64, y: f64, buf_width: u32) -> Option<WindowButton> {
        CpuRenderer::window_button_at_position(self, x, y, buf_width)
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

    fn hit_test_context_menu(&self, menu: &ContextMenu, x: f64, y: f64) -> Option<usize> {
        CpuRenderer::hit_test_context_menu(self, menu, x, y)
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

    fn hit_test_security_popup(
        &self,
        popup: &SecurityPopup,
        x: f64,
        y: f64,
        buf_width: usize,
        buf_height: usize,
    ) -> bool {
        CpuRenderer::hit_test_security_popup(self, popup, x, y, buf_width, buf_height)
    }

    fn security_badge_rect(
        &self,
        tab_index: usize,
        tab_count: usize,
        buf_width: u32,
        security_count: usize,
    ) -> Option<(u32, u32, u32, u32)> {
        CpuRenderer::security_badge_rect(self, tab_index, tab_count, buf_width, security_count)
    }
}
