#![cfg_attr(not(feature = "gpu"), allow(irrefutable_let_patterns))]

use crate::gui::renderer::backend::RendererBackend;
use crate::gui::*;

use super::render_shared::{build_frame_params, draw_frame_content, make_frame_params_input};

impl FerrumWindow {
    /// CPU rendering path: acquires the softbuffer surface, clears it,
    /// delegates all drawing to `draw_frame_content`, then presents.
    pub(super) fn render_cpu_frame(&mut self, w: NonZeroU32, h: NonZeroU32, bw: usize, bh: usize) {
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

        let RendererBackend::Cpu { renderer, surface } = &mut self.backend else {
            return;
        };

        // Ensure surface size matches window.
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };

        // Clear the full frame.
        buffer.fill(renderer.default_bg_pixel());

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
            renderer.as_mut(),
            &mut buffer,
            bw,
            bh,
            &params,
            #[cfg(not(target_os = "macos"))]
            &state,
            #[cfg(not(target_os = "macos"))]
            &frame_tab_infos,
        );

        if let Err(e) = buffer.present() { eprintln!("[ferrum] buffer present failed: {e}"); }
    }
}
