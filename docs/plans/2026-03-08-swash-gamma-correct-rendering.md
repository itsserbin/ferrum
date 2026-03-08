# Swash + Gamma-Correct Rendering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace fontdue with swash, fix gamma-incorrect alpha blending in both renderers,
and migrate the GPU grid pass to a render pipeline with `Rgba8UnormSrgb` output for automatic
linear→sRGB encoding.

**Architecture:** `GlyphRasterizer` (new shared module) wraps swash and is used by both the
GPU atlas and CPU glyph cache. The GPU grid pass changes from a compute shader writing to a
`rgba8unorm` storage texture to a render pipeline writing to a `Rgba8UnormSrgb` attachment;
all color blending happens in linear space and the GPU encodes to sRGB automatically on output.
The CPU renderer uses a precomputed LUT for sRGB↔linear conversion and applies the same
gamma-correct blend logic.

**Tech Stack:** `swash` (pure-Rust font rasterizer), `wgpu` (existing GPU), `winit` (existing
windowing — provides `scale_factor()` for DPI detection).

---

## Task 1: Add sRGB helpers to `color.rs`

**Files:**
- Modify: `src/core/color.rs`

These two functions are foundational — every subsequent task depends on them.
`channel_to_linear` converts an sRGB byte (0–255) to a linear f32 using the simplified γ=2.2
approximation, which is fast and sufficient for terminal rendering. The full IEC 61966-2-1
piecewise formula is not needed here.

**Step 1: Add the helpers after the existing `from_256` block**

```rust
/// Converts an sRGB-encoded byte (0–255) to a linear light value (0.0–1.0).
///
/// Uses the γ = 2.2 approximation. Sufficient for terminal rendering.
pub fn channel_to_linear(c: u8) -> f32 {
    (c as f32 / 255.0).powf(2.2)
}

/// Converts a linear light value (0.0–1.0) to an sRGB-encoded byte (0–255).
pub fn channel_to_srgb(c: f32) -> u8 {
    (c.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8
}
```

**Step 2: Add unit tests inside the existing `#[cfg(test)]` block**

```rust
#[test]
fn channel_roundtrip() {
    // Pure black and white survive the roundtrip exactly.
    assert_eq!(Color::channel_to_srgb(Color::channel_to_linear(0)), 0);
    assert_eq!(Color::channel_to_srgb(Color::channel_to_linear(255)), 255);
}

#[test]
fn channel_to_linear_midpoint() {
    // sRGB 128 should decode to roughly 0.216 linear (γ=2.2: (128/255)^2.2).
    let v = Color::channel_to_linear(128);
    assert!((v - 0.216).abs() < 0.005, "got {v}");
}

#[test]
fn channel_to_srgb_midpoint() {
    // linear 0.5 should encode to roughly 186 in sRGB (0.5^(1/2.2) * 255).
    let v = Color::channel_to_srgb(0.5);
    assert!((v as i32 - 186).abs() <= 1, "got {v}");
}
```

**Step 3: Run tests**

```bash
cargo test channel_roundtrip channel_to_linear_midpoint channel_to_srgb_midpoint -- --nocapture
```

Expected: all three pass.

**Step 4: Commit**

```bash
git add src/core/color.rs
git commit -m "feat(color): add channel_to_linear and channel_to_srgb helpers"
```

---

## Task 2: Add swash dependency, remove fontdue

**Files:**
- Modify: `Cargo.toml`

**Step 1: Edit `Cargo.toml`**

Remove the line:
```toml
fontdue = "0.9"
```

Add instead:
```toml
swash = "0.1"
```

**Step 2: Verify the project still compiles (with errors expected on fontdue usages)**

```bash
cargo check 2>&1 | grep "^error" | head -20
```

Expected: errors referencing `fontdue` in `src/config/fonts.rs`,
`src/gui/renderer/gpu/atlas.rs`, `src/gui/renderer/gpu/mod.rs`,
`src/gui/renderer/cpu/mod.rs`, `src/gui/renderer/cpu/primitives.rs`.
Note them — they will all be resolved in subsequent tasks.

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore(deps): replace fontdue with swash"
```

---

## Task 3: Create `GlyphRasterizer` module

**Files:**
- Create: `src/gui/renderer/rasterizer.rs`
- Modify: `src/gui/renderer/mod.rs` (add `pub mod rasterizer;`)

This is the core new abstraction. Both the GPU atlas and CPU renderer will use it.
swash requires font data as `&[u8]`; the `ScaleContext` is reused across rasterizations.

**Step 1: Create `src/gui/renderer/rasterizer.rs`**

```rust
//! Gamma-correct glyph rasterization via swash.
//!
//! Provides [`GlyphRasterizer`] which is shared by the GPU atlas and CPU glyph cache.
//! The rasterization mode (grayscale vs LCD subpixel) is selected from the display
//! scale factor at creation time and can be updated on `ScaleFactorChanged`.

use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::shape::ShapeContext;
use swash::text::Script;
use swash::zeno::Format;
use swash::{CacheKey, FontRef};

/// How to rasterize glyph coverage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RasterMode {
    /// One byte per pixel. Used on Retina (scale_factor >= 2.0).
    Grayscale,
    /// Three bytes per pixel (R, G, B coverage separately).
    /// Used on non-Retina (scale_factor < 2.0).
    LcdSubpixel,
}

impl RasterMode {
    /// Selects mode based on winit `scale_factor`.
    pub fn from_scale_factor(scale: f64) -> Self {
        if scale >= 2.0 { RasterMode::Grayscale } else { RasterMode::LcdSubpixel }
    }
}

/// Coverage data produced by rasterizing one glyph.
pub enum GlyphCoverage {
    /// One byte per pixel: linear alpha coverage.
    Grayscale(Vec<u8>),
    /// Three bytes per pixel: per-channel coverage `[R, G, B]`.
    Lcd(Vec<[u8; 3]>),
}

