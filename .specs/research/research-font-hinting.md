---
title: Research - Font Rasterization with Hinting Support
task_file: User request via conversation
scratchpad: .specs/scratchpad/915d7cf7.md
created: 2026-02-15
status: complete
---

# Research: Font Rasterization with Hinting Support for Terminal Emulator

## Executive Summary

Four Rust font rasterization crates evaluated for replacing fontdue. **swash (0.2.6)** and **freetype-rs (0.36.0)** both support font hinting and are suitable for terminal emulators. ab_glyph (0.2.32) lacks hinting like fontdue. cosmic-text (0.14.2) includes hinting but is overkill for simple character-by-character rendering. **Recommendation: swash** for pure Rust simplicity with zero system dependencies on WSL2, or freetype-rs for battle-tested industry standard.

## Related Existing Research

- research-terminal-crate-ecosystem.md - Context on terminal emulator libraries
- research-rust-crates-verification.md - Crate version verification patterns

---

## Documentation & References

| Resource | Description | Relevance | Link |
|----------|-------------|-----------|------|
| swash docs.rs | API documentation for pure Rust font library | Primary candidate | https://docs.rs/swash/0.2.6/ |
| freetype-rs docs.rs | Rust bindings to FreeType C library | Alternative candidate | https://docs.rs/freetype-rs/0.36.0/ |
| swash GitHub | Source repository with examples | Implementation reference | https://github.com/dfrg/swash |
| freetype-rs GitHub | Source repository and examples | Implementation reference | https://github.com/PistonDevelopers/freetype-rs |
| COSMIC terminal | Production terminal using swash | Real-world validation | https://github.com/pop-os/cosmic-term |
| ab_glyph docs.rs | Pure Rust without hinting | Comparison baseline | https://docs.rs/ab_glyph/0.2.32/ |
| cosmic-text docs.rs | High-level text layout with swash | Complexity comparison | https://docs.rs/cosmic-text/0.14.2/ |

### Key Concepts

- **Font Hinting**: Instructions embedded in fonts to align glyphs to pixel grid at small sizes, improving clarity
- **Asymmetric Vertical Hinting**: Vertical hinting applied while horizontal uses subpixel rendering (swash approach)
- **TrueType Hinting**: Grid-fitting algorithm in TrueType fonts, industry standard (FreeType support)
- **Pure Rust**: No C dependencies, cross-platform without system packages (swash, ab_glyph, cosmic-text)
- **ScaleContext**: swash's reusable context for building glyph scalers with size and hinting parameters

---

## Libraries & Tools

| Name | Purpose | Hinting | Pure Rust | System Deps | Version | Notes |
|------|---------|---------|-----------|-------------|---------|-------|
| **swash** | Font rasterization + shaping | ✅ Asymmetric | ✅ | None | 0.2.6 | Used by COSMIC terminal |
| **freetype-rs** | FreeType bindings | ✅ Full TrueType | ❌ | libfreetype | 0.36.0 | Industry standard |
| **ab_glyph** | Fast font rasterization | ❌ | ✅ | None | 0.2.32 | No hinting support |
| **cosmic-text** | Text layout engine | ✅ (via swash) | ✅ | None | 0.14.2 | Overkill for terminals |
| **fontdue** (current) | Fast font rasterization | ❌ | ✅ | None | 0.9 | Current, lacks hinting |

### Recommended Stack

**Primary: swash (0.2.6)**
- Pure Rust with asymmetric vertical hinting support
- Zero system dependencies (critical for WSL2 simplicity)
- Production-proven in COSMIC terminal emulator
- Medium complexity API but perfect abstraction level

**Alternative: freetype-rs (0.36.0)**
- Industry standard FreeType library with full hinting
- Requires libfreetype6-dev system package
- Battle-tested, used indirectly by many terminals
- Simple API, well-documented

---

## Patterns & Approaches

### Pattern 1: Direct Font Rasterization (swash, freetype-rs)

**When to use**: Terminal emulator with character-by-character rendering on monospace grid

**Trade-offs**:
- Pros: Full control, minimal overhead, straightforward API
- Cons: Need to manage glyph caching manually

**Example**: Create rasterizer wrapper with font data, size, and render method per character

### Pattern 2: Text Layout Engine (cosmic-text)

**When to use**: Rich text applications with complex shaping, bidirectional text, ligatures

