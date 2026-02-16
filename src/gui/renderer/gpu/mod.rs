//! GPU renderer backend using wgpu compute + render pipelines.
//!
//! Three-pass architecture:
//! 1. Grid compute shader — renders terminal cells into a texture
//! 2. UI fragment shader — renders draw commands (tab bar, overlays) via SDF
//! 3. Composite shader — blends grid + UI into the final swapchain surface

pub mod atlas;
pub mod buffers;
pub mod pipelines;

use std::sync::Arc;

use fontdue::{Font, FontSettings};
use wgpu;
use winit::window::Window;

use crate::core::{Color, CursorStyle, Grid, Selection};

use super::metrics::FontMetrics;
use super::traits;
use super::{
    ContextAction, ContextMenu, SecurityPopup, TabBarHit, TabInfo, WindowButton,
    FONT_SIZE, MIN_TAB_WIDTH, MIN_TAB_WIDTH_FOR_TITLE,
};

use atlas::GlyphAtlas;
use buffers::*;

// Catppuccin Mocha palette constants (same as CPU renderer).
const BAR_BG: u32 = 0x181825;
const ACTIVE_TAB_BG: u32 = 0x1E1E2E;
const INACTIVE_TAB_HOVER: u32 = 0x313244;
const TAB_TEXT_ACTIVE: u32 = 0xCDD6F4;
const TAB_TEXT_INACTIVE: u32 = 0x6C7086;
const TAB_BORDER: u32 = 0x313244;
const CLOSE_HOVER_BG_COLOR: u32 = 0xF38BA8;

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