/// Metadata and coverage for a rasterized glyph.
pub struct RasterizedGlyph {
    pub coverage: GlyphCoverage,
    pub width:    u32,
    pub height:   u32,
    /// Horizontal offset from cell origin to the glyph's left edge (can be negative).
    pub left:     i32,
    /// Distance from the cell top to the glyph's top edge.
    pub top:      i32,
}

/// Cell-layout metrics derived from a font at a given size.
pub struct GlyphMetrics {
    pub cell_width:  u32,
    pub cell_height: u32,
    pub ascent:      i32,
}

/// Wraps swash rasterization. Shared between GPU atlas and CPU renderer.
pub struct GlyphRasterizer {
    scale_ctx:    ScaleContext,
    font_data:    &'static [u8],
    fallback_data: Vec<&'static [u8]>,
    pub font_size: f32,
    pub mode:      RasterMode,
}

impl GlyphRasterizer {
    /// Creates a new rasterizer from static font bytes (compiled into the binary).
    pub fn new(
        font_data:    &'static [u8],
        fallback_data: Vec<&'static [u8]>,
        font_size:    f32,
        mode:         RasterMode,
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

    /// Returns the font that contains `ch` (primary first, then fallbacks).
    fn font_for(&self, ch: char) -> FontRef<'_> {
        let primary = FontRef::from_index(self.font_data, 0).expect("primary font");
        if primary.charmap().map(ch) != 0 {
            return primary;
        }
        for fb in &self.fallback_data {
            let f = FontRef::from_index(fb, 0).expect("fallback font");
            if f.charmap().map(ch) != 0 {
                return f;
            }
        }
        primary
    }

    /// Rasterizes `ch` and returns coverage data, or `None` for empty glyphs (e.g. space).
    pub fn rasterize(&mut self, ch: char) -> Option<RasterizedGlyph> {
        let font = self.font_for(ch);
        let glyph_id = font.charmap().map(ch);
        if glyph_id == 0 && ch != '\0' {
            return None;
        }

        let format = match self.mode {
            RasterMode::Grayscale   => Format::Alpha,
            RasterMode::LcdSubpixel => Format::Subpixel,
        };

        let mut scaler = self.scale_ctx
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
                // swash subpixel output: 3 bytes per pixel (R, G, B coverage).
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
        let font = FontRef::from_index(self.font_data, 0).expect("primary font");
        let metrics = font.metrics(&[]).scale(self.font_size);

        let cell_height = (metrics.ascent - metrics.descent + metrics.leading).ceil() as u32;
        let m_id = font.charmap().map('M');
        let mut scaler = self.scale_ctx.builder(font).size(self.font_size).build();
        let adv = scaler.scale_advance_width(m_id).unwrap_or(cell_height as f32);
        let cell_width = adv.ceil() as u32;
        let ascent = metrics.ascent.ceil() as i32;

        GlyphMetrics { cell_width, cell_height, ascent }
    }
}
```

**Step 2: Register the module in `src/gui/renderer/mod.rs`**

Find the block of `pub mod` / `mod` declarations and add:
```rust
pub mod rasterizer;
```

**Step 3: Add unit tests inside `rasterizer.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_rasterizer() -> GlyphRasterizer {
        let font_data = crate::config::fonts::font_data(crate::config::FontFamily::JetBrainsMono);
        GlyphRasterizer::new(font_data, vec![], 14.0, RasterMode::Grayscale)
    }

    #[test]
    fn raster_mode_from_scale_factor() {
        assert_eq!(RasterMode::from_scale_factor(1.0), RasterMode::LcdSubpixel);
        assert_eq!(RasterMode::from_scale_factor(2.0), RasterMode::Grayscale);
        assert_eq!(RasterMode::from_scale_factor(3.0), RasterMode::Grayscale);
    }

    #[test]
    fn rasterize_ascii_returns_coverage() {
        let mut r = make_rasterizer();
        let glyph = r.rasterize('A').expect("'A' should rasterize");
        assert!(glyph.width > 0 && glyph.height > 0);
        match glyph.coverage {
            GlyphCoverage::Grayscale(data) => {
                assert_eq!(data.len(), (glyph.width * glyph.height) as usize);
                assert!(data.iter().any(|&b| b > 0), "coverage should be non-zero");
            }
            GlyphCoverage::Lcd(_) => panic!("expected grayscale"),
        }
    }

    #[test]
    fn rasterize_space_returns_none() {
        let mut r = make_rasterizer();
        assert!(r.rasterize(' ').is_none());
    }

    #[test]
    fn metrics_are_nonzero() {
        let mut r = make_rasterizer();
        let m = r.metrics();
        assert!(m.cell_width > 0);
        assert!(m.cell_height > 0);
        assert!(m.ascent > 0);
    }
}
```

**Step 4: Run tests**

```bash
cargo test raster_mode_from_scale_factor rasterize_ascii_returns_coverage \
    rasterize_space_returns_none metrics_are_nonzero -- --nocapture
```

Expected: all pass.

**Step 5: Commit**

```bash
git add src/gui/renderer/rasterizer.rs src/gui/renderer/mod.rs
git commit -m "feat(renderer): add GlyphRasterizer with swash, RasterMode, adaptive DPI"
```

---

## Task 4: Update `config/fonts.rs` — return raw bytes, not fontdue types

**Files:**
- Modify: `src/config/fonts.rs`
- Modify: `src/config/mod.rs` (update public re-exports if any)

`load_fonts` currently returns `(fontdue::Font, Vec<fontdue::Font>)`. We change it to return
`(&'static [u8], Vec<&'static [u8]>)` — raw font bytes that swash and `GlyphRasterizer` need.
Fontdue types disappear from this module entirely.

**Step 1: Rewrite `load_fonts` in `src/config/fonts.rs`**

Replace the entire `load_fonts` function:
```rust
/// Returns raw font bytes for the primary font and fallback chain.
///
/// The returned slices point to data compiled into the binary (`include_bytes!`).
pub(crate) fn load_fonts(family: FontFamily) -> (&'static [u8], Vec<&'static [u8]>) {
    let primary = font_data(family);
    let fallbacks = fallback_fonts_data().to_vec();
    (primary, fallbacks)
}
```

**Step 2: Update the tests in `fonts.rs` to use swash for validation**

Replace the fontdue validation in `all_fonts_load_as_valid` and `fallback_fonts_load_as_valid`:

```rust
#[test]
fn all_fonts_load_as_valid() {
    for family in [
        FontFamily::JetBrainsMono,
        FontFamily::FiraCode,
        FontFamily::CascadiaCode,
        FontFamily::UbuntuMono,
        FontFamily::SourceCodePro,
    ] {
        let data = font_data(family);
        let font = swash::FontRef::from_index(data, 0);
        assert!(font.is_some(), "{family:?} should parse as a valid swash font");
    }
}