**Trade-offs**:
- Pros: Handles complex text automatically, built-in glyph cache
- Cons: Heavy dependency tree, unnecessary complexity for simple terminal

**Example**: FontSystem + Buffer + SwashCache for full text layout

---

## Similar Implementations

### COSMIC Terminal (cosmic-term)

- **Source**: https://github.com/pop-os/cosmic-term
- **Approach**: Uses cosmic-text (which uses swash) for rendering terminal text
- **Applicability**: Validates swash production readiness, shows it handles terminal use case

### Alacritty

- **Source**: Terminal emulator landscape (indirect FreeType usage)
- **Approach**: GPU-accelerated rendering with font rasterization
- **Applicability**: Demonstrates freetype-rs viability for high-performance terminals

---

## Potential Issues

| Issue | Impact | Mitigation |
|-------|--------|------------|
| freetype-rs 0.38.0 docs.rs build failure | Low | Use 0.36.0 (last successful build), stable API |
| System dependency on WSL2 (freetype) | Medium | Single apt install: `sudo apt install libfreetype6-dev pkg-config` |
| swash API complexity vs fontdue | Medium | Study examples, wrap in simple interface (see code examples below) |
| swash version 0.2.x API stability | Medium | Pin exact version in Cargo.toml |
| Memory face loading requires unsafe (freetype) | Low | One-time unsafe block, well-documented FFI pattern |
| Performance compared to fontdue | Low | Both options are production-optimized, likely faster with hinting |

---

## Recommendations

### 1. swash (0.2.6) - PRIMARY RECOMMENDATION

**Why**: Pure Rust, asymmetric vertical hinting, zero system dependencies on WSL2, production-proven in COSMIC terminal, perfect abstraction level for character-by-character rendering.

**Trade-off**: Medium API complexity (ScaleContext → builder → scaler → render pipeline) vs fontdue's simpler API.

**Best for**: User learning Rust who prefers KISS and wants to avoid system dependency management on WSL2.

### 2. freetype-rs (0.36.0) - SOLID ALTERNATIVE

**Why**: Industry standard FreeType library, full TrueType hinting support, simple API, battle-tested in production terminals.

**Trade-off**: Requires system package libfreetype6-dev (one-time apt install), unsafe FFI block for memory face loading.

**Best for**: Maximum reliability and typographic quality, willing to manage one system dependency.

### 3. cosmic-text (0.14.2) - NOT RECOMMENDED FOR SIMPLE TERMINAL

**Why**: Includes hinting via swash but brings entire text layout engine with shaping, bidirectional support, ligatures. Massive overkill for character-by-character monospace grid rendering.

**Alternative**: Use swash directly for better control and less complexity.

### 4. ab_glyph (0.2.32) - REJECT

**Why**: No hinting support - same limitation as current fontdue. Fast but unsuitable for small text clarity.

---

## Implementation Guidance

### Option 1: swash (Pure Rust, Recommended)

#### Installation

```toml
[dependencies]
swash = "0.2.6"
```

No system dependencies needed. Works immediately on WSL2.

#### Configuration

None required - pure Rust crate.

#### Integration Points

1. Replace `fontdue::Font` with `swash::FontRef`
2. Create `ScaleContext` once (reusable across characters)
3. Build scaler with size and hinting enabled
4. Use `Render::new()` to rasterize each character
5. Extract bitmap data and placement from returned `Image`

#### Migration Steps

1. Add swash = "0.2.6" to Cargo.toml
2. Create new module `src/gui/font_swash.rs` with FontRasterizer wrapper
3. Load font: `FontRef::from_index(font_bytes, 0)`
4. Replace fontdue rasterize calls with swash render pipeline
5. Test on WSL2 with JetBrains Mono at sizes 12-16px

### Option 2: freetype-rs (System Dependency)

#### Installation

System dependency:
```bash
sudo apt install libfreetype6-dev pkg-config  # On WSL2 Ubuntu
```

Cargo dependency:
```toml
[dependencies]
freetype = "0.36.0"  # Use 0.36.0, not 0.38.0 (docs.rs build failed)
```

#### Configuration

Ensure pkg-config can find freetype (usually automatic after apt install).

#### Integration Points

