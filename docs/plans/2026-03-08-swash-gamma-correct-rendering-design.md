# Design: swash + Gamma-Correct Rendering

**Date:** 2026-03-08
**Branch:** feat/swash-gamma-correct-rendering
**Problem:** Terminal colors appear washed out compared to Ghostty due to gamma-incorrect
alpha blending and grayscale-only font rasterization.

## Root Cause

Both renderers blend glyph coverage (linear alpha from fontdue) against sRGB palette values
as if the palette were linear. This produces lighter-than-correct anti-aliased edges, making
text appear pale and less saturated.

Additionally, fontdue produces only grayscale coverage with no LCD subpixel support, which
further reduces perceived sharpness on non-Retina displays.

## Goals

1. Replace fontdue with swash for font rasterization.
2. Enable adaptive rendering: LCD subpixel on non-Retina (scale_factor < 2.0), grayscale AA
   on Retina (scale_factor >= 2.0).
3. Fix gamma-incorrect blending in both GPU and CPU renderers.
4. Migrate the GPU grid pass from a compute shader to a render pipeline using
   `Rgba8UnormSrgb` as the color attachment, so the GPU handles linear→sRGB encoding
   automatically.
5. Keep the composite shader and non-sRGB surface unchanged.

## Architecture

### Data Flow (after)

```
swash
  Grayscale (Retina >=2x):   Vec<u8>  alpha coverage
  LCD Subpixel (1x):         Vec<u8>  per-channel R,G,B coverage
        |
        v
GlyphRasterizer  (src/gui/renderer/rasterizer.rs)
  shared by GPU atlas + CPU glyph cache
        |
   +---------+
   |         |
GPU Atlas   CPU glyph cache
Rgba8Unorm  (same GlyphCoverage enum)
   |
   v
render pipeline  (was: compute pipeline)
grid fragment shader:
  unpack_srgb(fg), unpack_srgb(bg)  ->  linear
  blend in linear  (grayscale OR per-channel LCD)
  output linear  ->  Rgba8UnormSrgb attachment  (GPU encodes to sRGB)
   |
   v
composite shader  (unchanged)
   |
   v
non-sRGB surface  (unchanged)
```

### Key Principles

- Palette colors remain stored as sRGB hex values. Conversion to linear happens only at
  blend time, never at rest.
- `RasterMode` is determined once from `window.scale_factor()` and rebuilt on
  `ScaleFactorChanged`.
- GPU grid texture changes from `Rgba8Unorm` to `Rgba8UnormSrgb`. The composite shader
  reads it as linear automatically (wgpu decodes on sample).
- The non-sRGB surface and composite shader require no changes.

## New Module: `src/gui/renderer/rasterizer.rs`

```rust
pub enum RasterMode {
    Grayscale,      // scale_factor >= 2.0
    LcdSubpixel,    // scale_factor < 2.0
}

impl RasterMode {
    pub fn from_scale_factor(scale: f64) -> Self {
        if scale >= 2.0 { RasterMode::Grayscale } else { RasterMode::LcdSubpixel }
    }
}

pub enum GlyphCoverage {
    Grayscale(Vec<u8>),    // 1 byte per pixel
    Lcd(Vec<[u8; 3]>),     // 3 bytes per pixel: R_cov, G_cov, B_cov
}

pub struct RasterizedGlyph {
    pub coverage: GlyphCoverage,
    pub width:    u32,
    pub height:   u32,
    pub left:     i32,
    pub top:      i32,
}

pub struct GlyphRasterizer {
    context:   swash::scale::ScaleContext,
    font_data: Vec<u8>,
    fallbacks: Vec<Vec<u8>>,
    font_size: f32,
    mode:      RasterMode,
}

impl GlyphRasterizer {
    pub fn new(font_data: Vec<u8>, fallbacks: Vec<Vec<u8>>,
               font_size: f32, mode: RasterMode) -> Self;
    pub fn rasterize(&mut self, ch: char) -> Option<RasterizedGlyph>;
    pub fn metrics(&self, ch: char) -> GlyphMetrics;
    pub fn rebuild(&mut self, font_size: f32, mode: RasterMode);
}
```

swash internals:
- `Format::Alpha` for Grayscale mode
- `Format::Subpixel` for LcdSubpixel mode
- Hinting always enabled

## GPU Pipeline Changes

### Grid texture

```rust
// Before
TextureFormat::Rgba8Unorm

// After
TextureFormat::Rgba8UnormSrgb
```

