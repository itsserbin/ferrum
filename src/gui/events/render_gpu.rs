#[cfg(feature = "gpu")]
use crate::gui::renderer::backend::RendererBackend;
#[cfg(feature = "gpu")]
use crate::gui::*;

#[cfg(feature = "gpu")]
use super::render_shared::{build_frame_params, draw_frame_content, make_frame_params_input};

#[cfg(feature = "gpu")]
impl FerrumWindow {
    /// GPU rendering path: resizes the wgpu surface, delegates all drawing
    /// to `draw_frame_content` with a dummy buffer, then presents via wgpu.
    pub(super) fn render_gpu_frame(&mut self, w: NonZeroU32, h: NonZeroU32, bw: usize, bh: usize) {
        // Build tab bar metadata (not needed on macOS -- native tab bar).
        #[cfg(not(target_os = "macos"))]
        let state = self.build_tab_bar_state(bw);
        #[cfg(not(target_os = "macos"))]
        let frame_tab_infos = state.render_tab_infos();

        // Extract metrics from the backend before pattern-matching it mutably,
        // so the borrow checker sees separate borrows for the backend and the
        // remaining FerrumWindow fields used to build FrameParams.
        let tab_layout_metrics = self.backend.tab_layout_metrics();
        let tab_bar_h = self.backend.tab_bar_height_px();

        let RendererBackend::Gpu(gpu) = &mut self.backend else {
            return;
        };

        gpu.resize(w.get(), h.get());

        // Dummy buffer -- GPU renderer ignores buffer params.
        let mut dummy = [];

        // Build read-only frame params from the other fields of self
        // (split borrow: self.backend is already mutably borrowed above).
        let params = build_frame_params(
            make_frame_params_input!(self),
            &tab_layout_metrics,
            tab_bar_h,
            bw as u32,
            bh as u32,
        );

        draw_frame_content(
            gpu.as_mut(),
            &mut dummy,
            bw,
            bh,
            &params,
            #[cfg(not(target_os = "macos"))]
            &state,
            #[cfg(not(target_os = "macos"))]
            &frame_tab_infos,
        );

        // Present the frame via wgpu.
        gpu.present_frame();
    }
}