1. Initialize Library once: `Library::init()`
2. Load face from memory with FT_New_Memory_Face (unsafe FFI)
3. Set pixel size: `face.set_pixel_sizes(size, size)`
4. Render character: `face.load_char(ch as usize, LoadFlag::RENDER)`
5. Access bitmap: `face.glyph().bitmap()`

#### Migration Steps

1. Install libfreetype6-dev system package
2. Add freetype = "0.36.0" to Cargo.toml
3. Create font loading wrapper with unsafe FT_New_Memory_Face block
4. Replace fontdue calls with load_char + bitmap access
5. Test with LoadFlag::RENDER and experiment with hinting modes if needed

---

## Code Examples

### swash: Complete Terminal Font Rasterizer

```rust
// src/gui/font_swash.rs
use swash::{
    FontRef, GlyphId,
    scale::{ScaleContext, Render, Source, Image, StrikeWith},
    zeno::Format,
};

pub struct FontRasterizer<'a> {
    font: FontRef<'a>,
    context: ScaleContext,
    size: f32,
}

impl<'a> FontRasterizer<'a> {
    pub fn new(font_data: &'a [u8], size: f32) -> Option<Self> {
        let font = FontRef::from_index(font_data, 0)?;
        let context = ScaleContext::new();
        Some(Self { font, context, size })
    }

    pub fn rasterize(&mut self, ch: char) -> Option<RasterizedGlyph> {
        let glyph_id = self.font.charmap().map(ch as u32)?;

        let mut scaler = self.context.builder(self.font)
            .size(self.size)
            .hint(true)  // Enable hinting
            .build();

        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)  // Or Format::Subpixel for RGB subpixel
        .render(&mut scaler, glyph_id)?;

        Some(RasterizedGlyph {
            data: image.data.to_vec(),
            width: image.placement.width as usize,
            height: image.placement.height as usize,
            left: image.placement.left,
            top: image.placement.top,
        })
    }

    pub fn advance(&mut self, ch: char) -> Option<f32> {
        let glyph_id = self.font.charmap().map(ch as u32)?;
        let mut scaler = self.context.builder(self.font)
            .size(self.size)
            .hint(true)
            .build();
        scaler.glyph(glyph_id).map(|g| g.advance())
    }
}

pub struct RasterizedGlyph {
    pub data: Vec<u8>,    // Alpha coverage values (0-255)
    pub width: usize,
    pub height: usize,
    pub left: i32,        // Bearing X (offset from origin)
    pub top: i32,         // Bearing Y (offset from baseline)
}
```

**Usage**:
```rust
let font_bytes = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
let mut rasterizer = FontRasterizer::new(font_bytes, 16.0).unwrap();

let glyph = rasterizer.rasterize('A').unwrap();
// Render glyph.data to screen at position (x + glyph.left, y + glyph.top)
// glyph.data is alpha coverage, blend with foreground color
```

### freetype-rs: Complete Terminal Font Rasterizer

```rust
// src/gui/font_freetype.rs
use freetype::{Library, Face, face::LoadFlag};

pub struct FontRasterizer {
    _lib: Library,
    face: Face,
    _font_data: Vec<u8>,  // Keep alive for Face
}

impl FontRasterizer {
    pub fn new(font_data: Vec<u8>, pixel_size: u32) -> anyhow::Result<Self> {
        let lib = Library::init()?;

        // Load from memory - requires unsafe FFI
        let face = unsafe {
            let mut face_ptr = std::ptr::null_mut();
            let err = freetype::ffi::FT_New_Memory_Face(
                lib.raw(),
                font_data.as_ptr(),
                font_data.len() as freetype::ffi::FT_Long,
                0, // face index
                &mut face_ptr,
            );
            if err != freetype::ffi::FT_Err_Ok {
                anyhow::bail!("FT_New_Memory_Face failed: error code {}", err);
            }
            Face::from_raw(lib.raw(), face_ptr, None)
        };

        face.set_pixel_sizes(pixel_size, pixel_size)?;

        Ok(Self {
            _lib: lib,
            face,
            _font_data: font_data,  // Keep font data alive
        })
    }

    pub fn rasterize(&self, ch: char) -> anyhow::Result<RasterizedGlyph> {
        // LoadFlag::RENDER enables hinting by default
        self.face.load_char(ch as usize, LoadFlag::RENDER)?;

        let glyph = self.face.glyph();
        let bitmap = glyph.bitmap();

        Ok(RasterizedGlyph {
            data: bitmap.buffer().to_vec(),
            width: bitmap.width() as usize,
            height: bitmap.rows() as usize,
            left: glyph.bitmap_left(),
            top: glyph.bitmap_top(),
            advance: (glyph.advance().x >> 6) as usize, // 26.6 fixed point → pixels
        })
    }
}

pub struct RasterizedGlyph {
    pub data: Vec<u8>,    // Grayscale bitmap (0-255)
    pub width: usize,
    pub height: usize,
    pub left: i32,        // Bearing X
    pub top: i32,         // Bearing Y
    pub advance: usize,   // Horizontal advance
}
```

