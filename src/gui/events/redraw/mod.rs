mod animation;

#[cfg(feature = "gpu")]
use renderer::backend::RendererBackend;
use super::super::*;

/// Debounce delay before sending SIGWINCH after the last resize event.
/// Allows the user to finish dragging before the shell redraws its prompt.
const SIGWINCH_DEBOUNCE_MS: u64 = 80;
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
use animation::{NATIVE_TAB_SYNC_ATTEMPTS, NATIVE_TAB_SYNC_INTERVAL};

impl FerrumWindow {
    #[cfg(target_os = "macos")]
    pub(in super::super) fn schedule_native_tab_bar_resync(&mut self) {
        self.pending_native_tab_syncs = NATIVE_TAB_SYNC_ATTEMPTS;
        self.next_native_tab_sync_at = Some(Instant::now());
        self.window.request_redraw();
    }

    pub(in super::super) fn apply_pending_resize(&mut self) {
        if self.pending_grid_resize {
            self.pending_grid_resize = false;
            // Resize terminal grids so rendering is correct.
            // Cursor stays hidden until SIGWINCH is sent (see sigwinch_deadline),
            // at which point the shell redraws to the correct position.
            // Skip reflow: the shell will redraw via SIGWINCH, so intermediate
            // reflow only produces visual noise (text appears to shift then snap back).
            self.resize_all_panes(false);
        }
    }

    pub(crate) fn on_scale_factor_changed(&mut self, scale_factor: f64) {
        let prev_scale = self.backend.ui_scale();
        self.backend.set_scale(scale_factor);
        const SCALE_EPSILON: f64 = 1e-6;
        if (self.backend.ui_scale() - prev_scale).abs() < SCALE_EPSILON {
            return;
        }
        self.pending_grid_resize = true;
        // A DPI change alters cell pixel dimensions, so the grid row/col count
        // can change. Defer SIGWINCH the same way a window resize does.
        self.sigwinch_deadline =
            Some(Instant::now() + Duration::from_millis(SIGWINCH_DEBOUNCE_MS));
    }

    pub(crate) fn on_resized(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        // Reconfigure the GPU surface immediately so the OS compositor does not
        // stretch the previous frame to fit the new window dimensions.
        self.backend.notify_resize(size.width, size.height);
        self.pending_grid_resize = true;
        self.window.set_cursor_visible(false);
        // Defer SIGWINCH: reset deadline on every resize event so SIGWINCH fires
        // only once, ~80 ms after the user stops dragging.
        self.sigwinch_deadline =
            Some(Instant::now() + Duration::from_millis(SIGWINCH_DEBOUNCE_MS));
        self.window.request_redraw();
    }

    pub(crate) fn on_redraw_requested(&mut self) {
        #[cfg(target_os = "macos")]
        {
            platform::macos::sync_native_tab_bar_visibility(&self.window);
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
        self.render_frame();
    }

    fn render_frame(&mut self) {
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
