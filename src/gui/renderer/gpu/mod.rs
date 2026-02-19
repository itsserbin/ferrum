//! GPU renderer backend using wgpu compute + render pipelines.
//!
//! Three-pass architecture:
//! 1. Grid compute shader — renders terminal cells into a texture
//! 2. UI fragment shader — renders draw commands (tab bar, overlays) via SDF
//! 3. Composite shader — blends grid + UI into the final swapchain surface

pub mod atlas;
pub mod buffers;
mod frame;
mod hit_test;
pub mod pipelines;
mod setup;
mod tab_layout;
mod ui_commands;

use fontdue::Font;
use wgpu;

use crate::core::{Cell, Color, CursorStyle, Grid, Selection};

#[cfg(not(target_os = "macos"))]
use super::WindowButton;
use super::metrics::FontMetrics;
use super::traits;
use super::{ContextMenu, FONT_SIZE, SecurityPopup, TabBarHit, TabInfo};

use atlas::GlyphAtlas;
use buffers::*;

// Catppuccin Mocha palette constants (same as CPU renderer).
const BAR_BG: u32 = 0x181825;
const ACTIVE_TAB_BG: u32 = 0x1E1E2E;
const INACTIVE_TAB_HOVER: u32 = 0x313244;
const TAB_TEXT_ACTIVE: u32 = 0xCDD6F4;
const TAB_TEXT_INACTIVE: u32 = 0x6C7086;
const TAB_BORDER: u32 = 0x313244;
const CLOSE_HOVER_BG_COLOR: u32 = 0x585B70;
const RENAME_FIELD_BG: u32 = 0x24273A;
const RENAME_FIELD_BORDER: u32 = 0x6C7086;
const RENAME_SELECTION_BG: u32 = 0xB4BEFE;

#[cfg(not(target_os = "macos"))]
const WIN_BTN_WIDTH: u32 = 46;

const SCROLLBAR_MIN_THUMB: u32 = 20;

const INSERTION_COLOR: u32 = 0xCBA6F7;

/// Maximum number of UI draw commands per frame.
const MAX_UI_COMMANDS: usize = 4096;

/// GPU-based renderer using wgpu compute and render pipelines.
pub struct GpuRenderer {
    // wgpu core
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    // Textures
    grid_texture: wgpu::Texture,
    grid_texture_view: wgpu::TextureView,
    ui_texture: wgpu::Texture,
    ui_texture_view: wgpu::TextureView,

    // Pipelines
    grid_pipeline: wgpu::ComputePipeline,
    ui_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,

    // Bind group layouts
    grid_bind_group_layout: wgpu::BindGroupLayout,
    ui_bind_group_layout: wgpu::BindGroupLayout,
    composite_bind_group_layout: wgpu::BindGroupLayout,

    // Buffers
    grid_uniform_buffer: wgpu::Buffer,
    grid_cell_buffer: wgpu::Buffer,
    glyph_info_buffer: wgpu::Buffer,
    ui_uniform_buffer: wgpu::Buffer,
    ui_command_buffer: wgpu::Buffer,
    composite_uniform_buffer: wgpu::Buffer,

    // Sampler for composite and UI
    sampler: wgpu::Sampler,

    // Atlas
    atlas: GlyphAtlas,

    // Font & metrics
    font: Font,
    metrics: FontMetrics,

    // UI command accumulator (filled during draw_* calls, flushed in present).
    commands: Vec<GpuDrawCommand>,

    // Grid state for the current frame (filled in render(), dispatched in present()).
    grid_cells: Vec<PackedCell>,
    grid_uniforms: GridUniforms,
    grid_dirty: bool,

    // Window dimensions
    width: u32,
    height: u32,
}

