// Grid compute shader — renders terminal cells into a texture.
//
// Each workgroup thread handles one pixel. The shader determines which
// cell the pixel belongs to, fills the background color, samples the
// glyph from the atlas, applies foreground color with alpha blending,
// and draws underlines. Each dispatch renders one batch at `origin_x/y`
// in the grid texture.

// ---- Uniforms (48 bytes, 16-byte aligned) ----

struct GridUniforms {
    cols:         u32,   // terminal column count
    rows:         u32,   // terminal row count
    cell_width:   u32,   // cell width in pixels
    cell_height:  u32,   // cell height in pixels
    origin_x:     u32,   // batch origin X in grid texture pixels
    origin_y:     u32,   // batch origin Y in grid texture pixels
    bg_color:     u32,   // default background color as 0xRRGGBB
    _pad0:        u32,
    tex_width:    u32,   // output texture width in pixels
    tex_height:   u32,   // output texture height in pixels
    _pad1:        u32,
    _pad2:        u32,
}

// ---- Per-cell data (16 bytes, tightly packed) ----

struct Cell {
    codepoint: u32,      // Unicode codepoint (0 = empty, 32 = space)
    fg:        u32,      // foreground  0xRRGGBB
    bg:        u32,      // background  0xRRGGBB
    attrs:     u32,      // bit 0: bold
                         // bit 1: italic
                         // bit 2: underline (any style)
                         // bit 3: reverse video
                         // bit 4: dim
                         // bit 5: strikethrough
                         // bits 6-7: underline style (0=none, 1=single, 2=double, 3=reserved)
}

// ---- Glyph lookup entry (32 bytes, 16-byte aligned) ----

struct GlyphInfo {
    x:        f32,       // atlas pixel X
    y:        f32,       // atlas pixel Y
    w:        f32,       // glyph width  in atlas pixels
    h:        f32,       // glyph height in atlas pixels
    offset_x: f32,       // X offset from cell origin
    offset_y: f32,       // Y offset from cell top
    _pad1:    f32,
    _pad2:    f32,
}

// ---- Bindings ----

@group(0) @binding(0) var<uniform>       uniforms: GridUniforms;
@group(0) @binding(1) var<storage, read> cells:    array<Cell>;
@group(0) @binding(2) var<storage, read> glyphs:   array<GlyphInfo>;
@group(0) @binding(3) var               atlas:     texture_2d<f32>;
@group(0) @binding(4) var               output:    texture_storage_2d<rgba8unorm, write>;

// ---- Helpers ----

// Unpack 0xRRGGBB into linear vec3<f32>.
fn unpack_rgb(c: u32) -> vec3<f32> {
    return vec3<f32>(
        f32((c >> 16u) & 0xFFu) / 255.0,
        f32((c >>  8u) & 0xFFu) / 255.0,
        f32( c         & 0xFFu) / 255.0,
    );
}

// ---- Entry point ----

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let local_x = gid.x;
    let local_y = gid.y;
    let pixel_x = local_x + uniforms.origin_x;
    let pixel_y = local_y + uniforms.origin_y;

    // Out-of-texture guard (dispatch may overshoot texture dimensions).
    if pixel_x >= uniforms.tex_width || pixel_y >= uniforms.tex_height {
        return;
    }

    // Batch-local bounds (dispatch can overshoot because of workgroup ceil-div).
    let batch_w = uniforms.cols * uniforms.cell_width;
    let batch_h = uniforms.rows * uniforms.cell_height;
    if local_x >= batch_w || local_y >= batch_h {
        return;
    }

    // Which cell does this pixel belong to?
    let col = local_x / uniforms.cell_width;
    let row = local_y / uniforms.cell_height;
    let cell_idx = row * uniforms.cols + col;
    let cell = cells[cell_idx];

    // Local position within the cell.
    let cell_x = local_x - col * uniforms.cell_width;
    let cell_y = local_y - row * uniforms.cell_height;

    // Resolve foreground / background, honoring reverse video.
    var fg = cell.fg;
    var bg = cell.bg;
    let is_reverse = (cell.attrs & 8u) != 0u;
    if is_reverse {
        let tmp = fg;
        fg = bg;
        bg = tmp;
    }

    // Dim: reduce foreground intensity by 40%
    let is_dim = (cell.attrs & 16u) != 0u;

    // Start with background color.
    var color = vec4<f32>(unpack_rgb(bg), 1.0);

    // Sample glyph from atlas for visible codepoints (skip space and below).
    let glyph_count = arrayLength(&glyphs);
    if cell.codepoint > 32u && cell.codepoint < glyph_count {
        let glyph = glyphs[cell.codepoint];

        if glyph.w > 0.0 {
            // Pixel position relative to glyph bounding box.
            let gx = f32(cell_x) - glyph.offset_x;
            let gy = f32(cell_y) - glyph.offset_y;

            if gx >= 0.0 && gx < glyph.w && gy >= 0.0 && gy < glyph.h {
                // Integer texel coordinates into the atlas.
                let tex_x = i32(glyph.x + gx);
                let tex_y = i32(glyph.y + gy);

                // textureLoad is the correct function for compute shaders
                // (no sampler needed, integer coordinates, explicit mip level).
                let alpha = textureLoad(atlas, vec2<i32>(tex_x, tex_y), 0).r;

                // Blend foreground over background using glyph alpha.
                var fg_color = unpack_rgb(fg);
                if is_dim {
                    fg_color = fg_color * 0.6;
                }
                color = vec4<f32>(
                    mix(color.rgb, fg_color, alpha),
                    1.0,
                );
            }
        }
    }

    // Compute potentially dimmed foreground for decorations.
    var decor_fg = unpack_rgb(fg);
    if is_dim {
        decor_fg = decor_fg * 0.6;
    }

    // Underline styles (bits 6-7)
    let underline_style = (cell.attrs >> 6u) & 3u;
    if underline_style == 1u && cell_y >= uniforms.cell_height - 2u {
        // Single underline: 2px at bottom
        color = vec4<f32>(decor_fg, 1.0);
    } else if underline_style == 2u && (cell_y == uniforms.cell_height - 1u || cell_y == uniforms.cell_height - 3u) {
        // Double underline: two 1px lines
        color = vec4<f32>(decor_fg, 1.0);
    }
    // value 3 reserved for future use

    // Strikethrough: 1px line at vertical center
    let is_strikethrough = (cell.attrs & 32u) != 0u;
    if is_strikethrough && cell_y == uniforms.cell_height / 2u {
        color = vec4<f32>(decor_fg, 1.0);
    }

    textureStore(output, vec2<i32>(i32(pixel_x), i32(pixel_y)), color);
}