### Grid pipeline

```rust
// Before: ComputePipeline with storage texture
// After:  RenderPipeline with ColorAttachment

ColorTargetState {
    format: TextureFormat::Rgba8UnormSrgb,
    blend:  None,
    write_mask: ColorWrites::ALL,
}
```

### Grid shader (`shaders/grid.wgsl`)

Rewritten as vertex + fragment shader (fullscreen triangle, same approach as composite).

```wgsl
fn srgb_to_linear(c: f32) -> f32 {
    return pow(c, 2.2);
}

fn unpack_linear(packed: u32) -> vec3<f32> {
    let s = unpack_srgb(packed);        // divide by 255
    return vec3(srgb_to_linear(s.r),
                srgb_to_linear(s.g),
                srgb_to_linear(s.b));
}

// Grayscale blend
let a = textureSample(atlas, smp, uv).r;
let out_linear = mix(bg_linear, fg_linear, a);

// LCD blend
let cov = textureSample(atlas, smp, uv).rgb;
let out_linear = vec3(
    mix(bg_linear.r, fg_linear.r, cov.r),
    mix(bg_linear.g, fg_linear.g, cov.g),
    mix(bg_linear.b, fg_linear.b, cov.b),
);

// Output linear -> Rgba8UnormSrgb encodes automatically
return vec4(out_linear, 1.0);
```

Shader receives uniform `is_lcd: u32` to select blend path.

## CPU Renderer Changes

### `src/core/color.rs` — new helpers

```rust
impl Color {
    pub fn channel_to_linear(c: u8) -> f32 {
        (c as f32 / 255.0).powf(2.2)
    }
    pub fn channel_to_srgb(c: f32) -> u8 {
        (c.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8
    }
}
```

### `src/gui/renderer/cpu/mod.rs` — LUT

```rust
pub struct CpuRenderer {
    // existing fields...
    srgb_to_linear: [f32; 256],  // precomputed at init: (i/255)^2.2
}
```

### `src/gui/renderer/cpu/primitives.rs` — gamma-correct blend

```rust
// Grayscale
let t = alpha as f32 / 255.0;
let r = Color::channel_to_srgb(fg_lin.r * t + bg_lin.r * (1.0 - t));
// ... g, b same

// LCD
let [r_t, g_t, b_t] = [r_cov, g_cov, b_cov].map(|c| c as f32 / 255.0);
let r = Color::channel_to_srgb(fg_lin.r * r_t + bg_lin.r * (1.0 - r_t));
// ... g, b with respective coverage channels
```

## DPI-Adaptive Lifecycle

```
ScaleFactorChanged { scale_factor }
  -> new_mode = RasterMode::from_scale_factor(scale_factor)
  -> if new_mode != current_mode:
       rasterizer.rebuild(font_size, new_mode)
       gpu: rebuild_atlas()          // new texture format
       cpu: clear_glyph_cache()
```

## Files Changed

| File | Change |
|------|--------|
| `Cargo.toml` | remove fontdue, add swash |
| `src/gui/renderer/rasterizer.rs` | NEW — GlyphRasterizer, RasterMode, GlyphCoverage |
| `src/gui/renderer/gpu/atlas.rs` | full rewrite: swash, dual format (R8/Rgba8) |
| `src/gui/renderer/gpu/shaders/grid.wgsl` | rewrite as render shader (vs+fs) |
| `src/gui/renderer/gpu/pipelines.rs` | grid pipeline: compute → render |
| `src/gui/renderer/gpu/setup.rs` | grid texture: Rgba8Unorm → Rgba8UnormSrgb |
| `src/gui/renderer/gpu/frame.rs` | compute dispatch → render pass |
| `src/core/color.rs` | add channel_to_linear, channel_to_srgb |
| `src/gui/renderer/cpu/primitives.rs` | gamma-correct blend, swash coverage |
| `src/gui/renderer/cpu/mod.rs` | srgb_to_linear LUT, swash glyph cache |
| `src/config/fonts.rs` | font loading → raw bytes for swash |
| `src/gui/lifecycle/*.rs` | ScaleFactorChanged → rasterizer.rebuild() |

## Files Unchanged

`composite.wgsl`, `ui.wgsl`, all of `src/core/terminal.rs`, `selection.rs`,
`color.rs` (palette values), existing tests unrelated to rendering.

## Out of Scope

- harfbuzz text shaping (can be added later as a separate feature)
- Custom color themes / theme switching (separate concern)
- HDR rendering
