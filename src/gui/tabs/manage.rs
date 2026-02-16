use crate::gui::tabs::normalized_active_index_after_remove;
use crate::gui::*;

impl FerrumWindow {
    /// Closes a tab by index.
    pub(in crate::gui) fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }

        // Keep title for reopen (Ctrl+Shift+T).
        let title = self.tabs[index].title.clone();
        self.closed_tabs.push(ClosedTabInfo { title });

        self.adjust_rename_after_tab_remove(index);
        self.adjust_security_popup_after_tab_remove(index);
        self.tabs.remove(index);

        if self.tabs.is_empty() {
            self.pending_requests.push(WindowRequest::CloseWindow);
            return;
        }

        let len_before = self.tabs.len() + 1;
        self.active_tab =
            normalized_active_index_after_remove(self.active_tab, len_before, index).unwrap_or(0);
    }

    /// Duplicates a tab by creating a new session with copied title.
    pub(in crate::gui) fn duplicate_tab(
        &mut self,
        index: usize,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) {
        if index >= self.tabs.len() {
            return;
        }
        let title = format!("{} (копія)", self.tabs[index].title);
        let size = self.window.inner_size();
        let (rows, cols) = self.calc_grid_size(size.width, size.height);
        self.new_tab_with_title(rows, cols, Some(title), next_tab_id, tx);
    }

    /// Switches active tab.
    pub(in crate::gui) fn switch_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
            self.security_popup = None;
        }
    }

    /// Resizes all tab terminals and their PTY sessions.
    pub(in crate::gui) fn resize_all_tabs(&mut self, rows: usize, cols: usize) {
        for tab in &mut self.tabs {
            if tab.terminal.grid.rows == rows && tab.terminal.grid.cols == cols {
                continue;
            }
            tab.terminal.resize(rows, cols);
            if let Err(err) = tab.session.resize(rows as u16, cols as u16) {
                eprintln!("Failed to resize PTY for tab {}: {err}", tab.id);
            }
        }
    }

    /// Starts inline rename for the selected tab.
    pub(in crate::gui) fn start_rename(&mut self, tab_index: usize) {
        if tab_index < self.tabs.len() {
            let text = self.tabs[tab_index].title.clone();
            let cursor = text.len();
            self.renaming_tab = Some(RenameState {
                tab_index,
                text: text.clone(),
                original_title: text,
                cursor,
                selection_anchor: Some(0),
            });
        }
    }

    /// Commits the current rename: trims text, applies if non-empty, drops rename state.
    pub(in crate::gui) fn commit_rename(&mut self) {
        if let Some(rename) = self.renaming_tab.take() {
            let trimmed = rename.text.trim().to_string();
            if !trimmed.is_empty() {
                if let Some(tab) = self.tabs.get_mut(rename.tab_index) {
                    tab.title = trimmed;
                }
                #[cfg(target_os = "macos")]
                if let Some(tab) = self.tabs.get(rename.tab_index) {
                    self.window.set_title(&tab.title);
                }
            }
            // If trimmed is empty, the old title stays (we just dropped the rename state).
        }
    }

    /// Cancels the current rename, reverting to the original title.
    pub(in crate::gui) fn cancel_rename(&mut self) {
        if let Some(rename) = self.renaming_tab.take() {
            if let Some(tab) = self.tabs.get_mut(rename.tab_index) {
                tab.title = rename.original_title;
            }
        }
    }

    pub(in crate::gui) fn adjust_rename_after_tab_remove(&mut self, removed_index: usize) {
        let Some(rename) = self.renaming_tab.as_mut() else {
            return;
        };

        if rename.tab_index == removed_index {
            self.renaming_tab = None;
        } else if rename.tab_index > removed_index {
            rename.tab_index -= 1;
        }
    }

    pub(in crate::gui) fn adjust_security_popup_after_tab_remove(&mut self, removed_index: usize) {
        let Some(popup) = self.security_popup.as_mut() else {
            return;
        };

        if popup.tab_index == removed_index {
            self.security_popup = None;
        } else if popup.tab_index > removed_index {
            popup.tab_index -= 1;
        }
    }
}
