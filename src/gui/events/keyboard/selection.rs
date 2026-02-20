use crate::core::Selection;
use crate::gui::*;

use super::word_motion::HorizontalMotion;

impl FerrumWindow {
    pub(super) fn is_plain_shift_selection_combo(modifiers: ModifiersState) -> bool {
        modifiers.shift_key()
            && !modifiers.control_key()
            && !modifiers.alt_key()
            && !modifiers.super_key()
    }

    pub(super) fn is_native_word_selection_combo(modifiers: ModifiersState) -> bool {
        if !modifiers.shift_key() || modifiers.super_key() {
            return false;
        }

        #[cfg(target_os = "macos")]
        {
            modifiers.alt_key() && !modifiers.control_key()
        }

        #[cfg(not(target_os = "macos"))]
        {
            modifiers.control_key() && !modifiers.alt_key()
        }
    }

    pub(super) fn selection_from_cursor_bounds(
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

    pub(super) fn handle_shift_arrow_selection(&mut self, key: &Key) -> bool {
        let motion = match key {
            Key::Named(NamedKey::ArrowLeft) => HorizontalMotion::Left,
            Key::Named(NamedKey::ArrowRight) => HorizontalMotion::Right,
            _ => return false,
        };

        let modifiers = self.modifiers;
        let word_motion = Self::is_native_word_selection_combo(modifiers);
        if !Self::is_plain_shift_selection_combo(modifiers) && !word_motion {
            return false;
        }

        let (abs_row, anchor_col, cursor_col, target_col, grid_cols) = {
            let Some(leaf) = self.active_leaf_ref() else {
                return false;
            };
            if leaf.terminal.is_alt_screen() {
                return false;
            }

            let grid_cols = leaf.terminal.grid.cols;
            if grid_cols == 0 {
                return false;
            }

            let cursor_col = leaf.terminal.cursor_col.min(grid_cols);
            let target_col = if word_motion {
                Self::word_motion_target_col_from_leaf(leaf, cursor_col, motion)
            } else {
                match motion {
                    HorizontalMotion::Left => cursor_col.saturating_sub(1),
                    HorizontalMotion::Right => (cursor_col + 1).min(grid_cols),
                }
            };

            let abs_row = leaf.terminal.scrollback.len() + leaf.terminal.cursor_row;
            let anchor_col = self
                .keyboard_selection_anchor
                .filter(|anchor| anchor.row == abs_row)
                .map(|anchor| anchor.col)
                .unwrap_or(cursor_col);

            (abs_row, anchor_col, cursor_col, target_col, grid_cols)
        };

        let bytes = if word_motion {
            // Keep cursor and local selection in lock-step: synthesize character-wise arrows
            // for word-jumps instead of relying on shell-specific Meta+F/B semantics.
            Self::build_horizontal_cursor_move_bytes(cursor_col, target_col)
        } else {
            match motion {
                HorizontalMotion::Left => b"\x1b[D".to_vec(),
                HorizontalMotion::Right => b"\x1b[C".to_vec(),
            }
        };

        if !bytes.is_empty()
            && let Some(leaf) = self.active_leaf_mut()
        {
            leaf.scroll_offset = 0;
            let _ = leaf.pty_writer.write_all(&bytes);
            let _ = leaf.pty_writer.flush();
        }

        self.keyboard_selection_anchor = Some(crate::core::SelectionPoint {
            row: abs_row,
            col: anchor_col,
        });

        if let Some(leaf) = self.active_leaf_mut() {
            leaf.selection =
                Self::selection_from_cursor_bounds(abs_row, anchor_col, target_col, grid_cols);
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
    fn plain_shift_selection_combo_is_detected() {
        assert!(FerrumWindow::is_plain_shift_selection_combo(mods(
            false, true, false
        )));
        assert!(!FerrumWindow::is_plain_shift_selection_combo(mods(
            true, true, false
        )));
        assert!(!FerrumWindow::is_plain_shift_selection_combo(mods(
            false, true, true
        )));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_word_selection_combo_uses_alt_shift_on_macos() {
        assert!(FerrumWindow::is_native_word_selection_combo(mods(
            false, true, true
        )));
        assert!(!FerrumWindow::is_native_word_selection_combo(mods(
            true, true, false
        )));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn native_word_selection_combo_uses_ctrl_shift_off_macos() {
        assert!(FerrumWindow::is_native_word_selection_combo(mods(
            true, true, false
        )));
        assert!(!FerrumWindow::is_native_word_selection_combo(mods(
            false, true, true
        )));
    }
}
