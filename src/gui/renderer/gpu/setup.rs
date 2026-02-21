//! GPU renderer initialization and resource management helpers.

use std::sync::Arc;

use fontdue::{Font, FontSettings};
use wgpu;
use winit::window::Window;

use crate::config::AppConfig;
use super::super::metrics::FontMetrics;
use super::atlas::GlyphAtlas;
use super::buffers::*;
use super::pipelines;
use super::MAX_UI_COMMANDS;

impl super::GpuRenderer {
    /// Creates a new GPU renderer, initializing wgpu device, pipelines, and textures.
    pub fn new(window: Arc<Window>, config: &AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
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

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("ferrum_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: Default::default(),
                experimental_features: Default::default(),
            }))?;

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
        let font_data = crate::config::font_data(config.font.family);
        let font =
            Font::from_bytes(font_data, FontSettings::default()).expect("font load fail");

        let fallback_data = crate::config::fallback_font_data();
        let fallback_font =
            Font::from_bytes(fallback_data, FontSettings::default()).expect("fallback font load fail");

        let mut metrics = FontMetrics {
            cell_width: 1,
            cell_height: 1,
            font_size: config.font.size,
            ui_scale: 1.0,
            ascent: 0,
            tab_bar_visible: false,
            base_font_size: config.font.size,
            base_line_padding: config.font.line_padding,
            base_tab_bar_height: config.layout.tab_bar_height,
            base_window_padding: config.layout.window_padding,
            base_scrollbar_width: config.layout.scrollbar_width,
            base_pane_inner_padding: config.layout.pane_inner_padding,
        };
        metrics.recompute(&font);

        let palette = config.theme.resolve();

        // Create glyph atlas.
        let atlas = GlyphAtlas::new(&device, &queue, &font, &fallback_font, metrics.font_size, metrics.ascent);

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

        let ui_uniform_buffer =
            Self::create_uniform_buffer(&device, std::mem::size_of::<UiUniforms>(), "ui_uniforms");
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

        Ok(super::GpuRenderer {
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
            fallback_font,
            metrics,
            palette,
            commands: Vec::with_capacity(MAX_UI_COMMANDS),
            grid_batches: Vec::new(),
            grid_dirty: false,
            width,
            height,
        })
    }

    // ── Texture / buffer helpers ──────────────────────────────────────

    pub(super) fn create_offscreen_texture(
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

    pub(super) fn create_uniform_buffer(
        device: &wgpu::Device,
        size: usize,
        label: &str,
    ) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub(super) fn create_storage_buffer(
        device: &wgpu::Device,
        size: usize,
        label: &str,
    ) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub(super) fn create_storage_buffer_init(
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
    pub(super) fn resize_textures(&mut self) {
        let (gt, gtv) = Self::create_offscreen_texture(
            &self.device,
            self.width,
            self.height,
            "grid_texture",
            true,
        );
        self.grid_texture = gt;
        self.grid_texture_view = gtv;

        let (ut, utv) = Self::create_offscreen_texture(
            &self.device,
            self.width,
            self.height,
            "ui_texture",
            false,
        );
        self.ui_texture = ut;
        self.ui_texture_view = utv;
    }

    /// Applies config changes (font, metrics, atlas, palette).
    pub(in crate::gui::renderer) fn apply_config(&mut self, config: &crate::config::AppConfig) {
        let font_data = crate::config::font_data(config.font.family);
        self.font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())
            .expect("font load failed");
        self.metrics.update_bases(config);
        self.metrics.recompute(&self.font);
        self.rebuild_atlas();
        self.palette = config.theme.resolve();
    }

    /// Rebuilds glyph atlas and related buffer after scale change.
    pub(super) fn rebuild_atlas(&mut self) {
        self.atlas = GlyphAtlas::new(
            &self.device,
            &self.queue,
            &self.font,
            &self.fallback_font,
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
}
