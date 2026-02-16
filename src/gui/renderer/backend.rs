use std::sync::Arc;

use softbuffer::Surface;
use winit::window::Window;

use super::{ContextMenu, CpuRenderer, SecurityPopup, TabBarHit, TabInfo};

#[cfg(feature = "gpu")]
use super::gpu::GpuRenderer;
#[cfg(feature = "gpu")]
use super::traits::Renderer;

/// Enum-dispatch renderer backend.
///
/// Bundles the softbuffer surface with CpuRenderer (only CPU needs it).
/// GPU renderer owns its own wgpu surface internally.
pub enum RendererBackend {
    Cpu {
        renderer: CpuRenderer,
        surface: Surface<winit::event_loop::OwnedDisplayHandle, Arc<Window>>,
    },
    #[cfg(feature = "gpu")]
    Gpu(GpuRenderer),
}

impl RendererBackend {
    /// Creates a new renderer backend, attempting GPU first (if enabled) then falling back to CPU.
    pub fn new(
        window: Arc<Window>,
        context: &softbuffer::Context<winit::event_loop::OwnedDisplayHandle>,
    ) -> Self {
        #[cfg(feature = "gpu")]
        {
            match GpuRenderer::new(window.clone()) {
                Ok(gpu) => {
                    eprintln!("[ferrum] Using GPU renderer (wgpu)");
                    return RendererBackend::Gpu(gpu);
                }
                Err(e) => {
                    eprintln!("[ferrum] GPU renderer failed: {e}, falling back to CPU");
                }
            }
        }

        let surface = Surface::new(context, window.clone()).expect("softbuffer surface");
        let renderer = CpuRenderer::new();
        eprintln!("[ferrum] Using CPU renderer (softbuffer)");
        RendererBackend::Cpu { renderer, surface }
    }

    // ── Lifecycle ────────────────────────────────────────────────────