#[test]
fn fallback_fonts_load_as_valid() {
    for (i, data) in fallback_fonts_data().iter().enumerate() {
        let font = swash::FontRef::from_index(data, 0);
        assert!(font.is_some(), "fallback font {i} should parse as a valid swash font");
    }
}
```

Update `fallback_chain_covers_missing_glyphs` to use swash charmap:

```rust
#[test]
fn fallback_chain_covers_missing_glyphs() {
    let primary_data = font_data(FontFamily::JetBrainsMono);
    let primary = swash::FontRef::from_index(primary_data, 0).unwrap();

    let fallbacks: Vec<_> = fallback_fonts_data()
        .iter()
        .map(|d| swash::FontRef::from_index(d, 0).unwrap())
        .collect();

    let has_glyph = |font: &swash::FontRef, ch: char| font.charmap().map(ch) != 0;

    assert!(!has_glyph(&primary, '\u{23BF}'));
    assert!(has_glyph(&fallbacks[0], '\u{23BF}'));

    let claude_chars = [
        ('\u{23FA}', "⏺ prompt"),
        ('\u{25CF}', "● prompt fallback"),
        ('\u{23BF}', "⎿ response delimiter"),
        ('\u{273B}', "✻ idle"),
        ('\u{21AF}', "↯ interrupt"),
        ('\u{21BB}', "↻ retry"),
        ('\u{2714}', "✔ check"),
        ('\u{00D7}', "× cancel"),
        ('\u{23F8}', "⏸ plan mode"),
        ('\u{23F5}', "⏵ accept edits"),
        ('\u{2722}', "✢ spinner"),
        ('\u{2733}', "✳ spinner"),
        ('\u{2736}', "✶ spinner"),
        ('\u{273D}', "✽ spinner"),
        ('\u{2718}', "✘ cross"),
        ('\u{276F}', "❯ pointer"),
        ('\u{25B6}', "▶ play"),
        ('\u{23CE}', "⏎ return"),
        ('\u{25C7}', "◇ diamond"),
        ('\u{2630}', "☰ hamburger"),
    ];
    let mut missing = Vec::new();
    for (ch, name) in &claude_chars {
        let covered = has_glyph(&primary, *ch)
            || fallbacks.iter().any(|f| has_glyph(f, *ch));
        if !covered {
            missing.push(*name);
        }
    }
    let critical_missing: Vec<_> = missing.iter()
        .filter(|name| !name.contains("retry"))
        .collect();
    assert!(
        critical_missing.is_empty(),
        "Critical Claude Code icons not covered: {critical_missing:?}"
    );
}
```

**Step 3: Run font tests**

```bash
cargo test --lib config -- --nocapture
```

Expected: all pass.

**Step 4: Commit**

```bash
git add src/config/fonts.rs
git commit -m "refactor(config): load_fonts returns raw bytes, migrate tests to swash"
```

---

## Task 5: Rewrite GPU `atlas.rs` — use `GlyphRasterizer`

**Files:**
- Modify: `src/gui/renderer/gpu/atlas.rs`

The atlas currently takes `&fontdue::Font` in every call. We change it to take
`&mut GlyphRasterizer` instead. The texture format stays `R8Unorm` for grayscale mode;
`Rgba8Unorm` (RGB channels = subpixel coverage, A unused) for LCD mode.
`glyph_info_buffer_data()` and the `GlyphInfo` struct are unchanged.

**Step 1: Replace the entire `atlas.rs` content**

```rust
//! Glyph atlas — rasterizes glyphs via [`GlyphRasterizer`] and packs them into a GPU texture.
//!
//! Pre-populates ASCII 32..127 on creation; adds other glyphs lazily.
//! Texture format depends on raster mode:
//!   Grayscale  → R8Unorm   (1 byte per texel: alpha coverage)
//!   LcdSubpixel→ Rgba8Unorm (3 bytes per texel: R_cov, G_cov, B_cov; A unused)

use std::collections::HashMap;
use wgpu;

use crate::gui::renderer::rasterizer::{GlyphCoverage, GlyphRasterizer, RasterMode};

/// Per-glyph metadata stored in a GPU storage buffer. Must match the WGSL `GlyphInfo` layout.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlyphInfo {
    pub x:        f32,
    pub y:        f32,
    pub w:        f32,
    pub h:        f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub _pad1:    f32,
    pub _pad2:    f32,
}

pub struct GlyphAtlas {
    pub texture:      wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub atlas_width:  u32,
    pub atlas_height: u32,
    pub mode:         RasterMode,
    glyphs:           HashMap<u32, GlyphInfo>,
    next_x:           u32,
    next_y:           u32,
    row_height:       u32,
}

