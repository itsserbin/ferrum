use crate::core::{CursorStyle, Grid, Selection};

#[cfg(not(target_os = "macos"))]
use super::super::WindowButton;
use super::super::traits;
use super::super::{ContextMenu, SecurityPopup, TabBarHit, TabInfo};
use super::GpuRenderer;

impl traits::Renderer for GpuRenderer {
    fn set_scale(&mut self, scale_factor: f64) {
        let scale = if scale_factor.is_finite() {
            scale_factor.clamp(0.75, 4.0)
        } else {
            1.0
        };
        const SCALE_EPSILON: f64 = 1e-6;
        if (self.metrics.ui_scale - scale).abs() < SCALE_EPSILON {
            return;
        }
        self.metrics.ui_scale = scale;
        self.metrics.recompute(&self.font);
        self.rebuild_atlas();
    }

    fn set_tab_bar_visible(&mut self, visible: bool) {
        #[cfg(target_os = "macos")]
        {
            let _ = visible;
            self.metrics.tab_bar_visible = false;
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.metrics.tab_bar_visible = visible;
        }
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

    fn scrollbar_thumb_bounds(
        &self,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
    ) -> Option<(f32, f32)> {
        self.scrollbar_thumb_bounds_impl(buf_height, scroll_offset, scrollback_len, grid_rows)
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

    fn tab_hover_tooltip<'a>(
        &self,
        tabs: &'a [TabInfo<'a>],
        hovered_tab: Option<usize>,
        buf_width: u32,
    ) -> Option<&'a str> {
        self.tab_hover_tooltip_impl(tabs, hovered_tab, buf_width)
    }

    fn tab_insert_index_from_x(&self, x: f64, tab_count: usize, buf_width: u32) -> usize {
        self.tab_insert_index_from_x_impl(x, tab_count, buf_width)
    }

    fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32 {
        self.tab_width_val(tab_count, buf_width)
    }

    fn tab_strip_start_x(&self) -> u32 {
        self.tab_strip_start_x_val()
    }

    fn tab_origin_x(&self, tab_index: usize, tw: u32) -> u32 {
        self.tab_origin_x_val(tab_index, tw)
    }

    // ── Hit testing (delegates to hit_test) ───────────────────────────

    fn hit_test_tab_bar(&self, x: f64, y: f64, tab_count: usize, buf_width: u32) -> TabBarHit {
        self.hit_test_tab_bar_impl(x, y, tab_count, buf_width)
    }

    fn hit_test_tab_hover(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> Option<usize> {
        self.hit_test_tab_hover_impl(x, y, tab_count, buf_width)
    }

    fn hit_test_tab_security_badge(
        &self,
        x: f64,
        y: f64,
        tabs: &[TabInfo],
        buf_width: u32,
    ) -> Option<usize> {
        self.hit_test_tab_security_badge_impl(x, y, tabs, buf_width)
    }

    #[cfg(not(target_os = "macos"))]
    fn window_button_at_position(&self, x: f64, y: f64, buf_width: u32) -> Option<WindowButton> {
        self.window_button_at_position_impl(x, y, buf_width)
    }

    // ── Context menu (delegates to hit_test) ─────────────────────────

    fn draw_context_menu(
        &mut self,
        _buffer: &mut [u32],
        _buf_width: usize,
        _buf_height: usize,
        menu: &ContextMenu,
    ) {
        self.draw_context_menu_impl(menu);
    }

    fn hit_test_context_menu(&self, menu: &ContextMenu, x: f64, y: f64) -> Option<usize> {
        self.hit_test_context_menu_impl(menu, x, y)
    }

    // ── Security (delegates to hit_test) ─────────────────────────────

    fn draw_security_popup(
        &mut self,
        _buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        popup: &SecurityPopup,
    ) {
        self.draw_security_popup_impl(buf_width, buf_height, popup);
    }

    fn hit_test_security_popup(
        &self,
        popup: &SecurityPopup,
        x: f64,
        y: f64,
        buf_width: usize,
        buf_height: usize,
    ) -> bool {
        self.hit_test_security_popup_impl(popup, x, y, buf_width, buf_height)
    }

    fn security_badge_rect(
        &self,
        tab_index: usize,
        tab_count: usize,
        buf_width: u32,
        security_count: usize,
    ) -> Option<(u32, u32, u32, u32)> {
        self.security_badge_rect_impl(tab_index, tab_count, buf_width, security_count)
    }
}
