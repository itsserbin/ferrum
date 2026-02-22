use crate::core::{CursorStyle, Grid, Selection};
use crate::gui::pane::PaneRect;

use super::super::traits;
use super::super::types::SecurityPopup;
#[cfg(not(target_os = "macos"))]
use super::super::types::TabBarDrawParams;
#[cfg(not(target_os = "macos"))]
use super::super::TabInfo;
use super::super::{RenderTarget, ScrollbarState};
use super::CpuRenderer;

impl traits::Renderer for CpuRenderer {
    fn set_scale(&mut self, scale_factor: f64) {
        CpuRenderer::set_scale(self, scale_factor);
    }

    #[cfg(not(target_os = "macos"))]
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

    fn pane_inner_padding_px(&self) -> u32 {
        self.metrics.pane_inner_padding_px()
    }

    fn split_divider_color_pixel(&self) -> u32 {
        self.palette.split_divider_color.to_pixel()
    }

    fn default_bg_pixel(&self) -> u32 {
        self.palette.default_bg.to_pixel()
    }

    fn render(
        &mut self,
        target: &mut RenderTarget<'_>,
        grid: &Grid,
        selection: Option<&Selection>,
        viewport_start: usize,
    ) {
        CpuRenderer::render(self, target, grid, selection, viewport_start);
    }

    fn draw_cursor(
        &mut self,
        target: &mut RenderTarget<'_>,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
    ) {
        CpuRenderer::draw_cursor(self, target, row, col, grid, style);
    }

    fn render_in_rect(
        &mut self,
        target: &mut RenderTarget<'_>,
        grid: &Grid,
        selection: Option<&Selection>,
        viewport_start: usize,
        rect: PaneRect,
        fg_dim: f32,
    ) {
        CpuRenderer::render_in_rect(self, target, grid, selection, viewport_start, rect, fg_dim);
    }

    fn draw_cursor_in_rect(
        &mut self,
        target: &mut RenderTarget<'_>,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
        rect: PaneRect,
    ) {
        CpuRenderer::draw_cursor_in_rect(self, target, row, col, grid, style, rect);
    }

    fn render_scrollbar_in_rect(
        &mut self,
        target: &mut RenderTarget<'_>,
        state: &ScrollbarState,
        rect: PaneRect,
    ) {
        CpuRenderer::render_scrollbar_in_rect(self, target, state, rect);
    }

    fn render_scrollbar(
        &mut self,
        target: &mut RenderTarget<'_>,
        state: &ScrollbarState,
    ) {
        CpuRenderer::render_scrollbar(self, target, state);
    }

    #[cfg(not(target_os = "macos"))]
    fn draw_tab_bar(
        &mut self,
        target: &mut RenderTarget<'_>,
        params: &TabBarDrawParams<'_>,
    ) {
        CpuRenderer::draw_tab_bar(self, target, params);
    }

    #[cfg(not(target_os = "macos"))]
    fn draw_tab_drag_overlay(
        &mut self,
        target: &mut RenderTarget<'_>,
        tabs: &[TabInfo],
        source_index: usize,
        current_x: f64,
        indicator_x: f32,
    ) {
        CpuRenderer::draw_tab_drag_overlay(self, target, tabs, source_index, current_x, indicator_x);
    }

    #[cfg(not(target_os = "macos"))]
    fn draw_tab_tooltip(
        &mut self,
        target: &mut RenderTarget<'_>,
        mouse_pos: (f64, f64),
        title: &str,
    ) {
        CpuRenderer::draw_tab_tooltip(self, target, mouse_pos, title);
    }

    fn draw_security_popup(
        &mut self,
        target: &mut RenderTarget<'_>,
        popup: &SecurityPopup,
    ) {
        CpuRenderer::draw_security_popup(self, target, popup);
    }
}