impl GpuRenderer {
    /// Creates a new GPU renderer, initializing wgpu device, pipelines, and textures.
    pub fn new(window: Arc<Window>) -> Result<Self, Box<dyn std::error::Error>> {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        // Initialize wgpu.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("ferrum_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: Default::default(),
                experimental_features: Default::default(),
            },
        ))?;

        // Surface configuration.
        // Prefer a non-sRGB format so the GPU doesn't apply automatic gamma
        // correction when writing to the surface. Our shaders output colors in
        // the same space as the palette constants (already perceptual/sRGB values
        // stored in linear textures), so an extra hardware sRGB conversion would
        // double-gamma and wash out colors.
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Font setup.
        let font_data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/JetBrainsMono-Regular.ttf"
        ));
        let font =
            Font::from_bytes(font_data as &[u8], FontSettings::default()).expect("font load fail");

        let mut metrics = FontMetrics {
            cell_width: 1,
            cell_height: 1,
            font_size: FONT_SIZE,
            ui_scale: 1.0,
            ascent: 0,
        };
        metrics.recompute(&font);

        // Create glyph atlas.
        let atlas = GlyphAtlas::new(&device, &queue, &font, metrics.font_size, metrics.ascent);

        // Create pipelines.
        let (grid_pipeline, grid_bind_group_layout) = pipelines::create_grid_pipeline(&device);
        let (ui_pipeline, ui_bind_group_layout) = pipelines::create_ui_pipeline(&device);
        let (composite_pipeline, composite_bind_group_layout) =
            pipelines::create_composite_pipeline(&device, surface_format);

        // Create intermediate textures.
        let (grid_texture, grid_texture_view) =
            Self::create_offscreen_texture(&device, width, height, "grid_texture", true);
        let (ui_texture, ui_texture_view) =
            Self::create_offscreen_texture(&device, width, height, "ui_texture", false);

        // Create buffers.
        let grid_uniform_buffer = Self::create_uniform_buffer(
            &device,
            std::mem::size_of::<GridUniforms>(),
            "grid_uniforms",
        );
        let grid_cell_buffer = Self::create_storage_buffer(
            &device,
            std::mem::size_of::<PackedCell>() * 256 * 64,
            "grid_cells",
        );
        let glyph_data = atlas.glyph_info_buffer_data();
        let glyph_info_buffer = Self::create_storage_buffer_init(
            &device,
            bytemuck::cast_slice(&glyph_data),
            "glyph_info",
        );

        let ui_uniform_buffer = Self::create_uniform_buffer(
            &device,
            std::mem::size_of::<UiUniforms>(),
            "ui_uniforms",
        );
        let ui_command_buffer = Self::create_storage_buffer(
            &device,
            std::mem::size_of::<GpuDrawCommand>() * MAX_UI_COMMANDS,
            "ui_commands",
        );
        let composite_uniform_buffer = Self::create_uniform_buffer(
            &device,
            std::mem::size_of::<CompositeUniforms>(),
            "composite_uniforms",
        );

        // Sampler for textures.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("tex_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(GpuRenderer {
            device,
            queue,
            surface,
            surface_config,
            grid_texture,
            grid_texture_view,
            ui_texture,
            ui_texture_view,
            grid_pipeline,
            ui_pipeline,
            composite_pipeline,
            grid_bind_group_layout,
            ui_bind_group_layout,
            composite_bind_group_layout,
            grid_uniform_buffer,
            grid_cell_buffer,
            glyph_info_buffer,
            ui_uniform_buffer,
            ui_command_buffer,
            composite_uniform_buffer,
            sampler,
            atlas,
            font,
            metrics,
            commands: Vec::with_capacity(MAX_UI_COMMANDS),
            grid_cells: Vec::new(),
            grid_uniforms: GridUniforms {
                cols: 0,
                rows: 0,
                cell_width: 0,
                cell_height: 0,
                atlas_width: 0,
                atlas_height: 0,
                baseline: 0,
                bg_color: Color::DEFAULT_BG.to_pixel(),
                tex_width: width,
                tex_height: height,
                _pad1: 0,
                _pad2: 0,
            },
            grid_dirty: false,
            width,
            height,
        })
    }

    // ── Texture / buffer helpers ──────────────────────────────────────

    fn create_offscreen_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
        storage: bool,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let usage = if storage {
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING
        } else {
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_uniform_buffer(device: &wgpu::Device, size: usize, label: &str) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn create_storage_buffer(device: &wgpu::Device, size: usize, label: &str) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn create_storage_buffer_init(
        device: &wgpu::Device,
        data: &[u8],
        label: &str,
    ) -> wgpu::Buffer {
        use wgpu::util::DeviceExt;
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: data,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        })
    }

    /// Rebuilds grid and UI textures after a window resize.
    fn resize_textures(&mut self) {
        let (gt, gtv) =
            Self::create_offscreen_texture(&self.device, self.width, self.height, "grid_texture", true);
        self.grid_texture = gt;
        self.grid_texture_view = gtv;

        let (ut, utv) =
            Self::create_offscreen_texture(&self.device, self.width, self.height, "ui_texture", false);
        self.ui_texture = ut;
        self.ui_texture_view = utv;
    }

    /// Rebuilds glyph atlas and related buffer after scale change.
    fn rebuild_atlas(&mut self) {
        self.atlas = GlyphAtlas::new(
            &self.device,
            &self.queue,
            &self.font,
            self.metrics.font_size,
            self.metrics.ascent,
        );
        let glyph_data = self.atlas.glyph_info_buffer_data();
        self.glyph_info_buffer = Self::create_storage_buffer_init(
            &self.device,
            bytemuck::cast_slice(&glyph_data),
            "glyph_info",
        );
    }

    // ── UI command helpers ────────────────────────────────────────────

    fn push_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: u32, alpha: f32) {
        if self.commands.len() >= MAX_UI_COMMANDS {
            return;
        }
        self.commands.push(GpuDrawCommand {
            cmd_type: CMD_RECT,
            param1: x,
            param2: y,
            param3: w,
            param4: h,
            param5: 0.0,
            param6: 0.0,
            color,
            alpha,
            _pad: 0.0,
        });
    }

    fn push_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        color: u32,
        alpha: f32,
    ) {
        if self.commands.len() >= MAX_UI_COMMANDS {
            return;
        }
        self.commands.push(GpuDrawCommand {
            cmd_type: CMD_ROUNDED_RECT,
            param1: x,
            param2: y,
            param3: w,
            param4: h,
            param5: r,
            param6: 0.0,
            color,
            alpha,
            _pad: 0.0,
        });
    }

    fn push_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
        color: u32,
        alpha: f32,
    ) {
        if self.commands.len() >= MAX_UI_COMMANDS {
            return;
        }
        self.commands.push(GpuDrawCommand {
            cmd_type: CMD_LINE,
            param1: x1,
            param2: y1,
            param3: x2,
            param4: y2,
            param5: width,
            param6: 0.0,
            color,
            alpha,
            _pad: 0.0,
        });
    }

    fn push_circle(&mut self, cx: f32, cy: f32, r: f32, color: u32, alpha: f32) {
        if self.commands.len() >= MAX_UI_COMMANDS {
            return;
        }
        self.commands.push(GpuDrawCommand {
            cmd_type: CMD_CIRCLE,
            param1: cx,
            param2: cy,
            param3: r,
            param4: 0.0,
            param5: 0.0,
            param6: 0.0,
            color,
            alpha,
            _pad: 0.0,
        });
    }

    fn push_glyph(
        &mut self,
        x: f32,
        y: f32,
        atlas_x: f32,
        atlas_y: f32,
        atlas_w: f32,
        atlas_h: f32,
        color: u32,
        alpha: f32,
    ) {
        if self.commands.len() >= MAX_UI_COMMANDS {
            return;
        }
        self.commands.push(GpuDrawCommand {
            cmd_type: CMD_GLYPH,
            param1: x,
            param2: y,
            param3: atlas_x,
            param4: atlas_y,
            param5: atlas_w,
            param6: atlas_h,
            color,
            alpha,
            _pad: 0.0,
        });
    }

    /// Pushes glyph draw commands for a string at the given position.
    fn push_text(&mut self, x: f32, y: f32, text: &str, color: u32, alpha: f32) {
        let cw = self.metrics.cell_width as f32;
        for (i, ch) in text.chars().enumerate() {
            let cp = ch as u32;
            let info = self.atlas.get_or_insert(cp, &self.font, self.metrics.font_size, &self.queue);
            if info.w > 0.0 && info.h > 0.0 {
                let gx = x + i as f32 * cw + info.offset_x;
                let gy = y + info.offset_y;
                self.push_glyph(gx, gy, info.x, info.y, info.w, info.h, color, alpha);
            }
        }
    }

    // ── Tab bar math (mirrors CpuRenderer) ────────────────────────────

    fn tab_strip_start_x_val(&self) -> u32 {
        #[cfg(target_os = "macos")]
        {
            self.metrics.scaled_px(78)
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.metrics.scaled_px(8)
        }
    }

    fn plus_button_reserved_width(&self) -> u32 {
        self.metrics.cell_width + self.metrics.scaled_px(20)
    }

    fn window_buttons_reserved_width(&self) -> u32 {
        #[cfg(not(target_os = "macos"))]
        {
            self.metrics.scaled_px(WIN_BTN_WIDTH) * 3
        }
        #[cfg(target_os = "macos")]
        {
            0
        }
    }

    fn tab_width_val(&self, tab_count: usize, buf_width: u32) -> u32 {
        let reserved = self.tab_strip_start_x_val()
            + self.plus_button_reserved_width()
            + self.metrics.scaled_px(8)
            + self.window_buttons_reserved_width();
        let available = buf_width.saturating_sub(reserved);
        let min_tw = self.metrics.scaled_px(MIN_TAB_WIDTH);
        let max_tw = self.metrics.scaled_px(240);
        (available / tab_count.max(1) as u32).clamp(min_tw, max_tw)
    }

    fn tab_origin_x_val(&self, tab_index: usize, tw: u32) -> u32 {
        self.tab_strip_start_x_val() + tab_index as u32 * tw
    }

    fn close_button_rect(&self, tab_index: usize, tw: u32) -> (u32, u32, u32, u32) {
        let btn_size = self.metrics.scaled_px(20);
        let x = self.tab_origin_x_val(tab_index, tw) + tw - btn_size - self.metrics.scaled_px(6);
        let y = (self.metrics.tab_bar_height_px().saturating_sub(btn_size)) / 2;
        (x, y, btn_size, btn_size)
    }

    fn plus_button_rect(&self, tab_count: usize, tw: u32) -> (u32, u32, u32, u32) {
        let btn_size = self.metrics.scaled_px(24);
        let x = self.tab_strip_start_x_val() + tab_count as u32 * tw + self.metrics.scaled_px(4);
        let y = (self.metrics.tab_bar_height_px().saturating_sub(btn_size)) / 2;
        (x, y, btn_size, btn_size)
    }

    fn should_show_number(&self, tw: u32) -> bool {
        tw < self.metrics.scaled_px(MIN_TAB_WIDTH_FOR_TITLE)
    }

    fn security_badge_rect_val(
        &self,
        tab_index: usize,
        tab_count: usize,
        buf_width: u32,
        security_count: usize,
    ) -> Option<(u32, u32, u32, u32)> {
        if security_count == 0 || tab_index >= tab_count {
            return None;
        }
        let tw = self.tab_width_val(tab_count, buf_width);
        let tab_x = self.tab_origin_x_val(tab_index, tw);
        let badge_min = self.metrics.scaled_px(10);
        let badge_max = self.metrics.scaled_px(15);
        let badge_size = self
            .metrics
            .cell_height
            .saturating_sub(self.metrics.scaled_px(10))
            .clamp(badge_min, badge_max);
        let count_chars = if security_count > 1 {
            security_count.min(99).to_string().len() as u32
        } else {
            0
        };
        let count_width = if count_chars > 0 {
            count_chars * self.metrics.cell_width + self.metrics.scaled_px(2)
        } else {
            0
        };
        let indicator_width = badge_size + count_width;
        let right_gutter = self.metrics.cell_width + self.metrics.scaled_px(10);
        let indicator_right = tab_x + tw.saturating_sub(right_gutter);
        let x = indicator_right.saturating_sub(indicator_width + self.metrics.scaled_px(2));
        let y = (self.metrics.tab_bar_height_px().saturating_sub(badge_size)) / 2;
        Some((x, y, badge_size, badge_size))
    }

    fn point_in_rect(x: f64, y: f64, rect: (u32, u32, u32, u32)) -> bool {
        let (rx, ry, rw, rh) = rect;
        x >= rx as f64 && x < (rx + rw) as f64 && y >= ry as f64 && y < (ry + rh) as f64
    }
}

