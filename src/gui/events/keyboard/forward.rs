use crate::gui::input::key_to_bytes_ex;
use crate::gui::*;

impl FerrumWindow {
    pub(in crate::gui::events::keyboard) fn forward_key_to_pty(&mut self, key: &Key) {
        let should_replace_selection = self
            .active_leaf_ref()
            .is_some_and(|leaf| leaf.selection.is_some())
            && Self::is_text_replacement_key(key, self.modifiers);
        if should_replace_selection {
            self.delete_terminal_selection(false);
        }

        if let Some(leaf) = self.active_leaf_mut() {
            leaf.scroll_offset = 0;
            leaf.clear_selection();
        }
        self.keyboard_selection_anchor = None;

        let (decckm, modify_other_keys) = self
            .active_leaf_ref()
            .map(|l| (l.terminal.decckm, l.terminal.modify_other_keys))
            .unwrap_or((false, 0));
        let Some(bytes) = key_to_bytes_ex(key, self.modifiers, decckm, modify_other_keys) else {
            return;
        };
        if let Some(leaf) = self.active_leaf_mut() {
            leaf.write_pty(&bytes);
        }
    }
}
