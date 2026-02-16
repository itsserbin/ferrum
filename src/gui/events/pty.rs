use crate::gui::tabs::normalized_active_index_after_remove;
use crate::gui::*;

impl FerrumWindow {
    /// Processes one PTY event.
    pub(in crate::gui) fn on_pty_event(&mut self, event: &PtyEvent) {
        match event {
            PtyEvent::Data { tab_id, bytes } => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == *tab_id) {
                    tab.terminal.process(bytes);
                    tab.scroll_offset = 0;

                    let popped = tab.terminal.drain_scrollback_popped();
                    if popped > 0 {
                        tab.selection = tab
                            .selection
                            .and_then(|sel| sel.adjust_for_scrollback_pop(popped));
                    }

                    for event in tab.terminal.drain_security_events() {
                        tab.security.record(event);
                    }

                    let responses = tab.terminal.drain_responses();
                    if !responses.is_empty() {
                        let _ = tab.pty_writer.write_all(&responses);
                        let _ = tab.pty_writer.flush();
                    }
                }
            }
            PtyEvent::Exited { tab_id } => {
                if let Some(idx) = self.tabs.iter().position(|t| t.id == *tab_id) {
                    self.tabs[idx].terminal.cleanup_after_process_exit();
                    for event in self.tabs[idx].terminal.drain_security_events() {
                        self.tabs[idx].security.record(event);
                    }

                    let len_before = self.tabs.len();
                    self.adjust_rename_after_tab_remove(idx);
                    self.adjust_security_popup_after_tab_remove(idx);
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
