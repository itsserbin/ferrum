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

use super::FONT_SIZE;
use super::metrics::FontMetrics;

use atlas::GlyphAtlas;
use buffers::*;

// Catppuccin Mocha palette constants (same as CPU renderer).
const BAR_BG: u32 = 0x181825;
const ACTIVE_TAB_BG: u32 = 0x1E1E2E;
const INACTIVE_TAB_HOVER: u32 = 0x313244;
const TAB_TEXT_ACTIVE: u32 = 0xCDD6F4;
const TAB_TEXT_INACTIVE: u32 = 0x6C7086;
const TAB_BORDER: u32 = 0x313244;
const CLOSE_HOVER_BG_COLOR: u32 = 0x585B70;
const RENAME_FIELD_BG: u32 = 0x24273A;
const RENAME_FIELD_BORDER: u32 = 0x6C7086;
const RENAME_SELECTION_BG: u32 = 0xB4BEFE;

#[cfg(not(target_os = "macos"))]
const WIN_BTN_WIDTH: u32 = 46;

const SCROLLBAR_MIN_THUMB: u32 = 20;

const INSERTION_COLOR: u32 = 0xCBA6F7;

/// Maximum number of UI draw commands per frame.
const MAX_UI_COMMANDS: usize = 4096;

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
    metrics: FontMetrics,

    // UI command accumulator (filled during draw_* calls, flushed in present).
    commands: Vec<GpuDrawCommand>,

    // Grid state for the current frame (filled in render(), dispatched in present()).
    grid_cells: Vec<PackedCell>,
    grid_uniforms: GridUniforms,
    grid_dirty: bool,

    // Window dimensions
    width: u32,
    height: u32,
}
