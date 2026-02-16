#[cfg(feature = "gpu")]
use crate::gui::renderer::backend::RendererBackend;
use crate::gui::*;
use std::time::{Duration, Instant};

const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const BLINK_WAKE_TOLERANCE: Duration = Duration::from_millis(20);
const SCROLLBAR_FADE_START: Duration = Duration::from_millis(1500);
const SCROLLBAR_FADE_END: Duration = Duration::from_millis(1800);
const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Quadratic ease-out: fast start, smooth deceleration.
#[cfg(not(target_os = "macos"))]
fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

impl FerrumWindow {
    pub(in crate::gui) fn animation_schedule(&self, now: Instant) -> Option<(Instant, bool)> {
        let cursor = self.cursor_animation_schedule(now);
        let scrollbar = self.scrollbar_animation_schedule(now);
        let tab_anim = self.tab_animation_schedule(now);

        let schedules = [cursor, scrollbar, tab_anim];
        let mut result: Option<(Instant, bool)> = None;
        for s in schedules.into_iter().flatten() {
            result = Some(match result {
                None => s,
                Some((deadline, redraw)) => (deadline.min(s.0), redraw || s.1),
            });
        }
        result
    }

    fn tab_animation_schedule(&self, now: Instant) -> Option<(Instant, bool)> {
        let anim = self.tab_reorder_animation.as_ref()?;
        let elapsed = now.saturating_duration_since(anim.started);
        let duration = Duration::from_millis(anim.duration_ms as u64);
        if elapsed >= duration {
            return None; // Animation finished, will be cleaned up on next redraw.
        }
        Some((now + ANIMATION_FRAME_INTERVAL, true))
    }

    /// Returns current animation offsets for tab slide animation (or None if not animating).
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui) fn tab_animation_offsets(&self) -> Option<Vec<f32>> {
        let anim = self.tab_reorder_animation.as_ref()?;
        let elapsed = anim.started.elapsed().as_secs_f32();
        let duration = anim.duration_ms as f32 / 1000.0;
        let t = (elapsed / duration).min(1.0);
        let factor = 1.0 - ease_out(t);
        let offsets: Vec<f32> = anim.offsets.iter().map(|o| o * factor).collect();
        Some(offsets)
    }

    fn cursor_animation_schedule(&self, now: Instant) -> Option<(Instant, bool)> {
        let tab = self.active_tab_ref()?;
        if tab.scroll_offset != 0
            || !tab.terminal.cursor_visible
            || !tab.terminal.cursor_style.is_blinking()
        {
            return None;
        }

        let interval_ms = CURSOR_BLINK_INTERVAL.as_millis();
        let elapsed_ms = now
            .saturating_duration_since(self.cursor_blink_start)
            .as_millis();
        let phase = elapsed_ms / interval_ms;

        let prev_boundary_ms = phase * interval_ms;
        let next_boundary_ms = (phase + 1) * interval_ms;

        let prev_boundary =
            self.cursor_blink_start + Duration::from_millis(prev_boundary_ms as u64);
        let next_boundary =
            self.cursor_blink_start + Duration::from_millis(next_boundary_ms as u64);

        let redraw_now = now >= prev_boundary
            && now.saturating_duration_since(prev_boundary) <= BLINK_WAKE_TOLERANCE;

        Some((next_boundary, redraw_now))
    }

    fn scrollbar_animation_schedule(&self, now: Instant) -> Option<(Instant, bool)> {
        let tab = self.active_tab_ref()?;
        if tab.terminal.scrollback.is_empty() || tab.scrollbar.hover || tab.scrollbar.dragging {
            return None;
        }

        let elapsed = now.saturating_duration_since(tab.scrollbar.last_activity);
        if elapsed < SCROLLBAR_FADE_START {
            return Some((tab.scrollbar.last_activity + SCROLLBAR_FADE_START, false));
        }
        if elapsed < SCROLLBAR_FADE_END {
            return Some((now + ANIMATION_FRAME_INTERVAL, true));
        }
        None
    }

    pub(in crate::gui) fn apply_pending_resize(&mut self) {
        if let Some((rows, cols)) = self.pending_grid_resize.take() {
            self.resize_all_tabs(rows, cols);
        }
    }

    pub(crate) fn on_scale_factor_changed(&mut self, scale_factor: f64) {
        let prev_scale = self.backend.ui_scale();
        self.backend.set_scale(scale_factor);
        if (self.backend.ui_scale() - prev_scale).abs() < f64::EPSILON {
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
        self.window.request_redraw();
    }

    pub(crate) fn on_redraw_requested(&mut self) {
        self.refresh_tab_bar_visibility();
        self.apply_pending_resize();

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
