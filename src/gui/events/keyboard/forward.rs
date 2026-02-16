use crate::gui::input::key_to_bytes;
use crate::gui::*;

impl FerrumWindow {
    pub(in crate::gui::events::keyboard) fn forward_key_to_pty(&mut self, key: &Key) {
        if let Some(tab) = self.active_tab_mut() {
            tab.scroll_offset = 0;
            tab.selection = None;
        }

        let decckm = self.active_tab_ref().is_some_and(|t| t.terminal.decckm);
        let Some(bytes) = key_to_bytes(key, self.modifiers, decckm) else {
            return;
        };
        if let Some(tab) = self.active_tab_mut() {
            let _ = tab.pty_writer.write_all(&bytes);
            let _ = tab.pty_writer.flush();
        }
    }
}
