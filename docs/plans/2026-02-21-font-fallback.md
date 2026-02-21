# Font Fallback Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Render missing glyphs (e.g. ⏺ U+23FA) from embedded Symbols Nerd Font Mono instead of showing tofu.

**Architecture:** Add a second `fontdue::Font` instance (fallback) alongside the primary font in both CPU and GPU renderers. Before rasterizing, check `primary.has_glyph(ch)` — if false, use fallback font. One shared glyph atlas/cache for both fonts.

**Tech Stack:** fontdue (`has_glyph()` for cmap lookup), existing CPU/GPU renderer infrastructure.

---

### Task 1: Add `fallback_font_data()` to config

**Files:**
- Modify: `src/config/fonts.rs`

**Step 1: Write the failing test**

Add to the existing `tests` module in `src/config/fonts.rs`:

```rust
#[test]
fn fallback_font_loads_as_valid() {
    let data = super::fallback_font_data();
    let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default());
    assert!(font.is_ok(), "fallback font should be a valid font");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test fallback_font_loads_as_valid`
Expected: FAIL — `fallback_font_data` not found

**Step 3: Write minimal implementation**

Add to `src/config/fonts.rs` before the `tests` module:

```rust
/// Returns the embedded Symbols Nerd Font Mono bytes (fallback for missing glyphs).
pub(crate) fn fallback_font_data() -> &'static [u8] {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/fonts/SymbolsNerdFontMono-Regular.ttf"
    ))
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test fallback_font_loads_as_valid`
Expected: PASS

**Step 5: Commit**

```bash
git add src/config/fonts.rs
git commit -m "feat: add fallback_font_data() for Symbols Nerd Font Mono"
```

---

### Task 2: Add fallback font to CPU renderer

**Files:**
- Modify: `src/gui/renderer/cpu/mod.rs`
- Modify: `src/gui/renderer/cpu/primitives.rs`

**Step 1: Add `fallback_font` field to `CpuRenderer`**

In `src/gui/renderer/cpu/mod.rs`, add field to struct:

```rust
pub struct CpuRenderer {
    pub(in crate::gui::renderer) font: Font,
    pub(in crate::gui::renderer) fallback_font: Font,  // NEW
    pub(in crate::gui::renderer) metrics: FontMetrics,
    pub(in crate::gui::renderer) glyph_cache: HashMap<char, GlyphBitmap>,
    pub(in crate::gui::renderer) palette: ThemePalette,
}
```

**Step 2: Initialize fallback font in `new()`**

In `CpuRenderer::new()`, after loading the primary font, add:

```rust
let fallback_data = crate::config::fallback_font_data();
let fallback_font = Font::from_bytes(fallback_data, FontSettings::default())
    .expect("fallback font load failed");
```

And add `fallback_font` to the struct constructor.

Note: `apply_config()` does NOT need to reload the fallback font — it never changes.

**Step 3: Use fallback in `draw_char()`**

In `src/gui/renderer/cpu/primitives.rs`, replace the glyph cache miss block (lines 38-48):

```rust
if !self.glyph_cache.contains_key(&character) {
    let font = if self.font.has_glyph(character) {
        &self.font
    } else {
        &self.fallback_font
    };
    let (metrics, bitmap) = font.rasterize(character, self.metrics.font_size);
    let cached = super::super::types::GlyphBitmap {
        data: bitmap,
        width: metrics.width,
        height: metrics.height,
        left: metrics.xmin,
        top: metrics.height as i32 + metrics.ymin,
    };
    self.glyph_cache.insert(character, cached);
}
```

**Step 4: Verify build**

Run: `cargo build --no-default-features`
Expected: compiles successfully

**Step 5: Run all tests**

Run: `cargo test`
Expected: all tests pass

**Step 6: Commit**

```bash
git add src/gui/renderer/cpu/mod.rs src/gui/renderer/cpu/primitives.rs
git commit -m "feat: add font fallback to CPU renderer"
```

---

### Task 3: Add fallback font to GPU renderer

**Files:**
- Modify: `src/gui/renderer/gpu/mod.rs`
- Modify: `src/gui/renderer/gpu/setup.rs`
- Modify: `src/gui/renderer/gpu/atlas.rs`
- Modify: `src/gui/renderer/gpu/grid_packing.rs`

**Step 1: Add `fallback_font` field to `GpuRenderer`**

In `src/gui/renderer/gpu/mod.rs`, add field to struct after `font`:

```rust
font: Font,
fallback_font: Font,  // NEW
```

**Step 2: Initialize fallback font in `setup.rs`**

