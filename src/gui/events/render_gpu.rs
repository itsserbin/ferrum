#[cfg(feature = "gpu")]
use crate::gui::renderer::backend::RendererBackend;
#[cfg(feature = "gpu")]
use crate::gui::*;

#[cfg(feature = "gpu")]
use super::render_shared::{draw_frame_content, FrameParams};

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

        let RendererBackend::Gpu(gpu) = &mut self.backend else {
            return;
        };

        gpu.resize(w.get(), h.get());

        // Dummy buffer -- GPU renderer ignores buffer params.
        let mut dummy = [];

        // Build read-only frame params from the other fields of self
        // (split borrow: self.backend is already mutably borrowed above).
        let params = FrameParams {
            active_tab: self.tabs.get(self.active_tab),
            cursor_blink_start: self.cursor_blink_start,
            hovered_tab: self.hovered_tab,
            mouse_pos: self.mouse_pos,
            pinned: self.pinned,
            security_popup: self.security_popup.as_ref(),
            context_menu: self.context_menu.as_ref(),
        };

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
