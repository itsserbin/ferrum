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

    fn write_pty_bytes(&mut self, bytes: &[u8]) {
        if let Some(tab) = self.active_tab_mut() {
            tab.scroll_offset = 0;
            tab.selection = None;
            let _ = tab.pty_writer.write_all(bytes);
            let _ = tab.pty_writer.flush();
        }
    }

    fn build_selection_delete_bytes(
        cursor_col: usize,
        target_col: usize,
        cells_to_delete: usize,
        use_backspace: bool,
    ) -> Vec<u8> {
        let mut bytes = Vec::new();

        if target_col < cursor_col {
            for _ in 0..(cursor_col - target_col) {
                bytes.extend_from_slice(b"\x1b[D");
            }
        } else if target_col > cursor_col {
            for _ in 0..(target_col - cursor_col) {
                bytes.extend_from_slice(b"\x1b[C");
            }
        }

        let delete_seq: &[u8] = if use_backspace { b"\x7f" } else { b"\x1b[3~" };
        for _ in 0..cells_to_delete {
            bytes.extend_from_slice(delete_seq);
        }

        bytes
    }

    fn handle_selection_delete_key(&mut self, key: &Key) -> bool {
        let use_backspace = matches!(key, Key::Named(NamedKey::Backspace));
        let use_delete = matches!(key, Key::Named(NamedKey::Delete));
        if !use_backspace && !use_delete {
            return false;
        }

        // Only plain Backspace/Delete should delete active terminal selection.
        if self.modifiers.shift_key()
            || self.modifiers.control_key()
            || self.modifiers.alt_key()
            || self.modifiers.super_key()
        {
            return false;
        }

        let (cursor_col, selection_start_col, selection_end_col) = {
            let Some(tab) = self.active_tab_ref() else {
                return false;
            };
            let Some(selection) = tab.selection else {
                return false;
            };
            let (start, end) = selection.normalized();
            let cursor_abs_row = tab.terminal.scrollback.len() + tab.terminal.cursor_row;
            if start.row != end.row || start.row != cursor_abs_row {
                return false;
            }

            (tab.terminal.cursor_col, start.col, end.col)
        };

        let target_col = if use_backspace {
            selection_end_col.saturating_add(1)
        } else {
            selection_start_col
        };
        let cells_to_delete = selection_end_col
            .saturating_sub(selection_start_col)
            .saturating_add(1);

        let bytes = Self::build_selection_delete_bytes(
            cursor_col,
            target_col,
            cells_to_delete,
            use_backspace,
        );
        self.write_pty_bytes(&bytes);
        true
    }

    fn normalize_non_text_key(logical: &Key, physical: &PhysicalKey) -> Key {
        if !matches!(logical, Key::Character(_)) {
            return logical.clone();
        }

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
}

#[cfg(test)]
mod tests {
    use super::FerrumWindow;
    use crate::gui::{Key, KeyCode, NamedKey, PhysicalKey};

    #[test]
    fn selection_delete_bytes_with_backspace_moves_to_right_edge_then_erases() {
        let bytes = FerrumWindow::build_selection_delete_bytes(3, 8, 3, true);
        assert_eq!(bytes, b"\x1b[C\x1b[C\x1b[C\x1b[C\x1b[C\x7f\x7f\x7f");
    }

    #[test]
    fn selection_delete_bytes_with_delete_moves_to_left_edge_then_erases() {
        let bytes = FerrumWindow::build_selection_delete_bytes(10, 6, 2, false);
        assert_eq!(bytes, b"\x1b[D\x1b[D\x1b[D\x1b[D\x1b[3~\x1b[3~");
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
}
