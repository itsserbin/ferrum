use crate::gui::*;

impl FerrumWindow {
    /// Handles Super+key text editing shortcuts (macOS emacs-style):
    /// Super+A/E/B/F/D/K/U mapped to readline control codes.
    pub(super) fn handle_super_text_shortcuts(&mut self, physical: &PhysicalKey) -> bool {
        if !self.modifiers.super_key() {
            return false;
        }

        if Self::physical_key_is(physical, KeyCode::KeyA) {
            self.write_pty_bytes(b"\x01"); // Ctrl+A - beginning of line
            return true;
        }
        if Self::physical_key_is(physical, KeyCode::KeyE) {
            self.write_pty_bytes(b"\x05"); // Ctrl+E - end of line
            return true;
        }
        if Self::physical_key_is(physical, KeyCode::KeyB) {
            self.write_pty_bytes(b"\x1bb"); // Alt+B - previous word
            return true;
        }
        if Self::physical_key_is(physical, KeyCode::KeyF) {
            self.write_pty_bytes(b"\x1bf"); // Alt+F - next word
            return true;
        }
        if Self::physical_key_is(physical, KeyCode::KeyD) {
            self.write_pty_bytes(b"\x1bd"); // Alt+D - delete next word
            return true;
        }
        if Self::physical_key_is(physical, KeyCode::KeyK) {
            self.write_pty_bytes(b"\x0b"); // Ctrl+K - delete to end of line
            return true;
        }
        if Self::physical_key_is(physical, KeyCode::KeyU) {
            self.write_pty_bytes(b"\x15"); // Ctrl+U - delete to beginning of line
            return true;
        }
        false
    }

    /// Handles Super+arrow/delete navigation shortcuts:
    /// Super+Arrow (scroll/line nav), Super+Backspace/Delete (line kill).
    pub(super) fn handle_super_navigation_shortcuts(&mut self, key: &Key) -> Option<bool> {
        if !self.modifiers.super_key() {
            return None;
        }

        match key {
            Key::Named(NamedKey::ArrowLeft) => {
                self.write_pty_bytes(b"\x01"); // Ctrl+A - beginning of line
                Some(true)
            }
            Key::Named(NamedKey::ArrowRight) => {
                self.write_pty_bytes(b"\x05"); // Ctrl+E - end of line
                Some(true)
            }
            Key::Named(NamedKey::ArrowUp) => {
                if let Some(leaf) = self.active_leaf_mut() {
                    leaf.scroll_offset = leaf.terminal.scrollback.len();
                }
                Some(true)
            }
            Key::Named(NamedKey::ArrowDown) => {
                if let Some(leaf) = self.active_leaf_mut() {
                    leaf.scroll_offset = 0;
                }
                Some(true)
            }
            Key::Named(NamedKey::Backspace) => {
                self.write_pty_bytes(b"\x15"); // Ctrl+U - delete to beginning of line
                Some(true)
            }
            Key::Named(NamedKey::Delete) => {
                self.write_pty_bytes(b"\x0b"); // Ctrl+K - delete to end of line
                Some(true)
            }
            _ => None,
        }
    }
}
