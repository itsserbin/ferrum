#[cfg(not(target_os = "macos"))]
use crate::gui::renderer::WindowButton;
use crate::gui::*;

impl FerrumWindow {
    /// Handles a click on a tab: double-click rename, switch, drag.
    pub(in crate::gui::events::mouse) fn handle_tab_click(
        &mut self,
        idx: usize,
        mx: f64,
        my: f64,
        had_rename: bool,
    ) {
        self.last_topbar_empty_click = None;
        let now = std::time::Instant::now();
        if self.last_tab_click.is_some_and(|(last_idx, last_time)| {
            last_idx == idx
                && now.duration_since(last_time).as_millis() < super::TAB_BAR_MULTI_CLICK_MS
        }) {
            self.start_rename(idx);
            self.last_tab_click = None;
            return;
        }
        self.last_tab_click = Some((idx, now));
        self.switch_tab(idx);
        self.start_drag(idx, mx, my, had_rename);
    }

    /// Handles a click on a window button (minimize/maximize/close).
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::events::mouse) fn handle_window_button_click(&mut self, btn: WindowButton) {
        self.last_topbar_empty_click = None;
        self.last_tab_click = None;
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
            now.duration_since(last).as_millis() < super::TAB_BAR_MULTI_CLICK_MS
        });
        if is_double_click {
            self.last_topbar_empty_click = None;
            let maximized = self.window.is_maximized();
            self.window.set_maximized(!maximized);
        } else {
            self.last_topbar_empty_click = Some(now);
            let _ = self.window.drag_window();
        }
    }
}
