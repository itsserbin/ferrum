mod entry;
mod forward;
mod rename;
mod shortcuts;

use crate::gui::*;

impl FerrumWindow {
    fn copy_selection_and_clear(&mut self) {
        self.copy_selection();
        if let Some(tab) = self.active_tab_mut() {
            tab.selection = None;
        }
    }

    fn is_modifier_only_key(key: &Key) -> bool {
        matches!(
            key,
            Key::Named(NamedKey::Control | NamedKey::Shift | NamedKey::Alt | NamedKey::Super)
        )
    }
}
