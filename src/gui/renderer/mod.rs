mod context_menu;
mod cursor;
mod scrollbar;
mod security;
mod tab_bar;
mod terminal;

use crate::core::{Color, CursorStyle, Grid, Selection};
use fontdue::{Font, FontSettings};
use std::collections::HashMap;

pub(super) const FONT_SIZE: f32 = 16.0;
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
pub const TAB_BAR_HEIGHT: u32 = 38;

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

/// Divider color (Catppuccin Mocha Surface0 #313244).
pub(super) const SEPARATOR_COLOR: Color = Color {
    r: 49,
    g: 50,
    b: 68,
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

/// Context-menu background.
pub(super) const MENU_BG: Color = Color {
    r: 30,
    g: 30,
    b: 46,
};

/// Context-menu hover background.
pub(super) const MENU_HOVER_BG: Color = Color {
    r: 69,
    g: 71,
    b: 90,
};

/// Top-left corner radius for the first tab (window corner).
pub(super) const FIRST_TAB_RADIUS: u32 = 6;

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

        let line_metrics = font
            .horizontal_line_metrics(FONT_SIZE)
            .expect("no horizontal line metrics");
        let asc = line_metrics.ascent.round() as i32;
        let desc = line_metrics.descent.round() as i32; // negative
        let ascent = asc + LINE_PADDING as i32 / 2;
        let cell_height = (asc - desc) as u32 + LINE_PADDING;

        // Measure advance width from 'M'
        let (m_metrics, _) = font.rasterize('M', FONT_SIZE);
        let cell_width = m_metrics.advance_width.round() as u32;

        Renderer {
            font,
            font_size: FONT_SIZE,
            cell_width,
            cell_height,
            ascent,
            glyph_cache: HashMap::new(),
        }
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
}