impl GlyphAtlas {
    /// Creates a new atlas and pre-populates printable ASCII (32..127).
    pub fn new(
        device:     &wgpu::Device,
        queue:      &wgpu::Queue,
        rasterizer: &mut GlyphRasterizer,
    ) -> Self {
        let atlas_width  = 1024u32;
        let atlas_height = 1024u32;
        let mode = rasterizer.mode;

        let format = match mode {
            RasterMode::Grayscale   => wgpu::TextureFormat::R8Unorm,
            RasterMode::LcdSubpixel => wgpu::TextureFormat::Rgba8Unorm,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph_atlas"),
            size: wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut atlas = GlyphAtlas {
            texture,
            texture_view,
            atlas_width,
            atlas_height,
            mode,
            glyphs: HashMap::new(),
            next_x: 0,
            next_y: 0,
            row_height: 0,
        };

        for cp in 32u32..127 {
            if let Some(ch) = char::from_u32(cp) {
                atlas.insert_glyph(queue, rasterizer, cp, ch);
            }
        }

        atlas
    }

    /// Returns glyph info for `codepoint`, inserting it lazily if missing.
    pub fn get_or_insert(
        &mut self,
        codepoint: u32,
        rasterizer: &mut GlyphRasterizer,
        queue: &wgpu::Queue,
    ) -> GlyphInfo {
        if let Some(&info) = self.glyphs.get(&codepoint) {
            return info;
        }
        if let Some(ch) = char::from_u32(codepoint) {
            self.insert_glyph(queue, rasterizer, codepoint, ch);
        }
        self.glyphs.get(&codepoint).copied().unwrap_or_default()
    }

    fn insert_glyph(
        &mut self,
        queue:      &wgpu::Queue,
        rasterizer: &mut GlyphRasterizer,
        codepoint:  u32,
        ch:         char,
    ) {
        let Some(glyph) = rasterizer.rasterize(ch) else {
            self.glyphs.insert(codepoint, GlyphInfo::default());
            return;
        };

        let gw = glyph.width;
        let gh = glyph.height;

        if self.next_x + gw > self.atlas_width {
            self.next_y += self.row_height;
            self.next_x = 0;
            self.row_height = 0;
        }
        if self.next_y + gh > self.atlas_height {
            self.glyphs.insert(codepoint, GlyphInfo::default());
            return;
        }

        let (bytes_per_row, upload_data): (u32, Vec<u8>) = match &glyph.coverage {
            GlyphCoverage::Grayscale(data) => (gw, data.clone()),
            GlyphCoverage::Lcd(data) => {
                // Pack [R, G, B] into RGBA: A=0 (unused).
                let rgba: Vec<u8> = data.iter()
                    .flat_map(|&[r, g, b]| [r, g, b, 0u8])
                    .collect();
                (gw * 4, rgba)
            }
        };

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture:   &self.texture,
                mip_level: 0,
                origin:    wgpu::Origin3d { x: self.next_x, y: self.next_y, z: 0 },
                aspect:    wgpu::TextureAspect::All,
            },
            &upload_data,
            wgpu::TexelCopyBufferLayout {
                offset:         0,
                bytes_per_row:  Some(bytes_per_row),
                rows_per_image: Some(gh),
            },
            wgpu::Extent3d { width: gw, height: gh, depth_or_array_layers: 1 },
        );

        let metrics = rasterizer.metrics();
        let offset_x = glyph.left as f32;
        let offset_y = (metrics.ascent - glyph.top) as f32;

        self.glyphs.insert(codepoint, GlyphInfo {
            x:        self.next_x as f32,
            y:        self.next_y as f32,
            w:        gw as f32,
            h:        gh as f32,
            offset_x,
            offset_y,
            _pad1: 0.0,
            _pad2: 0.0,
        });

        self.next_x    += gw + 1;
        self.row_height = self.row_height.max(gh + 1);
    }

    /// Builds the flat glyph info array for the GPU storage buffer.
    pub fn glyph_info_buffer_data(&self) -> Vec<GlyphInfo> {
        let max_cp = self.glyphs.keys().copied().max().unwrap_or(0) as usize;
        let len = (max_cp + 1).max(128);
        let mut data = vec![GlyphInfo::default(); len];
        for (&cp, &info) in &self.glyphs {
            if (cp as usize) < len {
                data[cp as usize] = info;
            }
        }
        data
    }
}
```

**Step 2: Ensure it compiles (atlas callers will break — fixed in Task 6)**

```bash
cargo check 2>&1 | grep "atlas" | head -20
```

**Step 3: Commit**

```bash
git add src/gui/renderer/gpu/atlas.rs
git commit -m "refactor(gpu/atlas): use GlyphRasterizer, dual R8/RGBA format for LCD"
```

---

## Task 6: Update `GpuRenderer` struct and callers to use `GlyphRasterizer`

**Files:**
- Modify: `src/gui/renderer/gpu/mod.rs`
- Modify: `src/gui/renderer/gpu/setup.rs`
- Modify: `src/gui/renderer/gpu/grid_packing.rs`

Replace `font: fontdue::Font` and `fallback_fonts: Vec<fontdue::Font>` fields with
`rasterizer: GlyphRasterizer`.

**Step 1: Update `GpuRenderer` struct in `mod.rs`**

Remove:
```rust
use fontdue::Font;
// ...
font: Font,
fallback_fonts: Vec<Font>,
```

Add:
```rust
use crate::gui::renderer::rasterizer::GlyphRasterizer;
// ...
pub(super) rasterizer: GlyphRasterizer,
```

**Step 2: Update `GpuRenderer::new` in `setup.rs`**

Replace:
```rust
let (font, fallback_fonts) = crate::config::load_fonts(config.font.family);
let mut metrics = FontMetrics::from_config(config);
metrics.recompute(&font);
// ...
let atlas = GlyphAtlas::new(&device, &queue, &font, &fallback_fonts,
    metrics.font_size, metrics.ascent);
```

With:
```rust
use crate::gui::renderer::rasterizer::{GlyphRasterizer, RasterMode};

let (font_data, fallback_data) = crate::config::load_fonts(config.font.family);
let scale_factor = window.scale_factor();
let mode = RasterMode::from_scale_factor(scale_factor);
let mut rasterizer = GlyphRasterizer::new(font_data, fallback_data, config.font.size, mode);
let cell_metrics = rasterizer.metrics();
let mut metrics = FontMetrics::from_config(config);
metrics.cell_width  = cell_metrics.cell_width;
metrics.cell_height = cell_metrics.cell_height;
metrics.ascent      = cell_metrics.ascent;

