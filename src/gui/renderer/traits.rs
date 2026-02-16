#![allow(dead_code)]

use crate::core::{CursorStyle, Grid, Selection};

use super::{ContextMenu, SecurityPopup, TabBarHit, TabInfo, WindowButton};

/// Selects which rendering backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Gpu,
    Cpu,
    Auto,
}

/// Trait defining the full renderer interface used by the GUI layer.
///
/// Both CPU (softbuffer) and GPU (wgpu) renderers implement this trait,
/// keeping the rest of the codebase backend-agnostic.
pub trait Renderer {
    // ── Lifecycle ────────────────────────────────────────────────────

    fn set_scale(&mut self, scale_factor: f64);

    // ── Metrics ─────────────────────────────────────────────────────

    fn cell_width(&self) -> u32;
    fn cell_height(&self) -> u32;
    fn tab_bar_height_px(&self) -> u32;
    fn window_padding_px(&self) -> u32;
    fn ui_scale(&self) -> f64;
    fn scaled_px(&self, base: u32) -> u32;
    fn scrollbar_hit_zone_px(&self) -> u32;

    // ── Terminal rendering ──────────────────────────────────────────

    fn render(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        grid: &Grid,
        selection: Option<&Selection>,
    );

    fn draw_cursor(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
    );

    // ── Scrollbar ───────────────────────────────────────────────────

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
    );

    fn scrollbar_thumb_bounds(
        &self,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
    ) -> Option<(f32, f32)>;

    // ── Tab bar ─────────────────────────────────────────────────────

    fn draw_tab_bar(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        tabs: &[TabInfo],
        hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
        tab_offsets: Option<&[f32]>,
    );

    fn draw_tab_drag_overlay(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        tabs: &[TabInfo],
        source_index: usize,
        current_x: f64,
        indicator_x: f32,
    );

    fn draw_tab_tooltip(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        mouse_pos: (f64, f64),
        title: &str,
    );

    fn tab_hover_tooltip<'a>(
        &self,
        tabs: &'a [TabInfo<'a>],
        hovered_tab: Option<usize>,
        buf_width: u32,
    ) -> Option<&'a str>;

    fn tab_insert_index_from_x(&self, x: f64, tab_count: usize, buf_width: u32) -> usize;

    fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32;
    fn tab_strip_start_x(&self) -> u32;
    fn tab_origin_x(&self, tab_index: usize, tw: u32) -> u32;

    // ── Hit testing ─────────────────────────────────────────────────

    fn hit_test_tab_bar(&self, x: f64, y: f64, tab_count: usize, buf_width: u32) -> TabBarHit;

    fn hit_test_tab_hover(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> Option<usize>;

    fn hit_test_tab_security_badge(
        &self,
        x: f64,
        y: f64,
        tabs: &[TabInfo],
        buf_width: u32,
    ) -> Option<usize>;

    #[cfg(not(target_os = "macos"))]
    fn window_button_at_position(&self, x: f64, y: f64, buf_width: u32) -> Option<WindowButton>;

    // ── Context menu ────────────────────────────────────────────────

    fn draw_context_menu(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        menu: &ContextMenu,
    );

    fn hit_test_context_menu(&self, menu: &ContextMenu, x: f64, y: f64) -> Option<usize>;

    // ── Security ────────────────────────────────────────────────────

    fn draw_security_popup(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        popup: &SecurityPopup,
    );

    fn hit_test_security_popup(
        &self,
        popup: &SecurityPopup,
        x: f64,
        y: f64,
        buf_width: usize,
        buf_height: usize,
    ) -> bool;

    fn security_badge_rect(
        &self,
        tab_index: usize,
        tab_count: usize,
        buf_width: u32,
        security_count: usize,
    ) -> Option<(u32, u32, u32, u32)>;
}
