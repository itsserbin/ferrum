use crate::gui::*;

impl App {
    /// Drains all pending PTY events from the channel, routing each to the correct window.
    pub(super) fn drain_pty_events(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(event) = self.rx.try_recv() {
            let tab_id = match &event {
                PtyEvent::Data { tab_id, .. } => *tab_id,
                PtyEvent::Exited { tab_id } => *tab_id,
            };

            // Find which window owns this tab.
            let win_id = self
                .windows
                .iter()
                .find(|(_, win)| win.tabs.iter().any(|t| t.id == tab_id))
                .map(|(id, _)| *id);

            if let Some(win_id) = win_id {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.on_pty_event(&event);
                    win.window.request_redraw();
                }
                self.process_window_requests(event_loop, win_id);
            }
        }
    }
}
