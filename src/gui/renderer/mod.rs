pub mod backend;
mod context_menu;
mod cursor;
#[cfg(feature = "gpu")]
pub mod gpu;
#[cfg(feature = "gpu")]
pub mod metrics;
mod scrollbar;
mod security;
mod tab_bar;
mod terminal;
pub mod traits;
pub mod types;

use crate::core::{Color, CursorStyle, Grid, Selection};
use fontdue::{Font, FontSettings};
use std::collections::HashMap;

pub use backend::RendererBackend;
pub use traits::Renderer;

pub(super) const FONT_SIZE: f32 = 15.0;
pub(super) const LINE_PADDING: u32 = 2;

/// Scrollbar thumb width in pixels.
pub const SCROLLBAR_WIDTH: u32 = 6;

/// Scrollbar hit zone width from right edge (wider than thumb for easier targeting).
pub const SCROLLBAR_HIT_ZONE: u32 = 14;

/// Margin between the thumb right edge and the window right edge.
pub const SCROLLBAR_MARGIN: u32 = 2;

/// Scrollbar thumb color — Catppuccin Mocha Overlay0 #6C7086.
pub(super) const SCROLLBAR_COLOR: Color = Color {
    r: 108,
    g: 112,
    b: 134,
};

/// Scrollbar thumb color when hovered/dragged — Catppuccin Mocha Overlay1 #7F849C.
pub(super) const SCROLLBAR_HOVER_COLOR: Color = Color {
    r: 127,
    g: 132,
    b: 156,
};

/// Tab bar height in pixels.
#[cfg(not(target_os = "macos"))]
pub const TAB_BAR_HEIGHT: u32 = 36;

/// Outer terminal padding inside the window.
pub const WINDOW_PADDING: u32 = 8;

/// Active-tab accent (Catppuccin Mocha Lavender #B4BEFE) — used by rename selection.
pub(super) const ACTIVE_ACCENT: Color = Color {
    r: 180,
    g: 190,
    b: 254,
};

/// Security indicator color (Catppuccin Mocha Yellow #F9E2AF).
pub(super) const SECURITY_ACCENT: Color = Color {
    r: 249,
    g: 226,
    b: 175,
};

/// Minimum tab width before switching to number-only display.
pub(super) const MIN_TAB_WIDTH_FOR_TITLE: u32 = 60;

/// Absolute minimum tab width (number + close button).
pub(super) const MIN_TAB_WIDTH: u32 = 36;

pub use types::*;
use types::GlyphBitmap;

/// CPU-based software renderer using softbuffer pixel buffers.
pub struct CpuRenderer {
    font: Font,
    font_size: f32,
    ui_scale: f64,
    pub cell_width: u32,
    pub cell_height: u32,
    ascent: i32,
    glyph_cache: HashMap<char, GlyphBitmap>,
}

