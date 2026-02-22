#[cfg(not(target_os = "macos"))]
use crate::gui::renderer::SecurityPopup;
use crate::gui::*;

impl FerrumWindow {
    #[cfg(not(target_os = "macos"))]
    /// Opens the security event popup for the given tab index, positioning it
    /// near the security badge in the tab bar.
    pub(in crate::gui::events::mouse) fn open_security_popup_for_tab(&mut self, tab_index: usize) {
        let Some(tab) = self.tabs.get_mut(tab_index) else {
            self.security_popup = None;
            return;
        };
        let Some(leaf) = tab.focused_leaf_mut() else {
            self.security_popup = None;
            return;
        };
        let events = leaf.security.take_active_events();
        if events.is_empty() {
            self.security_popup = None;
            return;
        }

        let event_count = events.len();
        let mut lines = Vec::with_capacity(events.len());
        for event in events.iter().rev() {
            let age = event.timestamp.elapsed().as_secs();
            lines.push(format!("{} ({}s ago)", event.kind.label(), age));
        }

        let buf_width = self.window.inner_size().width;
        let (popup_x, popup_y) = self
            .backend
            .security_badge_rect(tab_index, self.tabs.len(), buf_width, event_count)
            .map(|(x, y, w, h)| (x.saturating_sub(w), y + h + self.backend.scaled_px(6)))
            .unwrap_or((
                self.backend.scaled_px(16),
                self.backend.tab_bar_height_px() + self.backend.scaled_px(6),
            ));

        self.security_popup = Some(SecurityPopup {
            tab_index,
            x: popup_x,
            y: popup_y,
            title: crate::i18n::t().security_popup_title,
            lines,
        });
    }
}