let atlas = GlyphAtlas::new(&device, &queue, &mut rasterizer);
```

Also update `apply_config`:
```rust
pub(in crate::gui::renderer) fn apply_config(&mut self, config: &crate::config::AppConfig) {
    let (font_data, fallback_data) = crate::config::load_fonts(config.font.family);
    self.rasterizer = GlyphRasterizer::new(
        font_data, fallback_data,
        config.font.size,
        self.rasterizer.mode,
    );
    let cell_metrics = self.rasterizer.metrics();
    self.metrics.cell_width  = cell_metrics.cell_width;
    self.metrics.cell_height = cell_metrics.cell_height;
    self.metrics.ascent      = cell_metrics.ascent;
    self.rebuild_atlas();
    self.palette = config.theme.resolve();
}

pub(super) fn rebuild_atlas(&mut self) {
    self.atlas = GlyphAtlas::new(&self.device, &self.queue, &mut self.rasterizer);
    let glyph_data = self.atlas.glyph_info_buffer_data();
    self.glyph_info_buffer = Self::create_storage_buffer_init(
        &self.device,
        bytemuck::cast_slice(&glyph_data),
        "glyph_info",
    );
}
```

Store `rasterizer` in the struct init:
```rust
Ok(super::GpuRenderer {
    // ... existing fields ...
    rasterizer,
    // remove: font, fallback_fonts
})
```

**Step 3: Update `grid_packing.rs` — atlas call**

In `pack_grid_cells`, find:
```rust
let _ = self.atlas.get_or_insert(
    codepoint,
    &self.font,
    &self.fallback_fonts,
    self.metrics.font_size,
    &self.queue,
);
```

Replace with:
```rust
let _ = self.atlas.get_or_insert(
    codepoint,
    &mut self.rasterizer,
    &self.queue,
);
```

**Step 4: Check compilation**

```bash
cargo check 2>&1 | grep "^error" | head -30
```

Fix any remaining borrow issues (GpuRenderer borrows `rasterizer` mutably — may need
to clone atlas call or restructure to avoid simultaneous borrows).

**Step 5: Commit**

```bash
git add src/gui/renderer/gpu/mod.rs \
        src/gui/renderer/gpu/setup.rs \
        src/gui/renderer/gpu/grid_packing.rs
git commit -m "refactor(gpu): replace fontdue::Font with GlyphRasterizer in GpuRenderer"
```

---

## Task 7: Rewrite `grid.wgsl` as a render shader with gamma-correct blending

**Files:**
- Modify: `src/gui/renderer/gpu/shaders/grid.wgsl`

This replaces the compute shader with a vertex + fragment render shader.
The fragment shader positions itself identically to the compute shader (per-pixel cell lookup),
but now outputs linear color to `Rgba8UnormSrgb` (GPU encodes to sRGB automatically).
A uniform `is_lcd` selects per-channel (LCD) vs single-alpha (grayscale) blending.

**Step 1: Replace the entire `grid.wgsl` content**

```wgsl
// Grid render shader — renders terminal cells into a Rgba8UnormSrgb texture.
//
// A fullscreen triangle is drawn. The fragment shader determines which cell
// each pixel belongs to, blends glyph coverage against palette colors in
// linear light space, and outputs linear values. The Rgba8UnormSrgb attachment
// encodes them to sRGB automatically.

// ---- Uniforms ----

struct GridUniforms {
    cols:         u32,
    rows:         u32,
    cell_width:   u32,
    cell_height:  u32,
    origin_x:     u32,
    origin_y:     u32,
    bg_color:     u32,   // default background 0xRRGGBB (sRGB)
    is_lcd:       u32,   // 1 = LCD subpixel atlas, 0 = grayscale atlas
    tex_width:    u32,
    tex_height:   u32,
    _pad1:        u32,
    _pad2:        u32,
}

struct Cell {
    codepoint: u32,
    fg:        u32,   // 0xRRGGBB sRGB
    bg:        u32,   // 0xRRGGBB sRGB
    attrs:     u32,
}

struct GlyphInfo {
    x:        f32,
    y:        f32,
    w:        f32,
    h:        f32,
    offset_x: f32,
    offset_y: f32,
    _pad1:    f32,
    _pad2:    f32,
}

// ---- Bindings ----

@group(0) @binding(0) var<uniform>       uniforms: GridUniforms;
@group(0) @binding(1) var<storage, read> cells:    array<Cell>;
@group(0) @binding(2) var<storage, read> glyphs:   array<GlyphInfo>;
@group(0) @binding(3) var               atlas:     texture_2d<f32>;
@group(0) @binding(4) var               atlas_smp: sampler;

// ---- Vertex / Fragment IO ----

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0)       uv:       vec2<f32>,
}

// ---- Helpers ----

/// Unpack 0xRRGGBB and decode sRGB → linear (γ = 2.2 approximation).
fn unpack_linear(c: u32) -> vec3<f32> {
    let s = vec3<f32>(
        f32((c >> 16u) & 0xFFu) / 255.0,
        f32((c >>  8u) & 0xFFu) / 255.0,
        f32( c         & 0xFFu) / 255.0,
    );
    return pow(s, vec3<f32>(2.2));
}

// ---- Vertex stage: fullscreen triangle ----

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    let x = f32(i32(vi) / 2) * 4.0 - 1.0;
    let y = f32(i32(vi) % 2) * 4.0 - 1.0;
    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv       = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

