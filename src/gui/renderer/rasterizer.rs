//! Gamma-correct glyph rasterization via swash.
//!
//! Provides [`GlyphRasterizer`] which is shared by the GPU atlas and CPU glyph cache.
//! The rasterization mode (grayscale vs LCD subpixel) is selected from the display
//! scale factor at creation time and can be updated on `ScaleFactorChanged`.

use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::Format;
use swash::FontRef;

/// How to rasterize glyph coverage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RasterMode {
    /// One byte per pixel. Used on Retina displays (scale_factor >= 2.0).
    Grayscale,
    /// Three bytes per pixel (R, G, B coverage separately).
    /// Used on non-Retina displays (scale_factor < 2.0).
    LcdSubpixel,
}

impl RasterMode {
    /// Selects the rasterization mode from a winit `scale_factor`.
    pub fn from_scale_factor(scale: f64) -> Self {
        if scale >= 2.0 {
            RasterMode::Grayscale
        } else {
            RasterMode::LcdSubpixel
        }
    }
}

/// Coverage data produced by rasterizing one glyph.
pub enum GlyphCoverage {
    /// One byte per pixel: linear alpha coverage (0 = transparent, 255 = opaque).
    Grayscale(Vec<u8>),
    /// Three bytes per pixel: per-channel coverage `[R_cov, G_cov, B_cov]`.
    Lcd(Vec<[u8; 3]>),
}

/// Metadata and pixel coverage for a rasterized glyph.
pub struct RasterizedGlyph {
    pub coverage: GlyphCoverage,
    pub width:    u32,
    pub height:   u32,
    /// Horizontal offset from cell origin to the glyph's left edge (can be negative).
    pub left:     i32,
    /// Distance from baseline to the glyph's top edge.
    pub top:      i32,
}

/// Cell-layout metrics derived from the primary font at the current size.
pub struct GlyphMetrics {
    pub cell_width:  u32,
    pub cell_height: u32,
    pub ascent:      i32,
}

/// Wraps swash rasterization. Shared between the GPU atlas and CPU renderer.
pub struct GlyphRasterizer {
    scale_ctx:     ScaleContext,
    font_data:     &'static [u8],
    fallback_data: Vec<&'static [u8]>,
    pub font_size: f32,
    pub mode:      RasterMode,
}

impl GlyphRasterizer {
    /// Creates a new rasterizer from static font bytes (compiled into the binary).
    pub fn new(
        font_data:     &'static [u8],
        fallback_data: Vec<&'static [u8]>,
        font_size:     f32,
        mode:          RasterMode,
    ) -> Self {
        Self {
            scale_ctx: ScaleContext::new(),
            font_data,
            fallback_data,
            font_size,
            mode,
        }
    }

    /// Updates font size and/or raster mode. Call on settings change or DPI change.
    pub fn rebuild(&mut self, font_size: f32, mode: RasterMode) {
        self.font_size = font_size;
        self.mode = mode;
        self.scale_ctx = ScaleContext::new();
    }

