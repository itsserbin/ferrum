// UI overlay fragment shader — renders draw commands using SDF primitives.
//
// A single fullscreen triangle is rasterized. The fragment shader loops over
// a storage buffer of draw commands and evaluates each one via signed-distance
// functions, compositing the result with standard alpha-over blending.

// ---- Command type constants ----

const CMD_RECT:         u32 = 0u;
const CMD_ROUNDED_RECT: u32 = 1u;
const CMD_CIRCLE:       u32 = 2u;
const CMD_LINE:         u32 = 3u;
const CMD_GLYPH:        u32 = 4u;

// ---- Draw command (40 bytes, padded to multiple of 8) ----
//
// Layout per command type:
//   Rect:        param1=x  param2=y  param3=w   param4=h   param5=--    param6=--
//   RoundedRect: param1=x  param2=y  param3=w   param4=h   param5=r     param6=--
//   Circle:      param1=cx param2=cy param3=r    param4=--  param5=--    param6=--
//   Line:        param1=x1 param2=y1 param3=x2   param4=y2  param5=width param6=--
//   Glyph:       param1=x  param2=y  param3=atlX param4=atlY param5=atlW param6=atlH

struct DrawCommand {
    cmd_type: u32,
    param1:   f32,
    param2:   f32,
    param3:   f32,
    param4:   f32,
    param5:   f32,
    param6:   f32,
    color:    u32,       // 0xRRGGBB
    alpha:    f32,
    _pad:     f32,       // pad to 40 bytes (10 x 4)
}

// ---- Uniforms (32 bytes, 16-byte aligned) ----

struct UiUniforms {
    width:         f32,  // viewport width  in pixels
    height:        f32,  // viewport height in pixels
    atlas_width:   f32,  // glyph atlas width  in pixels
    atlas_height:  f32,  // glyph atlas height in pixels
    command_count: u32,  // number of active draw commands
    _pad1:         u32,
    _pad2:         u32,
    _pad3:         u32,
}

// ---- Bindings ----

@group(0) @binding(0) var<uniform>       uniforms:      UiUniforms;
@group(0) @binding(1) var<storage, read> commands:       array<DrawCommand>;
@group(0) @binding(2) var               atlas:           texture_2d<f32>;
@group(0) @binding(3) var               atlas_sampler:   sampler;

// ---- Vertex / Fragment IO ----

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0)       uv:       vec2<f32>,
}

// ---- Vertex stage: fullscreen triangle (3 vertices) ----

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    // Generates a triangle that covers [-1,3] x [-1,3] in clip space,
    // which fully covers the [-1,1] x [-1,1] viewport.
    let x = f32(i32(vi) / 2) * 4.0 - 1.0;
    let y = f32(i32(vi) % 2) * 4.0 - 1.0;
    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv       = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

// ---- SDF primitives ----

// Axis-aligned box.
fn sdf_box(p: vec2<f32>, center: vec2<f32>, half_size: vec2<f32>) -> f32 {
    let d = abs(p - center) - half_size;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}

// Rounded box (same as box but with corner radius subtracted).
fn sdf_rounded_box(p: vec2<f32>, center: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let d = abs(p - center) - half_size + vec2<f32>(radius);
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - radius;
}

// Circle.
fn sdf_circle(p: vec2<f32>, center: vec2<f32>, radius: f32) -> f32 {
    return length(p - center) - radius;
}

// Line segment (returns distance to the nearest point on the segment).
fn sdf_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h  = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

// Unpack 0xRRGGBB into vec3<f32>.
fn unpack_color(c: u32) -> vec3<f32> {
    return vec3<f32>(
        f32((c >> 16u) & 0xFFu) / 255.0,
        f32((c >>  8u) & 0xFFu) / 255.0,
        f32( c         & 0xFFu) / 255.0,
    );
}

// ---- Fragment stage ----

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<f32>(in.uv.x * uniforms.width, in.uv.y * uniforms.height);

    // Accumulator — start fully transparent.
    var result = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    for (var i = 0u; i < uniforms.command_count; i++) {
        let cmd = commands[i];
        let col = unpack_color(cmd.color);
        var cmd_alpha = 0.0;

        switch cmd.cmd_type {
            // -- Rect --
            case 0u: {
                let center = vec2<f32>(
                    cmd.param1 + cmd.param3 * 0.5,
                    cmd.param2 + cmd.param4 * 0.5,
                );
                let half = vec2<f32>(cmd.param3 * 0.5, cmd.param4 * 0.5);
                let d = sdf_box(pixel, center, half);
                cmd_alpha = cmd.alpha * (1.0 - smoothstep(-0.5, 0.5, d));
            }
            // -- Rounded rect --
            case 1u: {
                let center = vec2<f32>(
                    cmd.param1 + cmd.param3 * 0.5,
                    cmd.param2 + cmd.param4 * 0.5,
                );
                let half = vec2<f32>(cmd.param3 * 0.5, cmd.param4 * 0.5);
                let d = sdf_rounded_box(pixel, center, half, cmd.param5);
                cmd_alpha = cmd.alpha * (1.0 - smoothstep(-0.5, 0.5, d));
            }
            // -- Circle --
            case 2u: {
                let center = vec2<f32>(cmd.param1, cmd.param2);
                let d = sdf_circle(pixel, center, cmd.param3);
                cmd_alpha = cmd.alpha * (1.0 - smoothstep(-0.5, 0.5, d));
            }
            // -- Line --
            case 3u: {
                let a = vec2<f32>(cmd.param1, cmd.param2);
                let b = vec2<f32>(cmd.param3, cmd.param4);
                let half_w = cmd.param5 * 0.5;
                let d = sdf_segment(pixel, a, b);
                cmd_alpha = cmd.alpha * (1.0 - smoothstep(half_w - 0.5, half_w + 0.5, d));
            }
            // -- Glyph (atlas lookup) --
            case 4u: {
                let gx = pixel.x - cmd.param1;
                let gy = pixel.y - cmd.param2;
                if gx >= 0.0 && gx < cmd.param5 && gy >= 0.0 && gy < cmd.param6 {
                    let tex_x = i32(cmd.param3 + floor(gx));
                    let tex_y = i32(cmd.param4 + floor(gy));
                    cmd_alpha = cmd.alpha * textureLoad(atlas, vec2<i32>(tex_x, tex_y), 0).r;
                }
            }
            default: {}
        }

        // Alpha composite: source over destination.
        if cmd_alpha > 0.001 {
            result = vec4<f32>(
                mix(result.rgb, col, cmd_alpha),
                result.a + cmd_alpha * (1.0 - result.a),
            );
        }
    }

    return result;
}