// ── Renderer trait implementation ────────────────────────────────────

impl traits::Renderer for GpuRenderer {
    fn set_scale(&mut self, scale_factor: f64) {
        let scale = if scale_factor.is_finite() {
            scale_factor.clamp(0.75, 4.0)
        } else {
            1.0
        };
        if (self.metrics.ui_scale - scale).abs() < f64::EPSILON {
            return;
        }
        self.metrics.ui_scale = scale;
        self.metrics.recompute(&self.font);
        self.rebuild_atlas();
    }

    fn cell_width(&self) -> u32 {
        self.metrics.cell_width
    }

    fn cell_height(&self) -> u32 {
        self.metrics.cell_height
    }

    fn tab_bar_height_px(&self) -> u32 {
        self.metrics.tab_bar_height_px()
    }

    fn window_padding_px(&self) -> u32 {
        self.metrics.window_padding_px()
    }

    fn ui_scale(&self) -> f64 {
        self.metrics.ui_scale
    }

    fn scaled_px(&self, base: u32) -> u32 {
        self.metrics.scaled_px(base)
    }

    fn scrollbar_hit_zone_px(&self) -> u32 {
        self.metrics.scrollbar_hit_zone_px()
    }

    // ── Terminal rendering ────────────────────────────────────────────

    fn render(
        &mut self,
        _buffer: &mut [u32],
        _buf_width: usize,
        _buf_height: usize,
        grid: &Grid,
        selection: Option<&Selection>,
    ) {
        // Pack grid cells into the GPU buffer format.
        let rows = grid.rows;
        let cols = grid.cols;
        self.grid_cells.clear();
        self.grid_cells.reserve(rows * cols);

        for row in 0..rows {
            for col in 0..cols {
                let cell = grid.get(row, col);
                let selected = selection.is_some_and(|s| s.contains(row, col));

                let mut attrs = 0u32;
                if cell.bold {
                    attrs |= 1;
                }
                if cell.underline {
                    attrs |= 4;
                }
                if cell.reverse {
                    attrs |= 8;
                }
                if selected {
                    attrs |= 16;
                }

                let mut fg = cell.fg;
                // Bold: bright variant for base ANSI colors.
                if cell.bold {
                    for i in 0..8 {
                        if fg.r == Color::ANSI[i].r
                            && fg.g == Color::ANSI[i].g
                            && fg.b == Color::ANSI[i].b
                        {
                            fg = Color::ANSI[i + 8];
                            break;
                        }
                    }
                }

                self.grid_cells.push(PackedCell {
                    codepoint: cell.character as u32,
                    fg: fg.to_pixel(),
                    bg: cell.bg.to_pixel(),
                    attrs,
                });
            }
        }

        self.grid_uniforms = GridUniforms {
            cols: cols as u32,
            rows: rows as u32,
            cell_width: self.metrics.cell_width,
            cell_height: self.metrics.cell_height,
            atlas_width: self.atlas.atlas_width,
            atlas_height: self.atlas.atlas_height,
            baseline: self.metrics.ascent as u32,
            bg_color: Color::DEFAULT_BG.to_pixel(),
            tex_width: self.width,
            tex_height: self.height,
            _pad1: 0,
            _pad2: 0,
        };
        self.grid_dirty = true;
    }

    fn draw_cursor(
        &mut self,
        _buffer: &mut [u32],
        _buf_width: usize,
        _buf_height: usize,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
    ) {
        let x = col as f32 * self.metrics.cell_width as f32 + self.metrics.window_padding_px() as f32;
        let y = row as f32 * self.metrics.cell_height as f32
            + self.metrics.tab_bar_height_px() as f32
            + self.metrics.window_padding_px() as f32;
        let cw = self.metrics.cell_width as f32;
        let ch = self.metrics.cell_height as f32;
        let cursor_color = Color::DEFAULT_FG.to_pixel();

        match style {
            CursorStyle::BlinkingBlock | CursorStyle::SteadyBlock => {
                // Filled block — draw bg rect + inverted glyph.
                self.push_rect(x, y, cw, ch, cursor_color, 1.0);
                let cell = grid.get(row, col);
                if cell.character != ' ' {
                    let cp = cell.character as u32;
                    let info = self.atlas.get_or_insert(
                        cp,
                        &self.font,
                        self.metrics.font_size,
                        &self.queue,
                    );
                    if info.w > 0.0 && info.h > 0.0 {
                        let gx = x + info.offset_x;
                        let gy = y + info.offset_y;
                        self.push_glyph(
                            gx,
                            gy,
                            info.x,
                            info.y,
                            info.w,
                            info.h,
                            Color::DEFAULT_BG.to_pixel(),
                            1.0,
                        );
                    }
                }
            }
            CursorStyle::BlinkingUnderline | CursorStyle::SteadyUnderline => {
                self.push_rect(x, y + ch - 2.0, cw, 2.0, cursor_color, 1.0);
            }
            CursorStyle::BlinkingBar | CursorStyle::SteadyBar => {
                self.push_rect(x, y, 2.0, ch, cursor_color, 1.0);
            }
        }
    }

    // ── Scrollbar ─────────────────────────────────────────────────────

