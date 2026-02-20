#![cfg_attr(not(feature = "gpu"), allow(irrefutable_let_patterns))]

use crate::core::Color;
use crate::gui::renderer::backend::RendererBackend;
use crate::gui::*;

use super::render_shared::{draw_frame_content, FrameParams};

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
        buffer.fill(Color::DEFAULT_BG.to_pixel());

        // Build read-only frame params from the other fields of self
        // (split borrow: self.backend is already mutably borrowed above).
        let params = FrameParams {
            active_leaf: self.tabs.get(self.active_tab).and_then(|t| t.focused_leaf()),
            cursor_blink_start: self.cursor_blink_start,
            hovered_tab: self.hovered_tab,
            mouse_pos: self.mouse_pos,
            pinned: self.pinned,
            security_popup: self.security_popup.as_ref(),
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

        let _ = buffer.present();
    }
}
