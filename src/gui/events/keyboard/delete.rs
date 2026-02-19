use crate::gui::*;

use super::word_motion::HorizontalMotion;

impl FerrumWindow {
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

    pub(in crate::gui) fn delete_terminal_selection(&mut self, use_backspace: bool) -> bool {
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

    pub(super) fn cut_selection(&mut self) -> bool {
        if !self.active_tab_ref().is_some_and(|t| t.selection.is_some()) {
            return false;
        }

        self.copy_selection();
        if !self.delete_terminal_selection(false) {
            if let Some(tab) = self.active_tab_mut() {
                tab.selection = None;
            }
            self.keyboard_selection_anchor = None;
        }
        true
    }

    pub(super) fn handle_selection_delete_key(&mut self, key: &Key) -> bool {
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
        self.delete_terminal_selection(use_backspace)
    }

    fn is_word_delete_modifier(modifiers: ModifiersState) -> bool {
        if modifiers.shift_key() || modifiers.super_key() {
            return false;
        }
        modifiers.control_key() || modifiers.alt_key()
    }

    fn build_word_delete_bytes(cells_to_delete: usize, use_backspace: bool) -> Vec<u8> {
        let mut bytes = Vec::new();
        let delete_seq: &[u8] = if use_backspace { b"\x7f" } else { b"\x1b[3~" };
        for _ in 0..cells_to_delete {
            bytes.extend_from_slice(delete_seq);
        }
        bytes
    }

    fn build_forward_word_delete_bytes(cursor_col: usize, target_col: usize) -> Vec<u8> {
        let cells_to_delete = target_col.saturating_sub(cursor_col);
        let mut bytes = Self::build_horizontal_cursor_move_bytes(cursor_col, target_col);
        for _ in 0..cells_to_delete {
            bytes.extend_from_slice(b"\x7f");
        }
        bytes
    }

    pub(super) fn handle_word_delete_key(&mut self, key: &Key) -> bool {
        let use_backspace = matches!(key, Key::Named(NamedKey::Backspace));
        let use_delete = matches!(key, Key::Named(NamedKey::Delete));
        if !use_backspace && !use_delete {
            return false;
        }

        if !Self::is_word_delete_modifier(self.modifiers) {
            return false;
        }

        if self.active_tab_ref().is_some_and(|tab| tab.selection.is_some()) {
            if self.delete_terminal_selection(use_backspace) {
                return true;
            }
        }

        let (cursor_col, target_col) = {
            let Some(tab) = self.active_tab_ref() else {
                return false;
            };
            if tab.terminal.is_alt_screen() {
                return false;
            }

            let grid_cols = tab.terminal.grid.cols;
            if grid_cols == 0 {
                return false;
            }

            let cursor_col = tab.terminal.cursor_col.min(grid_cols);
            let target_col = if use_backspace {
                Self::word_motion_target_col(tab, cursor_col, HorizontalMotion::Left)
            } else {
                Self::word_motion_target_col(tab, cursor_col, HorizontalMotion::Right)
            };
            (cursor_col, target_col)
        };

        let bytes = if use_backspace {
            let cells_to_delete = cursor_col.saturating_sub(target_col);
            Self::build_word_delete_bytes(cells_to_delete, true)
        } else {
            Self::build_forward_word_delete_bytes(cursor_col, target_col)
        };
        if !bytes.is_empty() {
            self.write_pty_bytes(&bytes);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::gui::{FerrumWindow, ModifiersState};

    fn mods(ctrl: bool, shift: bool, alt: bool) -> ModifiersState {
        let mut state = ModifiersState::empty();
        state.set(ModifiersState::CONTROL, ctrl);
        state.set(ModifiersState::SHIFT, shift);
        state.set(ModifiersState::ALT, alt);
        state
    }

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
    fn word_delete_modifier_accepts_ctrl_or_alt_without_shift_super() {
        assert!(FerrumWindow::is_word_delete_modifier(mods(
            true, false, false
        )));
        assert!(FerrumWindow::is_word_delete_modifier(mods(
            false, false, true
        )));
        assert!(!FerrumWindow::is_word_delete_modifier(mods(
            true, true, false
        )));
    }

    #[test]
    fn word_delete_bytes_backspace_repeats_del() {
        let bytes = FerrumWindow::build_word_delete_bytes(3, true);
        assert_eq!(bytes, b"\x7f\x7f\x7f");
    }

    #[test]
    fn forward_word_delete_moves_right_then_erases_with_backspace() {
        let bytes = FerrumWindow::build_forward_word_delete_bytes(3, 6);
        assert_eq!(bytes, b"\x1b[C\x1b[C\x1b[C\x7f\x7f\x7f");
    }
}