    fn render_scrollbar(
        &mut self,
        _buffer: &mut [u32],
        _buf_width: usize,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
        opacity: f32,
        hover: bool,
    ) {
        if scrollback_len == 0 || opacity <= 0.0 {
            return;
        }

        let track_top =
            (self.metrics.tab_bar_height_px() + self.metrics.window_padding_px()) as f32;
        let track_bottom = buf_height as f32 - self.metrics.window_padding_px() as f32;
        let track_height = track_bottom - track_top;
        if track_height <= 0.0 {
            return;
        }

        let total_lines = scrollback_len + grid_rows;
        let viewport_ratio = grid_rows as f32 / total_lines as f32;
        let min_thumb = self.metrics.scaled_px(SCROLLBAR_MIN_THUMB) as f32;
        let thumb_height = (viewport_ratio * track_height)
            .max(min_thumb)
            .min(track_height);

        let max_offset = scrollback_len as f32;
        let scroll_ratio = (max_offset - scroll_offset as f32) / max_offset;
        let thumb_y = track_top + scroll_ratio * (track_height - thumb_height);

        let sb_width = self.metrics.scaled_px(super::SCROLLBAR_WIDTH) as f32;
        let sb_margin = self.metrics.scaled_px(super::SCROLLBAR_MARGIN) as f32;
        let thumb_x = self.width as f32 - sb_width - sb_margin;
        let radius = self.metrics.scaled_px(3) as f32;

        let color = if hover { 0x7F849C } else { 0x6C7086 };
        let base_alpha = 180.0 / 255.0;
        let alpha = base_alpha * opacity;

        self.push_rounded_rect(thumb_x, thumb_y, sb_width, thumb_height, radius, color, alpha);
    }

    fn scrollbar_thumb_bounds(
        &self,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
    ) -> Option<(f32, f32)> {
        if scrollback_len == 0 {
            return None;
        }

        let track_top =
            (self.metrics.tab_bar_height_px() + self.metrics.window_padding_px()) as f32;
        let track_bottom = buf_height as f32 - self.metrics.window_padding_px() as f32;
        let track_height = track_bottom - track_top;
        if track_height <= 0.0 {
            return None;
        }

        let total_lines = scrollback_len + grid_rows;
        let viewport_ratio = grid_rows as f32 / total_lines as f32;
        let min_thumb = self.metrics.scaled_px(SCROLLBAR_MIN_THUMB) as f32;
        let thumb_height = (viewport_ratio * track_height)
            .max(min_thumb)
            .min(track_height);

        let max_offset = scrollback_len as f32;
        let scroll_ratio = (max_offset - scroll_offset as f32) / max_offset;
        let thumb_y = track_top + scroll_ratio * (track_height - thumb_height);

        Some((thumb_y, thumb_height))
    }

    // ── Tab bar ───────────────────────────────────────────────────────

    fn draw_tab_bar(
        &mut self,
        _buffer: &mut [u32],
        buf_width: usize,
        _buf_height: usize,
        tabs: &[TabInfo],
        hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
        tab_offsets: Option<&[f32]>,
    ) {
        let tab_bar_h = self.metrics.tab_bar_height_px() as f32;
        let bw = buf_width as u32;

        // Bar background.
        self.push_rounded_rect(0.0, 0.0, bw as f32, tab_bar_h, self.metrics.scaled_px(10) as f32, BAR_BG, 1.0);

        let tw = self.tab_width_val(tabs.len(), bw);
        let text_y = (self.metrics.tab_bar_height_px().saturating_sub(self.metrics.cell_height)) / 2
            + self.metrics.scaled_px(1);
        let tab_padding_h = self.metrics.scaled_px(14);
        let use_numbers = self.should_show_number(tw);

        for (i, tab) in tabs.iter().enumerate() {
            let anim_offset = tab_offsets.and_then(|o| o.get(i)).copied().unwrap_or(0.0);
            let tab_x = self.tab_origin_x_val(i, tw) as f32 + anim_offset;
            let is_hovered = hovered_tab == Some(i);

            // Tab background.
            if tab.is_active {
                self.push_rect(tab_x, 0.0, tw as f32, tab_bar_h, ACTIVE_TAB_BG, 1.0);
            } else if is_hovered {
                self.push_rect(tab_x, 0.0, tw as f32, tab_bar_h, INACTIVE_TAB_HOVER, 1.0);
            }

            let fg_color = if tab.is_active {
                TAB_TEXT_ACTIVE
            } else {
                TAB_TEXT_INACTIVE
            };

            if tab.is_renaming {
                // Simplified rename rendering — just show the text.
                let rename_text = tab.rename_text.unwrap_or("");
                let text_x = tab_x + tab_padding_h as f32;
                let max_chars =
                    (tw.saturating_sub(tab_padding_h * 2) / self.metrics.cell_width) as usize;
                let display: String = rename_text.chars().take(max_chars).collect();
                self.push_text(text_x, text_y as f32, &display, TAB_TEXT_ACTIVE, 1.0);
            } else if use_numbers {
                let number_str = (i + 1).to_string();
                let show_close = tab.is_active || is_hovered;
                let close_reserved = if show_close {
                    self.metrics.scaled_px(20) + self.metrics.scaled_px(6)
                } else {
                    0
                };
                let text_w = number_str.len() as u32 * self.metrics.cell_width;
                let tx = tab_x + (tw.saturating_sub(text_w + close_reserved)) as f32 / 2.0;
                self.push_text(tx, text_y as f32, &number_str, fg_color, 1.0);

                if show_close {
                    self.draw_close_button_commands(i, tw, mouse_pos);
                }
            } else {
                // Normal mode: title + close button.
                let show_close = tab.is_active || is_hovered;
                let close_reserved = if show_close {
                    self.metrics.scaled_px(20) + self.metrics.scaled_px(6)
                } else {
                    0
                };
                let security_reserved = if tab.security_count > 0 {
                    let count_chars = tab.security_count.min(99).to_string().len() as u32;
                    let count_width = if tab.security_count > 1 {
                        count_chars * self.metrics.cell_width + self.metrics.scaled_px(2)
                    } else {
                        0
                    };
                    let badge_min = self.metrics.scaled_px(10);
                    let badge_max = self.metrics.scaled_px(15);
                    self.metrics
                        .cell_height
                        .saturating_sub(self.metrics.scaled_px(10))
                        .clamp(badge_min, badge_max)
                        + count_width
                        + self.metrics.scaled_px(6)
                } else {
                    0
                };
                let max_chars = (tw.saturating_sub(
                    tab_padding_h * 2 + close_reserved + security_reserved,
                ) / self.metrics.cell_width) as usize;
                let title: String = tab.title.chars().take(max_chars).collect();
                let tx = tab_x + tab_padding_h as f32;
                self.push_text(tx, text_y as f32, &title, fg_color, 1.0);

                if show_close {
                    self.draw_close_button_commands(i, tw, mouse_pos);
                }
            }
        }

        // New-tab (+) button.
        let plus_rect = self.plus_button_rect(tabs.len(), tw);
        let plus_hover = Self::point_in_rect(mouse_pos.0, mouse_pos.1, plus_rect);
        if plus_hover {
            let (px, py, pw, ph) = plus_rect;
            self.push_rounded_rect(
                px as f32,
                py as f32,
                pw as f32,
                ph as f32,
                self.metrics.scaled_px(5) as f32,
                INACTIVE_TAB_HOVER,
                1.0,
            );
        }
        let plus_fg = if plus_hover {
            TAB_TEXT_ACTIVE
        } else {
            TAB_TEXT_INACTIVE
        };
        let (px, py, pw, ph) = plus_rect;
        let center_x = px as f32 + pw as f32 * 0.5;
        let center_y = py as f32 + ph as f32 * 0.5;
        let half = (pw.min(ph) as f32 * 0.25).clamp(2.5, 5.0);
        let thickness = (1.25 * self.metrics.ui_scale as f32).clamp(1.15, 2.2);
        self.push_line(center_x - half, center_y, center_x + half, center_y, thickness, plus_fg, 1.0);
        self.push_line(center_x, center_y - half, center_x, center_y + half, thickness, plus_fg, 1.0);

        // Window control buttons (non-macOS).
        #[cfg(not(target_os = "macos"))]
        self.draw_window_buttons_commands(bw, mouse_pos);

        // Bottom separator line.
        let sep_y = tab_bar_h - 1.0;
        self.push_rect(0.0, sep_y, bw as f32, 1.0, TAB_BORDER, 0.7);
    }

