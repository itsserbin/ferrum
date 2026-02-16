mod entry;
mod forward;
mod rename;
mod shortcuts;

use crate::gui::*;

#[derive(Clone, Copy)]
enum HorizontalMotion {
    Left,
    Right,
}

impl FerrumWindow {
    fn copy_selection_and_clear(&mut self) {
        self.copy_selection();
        if let Some(tab) = self.active_tab_mut() {
            tab.selection = None;
        }
        self.keyboard_selection_anchor = None;
    }

    fn write_pty_bytes(&mut self, bytes: &[u8]) {
        if let Some(tab) = self.active_tab_mut() {
            tab.scroll_offset = 0;
            tab.selection = None;
            let _ = tab.pty_writer.write_all(bytes);
            let _ = tab.pty_writer.flush();
        }
        self.keyboard_selection_anchor = None;
    }

    fn is_word_char_for_motion(ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.' || ch == '/'
    }

    fn word_motion_target_col_for_line(
        line: &[char],
        cursor_col: usize,
        motion: HorizontalMotion,
    ) -> usize {
        let cols = line.len();
        if cols == 0 {
            return 0;
        }

        match motion {
            HorizontalMotion::Left => {
                if cursor_col == 0 {
                    return 0;
                }

                let mut idx = cursor_col.saturating_sub(1).min(cols.saturating_sub(1));
                while idx > 0 && !Self::is_word_char_for_motion(line[idx]) {
                    idx -= 1;
                }
                if idx == 0 && !Self::is_word_char_for_motion(line[idx]) {
                    return 0;
                }
                while idx > 0 && Self::is_word_char_for_motion(line[idx - 1]) {
                    idx -= 1;
                }
                idx
            }
            HorizontalMotion::Right => {
                let mut idx = cursor_col.min(cols);
                while idx < cols && !Self::is_word_char_for_motion(line[idx]) {
                    idx += 1;
                }
                while idx < cols && Self::is_word_char_for_motion(line[idx]) {
                    idx += 1;
                }
                idx
            }
        }
    }

    fn word_motion_target_col(
        tab: &TabState,
        cursor_col: usize,
        motion: HorizontalMotion,
    ) -> usize {
        let rows = tab.terminal.grid.rows;
        let cols = tab.terminal.grid.cols;
        if rows == 0 || cols == 0 {
            return 0;
        }

        let row = tab.terminal.cursor_row.min(rows.saturating_sub(1));
        let mut line = Vec::with_capacity(cols);
        for col in 0..cols {
            line.push(tab.terminal.grid.get(row, col).character);
        }

        Self::word_motion_target_col_for_line(&line, cursor_col, motion)
    }

    fn selection_from_cursor_bounds(
        abs_row: usize,
        anchor_col: usize,
        cursor_col: usize,
        grid_cols: usize,
    ) -> Option<Selection> {
        if grid_cols == 0 || anchor_col == cursor_col {
            return None;
        }

        let start_bound = anchor_col.min(cursor_col);
        let end_bound = anchor_col.max(cursor_col);
        if end_bound == 0 {
            return None;
        }

        let max_col = grid_cols.saturating_sub(1);
        let start_col = start_bound.min(max_col);
        let end_col = end_bound.saturating_sub(1).min(max_col);
        if start_col > end_col {
            return None;
        }

        Some(Selection {
            start: crate::core::SelectionPoint {
                row: abs_row,
                col: start_col,
            },
            end: crate::core::SelectionPoint {
                row: abs_row,
                col: end_col,
            },
        })
    }

    fn handle_shift_arrow_selection(&mut self, key: &Key) -> bool {
        let motion = match key {
            Key::Named(NamedKey::ArrowLeft) => HorizontalMotion::Left,
            Key::Named(NamedKey::ArrowRight) => HorizontalMotion::Right,
            _ => return false,
        };

        if !self.modifiers.shift_key() || self.modifiers.control_key() || self.modifiers.super_key()
        {
            return false;
        }

        let alt_word_motion = self.modifiers.alt_key();
        let (abs_row, anchor_col, target_col, grid_cols, bytes) = {
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
            let target_col = if alt_word_motion {
                Self::word_motion_target_col(tab, cursor_col, motion)
            } else {
                match motion {
                    HorizontalMotion::Left => cursor_col.saturating_sub(1),
                    HorizontalMotion::Right => (cursor_col + 1).min(grid_cols),
                }
            };

            let abs_row = tab.terminal.scrollback.len() + tab.terminal.cursor_row;
            let anchor_col = self
                .keyboard_selection_anchor
                .filter(|anchor| anchor.row == abs_row)
                .map(|anchor| anchor.col)
                .unwrap_or(cursor_col);
            let bytes: &'static [u8] = match (alt_word_motion, motion) {
                (false, HorizontalMotion::Left) => b"\x1b[D",
                (false, HorizontalMotion::Right) => b"\x1b[C",
                (true, HorizontalMotion::Left) => b"\x1bb",
                (true, HorizontalMotion::Right) => b"\x1bf",
            };

            (abs_row, anchor_col, target_col, grid_cols, bytes)
        };

        if let Some(tab) = self.active_tab_mut() {
            tab.scroll_offset = 0;
            let _ = tab.pty_writer.write_all(bytes);
            let _ = tab.pty_writer.flush();
        }

        self.keyboard_selection_anchor = Some(crate::core::SelectionPoint {
            row: abs_row,
            col: anchor_col,
        });

        if let Some(tab) = self.active_tab_mut() {
            tab.selection =
                Self::selection_from_cursor_bounds(abs_row, anchor_col, target_col, grid_cols);
        }

        true
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
    use super::HorizontalMotion;
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

    #[test]
    fn selection_from_cursor_bounds_selects_left_character() {
        let selection = FerrumWindow::selection_from_cursor_bounds(12, 5, 4, 80)
            .expect("selection should exist");
        assert_eq!(selection.start.row, 12);
        assert_eq!(selection.end.row, 12);
        assert_eq!(selection.start.col, 4);
        assert_eq!(selection.end.col, 4);
    }

    #[test]
    fn selection_from_cursor_bounds_selects_right_character() {
        let selection = FerrumWindow::selection_from_cursor_bounds(2, 2, 3, 80)
            .expect("selection should exist");
        assert_eq!(selection.start.col, 2);
        assert_eq!(selection.end.col, 2);
    }

    #[test]
    fn selection_from_cursor_bounds_is_none_without_span() {
        let selection = FerrumWindow::selection_from_cursor_bounds(0, 7, 7, 80);
        assert!(selection.is_none());
    }

    #[test]
    fn word_motion_left_skips_delimiters_then_word() {
        let line: Vec<char> = "alpha  beta".chars().collect();
        let target =
            FerrumWindow::word_motion_target_col_for_line(&line, 11, HorizontalMotion::Left);
        assert_eq!(target, 7);
    }

    #[test]
    fn word_motion_right_moves_to_end_of_next_word() {
        let line: Vec<char> = "alpha  beta".chars().collect();
        let target =
            FerrumWindow::word_motion_target_col_for_line(&line, 0, HorizontalMotion::Right);
        assert_eq!(target, 5);
    }
}
