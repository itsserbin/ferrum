use std::sync::Arc;

use softbuffer::Surface;
use winit::window::Window;

use crate::config::AppConfig;
use super::traits::Renderer;
#[cfg(not(target_os = "macos"))]
use super::TabInfo;
use super::{CpuRenderer, TabBarHit};

#[cfg(feature = "gpu")]
use super::gpu::GpuRenderer;

/// Enum-dispatch renderer backend.
///
/// Bundles the softbuffer surface with CpuRenderer (only CPU needs it).
/// GPU renderer owns its own wgpu surface internally.
pub enum RendererBackend {
    Cpu {
        renderer: Box<CpuRenderer>,
        surface: Box<Surface<winit::event_loop::OwnedDisplayHandle, Arc<Window>>>,
    },
    #[cfg(feature = "gpu")]
    Gpu(Box<GpuRenderer>),
}

impl RendererBackend {
    /// Creates a new renderer backend, attempting GPU first (if enabled) then falling back to CPU.
    pub fn new(
        window: Arc<Window>,
        context: &softbuffer::Context<winit::event_loop::OwnedDisplayHandle>,
        config: &AppConfig,
    ) -> Self {
        #[cfg(feature = "gpu")]
        {
            match GpuRenderer::new(window.clone(), config) {
                Ok(gpu) => {
                    eprintln!("[ferrum] Using GPU renderer (wgpu)");
                    return RendererBackend::Gpu(Box::new(gpu));
                }
                Err(e) => {
                    eprintln!("[ferrum] GPU renderer failed: {e}, falling back to CPU");
                }
            }
        }

        let surface = Box::new(Surface::new(context, window.clone()).expect("softbuffer surface"));
        let renderer = Box::new(CpuRenderer::new(config));
        eprintln!("[ferrum] Using CPU renderer (softbuffer)");
        RendererBackend::Cpu { renderer, surface }
    }

    // ── Trait-object helpers ──────────────────────────────────────────

    /// Returns a shared reference to the inner renderer as a trait object.
    fn as_renderer(&self) -> &dyn Renderer {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.as_ref(),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => gpu.as_ref(),
        }
    }

    /// Returns a mutable reference to the inner renderer as a trait object.
    fn as_renderer_mut(&mut self) -> &mut dyn Renderer {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.as_mut(),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => gpu.as_mut(),
        }
    }

    // ── Lifecycle ────────────────────────────────────────────────────

    /// Applies a full config change (font, metrics, palette).
    pub fn apply_config(&mut self, config: &AppConfig) {
        match self {
            Self::Cpu { renderer, .. } => renderer.apply_config(config),
            #[cfg(feature = "gpu")]
            Self::Gpu(gpu) => gpu.apply_config(config),
        }
    }

    pub fn set_scale(&mut self, scale_factor: f64) {
        self.as_renderer_mut().set_scale(scale_factor);
    }

    #[cfg(not(target_os = "macos"))]
    pub fn set_tab_bar_visible(&mut self, visible: bool) {
        self.as_renderer_mut().set_tab_bar_visible(visible);
    }

    // ── Metrics ─────────────────────────────────────────────────────

    pub fn cell_width(&self) -> u32 {
        self.as_renderer().cell_width()
    }

    pub fn cell_height(&self) -> u32 {
        self.as_renderer().cell_height()
    }

    pub fn tab_bar_height_px(&self) -> u32 {
        self.as_renderer().tab_bar_height_px()
    }

    pub fn window_padding_px(&self) -> u32 {
        self.as_renderer().window_padding_px()
    }

    pub fn ui_scale(&self) -> f64 {
        self.as_renderer().ui_scale()
    }

    pub fn scaled_px(&self, base: u32) -> u32 {
        self.as_renderer().scaled_px(base)
    }

    pub fn scrollbar_hit_zone_px(&self) -> u32 {
        self.as_renderer().scrollbar_hit_zone_px()
    }

    pub fn pane_inner_padding_px(&self) -> u32 {
        self.as_renderer().pane_inner_padding_px()
    }

    // ── Scrollbar ─────────────────────────────────────────────────────

    pub fn scrollbar_thumb_bounds(
        &self,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
    ) -> Option<(f32, f32)> {
        self.as_renderer().scrollbar_thumb_bounds(
            buf_height,
            scroll_offset,
            scrollback_len,
            grid_rows,
        )
    }

    // ── Tab bar metrics / hit testing ─────────────────────────────────

    pub fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32 {
        self.as_renderer().tab_width(tab_count, buf_width)
    }

    pub fn tab_origin_x(&self, tab_index: usize, tw: u32) -> u32 {
        self.as_renderer().tab_origin_x(tab_index, tw)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn tab_insert_index_from_x(&self, x: f64, tab_count: usize, buf_width: u32) -> usize {
        self.as_renderer()
            .tab_insert_index_from_x(x, tab_count, buf_width)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn tab_hover_tooltip<'a>(
        &self,
        tabs: &'a [TabInfo<'a>],
        hovered_tab: Option<usize>,
        buf_width: u32,
    ) -> Option<&'a str> {
        self.as_renderer()
            .tab_hover_tooltip(tabs, hovered_tab, buf_width)
    }

    pub fn hit_test_tab_bar(&self, x: f64, y: f64, tab_count: usize, buf_width: u32) -> TabBarHit {
        self.as_renderer()
            .hit_test_tab_bar(x, y, tab_count, buf_width)
    }

    pub fn hit_test_tab_hover(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> Option<usize> {
        self.as_renderer()
            .hit_test_tab_hover(x, y, tab_count, buf_width)
    }

    pub(in crate::gui) fn palette(&self) -> &crate::config::ThemePalette {
        match self {
            Self::Cpu { renderer, .. } => &renderer.palette,
            #[cfg(feature = "gpu")]
            Self::Gpu(gpu) => &gpu.palette,
        }
    }
}
