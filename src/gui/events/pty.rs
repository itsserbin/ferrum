use crate::gui::tabs::normalized_active_index_after_remove;
use crate::gui::*;

impl FerrumWindow {
    /// Processes one PTY event.
    pub(in crate::gui) fn on_pty_event(&mut self, event: &PtyEvent) {
        match event {
            PtyEvent::Data {
                tab_id,
                pane_id,
                bytes,
            } => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == *tab_id)
                    && let Some(leaf) = tab.pane_tree.find_leaf_mut(*pane_id)
                {
                    leaf.process_and_flush(bytes);
                }
            }
            PtyEvent::Exited { tab_id, pane_id } => {
                if let Some(idx) = self.tabs.iter().position(|t| t.id == *tab_id) {
                    let tab = &mut self.tabs[idx];

                    // Pane may have been closed manually (e.g. Cmd/Ctrl+W on split)
                    // before the PTY reader thread delivered Exited. Ignore stale events.
                    if tab.pane_tree.find_leaf(*pane_id).is_none() {
                        return;
                    }

                    // Cleanup is always required whether the pane or whole tab is being closed.
                    if let Some(leaf) = tab.pane_tree.find_leaf_mut(*pane_id) {
                        leaf.cleanup_and_drain_security();
                    }

                    // If the tab has multiple panes, close just the exited pane.
                    if tab.has_multiple_panes() {
                        // Close the pane in the tree.
                        tab.pane_tree.close(*pane_id);
                        // If the focused pane was the one that exited, pick a new one.
                        if tab.focused_pane == *pane_id {
                            tab.focused_pane = tab.focus_after_closing_pane(*pane_id).unwrap_or(0);
                        }
                        self.window.request_redraw();
                        return;
                    }

                    // Single pane: close the whole tab (existing behavior).

                    let len_before = self.tabs.len();
                    self.adjust_rename_after_tab_remove(idx);
                    self.tabs.remove(idx);
                    self.refresh_tab_bar_visibility();
                    if self.tabs.is_empty() {
                        self.pending_requests.push(WindowRequest::CloseWindow);
                        return;
                    }
                    self.active_tab =
                        normalized_active_index_after_remove(self.active_tab, len_before, idx)
                            .unwrap_or(0);
                }
            }
        }
    }
}
