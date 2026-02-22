#![cfg_attr(not(feature = "gpu"), allow(irrefutable_let_patterns))]

use crate::gui::renderer::backend::RendererBackend;
use crate::gui::*;

use crate::gui::renderer::shared::banner_layout::compute_update_banner_layout;
use crate::gui::state::UpdateInstallState;
use super::render_shared::{FrameParams, draw_frame_content};

impl FerrumWindow {
    /// CPU rendering path: acquires the softbuffer surface, clears it,
    /// delegates all drawing to `draw_frame_content`, then presents.
    pub(super) fn render_cpu_frame(&mut self, w: NonZeroU32, h: NonZeroU32, bw: usize, bh: usize) {
        // Build tab bar metadata (not needed on macOS -- native tab bar).
        #[cfg(not(target_os = "macos"))]
        let state = self.build_tab_bar_state(bw);
        #[cfg(not(target_os = "macos"))]
        let frame_tab_infos = state.render_tab_infos();

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
                    let m = renderer.tab_layout_metrics();
                    let tab_bar_h = renderer.tab_bar_height_px();
                    compute_update_banner_layout(tag, &m, bw as u32, bh as u32, tab_bar_h).map(|mut layout| {
                        layout.installing = self.update_install_state == UpdateInstallState::Installing;
                        layout
                    })
                })
            },
        };

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
