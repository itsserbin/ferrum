---
title: Research - fontdue Migration
task_file: Ad-hoc research request
scratchpad: .specs/scratchpad/7f3a9c2e.md
created: 2026-02-16
status: complete
---

# Research: fontdue Migration

## Executive Summary

fontdue 0.9.3 is a pure Rust font rasterizer that provides a significantly simpler API than freetype-rs, eliminates all system dependencies, and is ideal for terminal emulator use. The migration path is straightforward: direct f32 metrics (no 26.6 fixed-point conversions), grayscale coverage bitmaps (0-255), and clean Rust API design. For Ferrum terminal on WSL2, this eliminates the libfreetype6-dev system dependency and simplifies the build process.

## Related Existing Research

- **research-font-hinting.md** - Context on font rendering techniques; fontdue doesn't do manual hinting but has good outline quality
- **research-rust-crates-verification.md** - Crate verification methodology; fontdue 0.9.3 is latest stable

---

## Documentation & References

| Resource | Description | Relevance | Link |
|----------|-------------|-----------|------|
| fontdue API Docs | Official API documentation v0.9.3 | Complete API reference | [docs.rs](https://docs.rs/fontdue/latest/fontdue/) |
| Font struct | Core font loading and rasterization | Primary API for terminal | [Font](https://docs.rs/fontdue/latest/fontdue/struct.Font.html) |
| Metrics struct | Glyph positioning metadata | Required for glyph placement | [Metrics](https://docs.rs/fontdue/latest/fontdue/struct.Metrics.html) |
| LineMetrics struct | Font-level vertical metrics | Needed for line spacing | [LineMetrics](https://docs.rs/fontdue/latest/fontdue/struct.LineMetrics.html) |
| GitHub Repository | Source code and examples | Issue tracking and examples | [github.com/mooman219/fontdue](https://github.com/mooman219/fontdue) |
| crates.io | Package registry and version info | Version and dependency info | [crates.io/crates/fontdue](https://crates.io/crates/fontdue) |

### Key Concepts

- **Coverage bitmap**: Grayscale values 0-255 where 0 = transparent, 255 = fully opaque pixel
- **Subpixel rendering**: Optional RGB channel coverage for LCD displays (width × 3 bytes)
- **Pure Rust**: No FFI, no C dependencies, no system library requirements
- **Upfront parsing**: Font data fully parsed on load (not lazy) for consistent performance

---

## Libraries & Tools

| Name | Purpose | Maturity | Notes |
|------|---------|----------|-------|
| fontdue 0.9.3 | Font rasterization | Stable | Latest version, pure Rust |
| ttf-parser 0.21 | TTF/OTF parsing | Stable | Required dependency |
| freetype-rs 0.36.0 | Font rasterization (current) | Mature | Requires libfreetype6-dev |

### Recommended Stack

**Migrate from freetype-rs to fontdue 0.9.3**

Rationale:
- Eliminates system dependencies (libfreetype6-dev, pkg-config) → simpler WSL2 builds
- Cleaner API: direct f32/i32 values instead of 26.6 fixed-point
- Pure Rust safety: no unsafe FFI calls for metrics
- Simpler integration: `rasterize()` returns direct `(Metrics, Vec<u8>)`
- Terminal use case: no need for advanced shaping or hinting

---

## Patterns & Approaches

### Font Loading Pattern

**When to use**: Application startup, font change events

**Trade-offs**:
- Pro: Load once, reuse many times
- Pro: No lifetime dependencies
- Con: Upfront allocation (intentional design)

**Example**:
```rust
let font_data = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
let font = fontdue::Font::from_bytes(
    font_data as &[u8],
    fontdue::FontSettings::default()
)?;
```

### Metrics Extraction Pattern

**When to use**: Getting font/glyph dimensions for layout

**Trade-offs**:
- Pro: Direct f32 values, no conversion math
- Pro: Separate methods for line vs glyph metrics
- Con: Returns Option for line metrics (may be None)

**Example**:
```rust
// Font-level metrics
let line_metrics = font.horizontal_line_metrics(font_size).unwrap();
let ascent = line_metrics.ascent;  // f32, already in pixels
let descent = line_metrics.descent;  // f32, typically negative
let line_height = line_metrics.new_line_size;  // ascent - descent + line_gap

// Glyph-level metrics
let metrics = font.metrics('A', font_size);
let advance = metrics.advance_width;  // f32, subpixel precision
```

### Rasterization Pattern

**When to use**: Rendering each glyph to screen

**Trade-offs**:
- Pro: Single call returns both metrics and bitmap
- Pro: Coverage values ready for alpha blending
- Con: Allocates Vec<u8> per glyph (consider caching)

**Example**:
```rust
let (metrics, bitmap) = font.rasterize('g', font_size);

// Metrics describe the bitmap dimensions and positioning
let width = metrics.width;  // usize: bitmap width
let height = metrics.height;  // usize: bitmap height
let x_offset = metrics.xmin;  // i32: can be negative
let y_offset = metrics.ymin;  // i32: can be negative

// Bitmap is row-major, top-to-bottom, left-to-right
// Each byte is coverage: 0 = transparent, 255 = opaque
for y in 0..height {
    for x in 0..width {
        let coverage = bitmap[y * width + x];  // 0-255
        // Blend with background using coverage as alpha
    }
}
```

---

## API Comparison: freetype-rs → fontdue

### Loading Fonts

**freetype-rs (current)**:
```rust
let lib = freetype::Library::init()?;
let face = lib.new_face("/path/to/font.ttf", 0)?;
face.set_pixel_sizes(0, font_size)?;
```

**fontdue (new)**:
```rust
let font = fontdue::Font::from_bytes(
    font_data as &[u8],
    fontdue::FontSettings::default()
)?;
// Size passed per-rasterization, not set globally
```

### Getting Font Metrics

**freetype-rs (current)**:
```rust
let metrics = unsafe { (*(*face.raw()).size).metrics };
let ascent = (metrics.ascender >> 6) as f32;  // 26.6 fixed-point
let descent = (metrics.descender >> 6) as f32;
let height = (metrics.height >> 6) as f32;
```

**fontdue (new)**:
```rust
let line_metrics = font.horizontal_line_metrics(font_size)?;
let ascent = line_metrics.ascent;  // Already f32 pixels
let descent = line_metrics.descent;
let height = line_metrics.new_line_size;
```

### Getting Glyph Metrics

**freetype-rs (current)**:
```rust
face.load_char('A' as usize, freetype::face::LoadFlag::RENDER)?;
let glyph = face.glyph();
let advance = (glyph.advance().x >> 6) as f32;  // 26.6 fixed-point
```

**fontdue (new)**:
```rust
let metrics = font.metrics('A', font_size);
let advance = metrics.advance_width;  // Already f32
```

### Rasterizing Glyphs

**freetype-rs (current)**:
```rust
face.load_char('g' as usize, freetype::face::LoadFlag::RENDER)?;
let glyph = face.glyph();
let bitmap = glyph.bitmap();
let buffer = bitmap.buffer();  // &[u8]
let width = bitmap.width();
let height = bitmap.rows();
```

**fontdue (new)**:
```rust
let (metrics, bitmap) = font.rasterize('g', font_size);
// bitmap is Vec<u8>, metrics contains width/height
let width = metrics.width;
let height = metrics.height;
```

---

## Potential Issues

| Issue | Impact | Mitigation |
|-------|--------|------------|
| Layout API instability | Low | Don't use layout module (terminal doesn't need it) |
| No text shaping | Low | Terminal uses monospace, no complex ligatures needed |
| Solo maintained project | Medium | Active development, simple scope, can fork if needed |
| Upfront allocation | Low | Load font once at startup, acceptable memory trade-off |
| No hinting support | Low | Modern fonts have good outlines, hinting less critical |

---

## Recommendations

### 1. Migrate to fontdue 0.9.3 (High Priority)

**Rationale**: Eliminates system dependencies (libfreetype6-dev), provides simpler API aligned with Rust idioms, and is perfectly suited for terminal emulator needs.

**Benefits**:
- Zero system dependencies → easier builds on WSL2
- No unsafe code for metrics → safer
- Direct pixel values → no bit-shift conversions
- Pure Rust → better error messages, easier debugging

**Migration effort**: Low - API is simpler than freetype-rs

### 2. Use Basic Rasterization API

**Rationale**: Terminal emulator needs simple per-glyph rasterization at fixed sizes.

**Approach**:
- Use `font.rasterize(char, px)` for basic rendering
- Consider `rasterize_subpixel()` if LCD rendering desired
- Cache rasterized glyphs if performance needed

### 3. Handle Line Metrics Properly

**Rationale**: Proper vertical positioning requires understanding ascent/descent.

**Approach**:
- Call `horizontal_line_metrics()` once per font size
- Store ascent, descent, new_line_size for layout calculations
- Use `unwrap()` or handle None case (rare for valid fonts)

---

## Implementation Guidance

### Installation

```toml
[dependencies]
fontdue = "0.9.3"
```

Remove:
```toml
# Remove these
freetype-rs = "0.36.0"
```

Remove system dependencies:
```bash
# No longer needed
# libfreetype6-dev pkg-config
```

### Configuration

**FontSettings** (default is usually fine):
```rust
use fontdue::FontSettings;

// Default settings (recommended)
let settings = FontSettings::default();

// Custom settings if needed
let settings = FontSettings {
    collection_index: 0,  // For .ttc files
    scale: 40.0,          // Optional fixed scale
};
```

### Integration Points

**Replace in existing code**:

1. **Font loading** (in gui/renderer.rs or equivalent):
   ```rust
   // Old: freetype Library and Face
   // New: fontdue::Font
   pub struct Renderer {
       font: fontdue::Font,
       font_size: f32,
       // cached metrics
       ascent: f32,
       descent: f32,
       line_height: f32,
   }
   ```

2. **Initialization**:
   ```rust
   impl Renderer {
       pub fn new() -> Result<Self> {
           let font_data = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
           let font = fontdue::Font::from_bytes(
               font_data as &[u8],
               fontdue::FontSettings::default()
           )?;

           let font_size = 16.0;
           let line_metrics = font.horizontal_line_metrics(font_size)
               .ok_or("Failed to get line metrics")?;

           Ok(Self {
               font,
               font_size,
               ascent: line_metrics.ascent,
               descent: line_metrics.descent,
               line_height: line_metrics.new_line_size,
           })
       }
   }
   ```

3. **Glyph rendering**:
   ```rust
   fn render_glyph(&mut self, ch: char, x: f32, y: f32) {
       let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);

       // Calculate final position
       let glyph_x = x + metrics.xmin as f32;
       let glyph_y = y + metrics.ymin as f32;

       // Blit bitmap to framebuffer
       for row in 0..metrics.height {
           for col in 0..metrics.width {
               let coverage = bitmap[row * metrics.width + col];
               if coverage > 0 {
                   // Alpha blend pixel at (glyph_x + col, glyph_y + row)
                   self.blend_pixel(
                       glyph_x + col as f32,
                       glyph_y + row as f32,
                       coverage
                   );
               }
           }
       }
   }
   ```

### Error Handling

```rust
use fontdue::{Font, FontSettings, FontResult};

// Loading
let font: FontResult<Font> = Font::from_bytes(data, FontSettings::default());
match font {
    Ok(f) => { /* use f */ },
    Err(e) => eprintln!("Failed to load font: {}", e),
}

// Metrics (returns Option)
if let Some(line_metrics) = font.horizontal_line_metrics(size) {
    // use line_metrics
} else {
    // Handle missing metrics (rare)
}
```

---

## Code Examples

### Complete Font Loading & Rendering

```rust
use fontdue::{Font, FontSettings};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load font
    let font_data = std::fs::read("JetBrainsMono-Regular.ttf")?;
    let font = Font::from_bytes(font_data, FontSettings::default())?;

    let font_size = 16.0;

    // Get line metrics
    let line_metrics = font.horizontal_line_metrics(font_size)
        .ok_or("No line metrics")?;

    println!("Ascent: {}", line_metrics.ascent);
    println!("Descent: {}", line_metrics.descent);
    println!("Line height: {}", line_metrics.new_line_size);

    // Rasterize a glyph
    let (metrics, bitmap) = font.rasterize('A', font_size);

    println!("Glyph metrics:");
    println!("  Width: {}px", metrics.width);
    println!("  Height: {}px", metrics.height);
    println!("  X offset: {}", metrics.xmin);
    println!("  Y offset: {}", metrics.ymin);
    println!("  Advance: {}px", metrics.advance_width);
    println!("  Bitmap size: {} bytes", bitmap.len());

    Ok(())
}
```

### Terminal Cell Rendering

```rust
pub struct TerminalRenderer {
    font: Font,
    font_size: f32,
    cell_width: usize,
    cell_height: usize,
}

impl TerminalRenderer {
    pub fn render_cell(&self, ch: char, col: usize, row: usize, framebuffer: &mut [u8]) {
        let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);

        // Calculate cell position
        let cell_x = col * self.cell_width;
        let cell_y = row * self.cell_height;

        // Blit glyph into cell
        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let coverage = bitmap[y * metrics.width + x];
                if coverage > 0 {
                    let fb_x = cell_x + x + metrics.xmin as usize;
                    let fb_y = cell_y + y + metrics.ymin as usize;
                    // Alpha blend using coverage as alpha value
                    self.blend_pixel(framebuffer, fb_x, fb_y, coverage);
                }
            }
        }
    }
}
```

### Glyph Caching

```rust
use std::collections::HashMap;

pub struct GlyphCache {
    font: Font,
    font_size: f32,
    cache: HashMap<char, (Metrics, Vec<u8>)>,
}

impl GlyphCache {
    pub fn new(font: Font, font_size: f32) -> Self {
        Self {
            font,
            font_size,
            cache: HashMap::new(),
        }
    }

    pub fn get_glyph(&mut self, ch: char) -> &(Metrics, Vec<u8>) {
        self.cache.entry(ch).or_insert_with(|| {
            self.font.rasterize(ch, self.font_size)
        })
    }
}
```

---

## Sources

Primary sources (all verified 2026-02-16):

- [fontdue crate on crates.io](https://crates.io/crates/fontdue) - Version and metadata
- [fontdue API documentation](https://docs.rs/fontdue/latest/fontdue/) - Complete API reference
- [Font struct documentation](https://docs.rs/fontdue/latest/fontdue/struct.Font.html) - Core API
- [Metrics struct documentation](https://docs.rs/fontdue/latest/fontdue/struct.Metrics.html) - Glyph metrics
- [LineMetrics struct documentation](https://docs.rs/fontdue/latest/fontdue/struct.LineMetrics.html) - Font metrics
- [fontdue GitHub repository](https://github.com/mooman219/fontdue) - Source code and examples
- [lib.rs fontdue page](https://lib.rs/crates/fontdue) - Alternative registry view

Secondary sources:
- [Are we VFX yet? - Text Rendering](https://arewevfxyet.rs/ecosystem/textrendering/) - Ecosystem overview
- [Are we game yet? - Text Rendering](https://arewegameyet.rs/ecosystem/textrendering/) - Alternative overview

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| Source verification | ✅ | All sources from official docs.rs, crates.io, and GitHub repo |
| Recency check | ✅ | Latest version 0.9.3 (Feb 2026), all docs current |
| Alternatives explored | ✅ | Compared against freetype-rs; fontdue designed to replace rusttype/ab_glyph |
| Actionability | ✅ | Complete code examples, migration guide, exact API signatures |
| Evidence quality | ✅ | All API details from official documentation v0.9.3 |

**Limitations/Caveats**:
- Performance claims ("fastest font renderer") are from the project itself, not independently verified. For terminal emulator use, adequate performance is sufficient regardless of rankings.
- Layout API noted as "immature" by maintainer, but not needed for terminal emulator.
- Solo-maintained project; active but feature additions may be slower than bug fixes.
- No manual hinting support; relies on font outline quality (acceptable for modern fonts).

**All user research questions answered**:
1. ✅ Load TTF: `Font::from_bytes(data, FontSettings::default())`
2. ✅ Set size: Pass `px: f32` to rasterize/metrics methods
3. ✅ Font metrics: `horizontal_line_metrics(px)` → ascent, descent, line_gap, new_line_size
4. ✅ Rasterize: `rasterize(char, px)` → `(Metrics, Vec<u8>)`
5. ✅ Bitmap format: Vec<u8> grayscale coverage 0-255, row-major top-to-bottom
6. ✅ Glyph metrics: width, height, xmin, ymin, advance_width, advance_height
7. ✅ Version: 0.9.3 (latest stable)
8. ✅ Gotchas: Layout API unstable, no shaping (use Cosmic Text), upfront parsing
