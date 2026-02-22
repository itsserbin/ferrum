//! GPU buffer data structures for grid cells and UI draw commands.
//!
//! All structs are `#[repr(C)]` with `bytemuck::Pod` so they can be
//! uploaded to wgpu buffers with zero-copy.

/// Packed terminal cell for the grid compute shader (16 bytes).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PackedCell {
    /// Unicode codepoint (0 = empty, 32 = space).
    pub codepoint: u32,
    /// Foreground color as 0xRRGGBB.
    pub fg: u32,
    /// Background color as 0xRRGGBB.
    pub bg: u32,
    /// Attribute bitfield:
    ///   bit 0: bold
    ///   bit 1: italic
    ///   bit 2: underline (any style)
    ///   bit 3: reverse video
    ///   bit 4: dim
    ///   bit 5: strikethrough
    ///   bits 6-7: underline style (0=none, 1=single, 2=double, 3=curly)
    pub attrs: u32,
}

/// Grid uniforms uploaded once per frame (48 bytes, 16-byte aligned).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridUniforms {
    pub cols: u32,
    pub rows: u32,
    pub cell_width: u32,
    pub cell_height: u32,
    /// Top-left origin in the grid texture where this batch is drawn.
    pub origin_x: u32,
    pub origin_y: u32,
    pub bg_color: u32,
    pub _pad0: u32,
    pub tex_width: u32,
    pub tex_height: u32,
    pub _pad1: u32,
    pub _pad2: u32,
}

/// GPU draw command matching the WGSL `DrawCommand` layout (40 bytes).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuDrawCommand {
    pub cmd_type: u32,
    pub param1: f32,
    pub param2: f32,
    pub param3: f32,
    pub param4: f32,
    pub param5: f32,
    pub param6: f32,
    pub color: u32,
    pub alpha: f32,
    pub _pad: f32,
}

/// Command type constants matching the WGSL shader.
pub const CMD_RECT: u32 = 0;
pub const CMD_ROUNDED_RECT: u32 = 1;
#[cfg(not(target_os = "macos"))]
pub const CMD_CIRCLE: u32 = 2;
#[cfg(not(target_os = "macos"))]
pub const CMD_LINE: u32 = 3;
pub const CMD_GLYPH: u32 = 4;

/// UI uniforms uploaded once per frame (32 bytes, 16-byte aligned).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiUniforms {
    pub width: f32,
    pub height: f32,
    pub atlas_width: f32,
    pub atlas_height: f32,
    pub command_count: u32,
    pub _pad1: u32,
    pub _pad2: u32,
    pub _pad3: u32,
}

/// Composite uniforms (48 bytes, 16-byte aligned).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CompositeUniforms {
    pub tab_bar_height: f32,
    pub window_height: f32,
    pub window_width: f32,
    pub window_padding: f32,
    pub grid_pixel_width: f32,
    pub grid_pixel_height: f32,
    pub bg_color: u32,
    pub _padding: u32,
}