    fn draw_tab_drag_overlay(
        &mut self,
        _buffer: &mut [u32],
        buf_width: usize,
        _buf_height: usize,
        tabs: &[TabInfo],
        source_index: usize,
        current_x: f64,
        indicator_x: f32,
    ) {
        let tab_count = tabs.len();
        if source_index >= tab_count {
            return;
        }
        let tw = self.tab_width_val(tab_count, buf_width as u32);
        let tab_bar_h = self.metrics.tab_bar_height_px() as f32;

        // Ghost tab: rounded rect + shadow + subtle border.
        let ghost_x = (current_x - tw as f64 / 2.0).round() as f32;
        let ghost_y = self.metrics.scaled_px(2) as f32;
        let ghost_h = tab_bar_h - self.metrics.scaled_px(4) as f32;
        let ghost_radius = self.metrics.scaled_px(6) as f32;

        // Shadow.
        self.push_rounded_rect(ghost_x + 2.0, ghost_y + 2.0, tw as f32, ghost_h, ghost_radius, 0x000000, 0.24);
        // Body.
        self.push_rounded_rect(ghost_x, ghost_y, tw as f32, ghost_h, ghost_radius, ACTIVE_TAB_BG, 0.86);
        // Border.
        self.push_rounded_rect(ghost_x, ghost_y, tw as f32, ghost_h, ghost_radius, TAB_BORDER, 0.39);

        // Ghost title.
        let text_y = (self.metrics.tab_bar_height_px().saturating_sub(self.metrics.cell_height)) / 2
            + self.metrics.scaled_px(1);
        let use_numbers = self.should_show_number(tw);
        let label: String = if use_numbers {
            (source_index + 1).to_string()
        } else {
            let pad = self.metrics.scaled_px(14);
            let max = (tw.saturating_sub(pad * 2) / self.metrics.cell_width) as usize;
            tabs[source_index].title.chars().take(max).collect()
        };
        let lw = label.chars().count() as u32 * self.metrics.cell_width;
        let tx = ghost_x + ((tw as i32 - lw as i32) / 2).max(4) as f32;
        self.push_text(tx, text_y as f32, &label, TAB_TEXT_ACTIVE, 1.0);

        // Smooth insertion indicator at lerped position.
        let indicator_pad = self.metrics.scaled_px(4) as f32;
        self.push_rect(
            indicator_x,
            indicator_pad,
            self.metrics.scaled_px(2) as f32,
            tab_bar_h - indicator_pad * 2.0,
            INSERTION_COLOR,
            1.0,
        );
    }

    fn draw_tab_tooltip(
        &mut self,
        _buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        mouse_pos: (f64, f64),
        title: &str,
    ) {
        if title.is_empty() || buf_width == 0 || buf_height == 0 {
            return;
        }

        let padding_x = self.metrics.scaled_px(6) as f32;
        let padding_y = self.metrics.scaled_px(4) as f32;
        let content_chars = title.chars().count() as f32;
        let width = (content_chars * self.metrics.cell_width as f32 + padding_x * 2.0 + self.metrics.scaled_px(2) as f32)
            .min(buf_width as f32 - 4.0);
        let height = self.metrics.cell_height as f32 + padding_y * 2.0 + self.metrics.scaled_px(2) as f32;

        let mut x = mouse_pos.0 as f32 + self.metrics.scaled_px(10) as f32;
        let mut y = self.metrics.tab_bar_height_px() as f32 + self.metrics.scaled_px(6) as f32;
        x = x.min(buf_width as f32 - width - 2.0).max(2.0);
        y = y.min(buf_height as f32 - height - 2.0).max(2.0);

        let radius = self.metrics.scaled_px(6) as f32;
        self.push_rounded_rect(x, y, width, height, radius, ACTIVE_TAB_BG, 0.96);
        self.push_rounded_rect(x, y, width, height, radius, TAB_BORDER, 0.31);

        let text_x = x + self.metrics.scaled_px(1) as f32 + padding_x;
        let text_y = y + self.metrics.scaled_px(1) as f32 + padding_y;
        let max_chars =
            ((width - self.metrics.scaled_px(2) as f32 - padding_x * 2.0) / self.metrics.cell_width as f32)
                as usize;
        let display: String = title.chars().take(max_chars).collect();
        self.push_text(text_x, text_y, &display, TAB_TEXT_ACTIVE, 1.0);
    }