    pub fn set_scale(&mut self, scale_factor: f64) {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.set_scale(scale_factor),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::set_scale(gpu, scale_factor),
        }
    }

    pub fn set_tab_bar_visible(&mut self, visible: bool) {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.set_tab_bar_visible(visible),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::set_tab_bar_visible(gpu, visible),
        }
    }

    // ── Metrics ─────────────────────────────────────────────────────

    pub fn cell_width(&self) -> u32 {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.cell_width,
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::cell_width(gpu),
        }
    }

    pub fn cell_height(&self) -> u32 {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.cell_height,
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::cell_height(gpu),
        }
    }

    pub fn tab_bar_height_px(&self) -> u32 {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.tab_bar_height_px(),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::tab_bar_height_px(gpu),
        }
    }

    pub fn window_padding_px(&self) -> u32 {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.window_padding_px(),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::window_padding_px(gpu),
        }
    }

    pub fn ui_scale(&self) -> f64 {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.ui_scale(),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::ui_scale(gpu),
        }
    }

    pub fn scaled_px(&self, base: u32) -> u32 {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.scaled_px(base),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::scaled_px(gpu, base),
        }
    }

    pub fn scrollbar_hit_zone_px(&self) -> u32 {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.scrollbar_hit_zone_px(),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::scrollbar_hit_zone_px(gpu),
        }
    }

    // ── Scrollbar ─────────────────────────────────────────────────────

    pub fn scrollbar_thumb_bounds(
        &self,
        buf_height: usize,
        scroll_offset: usize,
        scrollback_len: usize,
        grid_rows: usize,
    ) -> Option<(f32, f32)> {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.scrollbar_thumb_bounds(
                buf_height,
                scroll_offset,
                scrollback_len,
                grid_rows,
            ),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::scrollbar_thumb_bounds(
                gpu,
                buf_height,
                scroll_offset,
                scrollback_len,
                grid_rows,
            ),
        }
    }

    // ── Tab bar metrics / hit testing ─────────────────────────────────

    pub fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32 {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.tab_width(tab_count, buf_width),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::tab_width(gpu, tab_count, buf_width),
        }
    }

    pub fn tab_origin_x(&self, tab_index: usize, tw: u32) -> u32 {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.tab_origin_x(tab_index, tw),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::tab_origin_x(gpu, tab_index, tw),
        }
    }

    pub fn tab_insert_index_from_x(&self, x: f64, tab_count: usize, buf_width: u32) -> usize {
        match self {
            RendererBackend::Cpu { renderer, .. } => {
                renderer.tab_insert_index_from_x(x, tab_count, buf_width)
            }
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => {
                Renderer::tab_insert_index_from_x(gpu, x, tab_count, buf_width)
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn tab_hover_tooltip<'a>(
        &self,
        tabs: &'a [TabInfo<'a>],
        hovered_tab: Option<usize>,
        buf_width: u32,
    ) -> Option<&'a str> {
        match self {
            RendererBackend::Cpu { renderer, .. } => {
                renderer.tab_hover_tooltip(tabs, hovered_tab, buf_width)
            }
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => {
                Renderer::tab_hover_tooltip(gpu, tabs, hovered_tab, buf_width)
            }
        }
    }

    pub fn hit_test_tab_bar(&self, x: f64, y: f64, tab_count: usize, buf_width: u32) -> TabBarHit {
        match self {
            RendererBackend::Cpu { renderer, .. } => {
                renderer.hit_test_tab_bar(x, y, tab_count, buf_width)
            }
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => {
                Renderer::hit_test_tab_bar(gpu, x, y, tab_count, buf_width)
            }
        }
    }

    pub fn hit_test_tab_hover(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> Option<usize> {
        match self {
            RendererBackend::Cpu { renderer, .. } => {
                renderer.hit_test_tab_hover(x, y, tab_count, buf_width)
            }
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => {
                Renderer::hit_test_tab_hover(gpu, x, y, tab_count, buf_width)
            }
        }
    }

    pub fn hit_test_tab_security_badge(
        &self,
        x: f64,
        y: f64,
        tabs: &[TabInfo],
        buf_width: u32,
    ) -> Option<usize> {
        match self {
            RendererBackend::Cpu { renderer, .. } => {
                renderer.hit_test_tab_security_badge(x, y, tabs, buf_width)
            }
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => {
                Renderer::hit_test_tab_security_badge(gpu, x, y, tabs, buf_width)
            }
        }
    }

    pub fn security_badge_rect(
        &self,
        tab_index: usize,
        tab_count: usize,
        buf_width: u32,
        security_count: usize,
    ) -> Option<(u32, u32, u32, u32)> {
        match self {
            RendererBackend::Cpu { renderer, .. } => {
                renderer.security_badge_rect(tab_index, tab_count, buf_width, security_count)
            }
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => {
                Renderer::security_badge_rect(gpu, tab_index, tab_count, buf_width, security_count)
            }
        }
    }

    // ── Context menu ────────────────────────────────────────────────

    pub fn hit_test_context_menu(&self, menu: &ContextMenu, x: f64, y: f64) -> Option<usize> {
        match self {
            RendererBackend::Cpu { renderer, .. } => renderer.hit_test_context_menu(menu, x, y),
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => Renderer::hit_test_context_menu(gpu, menu, x, y),
        }
    }

    // ── Security ────────────────────────────────────────────────────

    pub fn hit_test_security_popup(
        &self,
        popup: &SecurityPopup,
        x: f64,
        y: f64,
        buf_width: usize,
        buf_height: usize,
    ) -> bool {
        match self {
            RendererBackend::Cpu { renderer, .. } => {
                renderer.hit_test_security_popup(popup, x, y, buf_width, buf_height)
            }
            #[cfg(feature = "gpu")]
            RendererBackend::Gpu(gpu) => {
                Renderer::hit_test_security_popup(gpu, popup, x, y, buf_width, buf_height)
            }
        }
    }
}
