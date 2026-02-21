# Font Fallback Design

## Problem
Terminal shows tofu (crossed rectangle) for characters missing from the primary font (e.g. U+23FA ⏺ used by Claude Code).

## Solution
Embed Symbols Nerd Font Mono as a fixed fallback font. When a glyph is missing from the primary font, rasterize it from the fallback font instead.

## Approach
- Use `fontdue::Font::has_glyph()` to check glyph existence before rasterization (standard cmap lookup, same as Alacritty/WezTerm/Kitty)
- Single fallback font, always embedded, no user configuration
- One shared glyph atlas (GPU) / one shared cache (CPU) — shader unchanged

## Changes

| File | Change |
|---|---|
| `src/config/fonts.rs` | Add `fallback_font_data()` returning embedded Symbols Nerd Font Mono |
| `src/gui/renderer/cpu/mod.rs` | Add `fallback_font: Font` field |
| `src/gui/renderer/cpu/primitives.rs` | Check `has_glyph()` in `draw_char()`, use fallback if primary misses |
| `src/gui/renderer/gpu/mod.rs` | Add `fallback_font: Font` field |
| `src/gui/renderer/gpu/atlas.rs` | Check `has_glyph()` in `insert_glyph()`, use fallback if primary misses |

## What does NOT change
- `FontMetrics` — cell size determined by primary font
- `FontConfig` / `FontFamily` — no config changes
- GPU shaders — same atlas format
- `RendererBackend` dispatch
