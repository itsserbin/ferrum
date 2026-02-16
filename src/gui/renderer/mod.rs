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
pub const TAB_BAR_HEIGHT: u32 = 32;

/// Outer terminal padding inside the window.
pub const WINDOW_PADDING: u32 = 8;

/// Resize grab area near window borders.
pub const RESIZE_BORDER: f64 = 6.0;

/// Tab bar background (Catppuccin Mocha Crust #11111B).
pub(super) const TAB_BAR_BG: Color = Color {
    r: 17,
    g: 17,
    b: 27,
};

/// Hover background — subtle lift between Crust and Surface0 (#232334).
pub(super) const TAB_HOVER_BG: Color = Color {
    r: 35,
    g: 35,
    b: 52,
};

/// Active-tab accent (Catppuccin Mocha Lavender #B4BEFE) — used by rename selection.
pub(super) const ACTIVE_ACCENT: Color = Color {
    r: 180,
    g: 190,
    b: 254,
};

/// Close button hover circle background (Catppuccin Mocha Surface1 #45475A).
pub(super) const CLOSE_HOVER_BG: Color = Color {
    r: 69,
    g: 71,
    b: 90,
};

/// Security indicator color (Catppuccin Mocha Yellow #F9E2AF).
pub(super) const SECURITY_ACCENT: Color = Color {
    r: 249,
    g: 226,
    b: 175,
};

