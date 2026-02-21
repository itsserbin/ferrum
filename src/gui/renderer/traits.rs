use crate::core::{CursorStyle, Grid, Selection};
use crate::gui::pane::PaneRect;

use super::shared::scrollbar_math;
use super::shared::tab_hit_test;
use super::shared::tab_math::{self, TabLayoutMetrics};
use super::{SCROLLBAR_MIN_THUMB, SecurityPopup, TabBarHit, TabInfo};
use super::{RenderTarget, ScrollbarState};

/// Trait defining the full renderer interface used by the GUI layer.
///
/// Both CPU (softbuffer) and GPU (wgpu) renderers implement this trait,
/// keeping the rest of the codebase backend-agnostic.
pub trait Renderer {
    // ── Lifecycle ────────────────────────────────────────────────────

    fn set_scale(&mut self, scale_factor: f64);
    fn set_tab_bar_visible(&mut self, visible: bool);

    // ── Metrics ─────────────────────────────────────────────────────

    fn cell_width(&self) -> u32;
    fn cell_height(&self) -> u32;
    fn tab_bar_height_px(&self) -> u32;
    fn window_padding_px(&self) -> u32;
    fn ui_scale(&self) -> f64;
    fn scaled_px(&self, base: u32) -> u32;
    fn scrollbar_hit_zone_px(&self) -> u32;
    fn pane_inner_padding_px(&self) -> u32;

    /// Returns the split divider color as a pixel value (0x00RRGGBB).
    ///
    /// Sourced from the active theme palette by each renderer implementation.
    fn split_divider_color_pixel(&self) -> u32;

    /// Returns the default background color as a pixel value (0x00RRGGBB).
    ///
    /// Sourced from the active theme palette by each renderer implementation.
    fn default_bg_pixel(&self) -> u32;

    /// Builds a [`TabLayoutMetrics`] from the renderer's current state.
    fn tab_layout_metrics(&self) -> TabLayoutMetrics {
        TabLayoutMetrics {
            cell_width: self.cell_width(),
            cell_height: self.cell_height(),
            ui_scale: self.ui_scale(),
            tab_bar_height: self.tab_bar_height_px(),
        }
    }

    // ── Terminal rendering ──────────────────────────────────────────

    fn render(
        &mut self,
        target: &mut RenderTarget<'_>,
        grid: &Grid,
        selection: Option<&Selection>,
        viewport_start: usize,
    );

    fn draw_cursor(
        &mut self,
        target: &mut RenderTarget<'_>,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
    );

    /// Renders terminal cells into a sub-rectangle of the buffer.
    ///
    /// Used for multi-pane rendering where each pane occupies a portion of the
    /// window.  The default implementation is a no-op (GPU renderer overrides
    /// separately).
    fn render_in_rect(
        &mut self,
        _target: &mut RenderTarget<'_>,
        _grid: &Grid,
        _selection: Option<&Selection>,
        _viewport_start: usize,
        _rect: PaneRect,
        _fg_dim: f32,
    ) {
    }

    /// Draws the cursor at a position offset by a pane rectangle.
    ///
    /// Default implementation is a no-op (GPU renderer overrides separately).
    fn draw_cursor_in_rect(
        &mut self,
        _target: &mut RenderTarget<'_>,
        _row: usize,
        _col: usize,
        _grid: &Grid,
        _style: CursorStyle,
        _rect: PaneRect,
    ) {
    }

    /// Renders the scrollbar within a pane sub-rectangle.
    ///
    /// Default implementation is a no-op (GPU renderer overrides separately).
    fn render_scrollbar_in_rect(
        &mut self,
        _target: &mut RenderTarget<'_>,
        _state: &ScrollbarState,
        _rect: PaneRect,
    ) {
    }

    /// Draws a split divider rectangle (GPU path).
    ///
    /// CPU renderer can keep using direct buffer drawing in the shared path.
    fn draw_pane_divider(&mut self, _rect: PaneRect) {}

    // ── Scrollbar ───────────────────────────────────────────────────

    fn render_scrollbar(
        &mut self,
        target: &mut RenderTarget<'_>,
        state: &ScrollbarState,
    );

    fn scrollbar_thumb_bounds(
        &self,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
    ) -> Option<(f32, f32)> {
        let (track_top, track_bottom, min_thumb) = scrollbar_math::scrollbar_track_params(
            self.tab_bar_height_px(),
            self.window_padding_px(),
            buf_height,
            SCROLLBAR_MIN_THUMB,
            self.ui_scale(),
        );

        scrollbar_math::scrollbar_thumb_geometry(
            track_top,
            track_bottom,
            scroll_offset,
            scrollback_len,
            grid_rows,
            min_thumb,
        )
    }

    // ── Tab bar ─────────────────────────────────────────────────────

