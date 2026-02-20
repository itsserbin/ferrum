use crate::gui::input::key_to_bytes;
use crate::gui::*;

impl FerrumWindow {
    pub(in crate::gui::events::keyboard) fn forward_key_to_pty(&mut self, key: &Key) {
        let should_replace_selection = self.active_leaf_ref().is_some_and(|leaf| leaf.selection.is_some())
            && Self::is_text_replacement_key(key, self.modifiers);
        if should_replace_selection {
            let _ = self.delete_terminal_selection(false);
        }

        if let Some(leaf) = self.active_leaf_mut() {
            leaf.scroll_offset = 0;
            leaf.selection = None;
        }
        self.keyboard_selection_anchor = None;

        let decckm = self.active_leaf_ref().is_some_and(|l| l.terminal.decckm);
        let Some(bytes) = key_to_bytes(key, self.modifiers, decckm) else {
            return;
        };
        if let Some(leaf) = self.active_leaf_mut() {
            let _ = leaf.pty_writer.write_all(&bytes);
            let _ = leaf.pty_writer.flush();
        }
    }
}