impl CpuRenderer {
    pub fn new() -> Self {
        let font_data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/JetBrainsMono-Regular.ttf"
        ));
        let font = Font::from_bytes(font_data as &[u8], FontSettings::default())
            .expect("font load failed");

        let mut renderer = CpuRenderer {
            font,
            font_size: 1.0,
            ui_scale: 1.0,
            cell_width: 1,
            cell_height: 1,
            ascent: 0,
            glyph_cache: HashMap::new(),
        };
        renderer.recompute_metrics();
        renderer
    }

    fn recompute_metrics(&mut self) {
        let scaled_font_size = (FONT_SIZE as f64 * self.ui_scale).max(1.0) as f32;
        let line_padding = self.scaled_px(LINE_PADDING);
        let line_metrics = self
            .font
            .horizontal_line_metrics(scaled_font_size)
            .expect("no horizontal line metrics");
        let asc = line_metrics.ascent.round() as i32;
        let desc = line_metrics.descent.round() as i32; // negative
        self.ascent = asc + line_padding as i32 / 2;
        self.cell_height = ((asc - desc).max(1) as u32) + line_padding;

        // Measure advance width from 'M'
        let (m_metrics, _) = self.font.rasterize('M', scaled_font_size);
        self.cell_width = m_metrics.advance_width.round().max(1.0) as u32;
        self.font_size = scaled_font_size;
        self.glyph_cache.clear();
    }

    pub fn set_scale(&mut self, scale_factor: f64) {
        let scale = if scale_factor.is_finite() {
            scale_factor.clamp(0.75, 4.0)
        } else {
            1.0
        };
        if (self.ui_scale - scale).abs() < f64::EPSILON {
            return;
        }
        self.ui_scale = scale;
        self.recompute_metrics();
    }

    pub(crate) fn ui_scale(&self) -> f64 {
        self.ui_scale
    }

    pub(crate) fn scaled_px(&self, base: u32) -> u32 {
        if base == 0 {
            0
        } else {
            ((base as f64 * self.ui_scale).round() as u32).max(1)
        }
    }

    pub(crate) fn tab_bar_height_px(&self) -> u32 {
        #[cfg(target_os = "macos")]
        { 0 }
        #[cfg(not(target_os = "macos"))]
        { self.scaled_px(TAB_BAR_HEIGHT) }
    }

    pub(crate) fn window_padding_px(&self) -> u32 {
        self.scaled_px(WINDOW_PADDING)
    }

    pub(crate) fn scrollbar_width_px(&self) -> u32 {
        self.scaled_px(SCROLLBAR_WIDTH)
    }

    pub(crate) fn scrollbar_hit_zone_px(&self) -> u32 {
        self.scaled_px(SCROLLBAR_HIT_ZONE)
    }

    pub(crate) fn scrollbar_margin_px(&self) -> u32 {
        self.scaled_px(SCROLLBAR_MARGIN)
    }

    /// Draws one glyph at arbitrary pixel coordinates (used by tab bar and overlays).
    #[allow(clippy::too_many_arguments)]
    fn draw_char_at(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x: u32,
        y: u32,
        character: char,
        fg: Color,
    ) {
        self.draw_char(buffer, buf_width, buf_height, x, y, character, fg);
    }

    fn draw_bg(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x: u32,
        y: u32,
        color: Color,
    ) {
        let pixel = color.to_pixel();
        for dy in 0..self.cell_height as usize {
            let py = y as usize + dy;
            if py >= buf_height {
                break;
            }
            for dx in 0..self.cell_width as usize {
                let px = x as usize + dx;
                if px >= buf_width {
                    break;
                }
                buffer[py * buf_width + px] = pixel;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_char(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x: u32,
        y: u32,
        character: char,
        fg: Color,
    ) {
        if !self.glyph_cache.contains_key(&character) {
            let (metrics, bitmap) = self.font.rasterize(character, self.font_size);
            let cached = GlyphBitmap {
                data: bitmap,
                width: metrics.width,
                height: metrics.height,
                left: metrics.xmin,
                top: metrics.height as i32 + metrics.ymin,
            };
            self.glyph_cache.insert(character, cached);
        }
        let glyph = self.glyph_cache.get(&character).unwrap();

        for gy in 0..glyph.height {
            for gx in 0..glyph.width {
                let alpha = glyph.data[gy * glyph.width + gx];
                if alpha == 0 {
                    continue;
                }

                let sx = x as i32 + glyph.left + gx as i32;
                let sy = y as i32 + (self.ascent - glyph.top) + gy as i32;

                if sx >= 0 && sy >= 0 && (sx as usize) < buf_width && (sy as usize) < buf_height {
                    let idx = sy as usize * buf_width + sx as usize;
                    let a = alpha as u32;
                    let inv_a = 255 - a;
                    let bg_pixel = buffer[idx];
                    let bg_r = (bg_pixel >> 16) & 0xFF;
                    let bg_g = (bg_pixel >> 8) & 0xFF;
                    let bg_b = bg_pixel & 0xFF;
                    let r = (fg.r as u32 * a + bg_r * inv_a) / 255;
                    let g = (fg.g as u32 * a + bg_g * inv_a) / 255;
                    let b = (fg.b as u32 * a + bg_b * inv_a) / 255;
                    buffer[idx] = (r << 16) | (g << 8) | b;
                }
            }
        }
    }

    pub(in crate::gui::renderer) fn blend_pixel(dst: u32, src: u32, alpha: u8) -> u32 {
        if alpha == 255 {
            return src;
        }
        if alpha == 0 {
            return dst;
        }

        let a = alpha as u32;
        let inv = 255 - a;

        let dr = (dst >> 16) & 0xFF;
        let dg = (dst >> 8) & 0xFF;
        let db = dst & 0xFF;

        let sr = (src >> 16) & 0xFF;
        let sg = (src >> 8) & 0xFF;
        let sb = src & 0xFF;

        let r = (sr * a + dr * inv + 127) / 255;
        let g = (sg * a + dg * inv + 127) / 255;
        let b = (sb * a + db * inv + 127) / 255;

        (r << 16) | (g << 8) | b
    }

    pub(in crate::gui::renderer) fn draw_rounded_rect(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        radius: u32,
        color: u32,
        alpha: u8,
    ) {
        if w == 0 || h == 0 || alpha == 0 || buf_width == 0 || buf_height == 0 {
            return;
        }

        let r = radius.min(w / 2).min(h / 2) as i32;
        let max_x = buf_width as i32 - 1;
        let max_y = buf_height as i32 - 1;

        for py in 0..h as i32 {
            let sy = y + py;
            if sy < 0 || sy > max_y {
                continue;
            }

            for px in 0..w as i32 {
                let sx = x + px;
                if sx < 0 || sx > max_x {
                    continue;
                }

                let coverage = Self::rounded_coverage(px, py, w as i32, h as i32, r);
                if coverage <= 0.0 {
                    continue;
                }

                let idx = sy as usize * buf_width + sx as usize;
                if idx >= buffer.len() {
                    continue;
                }
                let aa_alpha = ((alpha as f32) * coverage).round().clamp(0.0, 255.0) as u8;
                if aa_alpha == 0 {
                    continue;
                }
                buffer[idx] = Self::blend_pixel(buffer[idx], color, aa_alpha);
            }
        }
    }

    fn rounded_coverage(px: i32, py: i32, w: i32, h: i32, r: i32) -> f32 {
        if px < 0 || py < 0 || px >= w || py >= h {
            return 0.0;
        }
        if r <= 0 {
            return 1.0;
        }

        let in_tl = px < r && py < r;
        let in_tr = px >= w - r && py < r;
        let in_bl = px < r && py >= h - r;
        let in_br = px >= w - r && py >= h - r;
        if !(in_tl || in_tr || in_bl || in_br) {
            return 1.0;
        }

        let cx = if in_tl || in_bl {
            r as f32 - 0.5
        } else {
            (w - r) as f32 - 0.5
        };
        let cy = if in_tl || in_tr {
            r as f32 - 0.5
        } else {
            (h - r) as f32 - 0.5
        };

        let dx = px as f32 + 0.5 - cx;
        let dy = py as f32 + 0.5 - cy;
        let rr = r as f32;
        let dist = (dx * dx + dy * dy).sqrt();
        (rr + 0.5 - dist).clamp(0.0, 1.0)
    }
}

// ── Renderer trait implementation for CpuRenderer ───────────────────

impl traits::Renderer for CpuRenderer {
    fn set_scale(&mut self, scale_factor: f64) {
        CpuRenderer::set_scale(self, scale_factor);
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
    ) {
        CpuRenderer::render(self, buffer, buf_width, buf_height, grid, selection);
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
        CpuRenderer::scrollbar_thumb_bounds(self, buf_height, scroll_offset, scrollback_len, grid_rows)
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
    ) {
        CpuRenderer::draw_tab_bar(self, buffer, buf_width, buf_height, tabs, hovered_tab, mouse_pos, tab_offsets);
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