// ---- Fragment stage ----

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_x = u32(in.position.x) - uniforms.origin_x;
    let pixel_y = u32(in.position.y) - uniforms.origin_y;

    // Batch-local bounds guard.
    let batch_w = uniforms.cols * uniforms.cell_width;
    let batch_h = uniforms.rows * uniforms.cell_height;
    if pixel_x >= batch_w || pixel_y >= batch_h {
        discard;
    }

    let col      = pixel_x / uniforms.cell_width;
    let row      = pixel_y / uniforms.cell_height;
    let cell_idx = row * uniforms.cols + col;
    let cell     = cells[cell_idx];

    let cell_x = pixel_x - col * uniforms.cell_width;
    let cell_y = pixel_y - row * uniforms.cell_height;

    // Wide-right spacer: shift glyph sample left by one cell.
    let is_wide_right = (cell.attrs & 256u) != 0u;
    var adj_cell_x = cell_x;
    if is_wide_right {
        adj_cell_x = cell_x + uniforms.cell_width;
    }

    // Resolve fg/bg with reverse video.
    var fg = cell.fg;
    var bg = cell.bg;
    if (cell.attrs & 8u) != 0u {
        let tmp = fg;
        fg = bg;
        bg = tmp;
    }

    let is_dim = (cell.attrs & 16u) != 0u;

    // Decode sRGB palette → linear.
    var fg_lin = unpack_linear(fg);
    let bg_lin = unpack_linear(bg);

    if is_dim {
        fg_lin = fg_lin * 0.6;
    }

    // Start with background.
    var color = bg_lin;

    // Glyph blending.
    let glyph_count = arrayLength(&glyphs);
    if cell.codepoint > 32u && cell.codepoint < glyph_count {
        let glyph = glyphs[cell.codepoint];
        if glyph.w > 0.0 {
            let gx = f32(adj_cell_x) - glyph.offset_x;
            let gy = f32(cell_y)     - glyph.offset_y;

            if gx >= 0.0 && gx < glyph.w && gy >= 0.0 && gy < glyph.h {
                let atlas_size = vec2<f32>(textureDimensions(atlas));
                let uv = vec2<f32>(
                    (glyph.x + gx) / atlas_size.x,
                    (glyph.y + gy) / atlas_size.y,
                );
                let sample = textureSampleLevel(atlas, atlas_smp, uv, 0.0);

                if uniforms.is_lcd == 1u {
                    // LCD: per-channel blend.
                    color = vec3<f32>(
                        mix(bg_lin.r, fg_lin.r, sample.r),
                        mix(bg_lin.g, fg_lin.g, sample.g),
                        mix(bg_lin.b, fg_lin.b, sample.b),
                    );
                } else {
                    // Grayscale: single-alpha blend.
                    color = mix(bg_lin, fg_lin, sample.r);
                }
            }
        }
    }

    // Decorations (underline, strikethrough) — use dimmed fg.
    var decor_lin = fg_lin;
    let underline_style = (cell.attrs >> 6u) & 3u;
    if underline_style == 1u && cell_y >= uniforms.cell_height - 2u {
        color = decor_lin;
    } else if underline_style == 2u &&
              (cell_y == uniforms.cell_height - 1u || cell_y == uniforms.cell_height - 3u) {
        color = decor_lin;
    }
    if (cell.attrs & 32u) != 0u && cell_y == uniforms.cell_height / 2u {
        color = decor_lin;
    }

    // Output linear. Rgba8UnormSrgb attachment encodes → sRGB automatically.
    return vec4<f32>(color, 1.0);
}
```

**Step 2: Note the key change in `GridUniforms`**

`_pad0` is replaced by `is_lcd: u32`. Update `buffers.rs` to match:
find `GridUniforms` and replace `_pad0: u32` with `is_lcd: u32`.

**Step 3: Commit**

```bash
git add src/gui/renderer/gpu/shaders/grid.wgsl
git commit -m "feat(gpu/shader): rewrite grid as render shader with linear blending"
```

---

## Task 8: Migrate grid pipeline from compute to render in `pipelines.rs`

**Files:**
- Modify: `src/gui/renderer/gpu/pipelines.rs`
- Modify: `src/gui/renderer/gpu/buffers.rs` (update `GridUniforms._pad0` → `is_lcd`)

**Step 1: Update `GridUniforms` in `buffers.rs`**

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridUniforms {
    pub cols:        u32,
    pub rows:        u32,
    pub cell_width:  u32,
    pub cell_height: u32,
    pub origin_x:    u32,
    pub origin_y:    u32,
    pub bg_color:    u32,
    pub is_lcd:      u32,   // was _pad0
    pub tex_width:   u32,
    pub tex_height:  u32,
    pub _pad1:       u32,
    pub _pad2:       u32,
}
```

**Step 2: Replace `create_grid_pipeline` in `pipelines.rs`**

```rust
/// Creates the grid render pipeline and its bind group layout.
///
/// Bindings:
///   0: GridUniforms (uniform)
///   1: cells array  (storage, read-only)
///   2: glyphs array (storage, read-only)
///   3: atlas texture (texture_2d<f32>)
///   4: atlas sampler
pub fn create_grid_pipeline(
    device: &wgpu::Device,
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    let shader_src = include_str!("shaders/grid.wgsl");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label:  Some("grid_shader"),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label:   Some("grid_bind_group_layout"),
        entries: &[
            // 0: uniforms
            wgpu::BindGroupLayoutEntry {
                binding:    0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // 1: cells
            wgpu::BindGroupLayoutEntry {
                binding:    1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // 2: glyphs
            wgpu::BindGroupLayoutEntry {
                binding:    2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // 3: atlas texture
            wgpu::BindGroupLayoutEntry {
                binding:    3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled:  false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type:   wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            // 4: atlas sampler
            wgpu::BindGroupLayoutEntry {
                binding:    4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label:                Some("grid_pipeline_layout"),
        bind_group_layouts:   &[&bind_group_layout],
        immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label:  Some("grid_render_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module:               &shader,
            entry_point:          Some("vs_main"),
            buffers:              &[],
            compilation_options:  Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module:      &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format:     wgpu::TextureFormat::Rgba8UnormSrgb,
                blend:      None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil:  None,
        multisample:    wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache:          None,
    });

    (pipeline, bind_group_layout)
}
```

**Step 3: Update `GpuRenderer` struct field type in `mod.rs`**

