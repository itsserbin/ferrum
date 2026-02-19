mod animation;

#[cfg(feature = "gpu")]
use crate::gui::renderer::backend::RendererBackend;
use crate::gui::*;
#[cfg(target_os = "macos")]
use std::time::Instant;

#[cfg(target_os = "macos")]
use animation::{NATIVE_TAB_SYNC_ATTEMPTS, NATIVE_TAB_SYNC_INTERVAL};

impl FerrumWindow {
    #[cfg(target_os = "macos")]
    pub(in crate::gui) fn schedule_native_tab_bar_resync(&mut self) {
        self.pending_native_tab_syncs = NATIVE_TAB_SYNC_ATTEMPTS;
        self.next_native_tab_sync_at = Some(Instant::now());
        self.window.request_redraw();
    }

    pub(in crate::gui) fn apply_pending_resize(&mut self) {
        if self.pending_grid_resize.take().is_some() {
            // Recalculate grid size from current window dimensions to avoid race condition
            // with native tab bar toggle on macOS (which can change window.inner_size()
            // between on_resized() and this apply call).
            let size = self.window.inner_size();
            let (rows, cols) = self.calc_grid_size(size.width, size.height);
            self.resize_all_tabs(rows, cols);
            // Show mouse cursor after resize completes
            self.window.set_cursor_visible(true);
        }
    }

    pub(crate) fn on_scale_factor_changed(&mut self, scale_factor: f64) {
        let prev_scale = self.backend.ui_scale();
        self.backend.set_scale(scale_factor);
        const SCALE_EPSILON: f64 = 1e-6;
        if (self.backend.ui_scale() - prev_scale).abs() < SCALE_EPSILON {
            return;
        }

        let size = self.window.inner_size();
        let (rows, cols) = self.calc_grid_size(size.width, size.height);
        self.pending_grid_resize = Some((rows, cols));
    }

    pub(crate) fn on_resized(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let (rows, cols) = self.calc_grid_size(size.width, size.height);
        // Coalesce rapid OS resize events and apply only the latest grid size on redraw.
        self.pending_grid_resize = Some((rows, cols));
        // Hide mouse cursor during resize to avoid visual glitches
        self.window.set_cursor_visible(false);
        self.window.request_redraw();
    }

    pub(crate) fn on_redraw_requested(&mut self) {
        #[cfg(target_os = "macos")]
        {
            crate::gui::platform::macos::sync_native_tab_bar_visibility(&self.window);
            if self.pending_native_tab_syncs > 0 {
                self.pending_native_tab_syncs -= 1;
                self.next_native_tab_sync_at = if self.pending_native_tab_syncs > 0 {
                    Some(Instant::now() + NATIVE_TAB_SYNC_INTERVAL)
                } else {
                    None
                };
            }
        }
        self.refresh_tab_bar_visibility();
        self.apply_pending_resize();
        self.advance_ui_animations();

        let size = self.window.inner_size();

        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        let bw = w.get() as usize;
        let bh = h.get() as usize;

        // Delegate to the appropriate backend-specific render path.
        #[cfg(feature = "gpu")]
        if matches!(self.backend, RendererBackend::Gpu(_)) {
            self.render_gpu_frame(w, h, bw, bh);
            return;
        }

        self.render_cpu_frame(w, h, bw, bh);
    }
}
