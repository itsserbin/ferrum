mod context_menu;
mod cursor;
mod scrollbar;
mod security;
mod tab_bar;
mod terminal;

use crate::core::{Color, CursorStyle, Grid, Selection};
use fontdue::{Font, FontSettings};
use std::collections::HashMap;

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

/// Tab context menu actions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextAction {
    Close,
    Rename,
    Duplicate,
}

/// Context menu state for one tab.
pub struct ContextMenu {
    pub x: u32,
    pub y: u32,
    pub tab_index: usize,
    pub items: Vec<(ContextAction, &'static str)>,
    pub hover_index: Option<usize>,
}

/// Render-time tab metadata.
pub struct TabInfo<'a> {
    pub title: &'a str,
    pub is_active: bool,
    pub security_count: usize,
    pub is_renaming: bool,
    pub rename_text: Option<&'a str>,
    pub rename_cursor: usize,
    pub rename_selection: Option<(usize, usize)>, // Byte range within rename_text.
}

pub struct SecurityPopup {
    pub tab_index: usize,
    pub x: u32,
    pub y: u32,
    pub title: &'static str,
    pub lines: Vec<String>,
}

/// Result of tab-bar hit testing.
#[derive(Debug)]
pub enum TabBarHit {
    /// Clicked on a tab by index.
    Tab(usize),
    /// Clicked on a tab close button by index.
    CloseTab(usize),
    /// Clicked on the new-tab button.
    NewTab,
    /// Clicked on a window control button (non-macOS).
    WindowButton(WindowButton),
    /// Clicked empty bar area (window drag).
    Empty,
}

/// Window control button type (non-macOS).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowButton {
    Minimize,
    Maximize,
    Close,
}

struct GlyphBitmap {
    data: Vec<u8>,
    width: usize,
    height: usize,
    left: i32,
    top: i32,
}

pub struct Renderer {
    font: Font,
    font_size: f32,
    ui_scale: f64,
    pub cell_width: u32,
    pub cell_height: u32,
    ascent: i32,
    glyph_cache: HashMap<char, GlyphBitmap>,
}

impl Renderer {
    pub fn new() -> Self {
        let font_data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/JetBrainsMono-Regular.ttf"
        ));
        let font = Font::from_bytes(font_data as &[u8], FontSettings::default())
            .expect("font load failed");

        let mut renderer = Renderer {
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
        self.scaled_px(TAB_BAR_HEIGHT)
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