    /// Returns the static font bytes for the given character — primary font if it has
    /// the glyph, otherwise the first fallback that does, or the primary as a last resort.
    ///
    /// Returning `&'static [u8]` instead of `FontRef<'_>` avoids a split-borrow
    /// conflict between `self.font_data` (immutable) and `self.scale_ctx` (mutable).
    fn font_bytes_for_char(&self, ch: char) -> &'static [u8] {
        let primary = FontRef::from_index(self.font_data, 0)
            .expect("primary font data is valid");
        if primary.charmap().map(ch) != 0 {
            return self.font_data;
        }
        for fb in &self.fallback_data {
            let f = FontRef::from_index(fb, 0).expect("fallback font data is valid");
            if f.charmap().map(ch) != 0 {
                return fb;
            }
        }
        self.font_data
    }

    /// Rasterizes `ch` and returns coverage data, or `None` for empty glyphs (e.g. space).
    pub fn rasterize(&mut self, ch: char) -> Option<RasterizedGlyph> {
        // Resolve font bytes first to avoid a split-borrow conflict between
        // font_data (immutable) and scale_ctx (mutable).
        let font_bytes = self.font_bytes_for_char(ch);
        let font = FontRef::from_index(font_bytes, 0).expect("font data is valid");
        let glyph_id = font.charmap().map(ch);
        if glyph_id == 0 {
            return None;
        }

        let format = match self.mode {
            RasterMode::Grayscale    => Format::Alpha,
            RasterMode::LcdSubpixel  => Format::Subpixel,
        };

        let mut scaler = self
            .scale_ctx
            .builder(font)
            .size(self.font_size)
            .hint(true)
            .build();

        let image = Render::new(&[Source::Outline])
            .format(format)
            .render(&mut scaler, glyph_id)?;

        let w = image.placement.width;
        let h = image.placement.height;
        if w == 0 || h == 0 {
            return None;
        }

        let coverage = match self.mode {
            RasterMode::Grayscale => GlyphCoverage::Grayscale(image.data),
            RasterMode::LcdSubpixel => {
                let pixels = image.data
                    .chunks_exact(3)
                    .map(|c| [c[0], c[1], c[2]])
                    .collect();
                GlyphCoverage::Lcd(pixels)
            }
        };

        Some(RasterizedGlyph {
            coverage,
            width:  w,
            height: h,
            left:   image.placement.left,
            top:    image.placement.top,
        })
    }

    /// Returns cell layout metrics (cell_width, cell_height, ascent) for the current font/size.
    pub fn metrics(&mut self) -> GlyphMetrics {
        let font = FontRef::from_index(self.font_data, 0)
            .expect("primary font data is valid");
        let m = font.metrics(&[]).scale(self.font_size);
        let cell_height = (m.ascent + m.descent + m.leading).ceil() as u32;

        let m_id = font.charmap().map('M');
        let adv = font.glyph_metrics(&[]).scale(self.font_size).advance_width(m_id);
        let cell_width = adv.ceil() as u32;
        let ascent = m.ascent.ceil() as i32;

        GlyphMetrics { cell_width, cell_height, ascent }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jetbrains_mono() -> &'static [u8] {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/JetBrainsMono-Regular.ttf"
        ))
    }

    fn make_rasterizer() -> GlyphRasterizer {
        GlyphRasterizer::new(jetbrains_mono(), vec![], 14.0, RasterMode::Grayscale)
    }

    #[test]
    fn raster_mode_from_scale_factor() {
        assert_eq!(RasterMode::from_scale_factor(1.0), RasterMode::LcdSubpixel);
        assert_eq!(RasterMode::from_scale_factor(1.9), RasterMode::LcdSubpixel);
        assert_eq!(RasterMode::from_scale_factor(2.0), RasterMode::Grayscale);
        assert_eq!(RasterMode::from_scale_factor(3.0), RasterMode::Grayscale);
    }

    #[test]
    fn rasterize_ascii_returns_grayscale_coverage() {
        let mut r = make_rasterizer();
        let glyph = r.rasterize('A').expect("'A' should rasterize");
        assert!(glyph.width > 0 && glyph.height > 0);
        match glyph.coverage {
            GlyphCoverage::Grayscale(data) => {
                assert_eq!(data.len(), (glyph.width * glyph.height) as usize);
                assert!(data.iter().any(|&b| b > 0), "coverage should be non-zero");
            }
            GlyphCoverage::Lcd(_) => panic!("expected grayscale, got LCD"),
        }
    }

    #[test]
    fn rasterize_space_returns_none() {
        let mut r = make_rasterizer();
        assert!(r.rasterize(' ').is_none());
    }

    #[test]
    fn rasterize_lcd_returns_three_channel_coverage() {
        let mut r = GlyphRasterizer::new(jetbrains_mono(), vec![], 14.0, RasterMode::LcdSubpixel);
        let glyph = r.rasterize('A').expect("'A' should rasterize in LCD mode");
        assert!(glyph.width > 0 && glyph.height > 0);
        match glyph.coverage {
            GlyphCoverage::Lcd(data) => {
                let expected = (glyph.width * glyph.height) as usize;
                assert!(
                    data.len() >= expected,
                    "LCD data ({}) must cover at least width×height ({}) pixels",
                    data.len(),
                    expected,
                );
            }
            GlyphCoverage::Grayscale(_) => panic!("expected LCD, got grayscale"),
        }
    }

    #[test]
    fn metrics_are_nonzero() {
        let mut r = make_rasterizer();
        let m = r.metrics();
        assert!(m.cell_width > 0, "cell_width must be > 0");
        assert!(m.cell_height > 0, "cell_height must be > 0");
        assert!(m.ascent > 0, "ascent must be > 0");
    }

    #[test]
    fn rebuild_changes_mode() {
        let mut r = make_rasterizer();
        assert_eq!(r.mode, RasterMode::Grayscale);
        r.rebuild(16.0, RasterMode::LcdSubpixel);
        assert_eq!(r.mode, RasterMode::LcdSubpixel);
        assert_eq!(r.font_size, 16.0);
    }
}
