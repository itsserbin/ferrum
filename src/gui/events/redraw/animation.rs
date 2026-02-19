use crate::gui::*;
use std::time::{Duration, Instant};

pub(super) const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const BLINK_WAKE_TOLERANCE: Duration = Duration::from_millis(20);
const SCROLLBAR_FADE_START: Duration = Duration::from_millis(1500);
const SCROLLBAR_FADE_END: Duration = Duration::from_millis(1800);
pub(super) const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);
#[cfg(not(target_os = "macos"))]
const UI_ANIMATION_SPEED: f32 = 18.0;
#[cfg(not(target_os = "macos"))]
const UI_SETTLE_EPSILON: f32 = 0.01;
#[cfg(not(target_os = "macos"))]
const CONTEXT_MENU_OPEN_DURATION: Duration = Duration::from_millis(140);
#[cfg(target_os = "macos")]
pub(super) const NATIVE_TAB_SYNC_INTERVAL: Duration = Duration::from_millis(60);
#[cfg(target_os = "macos")]
pub(super) const NATIVE_TAB_SYNC_ATTEMPTS: u8 = 6;

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
        let ui_anim = self.ui_animation_schedule(now);

        let schedules = [cursor, scrollbar, tab_anim, ui_anim];
        let mut result: Option<(Instant, bool)> = None;
        for s in schedules.into_iter().flatten() {
            result = Some(match result {
                None => s,
                Some((deadline, redraw)) => (deadline.min(s.0), redraw || s.1),
            });
        }

        #[cfg(target_os = "macos")]
        if let Some(s) = self.native_tab_sync_schedule(now) {
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

    fn ui_animation_schedule(&self, now: Instant) -> Option<(Instant, bool)> {
        #[cfg(target_os = "macos")]
        {
            let _ = now;
            None
        }
        #[cfg(not(target_os = "macos"))]
        {
            let close_hover = self.close_hovered_tab();
            let mut pending = false;

            for i in 0..self.tabs.len() {
                let tab_target = if self.hovered_tab == Some(i) && i != self.active_tab {
                    1.0
                } else {
                    0.0
                };
                let close_target = if close_hover == Some(i) { 1.0 } else { 0.0 };

                let tab_value = self.tab_hover_progress.get(i).copied().unwrap_or(0.0);
                let close_value = self.close_hover_progress.get(i).copied().unwrap_or(0.0);
                if (tab_value - tab_target).abs() > UI_SETTLE_EPSILON
                    || (close_value - close_target).abs() > UI_SETTLE_EPSILON
                {
                    pending = true;
                    break;
                }
            }

            if let Some(menu) = self.context_menu.as_ref() {
                if menu.opened_at.elapsed() < CONTEXT_MENU_OPEN_DURATION {
                    pending = true;
                } else {
                    for i in 0..menu.items.len() {
                        let target = if menu.hover_index == Some(i) {
                            1.0
                        } else {
                            0.0
                        };
                        let value = menu.hover_progress.get(i).copied().unwrap_or(0.0);
                        if (value - target).abs() > UI_SETTLE_EPSILON {
                            pending = true;
                            break;
                        }
                    }
                }
            }

            if pending {
                Some((now + ANIMATION_FRAME_INTERVAL, true))
            } else {
                None
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn close_hovered_tab(&self) -> Option<usize> {
        if self.mouse_pos.1 >= self.backend.tab_bar_height_px() as f64 {
            return None;
        }
        let buf_width = self.window.inner_size().width;
        match self.backend.hit_test_tab_bar(
            self.mouse_pos.0,
            self.mouse_pos.1,
            self.tabs.len(),
            buf_width,
        ) {
            crate::gui::renderer::TabBarHit::CloseTab(idx) => Some(idx),
            _ => None,
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn animate_scalar(value: &mut f32, target: f32, factor: f32) {
        *value += (target - *value) * factor;
        if (*value - target).abs() < 0.002 {
            *value = target;
        }
    }

    pub(in crate::gui) fn advance_ui_animations(&mut self) {
        #[cfg(target_os = "macos")]
        {
            return;
        }

        #[cfg(not(target_os = "macos"))]
        {
            let now = Instant::now();
            let dt = now
                .saturating_duration_since(self.ui_animation_last_tick)
                .as_secs_f32()
                .clamp(0.0, 0.05);
            self.ui_animation_last_tick = now;

            let factor = (dt * UI_ANIMATION_SPEED).clamp(0.0, 1.0);
            const ANIMATION_EPSILON: f32 = 1e-6;
            if factor <= ANIMATION_EPSILON {
                return;
            }

            self.tab_hover_progress.resize(self.tabs.len(), 0.0);
            self.close_hover_progress.resize(self.tabs.len(), 0.0);
            let close_hover = self.close_hovered_tab();

            for i in 0..self.tabs.len() {
                let tab_target = if self.hovered_tab == Some(i) && i != self.active_tab {
                    1.0
                } else {
                    0.0
                };
                Self::animate_scalar(&mut self.tab_hover_progress[i], tab_target, factor);

                let close_target = if close_hover == Some(i) { 1.0 } else { 0.0 };
                Self::animate_scalar(&mut self.close_hover_progress[i], close_target, factor);
            }

            if let Some(menu) = self.context_menu.as_mut() {
                menu.hover_progress.resize(menu.items.len(), 0.0);
                for i in 0..menu.items.len() {
                    let target = if menu.hover_index == Some(i) {
                        1.0
                    } else {
                        0.0
                    };
                    Self::animate_scalar(&mut menu.hover_progress[i], target, factor);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn native_tab_sync_schedule(&self, now: Instant) -> Option<(Instant, bool)> {
        if self.pending_native_tab_syncs == 0 {
            return None;
        }
        let deadline = self.next_native_tab_sync_at.unwrap_or(now);
        Some((deadline, now >= deadline))
    }
}
