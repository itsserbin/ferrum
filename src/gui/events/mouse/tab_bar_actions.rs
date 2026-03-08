#[cfg(not(target_os = "macos"))]
use crate::gui::renderer::WindowButton;
use crate::gui::*;

impl FerrumWindow {
    /// Clears both click-tracking timestamps.
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::events::mouse) fn clear_click_state(&mut self) {
        self.last_topbar_empty_click = None;
        self.last_tab_click = None;
    }

    /// Shared double-click rename detection and tab switch logic.
    /// Returns `true` if a rename was started (caller should not proceed further).
    fn handle_tab_click_impl(&mut self, idx: usize) -> bool {
        self.last_topbar_empty_click = None;
        let now = std::time::Instant::now();
        if self.last_tab_click.is_some_and(|(last_idx, last_time)| {
            last_idx == idx
                && now.duration_since(last_time).as_millis() < super::MULTI_CLICK_TIMEOUT_MS
        }) {
            self.start_rename(idx);
            self.last_tab_click = None;
            return true;
        }
        self.last_tab_click = Some((idx, now));
        self.switch_tab(idx);
        false
    }

    /// Handles a click on a tab: double-click rename, switch.
    #[cfg(target_os = "macos")]
    pub(in crate::gui::events::mouse) fn handle_tab_click(&mut self, idx: usize) {
        self.handle_tab_click_impl(idx);
    }

    /// Handles a click on a tab: double-click rename, switch, drag.
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::events::mouse) fn handle_tab_click(
        &mut self,
        idx: usize,
        mx: f64,
        my: f64,
        had_rename: bool,
    ) {
        if !self.handle_tab_click_impl(idx) {
            self.start_drag(idx, mx, my, had_rename);
        }
    }

    /// Handles a click on a window button (minimize/maximize/close).
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::events::mouse) fn handle_window_button_click(&mut self, btn: WindowButton) {
        self.clear_click_state();
        match btn {
            WindowButton::Minimize => {
                self.window.set_minimized(true);
            }
            WindowButton::Maximize => {
                let maximized = self.window.is_maximized();
                self.window.set_maximized(!maximized);
            }
            WindowButton::Close => {
                self.request_close_window();
            }
        }
    }

    /// Handles a click on the empty bar area: double-click maximize, drag.
    pub(in crate::gui::events::mouse) fn handle_empty_bar_click(&mut self) {
        self.last_tab_click = None;
        let now = std::time::Instant::now();
        let is_double_click = self.last_topbar_empty_click.is_some_and(|last| {
            now.duration_since(last).as_millis() < super::MULTI_CLICK_TIMEOUT_MS
        });
        if is_double_click {
            self.last_topbar_empty_click = None;
            let maximized = self.window.is_maximized();
            self.window.set_maximized(!maximized);
        } else {
            self.last_topbar_empty_click = Some(now);
            if let Err(e) = self.window.drag_window() { eprintln!("[ferrum] drag_window failed: {e}"); }
        }
    }
}
