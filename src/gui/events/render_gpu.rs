#[cfg(feature = "gpu")]
use crate::gui::renderer::backend::RendererBackend;
#[cfg(feature = "gpu")]
use crate::gui::*;

#[cfg(feature = "gpu")]
use crate::gui::renderer::shared::banner_layout::compute_update_banner_layout;
#[cfg(feature = "gpu")]
use crate::gui::state::UpdateInstallState;
#[cfg(feature = "gpu")]
use super::render_shared::{FrameParams, draw_frame_content};

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
            tab: self.tabs.get(self.active_tab),
            cursor_blink_start: self.cursor_blink_start,
            cursor_blink_interval_ms: self.cursor_blink_interval_ms,
            #[cfg(not(target_os = "macos"))]
            hovered_tab: self.hovered_tab,
            #[cfg(not(target_os = "macos"))]
            mouse_pos: self.mouse_pos,
            #[cfg(not(target_os = "macos"))]
            pinned: self.pinned,
            update_banner: if self.update_banner_dismissed
                || self.update_install_state == UpdateInstallState::Done
            {
                None
            } else {
                self.pending_update_tag.as_deref().and_then(|tag| {
                    let m = gpu.tab_layout_metrics();
                    let tab_bar_h = gpu.tab_bar_height_px();
                    compute_update_banner_layout(tag, &m, bw as u32, bh as u32, tab_bar_h).map(|mut layout| {
                        layout.installing = self.update_install_state == UpdateInstallState::Installing;
                        layout
                    })
                })
            },
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