/// Context-menu hover background.
pub(super) const MENU_HOVER_BG: Color = Color {
    r: 69,
    g: 71,
    b: 90,
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
    /// Clicked empty bar area (window drag).
    Empty,
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

    pub(crate) fn resize_border_px(&self) -> f64 {
        RESIZE_BORDER * self.ui_scale
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

                if r > 0 {
                    let in_tl = px < r && py < r;
                    let in_tr = px >= w as i32 - r && py < r;
                    let in_bl = px < r && py >= h as i32 - r;
                    let in_br = px >= w as i32 - r && py >= h as i32 - r;

                    if in_tl || in_tr || in_bl || in_br {
                        let cx = if in_tl || in_bl {
                            r as f32 - 0.5
                        } else {
                            (w as i32 - r) as f32 - 0.5
                        };
                        let cy = if in_tl || in_tr {
                            r as f32 - 0.5
                        } else {
                            (h as i32 - r) as f32 - 0.5
                        };
                        let dx = px as f32 + 0.5 - cx;
                        let dy = py as f32 + 0.5 - cy;
                        let rr = r as f32;
                        if dx * dx + dy * dy > rr * rr {
                            continue;
                        }
                    }
                }

                let idx = sy as usize * buf_width + sx as usize;
                if idx >= buffer.len() {
                    continue;
                }
                buffer[idx] = Self::blend_pixel(buffer[idx], color, alpha);
            }
        }
    }

    fn rounded_contains(px: i32, py: i32, w: i32, h: i32, r: i32) -> bool {
        if px < 0 || py < 0 || px >= w || py >= h {
            return false;
        }
        if r <= 0 {
            return true;
        }

        let in_tl = px < r && py < r;
        let in_tr = px >= w - r && py < r;
        let in_bl = px < r && py >= h - r;
        let in_br = px >= w - r && py >= h - r;
        if !(in_tl || in_tr || in_bl || in_br) {
            return true;
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
        dx * dx + dy * dy <= rr * rr
    }

    pub(in crate::gui::renderer) fn draw_liquid_glass_panel(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        radius: u32,
        tint: Color,
    ) {
        if w == 0 || h == 0 || buf_width == 0 || buf_height == 0 {
            return;
        }

        let x_i = x as i32;
        let y_i = y as i32;
        let w_i = w as i32;
        let h_i = h as i32;
        let r_i = radius.min(w / 2).min(h / 2) as i32;
        let blur_radius = self.scaled_px(3).clamp(2, 8) as i32;
        let edge_soft = 1.75_f32;

        // Soft drop shadow under the panel.
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            x_i,
            y_i + self.scaled_px(1) as i32,
            w,
            h,
            radius,
            0x000000,
            28,
        );
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            x_i,
            y_i + self.scaled_px(3) as i32,
            w,
            h,
            radius + self.scaled_px(1),
            0x000000,
            14,
        );

        // Copy only the panel neighborhood for blur sampling.
        let src_x0 = (x_i - blur_radius - 1).max(0) as usize;
        let src_y0 = (y_i - blur_radius - 1).max(0) as usize;
        let src_x1 = (x_i + w_i + blur_radius + 1).min(buf_width as i32) as usize;
        let src_y1 = (y_i + h_i + blur_radius + 1).min(buf_height as i32) as usize;
        let src_w = src_x1.saturating_sub(src_x0);
        let src_h = src_y1.saturating_sub(src_y0);
        if src_w == 0 || src_h == 0 {
            return;
        }

        let mut src = vec![0u32; src_w * src_h];
        for row in 0..src_h {
            let dst_row = row * src_w;
            let src_row = (src_y0 + row) * buf_width + src_x0;
            src[dst_row..(dst_row + src_w)].copy_from_slice(&buffer[src_row..(src_row + src_w)]);
        }

        let tint_px = tint.to_pixel();
        let panel_alpha = 232u8;
        let tint_alpha = 86u8;
        let top_gloss = 76u8;
        let bottom_shadow = 34u8;

        for py in 0..h_i {
            for px in 0..w_i {
                if !Self::rounded_contains(px, py, w_i, h_i, r_i) {
                    continue;
                }

                let gx = x_i + px;
                let gy = y_i + py;
                if gx < 0 || gy < 0 || gx >= buf_width as i32 || gy >= buf_height as i32 {
                    continue;
                }

                // Backdrop blur sample.
                let mut acc_r: u32 = 0;
                let mut acc_g: u32 = 0;
                let mut acc_b: u32 = 0;
                let mut samples: u32 = 0;
                let sy0 = (gy - blur_radius).max(src_y0 as i32);
                let sy1 = (gy + blur_radius).min(src_y1 as i32 - 1);
                let sx0 = (gx - blur_radius).max(src_x0 as i32);
                let sx1 = (gx + blur_radius).min(src_x1 as i32 - 1);

                for sy in sy0..=sy1 {
                    let ry = (sy as usize - src_y0) * src_w;
                    for sx in sx0..=sx1 {
                        let rx = sx as usize - src_x0;
                        let p = src[ry + rx];
                        acc_r += (p >> 16) & 0xFF;
                        acc_g += (p >> 8) & 0xFF;
                        acc_b += p & 0xFF;
                        samples += 1;
                    }
                }
                if samples == 0 {
                    continue;
                }

                let mut glass =
                    ((acc_r / samples) << 16) | ((acc_g / samples) << 8) | (acc_b / samples);
                glass = Self::blend_pixel(glass, tint_px, tint_alpha);

                let top_t = if h_i <= 1 {
                    1.0
                } else {
                    1.0 - py as f32 / (h_i - 1) as f32
                };
                let gloss_alpha = (top_t * top_t * top_gloss as f32).round() as u8;
                glass = Self::blend_pixel(glass, 0xFFFFFF, gloss_alpha);

                let bottom_t = if h_i <= 1 {
                    0.0
                } else {
                    py as f32 / (h_i - 1) as f32
                };
                let shadow_alpha = (bottom_t * bottom_t * bottom_shadow as f32).round() as u8;
                glass = Self::blend_pixel(glass, 0x0A0D14, shadow_alpha);

                let edge_dist = (px.min(w_i - 1 - px).min(py).min(h_i - 1 - py)) as f32;
                if edge_dist < edge_soft {
                    let edge_t = (1.0 - edge_dist / edge_soft).clamp(0.0, 1.0);
                    let rim_alpha = (edge_t * 92.0).round() as u8;
                    let rim_color = if py < h_i / 2 { 0xFFFFFF } else { 0x1A202D };
                    glass = Self::blend_pixel(glass, rim_color, rim_alpha);
                }

                let idx = gy as usize * buf_width + gx as usize;
                if idx < buffer.len() {
                    buffer[idx] = Self::blend_pixel(buffer[idx], glass, panel_alpha);
                }
            }
        }

        // Specular "liquid" streak near the top of the glass.
        let streak_h = ((h as f32 * 0.34).round() as i32).max(self.scaled_px(4) as i32);
        for py in 0..streak_h.min(h_i) {
            let py_t = 1.0 - py as f32 / streak_h.max(1) as f32;
            for px in 0..w_i {
                if !Self::rounded_contains(px, py, w_i, h_i, r_i) {
                    continue;
                }
                let wave = ((px as f32 / w_i.max(1) as f32) * std::f32::consts::PI)
                    .sin()
                    .mul_add(0.35, 0.65);
                let alpha = (py_t * py_t * wave * 56.0).round() as u8;
                if alpha == 0 {
                    continue;
                }
                let gx = x_i + px;
                let gy = y_i + py;
                if gx < 0 || gy < 0 || gx >= buf_width as i32 || gy >= buf_height as i32 {
                    continue;
                }
                let idx = gy as usize * buf_width + gx as usize;
                if idx < buffer.len() {
                    buffer[idx] = Self::blend_pixel(buffer[idx], 0xFFFFFF, alpha);
                }
            }
        }
    }
}
