mod clipboard_shortcuts;
mod delete;
mod entry;
mod forward;
mod navigation_shortcuts;
mod rename;
mod selection;
mod shortcuts;
mod tab_shortcuts;
mod word_motion;

use crate::gui::*;

impl FerrumWindow {
    pub(in crate::gui) fn write_pty_bytes(&mut self, bytes: &[u8]) {
        if let Some(leaf) = self.active_leaf_mut() {
            leaf.scroll_offset = 0;
            leaf.selection = None;
            leaf.write_pty(bytes);
        }
        self.keyboard_selection_anchor = None;
    }

    fn normalize_non_text_key(logical: &Key, physical: &PhysicalKey) -> Key {
        let PhysicalKey::Code(code) = physical else {
            return logical.clone();
        };

        let named = match code {
            KeyCode::ArrowLeft => Some(NamedKey::ArrowLeft),
            KeyCode::ArrowRight => Some(NamedKey::ArrowRight),
            KeyCode::ArrowUp => Some(NamedKey::ArrowUp),
            KeyCode::ArrowDown => Some(NamedKey::ArrowDown),
            KeyCode::Home => Some(NamedKey::Home),
            KeyCode::End => Some(NamedKey::End),
            KeyCode::PageUp => Some(NamedKey::PageUp),
            KeyCode::PageDown => Some(NamedKey::PageDown),
            KeyCode::Insert => Some(NamedKey::Insert),
            KeyCode::Delete => Some(NamedKey::Delete),
            KeyCode::Backspace => Some(NamedKey::Backspace),
            KeyCode::Tab => Some(NamedKey::Tab),
            KeyCode::Enter => Some(NamedKey::Enter),
            KeyCode::Escape => Some(NamedKey::Escape),
            _ => None,
        };

        named.map_or_else(|| logical.clone(), Key::Named)
    }

    fn is_modifier_only_key(key: &Key) -> bool {
        matches!(
            key,
            Key::Named(NamedKey::Control | NamedKey::Shift | NamedKey::Alt | NamedKey::Super)
        )
    }

    fn is_text_replacement_key(key: &Key, modifiers: ModifiersState) -> bool {
        if modifiers.control_key() || modifiers.alt_key() || modifiers.super_key() {
            return false;
        }

        match key {
            Key::Character(c) => !c.is_empty() && !c.chars().any(char::is_control),
            Key::Named(NamedKey::Space) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gui::{FerrumWindow, Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

    fn mods(ctrl: bool, shift: bool, alt: bool) -> ModifiersState {
        let mut state = ModifiersState::empty();
        state.set(ModifiersState::CONTROL, ctrl);
        state.set(ModifiersState::SHIFT, shift);
        state.set(ModifiersState::ALT, alt);
        state
    }

    #[test]
    fn normalize_non_text_key_maps_character_arrow_from_physical_code() {
        let key = FerrumWindow::normalize_non_text_key(
            &Key::Character("".into()),
            &PhysicalKey::Code(KeyCode::ArrowLeft),
        );
        assert_eq!(key, Key::Named(NamedKey::ArrowLeft));
    }

    #[test]
    fn normalize_non_text_key_keeps_regular_character_keys() {
        let key = FerrumWindow::normalize_non_text_key(
            &Key::Character("x".into()),
            &PhysicalKey::Code(KeyCode::KeyX),
        );
        assert_eq!(key, Key::Character("x".into()));
    }

    #[test]
    fn text_replacement_key_detects_plain_printable_input() {
        assert!(FerrumWindow::is_text_replacement_key(
            &Key::Character("x".into()),
            mods(false, false, false)
        ));
        assert!(FerrumWindow::is_text_replacement_key(
            &Key::Character("X".into()),
            mods(false, true, false)
        ));
    }

    #[test]
    fn text_replacement_key_rejects_modified_or_non_text_keys() {
        assert!(!FerrumWindow::is_text_replacement_key(
            &Key::Character("x".into()),
            mods(true, false, false)
        ));
        assert!(!FerrumWindow::is_text_replacement_key(
            &Key::Named(NamedKey::ArrowLeft),
            mods(false, false, false)
        ));
    }
}