```rust
// Before
grid_pipeline: wgpu::ComputePipeline,

// After
grid_pipeline: wgpu::RenderPipeline,
```

**Step 4: Commit**

```bash
git add src/gui/renderer/gpu/pipelines.rs \
        src/gui/renderer/gpu/buffers.rs \
        src/gui/renderer/gpu/mod.rs
git commit -m "feat(gpu/pipeline): migrate grid from compute to render pipeline, Rgba8UnormSrgb"
```

---

## Task 9: Update grid texture format and pass encoding

**Files:**
- Modify: `src/gui/renderer/gpu/setup.rs`
- Modify: `src/gui/renderer/gpu/gpu_passes.rs`
- Modify: `src/gui/renderer/gpu/grid_packing.rs` (set `is_lcd` in uniforms)

**Step 1: Update grid texture in `setup.rs`**

In `create_offscreen_texture` call for `"grid_texture"`, change:

```rust
// Before (in GpuRenderer::new)
let (grid_texture, grid_texture_view) =
    Self::create_offscreen_texture(&device, width, height, "grid_texture", true);
```

The `true` parameter selects `STORAGE_BINDING` — no longer needed. Change the call to pass
`Rgba8UnormSrgb` explicitly. Update `create_offscreen_texture` signature to accept format:

```rust
pub(super) fn create_offscreen_texture(
    device:  &wgpu::Device,
    width:   u32,
    height:  u32,
    label:   &str,
    format:  wgpu::TextureFormat,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count:    1,
        dimension:       wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}
```

Callers in `new` and `resize_textures`:

```rust
let (grid_texture, grid_texture_view) =
    Self::create_offscreen_texture(&device, width, height, "grid_texture",
        wgpu::TextureFormat::Rgba8UnormSrgb);

let (ui_texture, ui_texture_view) =
    Self::create_offscreen_texture(&device, width, height, "ui_texture",
        wgpu::TextureFormat::Rgba8Unorm);
```

**Step 2: Replace `encode_grid_batch_pass` in `gpu_passes.rs`**

```rust
/// Encodes one grid render batch (Pass 1).
pub(super) fn encode_grid_batch_pass(
    &self,
    encoder:        &mut wgpu::CommandEncoder,
    dispatch_width:  u32,
    dispatch_height: u32,
) {
    if dispatch_width == 0 || dispatch_height == 0 {
        return;
    }

    let grid_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label:   Some("grid_bind_group"),
        layout:  &self.grid_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding:  0,
                resource: self.grid_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding:  1,
                resource: self.grid_cell_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding:  2,
                resource: self.glyph_info_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding:  3,
                resource: wgpu::BindingResource::TextureView(&self.atlas.texture_view),
            },
            wgpu::BindGroupEntry {
                binding:  4,
                resource: wgpu::BindingResource::Sampler(&self.sampler),
            },
        ],
    });

    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("grid_render_pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view:           &self.grid_texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load:  wgpu::LoadOp::Load,   // grid_packing clears first batch
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes:         None,
        occlusion_query_set:      None,
        multiview_mask:           None,
    });
    render_pass.set_pipeline(&self.grid_pipeline);
    render_pass.set_bind_group(0, &grid_bind_group, &[]);
    render_pass.draw(0..3, 0..1);
}
```

**Step 3: Update `GridUniforms` construction in `grid_packing.rs`**

In `ensure_grid_frame_started` and `queue_grid_batch`, set `is_lcd`:

```rust
uniforms: GridUniforms {
    // ...existing fields...
    is_lcd: if self.rasterizer.mode == RasterMode::LcdSubpixel { 1 } else { 0 },
    // ...
},
```

Add `use crate::gui::renderer::rasterizer::RasterMode;` at the top if needed.

**Step 4: Check compilation**

```bash
cargo check
```

**Step 5: Commit**

```bash
git add src/gui/renderer/gpu/setup.rs \
        src/gui/renderer/gpu/gpu_passes.rs \
        src/gui/renderer/gpu/grid_packing.rs
git commit -m "feat(gpu): grid texture Rgba8UnormSrgb, encode as render pass, set is_lcd uniform"
```

---

## Task 10: Update CPU renderer — swash + gamma-correct blend

**Files:**
- Modify: `src/gui/renderer/cpu/mod.rs`
- Modify: `src/gui/renderer/cpu/primitives.rs`

**Step 1: Update `CpuRenderer` struct in `cpu/mod.rs`**

Remove all fontdue fields. Add:

```rust
use crate::core::Color;
use crate::gui::renderer::rasterizer::{GlyphCoverage, GlyphRasterizer, RasterMode};

pub struct CpuRenderer {
    // ... existing fields (metrics, palette, etc.) ...
    rasterizer:     GlyphRasterizer,
    glyph_cache:    HashMap<char, CachedGlyph>,
    srgb_to_linear: [f32; 256],  // precomputed: (i/255)^2.2
}

struct CachedGlyph {
    coverage: GlyphCoverage,
    width:    u32,
    height:   u32,
    left:     i32,
    top:      i32,
}
```

Initialize the LUT in the constructor:

```rust
fn build_srgb_lut() -> [f32; 256] {
    let mut lut = [0f32; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        *v = (i as f32 / 255.0).powf(2.2);
    }
    lut
}
```

**Step 2: Rewrite `draw_char` in `cpu/primitives.rs`**

