//! Pipeline creation helpers for the three GPU render passes.

use wgpu;

// ── Low-level bind group layout entry constructors ──────────────────────────

/// A uniform buffer entry (vertex + fragment visibility).
fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// A read-only storage buffer entry (fragment visibility).
fn storage_ro_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// A 2D float texture entry (fragment visibility).
fn texture_2d_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        },
        count: None,
    }
}

/// A filtering sampler entry (fragment visibility).
fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

// ── Shared pipeline helpers ──────────────────────────────────────────────────

fn create_shader(device: &wgpu::Device, label: &str, src: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(src.into()),
    })
}

/// Creates a render pipeline with the standard fullscreen-triangle vertex stage.
///
/// - `label` — pipeline label (also used to derive layout/fragment labels)
/// - `shader` — compiled shader module (must export `vs_main` and `fs_main`)
/// - `layout` — pre-built pipeline layout
/// - `target_format` — surface / offscreen texture format
/// - `blend` — `None` for opaque, `Some(BlendState)` for alpha-compositing
fn make_render_pipeline(
    device: &wgpu::Device,
    label: &str,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    target_format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

// ── Public pipeline constructors ─────────────────────────────────────────────

/// Creates the grid render pipeline and its bind group layout.
///
/// Bindings:
///   0: GridUniforms (uniform)
///   1: cells array  (storage, read-only)
///   2: glyphs array (storage, read-only)
///   3: atlas texture (texture_2d<f32>)
///   4: atlas sampler
pub fn create_grid_pipeline(
    device: &wgpu::Device,
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    let shader = create_shader(device, "grid_shader", include_str!("shaders/grid.wgsl"));

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("grid_bind_group_layout"),
        entries: &[
            uniform_entry(0),    // GridUniforms
            storage_ro_entry(1), // cells
            storage_ro_entry(2), // glyphs
            texture_2d_entry(3), // atlas texture
            sampler_entry(4),    // atlas sampler
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("grid_pipeline_layout"),
        bind_group_layouts: &[&bind_group_layout],
        immediate_size: 0,
    });

    let pipeline = make_render_pipeline(
        device,
        "grid_render_pipeline",
        &shader,
        &pipeline_layout,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
    );

    (pipeline, bind_group_layout)
}

/// Creates the UI render pipeline and its bind group layout.
///
/// Bindings:
///   0: UiUniforms (uniform)
///   1: commands array (storage, read-only)
///   2: atlas texture (texture_2d<f32>)
///   3: atlas sampler
pub fn create_ui_pipeline(device: &wgpu::Device) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    let shader = create_shader(device, "ui_shader", include_str!("shaders/ui.wgsl"));

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ui_bind_group_layout"),
        entries: &[
            uniform_entry(0),    // UiUniforms
            storage_ro_entry(1), // commands
            texture_2d_entry(2), // atlas texture
            sampler_entry(3),    // atlas sampler
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("ui_pipeline_layout"),
        bind_group_layouts: &[&bind_group_layout],
        immediate_size: 0,
    });

    let pipeline = make_render_pipeline(
        device,
        "ui_render_pipeline",
        &shader,
        &pipeline_layout,
        wgpu::TextureFormat::Rgba8Unorm,
        Some(wgpu::BlendState::ALPHA_BLENDING),
    );

    (pipeline, bind_group_layout)
}

/// Creates the composite render pipeline and its bind group layout.
///
/// Bindings:
///   0: grid_texture (texture_2d<f32>)
///   1: ui_texture (texture_2d<f32>)
///   2: tex_sampler
///   3: CompositeUniforms (uniform)
pub fn create_composite_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
    let shader = create_shader(device, "composite_shader", include_str!("shaders/composite.wgsl"));

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("composite_bind_group_layout"),
        entries: &[
            texture_2d_entry(0), // grid_texture
            texture_2d_entry(1), // ui_texture
            sampler_entry(2),    // tex_sampler
            uniform_entry(3),    // CompositeUniforms
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("composite_pipeline_layout"),
        bind_group_layouts: &[&bind_group_layout],
        immediate_size: 0,
    });

    let pipeline = make_render_pipeline(
        device,
        "composite_render_pipeline",
        &shader,
        &pipeline_layout,
        surface_format,
        None,
    );

    (pipeline, bind_group_layout)
}