    fn tab_hover_tooltip<'a>(
        &self,
        tabs: &'a [TabInfo<'a>],
        hovered_tab: Option<usize>,
        buf_width: u32,
    ) -> Option<&'a str> {
        let idx = hovered_tab?;
        let tab = tabs.get(idx)?;
        if tab.is_renaming || tab.title.is_empty() {
            return None;
        }

        let tw = self.tab_width_val(tabs.len(), buf_width);
        if self.should_show_number(tw) {
            return Some(tab.title);
        }

        let tab_padding_h = self.metrics.scaled_px(14);
        let show_close = tab.is_active || hovered_tab == Some(idx);
        let close_reserved = if show_close {
            self.metrics.scaled_px(20) + self.metrics.scaled_px(6)
        } else {
            0
        };
        let security_reserved = if tab.security_count > 0 {
            let count_chars = tab.security_count.min(99).to_string().len() as u32;
            let count_width = if tab.security_count > 1 {
                count_chars * self.metrics.cell_width + self.metrics.scaled_px(2)
            } else {
                0
            };
            let badge_min = self.metrics.scaled_px(10);
            let badge_max = self.metrics.scaled_px(15);
            self.metrics
                .cell_height
                .saturating_sub(self.metrics.scaled_px(10))
                .clamp(badge_min, badge_max)
                + count_width
                + self.metrics.scaled_px(6)
        } else {
            0
        };
        let max_chars =
            (tw.saturating_sub(tab_padding_h * 2 + close_reserved + security_reserved)
                / self.metrics.cell_width) as usize;
        let title_chars = tab.title.chars().count();
        (title_chars > max_chars).then_some(tab.title)
    }

    fn tab_insert_index_from_x(&self, x: f64, tab_count: usize, buf_width: u32) -> usize {
        let tw = self.tab_width_val(tab_count, buf_width);
        let start = self.tab_strip_start_x_val() as f64;
        let mut idx = tab_count;
        for i in 0..tab_count {
            let center = start + i as f64 * tw as f64 + tw as f64 / 2.0;
            if x < center {
                idx = i;
                break;
            }
        }
        idx
    }

    fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32 {
        self.tab_width_val(tab_count, buf_width)
    }

    fn tab_strip_start_x(&self) -> u32 {
        self.tab_strip_start_x_val()
    }

    fn tab_origin_x(&self, tab_index: usize, tw: u32) -> u32 {
        self.tab_origin_x_val(tab_index, tw)
    }

    // ── Hit testing ───────────────────────────────────────────────────

    fn hit_test_tab_bar(&self, x: f64, y: f64, tab_count: usize, buf_width: u32) -> TabBarHit {
        if y >= self.metrics.tab_bar_height_px() as f64 {
            return TabBarHit::Empty;
        }

        #[cfg(not(target_os = "macos"))]
        if let Some(btn) = self.window_button_at_position(x, y, buf_width) {
            return TabBarHit::WindowButton(btn);
        }

        let tw = self.tab_width_val(tab_count, buf_width);
        let tab_strip_start = self.tab_strip_start_x_val();

        let (px, py, pw, ph) = self.plus_button_rect(tab_count, tw);
        if x >= px as f64 && x < (px + pw) as f64 && y >= py as f64 && y < (py + ph) as f64 {
            return TabBarHit::NewTab;
        }

        if x < tab_strip_start as f64 {
            return TabBarHit::Empty;
        }

        let rel_x = x as u32 - tab_strip_start;
        let tab_index = rel_x / tw;
        if (tab_index as usize) < tab_count {
            let idx = tab_index as usize;
            let (cx, cy, cw, ch) = self.close_button_rect(idx, tw);
            if x >= cx as f64
                && x < (cx + cw) as f64
                && y >= cy as f64
                && y < (cy + ch) as f64
            {
                return TabBarHit::CloseTab(idx);
            }
            return TabBarHit::Tab(idx);
        }

        TabBarHit::Empty
    }

    fn hit_test_tab_hover(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> Option<usize> {
        if y >= self.metrics.tab_bar_height_px() as f64 || tab_count == 0 {
            return None;
        }
        let tw = self.tab_width_val(tab_count, buf_width);
        let tab_strip_start = self.tab_strip_start_x_val();
        if x < tab_strip_start as f64 {
            return None;
        }
        let rel_x = x as u32 - tab_strip_start;
        let idx = rel_x / tw;
        if (idx as usize) < tab_count {
            Some(idx as usize)
        } else {
            None
        }
    }

    fn hit_test_tab_security_badge(
        &self,
        x: f64,
        y: f64,
        tabs: &[TabInfo],
        buf_width: u32,
    ) -> Option<usize> {
        for (idx, tab) in tabs.iter().enumerate() {
            if tab.security_count == 0 {
                continue;
            }
            let Some((sx, sy, sw, sh)) =
                self.security_badge_rect_val(idx, tabs.len(), buf_width, tab.security_count)
            else {
                continue;
            };
            if x >= sx as f64
                && x < (sx + sw) as f64
                && y >= sy as f64
                && y < (sy + sh) as f64
            {
                return Some(idx);
            }
        }
        None
    }

    #[cfg(not(target_os = "macos"))]
    fn window_button_at_position(&self, x: f64, y: f64, buf_width: u32) -> Option<WindowButton> {
        let bar_h = self.metrics.tab_bar_height_px();
        if y >= bar_h as f64 {
            return None;
        }
        let btn_w = self.metrics.scaled_px(WIN_BTN_WIDTH);
        let close_x = buf_width.saturating_sub(btn_w);
        let min_x = buf_width.saturating_sub(btn_w * 2);
        let minimize_x = buf_width.saturating_sub(btn_w * 3);

        if x >= close_x as f64 && x < buf_width as f64 {
            Some(WindowButton::Close)
        } else if x >= min_x as f64 && x < (min_x + btn_w) as f64 {
            Some(WindowButton::Maximize)
        } else if x >= minimize_x as f64 && x < (minimize_x + btn_w) as f64 {
            Some(WindowButton::Minimize)
        } else {
            None
        }
    }

    // ── Context menu ──────────────────────────────────────────────────

    fn draw_context_menu(
        &mut self,
        _buffer: &mut [u32],
        _buf_width: usize,
        _buf_height: usize,
        menu: &ContextMenu,
    ) {
        let mw = menu.width(self.metrics.cell_width) as f32;
        let ih = menu.item_height(self.metrics.cell_height) as f32;
        let mh = menu.height(self.metrics.cell_height) as f32;
        let mx = menu.x as f32;
        let my = menu.y as f32;
        let radius = self.metrics.scaled_px(6) as f32;

        // Background.
        self.push_rounded_rect(mx, my, mw, mh, radius, 0x1E2433, 0.97);
        self.push_rounded_rect(mx, my, mw, mh, radius, 0xFFFFFF, 0.08);

        for (i, (action, label)) in menu.items.iter().enumerate() {
            let item_y = my + self.metrics.scaled_px(2) as f32 + i as f32 * ih;

            if menu.hover_index == Some(i) {
                let hover_x = mx + self.metrics.scaled_px(4) as f32;
                let hover_w = mw - self.metrics.scaled_px(8) as f32;
                let hover_h = ih - self.metrics.scaled_px(1) as f32;
                self.push_rounded_rect(hover_x, item_y, hover_w, hover_h, radius, 0x31394D, 0.86);
            }

            let fg = if *action == ContextAction::Close {
                0xF38BA8
            } else {
                Color::DEFAULT_FG.to_pixel()
            };

            let text_x = mx + self.metrics.cell_width as f32;
            let text_y = item_y + self.metrics.scaled_px(2) as f32;
            self.push_text(text_x, text_y, label, fg, 1.0);
        }
    }

    fn hit_test_context_menu(&self, menu: &ContextMenu, x: f64, y: f64) -> Option<usize> {
        let mw = menu.width(self.metrics.cell_width);
        let ih = menu.item_height(self.metrics.cell_height);
        let mh = menu.height(self.metrics.cell_height);

        if x < menu.x as f64
            || x >= (menu.x + mw) as f64
            || y < menu.y as f64
            || y >= (menu.y + mh) as f64
        {
            return None;
        }

        let rel_y = (y - menu.y as f64 - 2.0) as u32;
        let idx = rel_y / ih;
        if (idx as usize) < menu.items.len() {
            Some(idx as usize)
        } else {
            None
        }
    }

    // ── Security ──────────────────────────────────────────────────────

    fn draw_security_popup(
        &mut self,
        _buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        popup: &SecurityPopup,
    ) {
        let pw = popup.width(self.metrics.cell_width);
        let ph = popup.height(self.metrics.cell_height);
        let width = pw.min(buf_width as u32) as f32;
        let height = ph.min(buf_height as u32) as f32;
        let x = popup.x.min((buf_width as u32).saturating_sub(pw)) as f32;
        let y = popup.y.min((buf_height as u32).saturating_sub(ph)) as f32;
        let radius = self.metrics.scaled_px(6) as f32;

        self.push_rounded_rect(x, y, width, height, radius, 0x1E2433, 0.97);
        self.push_rounded_rect(x, y, width, height, radius, 0xFFFFFF, 0.08);

        // Title.
        let header_y = y + self.metrics.scaled_px(2) as f32;
        let header_x = x + self.metrics.cell_width as f32 / 2.0;
        self.push_text(
            header_x,
            header_y,
            popup.title,
            super::SECURITY_ACCENT.to_pixel(),
            1.0,
        );

        // Separator line.
        let line_h = popup.line_height(self.metrics.cell_height) as f32;
        let sep_y = y + line_h;
        self.push_rect(
            x + self.metrics.scaled_px(3) as f32,
            sep_y,
            width - self.metrics.scaled_px(6) as f32,
            1.0,
            super::SECURITY_ACCENT.to_pixel(),
            0.47,
        );

        // Content lines.
        for (line_idx, line) in popup.lines.iter().enumerate() {
            let text_y = y + line_h + self.metrics.scaled_px(4) as f32 + line_idx as f32 * line_h;
            let text_x = x + self.metrics.cell_width as f32 / 2.0;
            let full_line = format!("\u{2022} {}", line);
            self.push_text(text_x, text_y, &full_line, Color::DEFAULT_FG.to_pixel(), 1.0);
        }
    }

    fn hit_test_security_popup(
        &self,
        popup: &SecurityPopup,
        x: f64,
        y: f64,
        buf_width: usize,
        buf_height: usize,
    ) -> bool {
        let pw = popup.width(self.metrics.cell_width);
        let ph = popup.height(self.metrics.cell_height);
        let width = pw.min(buf_width as u32);
        let height = ph.min(buf_height as u32);
        let px = popup.x.min((buf_width as u32).saturating_sub(pw));
        let py = popup.y.min((buf_height as u32).saturating_sub(ph));
        x >= px as f64 && x < (px + width) as f64 && y >= py as f64 && y < (py + height) as f64
    }

    fn security_badge_rect(
        &self,
        tab_index: usize,
        tab_count: usize,
        buf_width: u32,
        security_count: usize,
    ) -> Option<(u32, u32, u32, u32)> {
        self.security_badge_rect_val(tab_index, tab_count, buf_width, security_count)
    }
}

