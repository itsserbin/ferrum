use crate::gui::*;

impl FerrumWindow {
    /// Handles clipboard shortcuts: Copy (Ctrl+C), Paste (Ctrl+V), Cut (Ctrl+X).
    /// Returns `Some(result)` if the key matched a clipboard action, `None` otherwise.
    pub(super) fn handle_clipboard_shortcuts(
        &mut self,
        key: &Key,
        physical: &PhysicalKey,
    ) -> Option<bool> {
        let is_copy_key = matches!(key, Key::Named(NamedKey::Copy))
            || Self::physical_key_is(physical, KeyCode::KeyC);
        if is_copy_key {
            if self.active_leaf_ref().is_some_and(|l| l.selection.is_some()) {
                self.copy_selection();
                return Some(true);
            }
            return Some(false);
        }

        let is_paste_key = matches!(key, Key::Named(NamedKey::Paste))
            || Self::physical_key_is(physical, KeyCode::KeyV);
        if is_paste_key {
            self.paste_clipboard();
            return Some(true);
        }

        if Self::physical_key_is(physical, KeyCode::KeyX) {
            return Some(self.cut_selection());
        }

        None
    }
}
