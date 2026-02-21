//! GPU renderer backend using wgpu compute + render pipelines.
//!
//! Three-pass architecture:
//! 1. Grid compute shader — renders terminal cells into a texture
//! 2. UI fragment shader — renders draw commands (tab bar, overlays) via SDF
//! 3. Composite shader — blends grid + UI into the final swapchain surface

pub mod atlas;
pub mod buffers;
mod cursors;
mod frame;
mod gpu_passes;
mod grid_packing;
mod hit_test;
mod overlays;
pub mod pipelines;
mod scrollbar;
mod setup;
mod tab_controls;
mod tab_layout;
mod tab_rendering;
mod trait_impl;
mod ui_commands;
mod window_buttons;

use fontdue::Font;
use wgpu;

use crate::config::ThemePalette;
use super::metrics::FontMetrics;

use atlas::GlyphAtlas;
use buffers::*;

// Tab-bar palette constants and SCROLLBAR_MIN_THUMB are centralized in
// the parent `renderer/mod.rs`.

/// Maximum number of UI draw commands per frame.
const MAX_UI_COMMANDS: usize = 4096;

/// One grid compute batch for the current frame.
struct GridBatch {
    cells: Vec<PackedCell>,
    uniforms: GridUniforms,
    dispatch_width: u32,
    dispatch_height: u32,
}

/// GPU-based renderer using wgpu compute and render pipelines.
pub struct GpuRenderer {
    // wgpu core
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    // Textures
    grid_texture: wgpu::Texture,
    grid_texture_view: wgpu::TextureView,
    ui_texture: wgpu::Texture,
    ui_texture_view: wgpu::TextureView,

    // Pipelines
    grid_pipeline: wgpu::ComputePipeline,
    ui_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,

    // Bind group layouts
    grid_bind_group_layout: wgpu::BindGroupLayout,
    ui_bind_group_layout: wgpu::BindGroupLayout,
    composite_bind_group_layout: wgpu::BindGroupLayout,

    // Buffers
    grid_uniform_buffer: wgpu::Buffer,
    grid_cell_buffer: wgpu::Buffer,
    glyph_info_buffer: wgpu::Buffer,
    ui_uniform_buffer: wgpu::Buffer,
    ui_command_buffer: wgpu::Buffer,
    composite_uniform_buffer: wgpu::Buffer,

    // Sampler for composite and UI
    sampler: wgpu::Sampler,

    // Atlas
    atlas: GlyphAtlas,

    // Font & metrics
    font: Font,
    fallback_fonts: Vec<Font>,
    metrics: FontMetrics,
    pub(in crate::gui::renderer) palette: ThemePalette,

    // UI command accumulator (filled during draw_* calls, flushed in present).
    commands: Vec<GpuDrawCommand>,

    // Grid batches for the current frame (single pane = 1 batch + clear).
    grid_batches: Vec<GridBatch>,
    grid_dirty: bool,

    // Window dimensions
    width: u32,
    height: u32,
}