// ── Renderer trait implementation ────────────────────────────────────

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
        // Pack grid cells into the GPU buffer format.
        let rows = grid.rows;
        let cols = grid.cols;
        self.grid_cells.clear();
        self.grid_cells.reserve(rows * cols);

        for row in 0..rows {
            let abs_row = viewport_start + row;
            for col in 0..cols {
                // Safe: iterating within grid bounds
                let cell = grid.get_unchecked(row, col);
                let selected = selection.is_some_and(|s| s.contains(abs_row, col));
                let codepoint = cell.character as u32;

                // Ensure non-ASCII terminal glyphs exist in the atlas before grid shading.
                if codepoint >= 128 {
                    let _ = self.atlas.get_or_insert(
                        codepoint,
                        &self.font,
                        self.metrics.font_size,
                        &self.queue,
                    );
                }

                let mut attrs = 0u32;
                if cell.bold {
                    attrs |= 1;
                }
                if cell.underline {
                    attrs |= 4;
                }
                if cell.reverse {
                    attrs |= 8;
                }
                if selected {
                    attrs |= 16;
                }

                let mut fg = cell.fg;
                // Bold: bright variant for base ANSI colors.
                if cell.bold {
                    for i in 0..8 {
                        if fg.r == Color::ANSI[i].r
                            && fg.g == Color::ANSI[i].g
                            && fg.b == Color::ANSI[i].b
                        {
                            fg = Color::ANSI[i + 8];
                            break;
                        }
                    }
                }

                self.grid_cells.push(PackedCell {
                    codepoint,
                    fg: fg.to_pixel(),
                    bg: cell.bg.to_pixel(),
                    attrs,
                });
            }
        }

        self.grid_uniforms = GridUniforms {
            cols: cols as u32,
            rows: rows as u32,
            cell_width: self.metrics.cell_width,
            cell_height: self.metrics.cell_height,
            atlas_width: self.atlas.atlas_width,
            atlas_height: self.atlas.atlas_height,
            baseline: self.metrics.ascent as u32,
            bg_color: Color::DEFAULT_BG.to_pixel(),
            tex_width: self.width,
            tex_height: self.height,
            _pad1: 0,
            _pad2: 0,
        };
        self.grid_dirty = true;
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
        let x =
            col as f32 * self.metrics.cell_width as f32 + self.metrics.window_padding_px() as f32;
        let y = row as f32 * self.metrics.cell_height as f32
            + self.metrics.tab_bar_height_px() as f32
            + self.metrics.window_padding_px() as f32;
        let cw = self.metrics.cell_width as f32;
        let ch = self.metrics.cell_height as f32;
        let cursor_color = Color::DEFAULT_FG.to_pixel();

        match style {
            CursorStyle::BlinkingBlock | CursorStyle::SteadyBlock => {
                // Filled block — draw bg rect + inverted glyph.
                self.push_rect(x, y, cw, ch, cursor_color, 1.0);
                let cell = grid.get(row, col).unwrap_or(&Cell::DEFAULT);
                if cell.character != ' ' {
                    let cp = cell.character as u32;
                    let info = self.atlas.get_or_insert(
                        cp,
                        &self.font,
                        self.metrics.font_size,
                        &self.queue,
                    );
                    if info.w > 0.0 && info.h > 0.0 {
                        let gx = x + info.offset_x;
                        let gy = y + info.offset_y;
                        self.push_glyph(
                            gx,
                            gy,
                            info.x,
                            info.y,
                            info.w,
                            info.h,
                            Color::DEFAULT_BG.to_pixel(),
                            1.0,
                        );
                    }
                }
            }
            CursorStyle::BlinkingUnderline | CursorStyle::SteadyUnderline => {
                self.push_rect(x, y + ch - 2.0, cw, 2.0, cursor_color, 1.0);
            }
            CursorStyle::BlinkingBar | CursorStyle::SteadyBar => {
                self.push_rect(x, y, 2.0, ch, cursor_color, 1.0);
            }
        }
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
        if scrollback_len == 0 || opacity <= 0.0 {
            return;
        }

        let track_top =
            (self.metrics.tab_bar_height_px() + self.metrics.window_padding_px()) as f32;
        let track_bottom = buf_height as f32 - self.metrics.window_padding_px() as f32;
        let track_height = track_bottom - track_top;
        if track_height <= 0.0 {
            return;
        }

        let total_lines = scrollback_len + grid_rows;
        let viewport_ratio = grid_rows as f32 / total_lines as f32;
        let min_thumb = self.metrics.scaled_px(SCROLLBAR_MIN_THUMB) as f32;
        let thumb_height = (viewport_ratio * track_height)
            .max(min_thumb)
            .min(track_height);

        let max_offset = scrollback_len as f32;
        let scroll_ratio = (max_offset - scroll_offset as f32) / max_offset;
        let thumb_y = track_top + scroll_ratio * (track_height - thumb_height);

        let sb_width = self.metrics.scaled_px(super::SCROLLBAR_WIDTH) as f32;
        let sb_margin = self.metrics.scaled_px(super::SCROLLBAR_MARGIN) as f32;
        let thumb_x = self.width as f32 - sb_width - sb_margin;
        let radius = self.metrics.scaled_px(3) as f32;

        let color = if hover { 0x7F849C } else { 0x6C7086 };
        let base_alpha = 180.0 / 255.0;
        let alpha = base_alpha * opacity;

        self.push_rounded_rect(
            thumb_x,
            thumb_y,
            sb_width,
            thumb_height,
            radius,
            color,
            alpha,
        );
    }

    fn scrollbar_thumb_bounds(
        &self,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
    ) -> Option<(f32, f32)> {
        if scrollback_len == 0 {
            return None;
        }

        let track_top =
            (self.metrics.tab_bar_height_px() + self.metrics.window_padding_px()) as f32;
        let track_bottom = buf_height as f32 - self.metrics.window_padding_px() as f32;
        let track_height = track_bottom - track_top;
        if track_height <= 0.0 {
            return None;
        }

        let total_lines = scrollback_len + grid_rows;
        let viewport_ratio = grid_rows as f32 / total_lines as f32;
        let min_thumb = self.metrics.scaled_px(SCROLLBAR_MIN_THUMB) as f32;
        let thumb_height = (viewport_ratio * track_height)
            .max(min_thumb)
            .min(track_height);

        let max_offset = scrollback_len as f32;
        let scroll_ratio = (max_offset - scroll_offset as f32) / max_offset;
        let thumb_y = track_top + scroll_ratio * (track_height - thumb_height);

        Some((thumb_y, thumb_height))
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
