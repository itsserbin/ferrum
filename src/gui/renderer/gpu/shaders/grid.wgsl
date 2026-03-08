// Grid render shader — renders terminal cells into a Rgba8Unorm texture.
//
// A fullscreen triangle is drawn. The fragment shader determines which cell
// each pixel belongs to, blends glyph coverage against palette colors in
// linear light space, and explicitly encodes the result to sRGB before output.

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

// ---- Per-cell data (16 bytes, tightly packed) ----

struct Cell {
    codepoint: u32,
    fg:        u32,   // 0xRRGGBB sRGB
    bg:        u32,   // 0xRRGGBB sRGB
    attrs:     u32,
                      // bit 0: bold
                      // bit 1: italic
                      // bit 2: underline (any style)
                      // bit 3: reverse video
                      // bit 4: dim
                      // bit 5: strikethrough
                      // bits 6-7: underline style (0=none, 1=single, 2=double, 3=reserved)
                      // bit 8: wide-right spacer
}

// ---- Glyph lookup entry (32 bytes, 16-byte aligned) ----

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

/// Decode one sRGB channel to linear light (IEC 61966-2-1).
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        return c / 12.92;
    }
    return pow((c + 0.055) / 1.055, 2.4);
}

/// Unpack 0xRRGGBB and decode sRGB → linear (IEC 61966-2-1, exact inverse of linear_to_srgb).
fn unpack_linear(c: u32) -> vec3<f32> {
    let s = vec3<f32>(
        f32((c >> 16u) & 0xFFu) / 255.0,
        f32((c >>  8u) & 0xFFu) / 255.0,
        f32( c         & 0xFFu) / 255.0,
    );
    return vec3<f32>(srgb_to_linear(s.r), srgb_to_linear(s.g), srgb_to_linear(s.b));
}

/// Encode one linear channel to sRGB (IEC 61966-2-1).
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        return c * 12.92;
    }
    return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}

fn linear_to_srgb3(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(linear_to_srgb(c.r), linear_to_srgb(c.g), linear_to_srgb(c.b));
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

    // Decode sRGB palette → linear light.
    var fg_lin = unpack_linear(fg);
    let bg_lin = unpack_linear(bg);

    if is_dim {
        fg_lin = fg_lin * 0.6;
    }

    // Start with background.
    var color = bg_lin;

    // Glyph blending — blend in linear space.
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
                    // LCD: per-channel blend in linear space.
                    color = vec3<f32>(
                        mix(bg_lin.r, fg_lin.r, sample.r),
                        mix(bg_lin.g, fg_lin.g, sample.g),
                        mix(bg_lin.b, fg_lin.b, sample.b),
                    );
                } else {
                    // Grayscale: single-alpha blend in linear space.
                    color = mix(bg_lin, fg_lin, sample.r);
                }
            }
        }
    }

    // Decorations (underline, strikethrough) — use (potentially dimmed) fg.
    let decor_lin = fg_lin;
    let underline_style = (cell.attrs >> 6u) & 3u;
    if underline_style == 1u && cell_y >= uniforms.cell_height - 2u {
        // Single underline: 2px at bottom.
        color = decor_lin;
    } else if underline_style == 2u &&
              (cell_y == uniforms.cell_height - 1u || cell_y == uniforms.cell_height - 3u) {
        // Double underline: two 1px lines.
        color = decor_lin;
    }
    // Strikethrough: 1px line at vertical center.
    if (cell.attrs & 32u) != 0u && cell_y == uniforms.cell_height / 2u {
        color = decor_lin;
    }

    // Encode linear → sRGB explicitly before writing to Rgba8Unorm attachment.
    return vec4<f32>(linear_to_srgb3(color), 1.0);
}
