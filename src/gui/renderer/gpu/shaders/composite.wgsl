// Composite shader — blends the grid texture and UI overlay into the
// final swapchain surface.
//
// A single fullscreen triangle is drawn. The fragment shader samples the
// grid texture at pixel coordinates (accounting for tab bar and window
// padding offsets) and composites the UI layer on top using standard
// alpha-over blending.

// ---- Vertex / Fragment IO ----

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0)       uv:       vec2<f32>,
}

// ---- Uniforms (32 bytes, 16-byte aligned) ----

struct CompositeUniforms {
    tab_bar_height:    f32, // pixels from top where the tab bar ends
    window_height:     f32, // total window height in pixels
    window_width:      f32, // total window width  in pixels
    window_padding:    f32, // padding around the grid area
    grid_pixel_width:  f32, // cols * cell_width
    grid_pixel_height: f32, // rows * cell_height
    bg_color:          u32, // default background 0xRRGGBB
    _padding:          u32,
}

// ---- Bindings ----

@group(0) @binding(0) var grid_texture: texture_2d<f32>;
@group(0) @binding(1) var ui_texture:   texture_2d<f32>;
@group(0) @binding(2) var tex_sampler:  sampler;
@group(0) @binding(3) var<uniform> uniforms: CompositeUniforms;

// ---- Helpers ----

fn unpack_rgb(c: u32) -> vec3<f32> {
    return vec3<f32>(
        f32((c >> 16u) & 0xFFu) / 255.0,
        f32((c >>  8u) & 0xFFu) / 255.0,
        f32( c         & 0xFFu) / 255.0,
    );
}

// ---- Vertex stage: fullscreen triangle (3 vertices) ----

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
    let pixel = vec2<f32>(in.uv.x * uniforms.window_width, in.uv.y * uniforms.window_height);

    let bg = vec4<f32>(unpack_rgb(uniforms.bg_color), 1.0);

    // The grid content starts at (window_padding, tab_bar_height + window_padding)
    // in screen space. In the grid texture, the grid cells are rendered starting
    // at pixel (0, 0) and extend to (grid_pixel_width, grid_pixel_height).
    let grid_origin_x = uniforms.window_padding;
    let grid_origin_y = uniforms.tab_bar_height + uniforms.window_padding;

    // Compute grid-local pixel coordinate.
    let gx = pixel.x - grid_origin_x;
    let gy = pixel.y - grid_origin_y;

    // Sample base color.
    var base_color: vec4<f32>;
    if pixel.y < uniforms.tab_bar_height {
        // Above the tab bar: use background color (UI overlay covers this area).
        base_color = bg;
    } else if gx >= 0.0 && gx < uniforms.grid_pixel_width &&
              gy >= 0.0 && gy < uniforms.grid_pixel_height {
        // Inside the grid area: sample from grid texture using pixel coordinates.
        let tex_size = vec2<f32>(textureDimensions(grid_texture));
        let grid_uv = vec2<f32>(gx / tex_size.x, gy / tex_size.y);
        base_color = textureSampleLevel(grid_texture, tex_sampler, grid_uv, 0.0);
    } else {
        // Outside grid area (padding, below grid): terminal background color.
        base_color = bg;
    }

    // Sample UI overlay.
    let ui = textureSampleLevel(ui_texture, tex_sampler, in.uv, 0.0);

    // Alpha composite: UI over grid.
    let out_rgb = mix(base_color.rgb, ui.rgb, ui.a);
    return vec4<f32>(out_rgb, 1.0);
}