In `GpuRenderer::new()` in `setup.rs`, after loading the primary font (line 76), add:

```rust
let fallback_data = crate::config::fallback_font_data();
let fallback_font =
    Font::from_bytes(fallback_data, FontSettings::default()).expect("fallback font load fail");
```

Add `fallback_font` to the struct constructor (after `font`).

**Step 3: Pass fallback font to atlas methods**

In `src/gui/renderer/gpu/atlas.rs`:

3a. Change `insert_glyph` signature to accept a second font:

```rust
fn insert_glyph(
    &mut self,
    queue: &wgpu::Queue,
    font: &fontdue::Font,
    fallback_font: &fontdue::Font,
    font_size: f32,
    codepoint: u32,
    ch: char,
)
```

At the top of `insert_glyph`, select the right font before rasterizing:

```rust
let raster_font = if font.has_glyph(ch) {
    font
} else {
    fallback_font
};
let (metrics, bitmap) = raster_font.rasterize(ch, font_size);
```

(Replace the existing `let (metrics, bitmap) = font.rasterize(ch, font_size);` line.)

3b. Change `get_or_insert` signature to accept a second font:

```rust
pub fn get_or_insert(
    &mut self,
    codepoint: u32,
    font: &fontdue::Font,
    fallback_font: &fontdue::Font,
    font_size: f32,
    queue: &wgpu::Queue,
) -> GlyphInfo
```

Update the call to `insert_glyph` inside `get_or_insert`:

```rust
self.insert_glyph(queue, font, fallback_font, font_size, codepoint, ch);
```

3c. Change `GlyphAtlas::new()` signature to accept fallback font:

```rust
pub fn new(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    font: &fontdue::Font,
    fallback_font: &fontdue::Font,
    font_size: f32,
    ascent: i32,
) -> Self
```

Update the pre-population loop call:

```rust
atlas.insert_glyph(queue, font, fallback_font, font_size, cp, ch);
```

**Step 4: Update all callers**

4a. In `grid_packing.rs`, update `pack_grid_cells()` call to `get_or_insert` (line 79-84):

```rust
let _ = self.atlas.get_or_insert(
    codepoint,
    &self.font,
    &self.fallback_font,
    self.metrics.font_size,
    &self.queue,
);
```

4b. In `setup.rs`, update `GlyphAtlas::new()` call:

```rust
let atlas = GlyphAtlas::new(&device, &queue, &font, &fallback_font, metrics.font_size, metrics.ascent);
```

4c. In `setup.rs`, update `rebuild_atlas()`:

```rust
pub(super) fn rebuild_atlas(&mut self) {
    self.atlas = GlyphAtlas::new(
        &self.device,
        &self.queue,
        &self.font,
        &self.fallback_font,
        self.metrics.font_size,
        self.metrics.ascent,
    );
    // ... rest unchanged
}
```

**Step 5: Verify build**

Run: `cargo build`
Expected: compiles successfully

**Step 6: Run all tests**

Run: `cargo test`
Expected: all tests pass

**Step 7: Run clippy**

Run: `cargo clippy`
Expected: zero warnings

**Step 8: Commit**

```bash
git add src/gui/renderer/gpu/mod.rs src/gui/renderer/gpu/setup.rs src/gui/renderer/gpu/atlas.rs src/gui/renderer/gpu/grid_packing.rs
git commit -m "feat: add font fallback to GPU renderer"
```

---

### Task 4: Add fallback coverage test

**Files:**
- Modify: `src/config/fonts.rs`

**Step 1: Write test that verifies fallback covers glyphs missing from primary**

Add to the `tests` module in `src/config/fonts.rs`:

```rust
#[test]
fn fallback_covers_nerd_font_symbols() {
    let primary_data = font_data(FontFamily::JetBrainsMono);
    let primary = fontdue::Font::from_bytes(primary_data, fontdue::FontSettings::default()).unwrap();

    let fallback_data = fallback_font_data();
    let fallback = fontdue::Font::from_bytes(fallback_data, fontdue::FontSettings::default()).unwrap();

    // U+23FA (⏺) is not in JetBrains Mono but is in Symbols Nerd Font Mono.
    assert!(!primary.has_glyph('\u{23FA}'), "primary should lack ⏺");
    assert!(fallback.has_glyph('\u{23FA}'), "fallback should have ⏺");
}
```

**Step 2: Run test**

Run: `cargo test fallback_covers_nerd_font_symbols`
Expected: PASS

**Step 3: Commit**

```bash
git add src/config/fonts.rs
git commit -m "test: verify fallback font covers missing glyphs"
```