// ── Internal helpers for UI command generation ───────────────────────

impl GpuRenderer {
    fn draw_close_button_commands(&mut self, tab_index: usize, tw: u32, mouse_pos: (f64, f64)) {
        let (cx, cy, cw, ch) = self.close_button_rect(tab_index, tw);
        let is_close_hovered = mouse_pos.0 >= cx as f64
            && mouse_pos.0 < (cx + cw) as f64
            && mouse_pos.1 >= cy as f64
            && mouse_pos.1 < (cy + ch) as f64
            && mouse_pos.1 < self.metrics.tab_bar_height_px() as f64;

        if is_close_hovered {
            let circle_r = cw.min(ch) as f32 / 2.0;
            let circle_cx = cx as f32 + cw as f32 / 2.0;
            let circle_cy = cy as f32 + ch as f32 / 2.0;
            self.push_circle(circle_cx, circle_cy, circle_r, CLOSE_HOVER_BG_COLOR, 1.0);
        }

        // X icon.
        let center_x = cx as f32 + cw as f32 * 0.5;
        let center_y = cy as f32 + ch as f32 * 0.5;
        let half = (cw.min(ch) as f32 * 0.22).clamp(2.5, 4.5);
        let thickness = (1.25 * self.metrics.ui_scale as f32).clamp(1.15, 2.2);
        self.push_line(
            center_x - half,
            center_y - half,
            center_x + half,
            center_y + half,
            thickness,
            TAB_TEXT_INACTIVE,
            1.0,
        );
        self.push_line(
            center_x + half,
            center_y - half,
            center_x - half,
            center_y + half,
            thickness,
            TAB_TEXT_INACTIVE,
            1.0,
        );
    }

    #[cfg(not(target_os = "macos"))]
    fn draw_window_buttons_commands(&mut self, buf_width: u32, mouse_pos: (f64, f64)) {
        let bar_h = self.metrics.tab_bar_height_px() as f32;
        let btn_w = self.metrics.scaled_px(WIN_BTN_WIDTH);
        let bw = buf_width;

        let buttons: [(u32, WindowButton); 3] = [
            (bw.saturating_sub(btn_w * 3), WindowButton::Minimize),
            (bw.saturating_sub(btn_w * 2), WindowButton::Maximize),
            (bw.saturating_sub(btn_w), WindowButton::Close),
        ];

        for &(btn_x, ref btn_type) in &buttons {
            let is_hovered = mouse_pos.0 >= btn_x as f64
                && mouse_pos.0 < (btn_x + btn_w) as f64
                && mouse_pos.1 >= 0.0
                && mouse_pos.1 < bar_h as f64;

            if is_hovered {
                let hover_bg = if *btn_type == WindowButton::Close {
                    0xF38BA8
                } else {
                    0x313244
                };
                self.push_rect(btn_x as f32, 0.0, btn_w as f32, bar_h, hover_bg, 1.0);
            }

            let icon_color = if is_hovered && *btn_type == WindowButton::Close {
                0xFFFFFF
            } else {
                0x6C7086
            };

            let center_x = btn_x as f32 + btn_w as f32 / 2.0;
            let center_y = bar_h / 2.0;
            let thickness = (1.25 * self.metrics.ui_scale as f32).clamp(1.15, 2.2);

            match btn_type {
                WindowButton::Minimize => {
                    let half_w = self.metrics.scaled_px(5) as f32;
                    self.push_line(
                        center_x - half_w,
                        center_y,
                        center_x + half_w,
                        center_y,
                        thickness,
                        icon_color,
                        1.0,
                    );
                }
                WindowButton::Maximize => {
                    let half = self.metrics.scaled_px(5) as f32;
                    let x0 = center_x - half;
                    let y0 = center_y - half;
                    let x1 = center_x + half;
                    let y1 = center_y + half;
                    self.push_line(x0, y0, x1, y0, thickness, icon_color, 1.0);
                    self.push_line(x0, y1, x1, y1, thickness, icon_color, 1.0);
                    self.push_line(x0, y0, x0, y1, thickness, icon_color, 1.0);
                    self.push_line(x1, y0, x1, y1, thickness, icon_color, 1.0);
                }
                WindowButton::Close => {
                    let half = self.metrics.scaled_px(5) as f32 * 0.7;
                    self.push_line(
                        center_x - half,
                        center_y - half,
                        center_x + half,
                        center_y + half,
                        thickness,
                        icon_color,
                        1.0,
                    );
                    self.push_line(
                        center_x + half,
                        center_y - half,
                        center_x - half,
                        center_y + half,
                        thickness,
                        icon_color,
                        1.0,
                    );
                }
            }
        }
    }

