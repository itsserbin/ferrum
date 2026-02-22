use crate::config::AppConfig;
#[cfg(not(target_os = "macos"))]
use crate::gui::tabs::create::NewTabParams;
use crate::gui::*;

impl FerrumWindow {
    /// Handles tab management shortcuts: new tab (T), close (W), new window (N),
    /// switch tab by digit, and Ctrl+Tab.
    #[cfg(target_os = "macos")]
    pub(super) fn handle_tab_management_shortcuts(
        &mut self,
        key: &Key,
        physical: &PhysicalKey,
        _next_tab_id: &mut u64,
        _tx: &mpsc::Sender<PtyEvent>,
        _config: &AppConfig,
    ) -> Option<bool> {
        if Self::physical_key_is(physical, KeyCode::KeyT) {
            let cwd = self.active_leaf_ref().and_then(|l| l.cwd());
            self.pending_requests.push(WindowRequest::NewTab { cwd });
            return Some(true);
        }

        if Self::physical_key_is(physical, KeyCode::KeyW) {
            // Cmd+W: close focused pane first, then tab, then window.
            if self
                .active_tab_ref()
                .is_some_and(|tab| tab.has_multiple_panes())
            {
                self.close_focused_pane();
            } else {
                self.close_tab(self.active_tab);
            }
            return Some(true);
        }

        if Self::physical_key_is(physical, KeyCode::KeyN) {
            let cwd = self.active_leaf_ref().and_then(|l| l.cwd());
            self.pending_requests.push(WindowRequest::NewWindow { cwd });
            return Some(true);
        }

        if let Some(digit_index) = Self::physical_digit_index(physical) {
            if digit_index == 8 {
                crate::gui::platform::macos::select_tab(&self.window, usize::MAX);
            } else {
                crate::gui::platform::macos::select_tab(&self.window, digit_index);
            }
            return Some(true);
        }

        if matches!(key, Key::Named(NamedKey::Tab)) {
            crate::gui::platform::macos::select_next_tab(&self.window);
            return Some(true);
        }

        None
    }

    /// Handles tab management shortcuts: new tab (T), close (W), new window (N),
    /// switch tab by digit, and Ctrl+Tab.
    #[cfg(not(target_os = "macos"))]
    pub(super) fn handle_tab_management_shortcuts(
        &mut self,
        key: &Key,
        physical: &PhysicalKey,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
        config: &AppConfig,
    ) -> Option<bool> {
        if Self::physical_key_is(physical, KeyCode::KeyT) {
            let cwd = self.active_leaf_ref().and_then(|l| l.cwd());
            let size = self.window.inner_size();
            let (rows, cols) = self.calc_grid_size(size.width, size.height);
            self.new_tab(NewTabParams {
                rows,
                cols,
                title: None,
                next_tab_id,
                tx,
                cwd,
                config,
            });
            return Some(true);
        }

        if Self::physical_key_is(physical, KeyCode::KeyW) {
            // Ctrl+W: close focused pane first, then tab, then window.
            if self
                .active_tab_ref()
                .is_some_and(|tab| tab.has_multiple_panes())
            {
                self.close_focused_pane();
            } else {
                self.close_tab(self.active_tab);
            }
            return Some(true);
        }

        if Self::physical_key_is(physical, KeyCode::KeyN) {
            let cwd = self.active_leaf_ref().and_then(|l| l.cwd());
            self.pending_requests.push(WindowRequest::NewWindow { cwd });
            return Some(true);
        }

        if let Some(digit_index) = Self::physical_digit_index(physical) {
            if digit_index == 8 {
                if !self.tabs.is_empty() {
                    self.active_tab = self.tabs.len() - 1;
                }
            } else {
                self.switch_tab(digit_index);
            }
            return Some(true);
        }

        if matches!(key, Key::Named(NamedKey::Tab)) {
            if !self.tabs.is_empty() {
                self.active_tab = (self.active_tab + 1) % self.tabs.len();
            }
            return Some(true);
        }

        None
    }
}
