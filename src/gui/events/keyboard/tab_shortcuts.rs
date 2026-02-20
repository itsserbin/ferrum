use crate::gui::*;

impl FerrumWindow {
    /// Handles tab management shortcuts: new tab (T), close (W), new window (N),
    /// switch tab by digit, and Ctrl+Tab.
    pub(super) fn handle_tab_management_shortcuts(
        &mut self,
        key: &Key,
        physical: &PhysicalKey,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) -> Option<bool> {
        #[cfg(target_os = "macos")]
        let _ = (&next_tab_id, tx);

        if Self::physical_key_is(physical, KeyCode::KeyT) {
            #[cfg(target_os = "macos")]
            {
                let cwd = self.active_leaf_ref().and_then(|l| l.cwd());
                self.pending_requests.push(WindowRequest::NewTab { cwd });
            }
            #[cfg(not(target_os = "macos"))]
            {
                let cwd = self.active_leaf_ref().and_then(|l| l.cwd());
                let size = self.window.inner_size();
                let (rows, cols) = self.calc_grid_size(size.width, size.height);
                self.new_tab(rows, cols, next_tab_id, tx, cwd);
            }
            return Some(true);
        }

        if Self::physical_key_is(physical, KeyCode::KeyW) {
            // Cmd/Ctrl+W: close focused pane first, then tab, then window.
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
            #[cfg(target_os = "macos")]
            {
                if digit_index == 8 {
                    crate::gui::platform::macos::select_tab(&self.window, usize::MAX);
                } else {
                    crate::gui::platform::macos::select_tab(&self.window, digit_index);
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                if digit_index == 8 {
                    if !self.tabs.is_empty() {
                        self.active_tab = self.tabs.len() - 1;
                    }
                } else {
                    self.switch_tab(digit_index);
                }
            }
            return Some(true);
        }

        if matches!(key, Key::Named(NamedKey::Tab)) {
            #[cfg(target_os = "macos")]
            crate::gui::platform::macos::select_next_tab(&self.window);
            #[cfg(not(target_os = "macos"))]
            {
                if !self.tabs.is_empty() {
                    self.active_tab = (self.active_tab + 1) % self.tabs.len();
                }
            }
            return Some(true);
        }

        None
    }
}