```rust
pub(in crate::gui::renderer) fn draw_char(
    &mut self,
    target: &mut RenderTarget<'_>,
    x: u32,
    y: u32,
    character: char,
    fg: Color,
) {
    use std::collections::hash_map::Entry;

    // Rasterize if not cached.
    if !self.glyph_cache.contains_key(&character) {
        if let Some(g) = self.rasterizer.rasterize(character) {
            self.glyph_cache.insert(character, CachedGlyph {
                width:    g.width,
                height:   g.height,
                left:     g.left,
                top:      g.top,
                coverage: g.coverage,
            });
        } else {
            return; // space or missing glyph
        }
    }
    let glyph = match self.glyph_cache.get(&character) {
        Some(g) => g,
        None    => return,
    };

    let m = self.rasterizer.metrics();

    // Decode fg to linear using precomputed LUT.
    let fg_lr = self.srgb_to_linear[fg.r as usize];
    let fg_lg = self.srgb_to_linear[fg.g as usize];
    let fg_lb = self.srgb_to_linear[fg.b as usize];

    for gy in 0..glyph.height as usize {
        for gx in 0..glyph.width as usize {
            let sx = x as i32 + glyph.left + gx as i32;
            let sy = y as i32 + (m.ascent - glyph.top) + gy as i32;
            if sx < 0 || sy < 0
                || sx as usize >= target.width
                || sy as usize >= target.height
            {
                continue;
            }
            let idx = sy as usize * target.width + sx as usize;
            let bg_pixel = target.buffer[idx];
            let bg_lr = self.srgb_to_linear[((bg_pixel >> 16) & 0xFF) as usize];
            let bg_lg = self.srgb_to_linear[((bg_pixel >>  8) & 0xFF) as usize];
            let bg_lb = self.srgb_to_linear[ (bg_pixel        & 0xFF) as usize];

            let pixel = match &glyph.coverage {
                GlyphCoverage::Grayscale(data) => {
                    let t = data[gy * glyph.width as usize + gx] as f32 / 255.0;
                    let r = Color::channel_to_srgb(fg_lr * t + bg_lr * (1.0 - t));
                    let g = Color::channel_to_srgb(fg_lg * t + bg_lg * (1.0 - t));
                    let b = Color::channel_to_srgb(fg_lb * t + bg_lb * (1.0 - t));
                    (r as u32) << 16 | (g as u32) << 8 | b as u32
                }
                GlyphCoverage::Lcd(data) => {
                    let [rt, gt, bt] = data[gy * glyph.width as usize + gx];
                    let r = Color::channel_to_srgb(
                        fg_lr * rt as f32 / 255.0 + bg_lr * (1.0 - rt as f32 / 255.0));
                    let g = Color::channel_to_srgb(
                        fg_lg * gt as f32 / 255.0 + bg_lg * (1.0 - gt as f32 / 255.0));
                    let b = Color::channel_to_srgb(
                        fg_lb * bt as f32 / 255.0 + bg_lb * (1.0 - bt as f32 / 255.0));
                    (r as u32) << 16 | (g as u32) << 8 | b as u32
                }
            };
            target.buffer[idx] = pixel;
        }
    }
}
```

**Step 3: Check compilation**

```bash
cargo check
```

**Step 4: Commit**

```bash
git add src/gui/renderer/cpu/mod.rs src/gui/renderer/cpu/primitives.rs
git commit -m "feat(cpu): gamma-correct glyph blend with swash, LCD per-channel and grayscale"
```

---

## Task 11: Handle DPI change — rebuild rasterizer on `ScaleFactorChanged`

**Files:**
- Grep for `ScaleFactorChanged` in `src/gui/lifecycle/` to find the handler
- Modify that file

**Step 1: Find the handler**

```bash
grep -rn "ScaleFactorChanged" src/gui/lifecycle/
```

**Step 2: Add rasterizer rebuild to the handler**

Find where `ScaleFactorChanged` is handled and add after the existing resize logic:

```rust
WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
    // ... existing resize handling ...

    let new_mode = RasterMode::from_scale_factor(*scale_factor);
    // GPU renderer
    if let Some(gpu) = window.renderer.as_gpu_mut() {
        if gpu.rasterizer.mode != new_mode {
            gpu.rasterizer.rebuild(gpu.metrics.font_size, new_mode);
            gpu.rebuild_atlas();
        }
    }
    // CPU renderer
    if let Some(cpu) = window.renderer.as_cpu_mut() {
        if cpu.rasterizer.mode != new_mode {
            cpu.rasterizer.rebuild(cpu.metrics.font_size, new_mode);
            cpu.glyph_cache.clear();
        }
    }
}
```

Note: adjust the renderer accessor method names to match what actually exists in
`src/gui/renderer/backend.rs`.

**Step 3: Full build and all tests**

```bash
cargo clippy
cargo test
```

Expected: zero warnings, all tests pass.

**Step 4: Commit**

```bash
git add src/gui/lifecycle/
git commit -m "feat(lifecycle): rebuild rasterizer and atlas on ScaleFactorChanged"
```

---

## Task 12: Final verification

**Step 1: Run full test suite**

```bash
cargo test -- --nocapture
```

**Step 2: Run clippy — must be zero warnings**

```bash
cargo clippy
```

**Step 3: Build and run the app to visually confirm**

```bash
cargo run
```

Open a shell in the terminal window. Run `ls --color` or a colorized prompt and compare
saturation against Ghostty. Anti-aliased text edges should appear darker/crisper than before.

**Step 4: Commit if any minor cleanups were needed**

```bash
git add -p
git commit -m "chore: post-migration cleanup"
```

---

## Reference: Key File Locations

| What | Where |
|------|-------|
| sRGB helpers | `src/core/color.rs` |
| GlyphRasterizer | `src/gui/renderer/rasterizer.rs` |
| GPU atlas | `src/gui/renderer/gpu/atlas.rs` |
| Grid shader | `src/gui/renderer/gpu/shaders/grid.wgsl` |
| Grid pipeline | `src/gui/renderer/gpu/pipelines.rs` |
| Grid pass encoding | `src/gui/renderer/gpu/gpu_passes.rs` |
| Grid uniforms | `src/gui/renderer/gpu/buffers.rs` |
| GPU setup / textures | `src/gui/renderer/gpu/setup.rs` |
| GPU struct | `src/gui/renderer/gpu/mod.rs` |
| CPU renderer | `src/gui/renderer/cpu/mod.rs`, `primitives.rs` |
| Font loading | `src/config/fonts.rs` |
| DPI lifecycle | `src/gui/lifecycle/` (grep ScaleFactorChanged) |