    #[cfg(not(target_os = "macos"))]
    fn draw_tab_bar(
        &mut self,
        target: &mut RenderTarget<'_>,
        tabs: &[TabInfo],
        hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
        tab_offsets: Option<&[f32]>,
        pinned: bool,
        settings_open: bool,
    );

    #[cfg(not(target_os = "macos"))]
    fn draw_tab_drag_overlay(
        &mut self,
        target: &mut RenderTarget<'_>,
        tabs: &[TabInfo],
        source_index: usize,
        current_x: f64,
        indicator_x: f32,
    );

    #[cfg(not(target_os = "macos"))]
    fn draw_tab_tooltip(
        &mut self,
        target: &mut RenderTarget<'_>,
        mouse_pos: (f64, f64),
        title: &str,
    );

    #[cfg(not(target_os = "macos"))]
    fn tab_hover_tooltip<'a>(
        &self,
        tabs: &'a [TabInfo<'a>],
        hovered_tab: Option<usize>,
        buf_width: u32,
    ) -> Option<&'a str> {
        let m = self.tab_layout_metrics();
        tab_hit_test::tab_hover_tooltip(tabs, hovered_tab, buf_width, &m)
    }

    fn tab_insert_index_from_x(&self, x: f64, tab_count: usize, buf_width: u32) -> usize {
        let m = self.tab_layout_metrics();
        tab_math::tab_insert_index_from_x(&m, x, tab_count, buf_width)
    }

    fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32 {
        let m = self.tab_layout_metrics();
        tab_math::calculate_tab_width(&m, tab_count, buf_width)
    }

    fn tab_origin_x(&self, tab_index: usize, tw: u32) -> u32 {
        let m = self.tab_layout_metrics();
        tab_math::tab_origin_x(&m, tab_index, tw)
    }

    /// Returns the close-button rectangle for a tab as `(x, y, w, h)`.
    fn close_button_rect(&self, tab_index: usize, tw: u32) -> (u32, u32, u32, u32) {
        let m = self.tab_layout_metrics();
        tab_math::close_button_rect(&m, tab_index, tw).to_tuple()
    }

    /// Returns the new-tab (+) button rectangle as `(x, y, w, h)`.
    fn plus_button_rect(&self, tab_count: usize, tw: u32) -> (u32, u32, u32, u32) {
        let m = self.tab_layout_metrics();
        tab_math::plus_button_rect(&m, tab_count, tw).to_tuple()
    }

    /// Returns the pin button rectangle as `(x, y, w, h)` (non-macOS only).
    #[cfg(not(target_os = "macos"))]
    fn pin_button_rect(&self) -> (u32, u32, u32, u32) {
        let m = self.tab_layout_metrics();
        tab_math::pin_button_rect(&m).to_tuple()
    }

    /// Returns the gear (settings) button rectangle as `(x, y, w, h)` (non-macOS only).
    #[cfg(not(target_os = "macos"))]
    fn gear_button_rect(&self) -> (u32, u32, u32, u32) {
        let m = self.tab_layout_metrics();
        tab_math::gear_button_rect(&m).to_tuple()
    }

    /// Returns `true` when the tab width is too narrow to display a title.
    fn should_show_number(&self, tw: u32) -> bool {
        let m = self.tab_layout_metrics();
        tab_math::should_show_number(&m, tw)
    }

    // ── Hit testing ─────────────────────────────────────────────────

    fn hit_test_tab_bar(&self, x: f64, y: f64, tab_count: usize, buf_width: u32) -> TabBarHit {
        let m = self.tab_layout_metrics();
        tab_hit_test::hit_test_tab_bar(x, y, tab_count, buf_width, &m)
    }

    fn hit_test_tab_hover(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> Option<usize> {
        let m = self.tab_layout_metrics();
        tab_hit_test::hit_test_tab_hover(x, y, tab_count, buf_width, &m)
    }

    fn hit_test_tab_security_badge(
        &self,
        x: f64,
        y: f64,
        tabs: &[TabInfo],
        buf_width: u32,
    ) -> Option<usize> {
        let m = self.tab_layout_metrics();
        tab_hit_test::hit_test_tab_security_badge(x, y, tabs, buf_width, &m)
    }

    // ── Security ────────────────────────────────────────────────────

    fn draw_security_popup(
        &mut self,
        target: &mut RenderTarget<'_>,
        popup: &SecurityPopup,
    );

    fn hit_test_security_popup(
        &self,
        popup: &SecurityPopup,
        x: f64,
        y: f64,
        buf_width: usize,
        buf_height: usize,
    ) -> bool {
        popup.hit_test(
            x,
            y,
            self.cell_width(),
            self.cell_height(),
            buf_width as u32,
            buf_height as u32,
        )
    }

    fn security_badge_rect(
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

    // ── Settings overlay ────────────────────────────────────────────

    fn draw_settings_overlay(
        &mut self,
        target: &mut RenderTarget<'_>,
        overlay: &crate::gui::settings::SettingsOverlay,
    );
}
