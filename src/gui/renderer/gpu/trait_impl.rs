use crate::core::{CursorStyle, Grid, Selection};
use crate::gui::pane::PaneRect;

use super::super::traits;
use super::super::{RenderTarget, ScrollbarState, SecurityPopup};
#[cfg(not(target_os = "macos"))]
use super::super::TabBarDrawParams;
#[cfg(not(target_os = "macos"))]
use super::super::TabInfo;
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

    #[cfg(not(target_os = "macos"))]
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

    fn pane_inner_padding_px(&self) -> u32 {
        self.metrics.pane_inner_padding_px()
    }

    fn split_divider_color_pixel(&self) -> u32 {
        self.palette.split_divider_color.to_pixel()
    }

    fn default_bg_pixel(&self) -> u32 {
        self.palette.default_bg.to_pixel()
    }

    // ── Terminal rendering ────────────────────────────────────────────

    fn render(
        &mut self,
        _target: &mut RenderTarget<'_>,
        grid: &Grid,
        selection: Option<&Selection>,
        viewport_start: usize,
    ) {
        let padding = self.metrics.window_padding_px();
        let max_width = self.width.saturating_sub(padding.saturating_mul(2));
        let max_height = self
            .height
            .saturating_sub(self.metrics.tab_bar_height_px() + padding.saturating_mul(2));
        let region = PaneRect { x: 0, y: 0, width: max_width, height: max_height };
        self.queue_grid_batch(grid, selection, viewport_start, region, 0.0);
    }

    fn draw_cursor(
        &mut self,
        _target: &mut RenderTarget<'_>,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
    ) {
        self.draw_cursor_impl(row, col, grid, style);
    }

    fn render_in_rect(
        &mut self,
        _target: &mut RenderTarget<'_>,
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
        let region = PaneRect { x: origin_x, y: origin_y, width: rect.width, height: rect.height };
        self.queue_grid_batch(grid, selection, viewport_start, region, fg_dim);
    }

    fn draw_cursor_in_rect(
        &mut self,
        _target: &mut RenderTarget<'_>,
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
        _target: &mut RenderTarget<'_>,
        state: &ScrollbarState,
        rect: PaneRect,
    ) {
        self.render_scrollbar_in_rect_impl(state, rect);
    }

    fn draw_pane_divider(&mut self, rect: PaneRect) {
        self.push_rect(
            rect.x as f32,
            rect.y as f32,
            rect.width as f32,
            rect.height as f32,
            self.palette.split_divider_color.to_pixel(),
            1.0,
        );
    }

    // ── Scrollbar ─────────────────────────────────────────────────────

    fn render_scrollbar(
        &mut self,
        target: &mut RenderTarget<'_>,
        state: &ScrollbarState,
    ) {
        self.render_scrollbar_impl(target.height, state);
    }

    // ── Tab bar (delegates to tab_layout) ─────────────────────────────

    #[cfg(not(target_os = "macos"))]
    fn draw_tab_bar(
        &mut self,
        target: &mut RenderTarget<'_>,
        params: &TabBarDrawParams<'_>,
    ) {
        self.draw_tab_bar_impl(target.width, params);
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
        self.draw_tab_drag_overlay_impl(target.width, tabs, source_index, current_x, indicator_x);
    }

    #[cfg(not(target_os = "macos"))]
    fn draw_tab_tooltip(
        &mut self,
        target: &mut RenderTarget<'_>,
        mouse_pos: (f64, f64),
        title: &str,
    ) {
        self.draw_tab_tooltip_impl(target.width, target.height, mouse_pos, title);
    }

    // ── Security ──────────────────────────────────────────────────────

    fn draw_security_popup(
        &mut self,
        target: &mut RenderTarget<'_>,
        popup: &SecurityPopup,
    ) {
        self.draw_security_popup_impl(target.width, target.height, popup);
    }
}