**Usage**:
```rust
let font_bytes = std::fs::read("assets/fonts/JetBrainsMono-Regular.ttf")?;
let rasterizer = FontRasterizer::new(font_bytes, 16)?;

let glyph = rasterizer.rasterize('A')?;
// Render glyph.data (grayscale) at position (x + glyph.left, y + glyph.top)
```

### Cargo.toml Changes

**For swash**:
```toml
[dependencies]
# Remove: fontdue = "0.9"
swash = "0.2.6"
```

**For freetype-rs**:
```toml
[dependencies]
# Remove: fontdue = "0.9"
freetype = "0.36.0"  # NOT 0.38.0
anyhow = "1"  # Already present
```

---

## Sources

### Crate Documentation
- [swash on crates.io](https://crates.io/crates/swash) - Version 0.2.6
- [swash docs.rs](https://docs.rs/swash/0.2.6/) - API documentation
- [freetype-rs on crates.io](https://crates.io/crates/freetype-rs) - Version 0.38.0 (use 0.36.0)
- [freetype-rs docs.rs 0.36.0](https://docs.rs/freetype-rs/0.36.0/) - Stable API docs
- [ab_glyph on crates.io](https://crates.io/crates/ab_glyph) - Version 0.2.32
- [cosmic-text on crates.io](https://crates.io/crates/cosmic-text) - Version 0.14.2

### Source Repositories
- [swash GitHub](https://github.com/dfrg/swash) - Pure Rust font library
- [freetype-rs GitHub](https://github.com/PistonDevelopers/freetype-rs) - FreeType bindings
- [ab_glyph GitHub](https://github.com/alexheretic/ab-glyph) - Pure Rust rasterizer
- [cosmic-text GitHub](https://github.com/pop-os/cosmic-text) - Text layout engine
- [COSMIC terminal GitHub](https://github.com/pop-os/cosmic-term) - Production terminal using swash

### System Dependencies
- [Linux From Scratch FreeType](https://www.linuxfromscratch.org/blfs/view/svn/general/freetype2.html)
- [Arch Linux freetype2 package](https://archlinux.org/packages/extra/x86_64/freetype2/)
- [Ubuntu freetype-config manpage](https://manpages.ubuntu.com/manpages/bionic/man1/freetype-config.1.html)

### Technical Discussions
- [Bevy font renderer issue](https://github.com/bevyengine/bevy/issues/2404) - ab_glyph limitations, swash comparison
- [Rust font rendering ecosystem discussion](https://users.rust-lang.org/t/the-state-of-fonts-parsers-glyph-shaping-and-text-layout-in-rust/32064)
- [font-kit freetype loader source](https://docs.rs/font-kit/latest/src/font_kit/loaders/freetype.rs.html) - FT_New_Memory_Face usage

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| Source verification | ✅ | All sources from crates.io, docs.rs, GitHub official repos |
| Recency check | ✅ | Versions verified as of Feb 2026: swash 0.2.6, freetype-rs 0.36.0, ab_glyph 0.2.32, cosmic-text 0.14.2 |
| Alternatives explored | ✅ | 4 alternatives evaluated (freetype-rs, swash, ab_glyph, cosmic-text) |
| Actionability | ✅ | Complete code examples, exact installation commands, migration steps |
| Evidence quality | ✅ | API documentation + production usage (COSMIC terminal) + code examples |

**Limitations/Caveats**:
- freetype-rs 0.38.0 docs.rs build failed - documentation references 0.36.0 (stable)
- Performance comparison is qualitative - no hard benchmarks gathered (both are production-ready)
- WSL2-specific testing not performed in this research - relying on general Linux compatibility
- swash API may evolve (0.2.x series) - recommend pinning exact version in Cargo.toml
