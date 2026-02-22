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
                    leaf.terminal.process(bytes);

                    let popped = leaf.terminal.drain_scrollback_popped();
                    if popped > 0 {
                        leaf.selection = leaf
                            .selection
                            .and_then(|sel| sel.adjust_for_scrollback_pop(popped));
                    }

                    for event in leaf.terminal.drain_security_events() {
                        leaf.security.record(event);
                    }

                    let responses = leaf.terminal.drain_responses();
                    if !responses.is_empty() {
                        leaf.write_pty(&responses);
                    }
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

                    // If the tab has multiple panes, close just the exited pane.
                    if tab.has_multiple_panes() {
                        // Run cleanup on the exiting pane's terminal.
                        if let Some(leaf) = tab.pane_tree.find_leaf_mut(*pane_id) {
                            leaf.terminal.cleanup_after_process_exit();
                            for event in leaf.terminal.drain_security_events() {
                                leaf.security.record(event);
                            }
                        }
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
                    if let Some(leaf) = tab.pane_tree.find_leaf_mut(*pane_id) {
                        leaf.terminal.cleanup_after_process_exit();
                        for event in leaf.terminal.drain_security_events() {
                            leaf.security.record(event);
                        }
                    }

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