    /// Encodes all GPU passes and presents the frame.
    /// Called as the last step in the frame after all draw_* methods.
    pub fn present_frame(&mut self) {
        // Upload grid data if dirty.
        if self.grid_dirty && !self.grid_cells.is_empty() {
            // Ensure cell buffer is large enough.
            let needed = self.grid_cells.len() * std::mem::size_of::<PackedCell>();
            if needed as u64 > self.grid_cell_buffer.size() {
                self.grid_cell_buffer =
                    Self::create_storage_buffer(&self.device, needed, "grid_cells");
            }
            self.queue.write_buffer(
                &self.grid_cell_buffer,
                0,
                bytemuck::cast_slice(&self.grid_cells),
            );
            self.queue.write_buffer(
                &self.grid_uniform_buffer,
                0,
                bytemuck::bytes_of(&self.grid_uniforms),
            );

            // Rebuild glyph info buffer in case new glyphs were added.
            let glyph_data = self.atlas.glyph_info_buffer_data();
            let glyph_bytes = bytemuck::cast_slice(&glyph_data);
            if glyph_bytes.len() as u64 > self.glyph_info_buffer.size() {
                self.glyph_info_buffer = Self::create_storage_buffer_init(
                    &self.device,
                    glyph_bytes,
                    "glyph_info",
                );
            } else {
                self.queue
                    .write_buffer(&self.glyph_info_buffer, 0, glyph_bytes);
            }
        }

        // Upload UI commands.
        let command_count = self.commands.len().min(MAX_UI_COMMANDS);
        let ui_uniforms = UiUniforms {
            width: self.width as f32,
            height: self.height as f32,
            atlas_width: self.atlas.atlas_width as f32,
            atlas_height: self.atlas.atlas_height as f32,
            command_count: command_count as u32,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };
        self.queue
            .write_buffer(&self.ui_uniform_buffer, 0, bytemuck::bytes_of(&ui_uniforms));

        if command_count > 0 {
            let cmd_bytes = bytemuck::cast_slice(&self.commands[..command_count]);
            if cmd_bytes.len() as u64 > self.ui_command_buffer.size() {
                self.ui_command_buffer = Self::create_storage_buffer(
                    &self.device,
                    cmd_bytes.len(),
                    "ui_commands",
                );
            }
            self.queue
                .write_buffer(&self.ui_command_buffer, 0, cmd_bytes);
        }

        // Composite uniforms.
        let grid_pixel_w = self.grid_uniforms.cols * self.grid_uniforms.cell_width;
        let grid_pixel_h = self.grid_uniforms.rows * self.grid_uniforms.cell_height;
        let composite_uniforms = CompositeUniforms {
            tab_bar_height: self.metrics.tab_bar_height_px() as f32,
            window_height: self.height as f32,
            window_width: self.width as f32,
            window_padding: self.metrics.window_padding_px() as f32,
            grid_pixel_width: grid_pixel_w as f32,
            grid_pixel_height: grid_pixel_h as f32,
            bg_color: Color::DEFAULT_BG.to_pixel(),
            _padding: 0,
        };
        self.queue.write_buffer(
            &self.composite_uniform_buffer,
            0,
            bytemuck::bytes_of(&composite_uniforms),
        );

        // Get surface texture.
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => {
                // Reconfigure and retry once.
                self.surface.configure(&self.device, &self.surface_config);
                match self.surface.get_current_texture() {
                    Ok(t) => t,
                    Err(_) => {
                        self.commands.clear();
                        self.grid_dirty = false;
                        return;
                    }
                }
            }
        };
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });

        // Pass 1: Grid compute.
        if self.grid_dirty && !self.grid_cells.is_empty() {
            let grid_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("grid_bind_group"),
                layout: &self.grid_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.grid_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.grid_cell_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.glyph_info_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.atlas.texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&self.grid_texture_view),
                    },
                ],
            });

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("grid_compute_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.grid_pipeline);
            compute_pass.set_bind_group(0, &grid_bind_group, &[]);

            // Dispatch enough workgroups to cover the entire texture so the
            // compute shader fills out-of-grid pixels with the background color.
            let wg_x = (self.width + 15) / 16;
            let wg_y = (self.height + 15) / 16;
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // Pass 2: UI render.
        {
            let ui_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ui_bind_group"),
                layout: &self.ui_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.ui_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.ui_command_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.atlas.texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.ui_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(&self.ui_pipeline);
            render_pass.set_bind_group(0, &ui_bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Fullscreen triangle.
        }

        // Pass 3: Composite.
        {
            let composite_bind_group =
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("composite_bind_group"),
                    layout: &self.composite_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&self.grid_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&self.ui_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: self.composite_uniform_buffer.as_entire_binding(),
                        },
                    ],
                });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composite_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(&self.composite_pipeline);
            render_pass.set_bind_group(0, &composite_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Reset per-frame state.
        self.commands.clear();
        self.grid_dirty = false;
    }

    /// Resizes the surface and internal textures.
    pub fn resize(&mut self, width: u32, height: u32) {
        let w = width.max(1);
        let h = height.max(1);
        if w == self.width && h == self.height {
            return;
        }
        self.width = w;
        self.height = h;
        self.surface_config.width = w;
        self.surface_config.height = h;
        self.surface.configure(&self.device, &self.surface_config);
        self.resize_textures();
    }
}
