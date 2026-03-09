use crate::core::PageCoord;
use crate::gui::*;
use std::cmp::Ordering;

impl FerrumWindow {
    fn compare_pos(a: Position, b: Position) -> Ordering {
        match a.row.cmp(&b.row) {
            Ordering::Equal => a.col.cmp(&b.col),
            other => other,
        }
    }

    fn is_word_char(ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.' || ch == '/'
    }

    fn word_bounds_at(&self, row: usize, col: usize) -> Option<(Position, Position)> {
        let leaf = self.active_leaf_ref()?;
        let vp_rows = leaf.terminal.screen.viewport_rows();
        let vp_cols = leaf.terminal.screen.cols();
        if row >= vp_rows || col >= vp_cols {
            return None;
        }

        let ch = leaf.terminal.screen.viewport_get(row, col).first_char();
        if !Self::is_word_char(ch) {
            let pos = Position { row, col };
            return Some((pos, pos));
        }

        let mut start_col = col;
        while start_col > 0 {
            let prev_ch = leaf.terminal.screen.viewport_get(row, start_col - 1).first_char();
            if !Self::is_word_char(prev_ch) {
                break;
            }
            start_col -= 1;
        }

        let mut end_col = col;
        while end_col + 1 < vp_cols {
            let next_ch = leaf.terminal.screen.viewport_get(row, end_col + 1).first_char();
            if !Self::is_word_char(next_ch) {
                break;
            }
            end_col += 1;
        }

        Some((
            Position {
                row,
                col: start_col,
            },
            Position { row, col: end_col },
        ))
    }

    fn line_bounds_at(&self, row: usize) -> Option<(Position, Position)> {
        let leaf = self.active_leaf_ref()?;
        let vp_rows = leaf.terminal.screen.viewport_rows();
        let vp_cols = leaf.terminal.screen.cols();
        if row >= vp_rows || vp_cols == 0 {
            return None;
        }
        Some((
            Position { row, col: 0 },
            Position {
                row,
                col: vp_cols - 1,
            },
        ))
    }

    fn viewport_start(&self) -> usize {
        match self.active_leaf_ref() {
            Some(leaf) => leaf
                .terminal
                .screen
                .scrollback_len()
                .saturating_sub(leaf.scroll_offset),
            None => 0,
        }
    }

    pub(in crate::gui) fn screen_to_abs(&self, screen_row: usize) -> usize {
        self.viewport_start() + screen_row
    }

    fn pos_to_abs(&self, pos: Position) -> PageCoord {
        PageCoord {
            abs_row: self.screen_to_abs(pos.row),
            col: pos.col,
        }
    }

    fn set_selection_from_positions(&mut self, start: Position, end: Position) {
        let abs_start = self.pos_to_abs(start);
        let abs_end = self.pos_to_abs(end);
        if let Some(leaf) = self.active_leaf_mut() {
            leaf.set_selection(Selection {
                start: abs_start,
                end: abs_end,
            });
        }
    }

    pub(in crate::gui) fn select_word_at(&mut self, row: usize, col: usize) {
        let Some((start, end)) = self.word_bounds_at(row, col) else {
            return;
        };
        self.set_selection_from_positions(start, end);
    }

    pub(in crate::gui) fn select_line_at(&mut self, row: usize) {
        let Some((start, end)) = self.line_bounds_at(row) else {
            return;
        };
        self.set_selection_from_positions(start, end);
    }

    pub(in crate::gui) fn update_drag_selection(&mut self, row: usize, col: usize) {
        let (max_row, max_col, existing_selection) = match self.active_leaf_ref() {
            Some(leaf) => (
                leaf.terminal.screen.viewport_rows().saturating_sub(1),
                leaf.terminal.screen.cols().saturating_sub(1),
                leaf.selection,
            ),
            None => return,
        };

        let vp = self.viewport_start();

        // Anchor is already in absolute coords
        let mut anchor = self
            .selection_anchor
            .unwrap_or(PageCoord { abs_row: vp + row, col });
        // Clamp anchor col (abs_row is absolute, no clamping to screen max_row)
        anchor.col = anchor.col.min(max_col);

        let abs_row = vp + row.min(max_row);
        let current = PageCoord {
            abs_row,
            col: col.min(max_col),
        };
        let mode = self.selection_drag_mode;

        if mode == SelectionDragMode::Character && current == anchor && existing_selection.is_none()
        {
            return;
        }

        // Convert anchor back to screen-relative for word_bounds_at/line_bounds_at
        let anchor_screen_row = anchor.abs_row.saturating_sub(vp).min(max_row);
        let current_screen_row = row.min(max_row);

        let selection = match mode {
            SelectionDragMode::Character => Selection {
                start: anchor,
                end: current,
            },
            SelectionDragMode::Word => {
                let anchor_screen = Position {
                    row: anchor_screen_row,
                    col: anchor.col,
                };
                let current_screen = Position {
                    row: current_screen_row,
                    col: current.col,
                };
                let (anchor_start, anchor_end) = self
                    .word_bounds_at(anchor_screen.row, anchor_screen.col)
                    .unwrap_or((anchor_screen, anchor_screen));
                let (current_start, current_end) = self
                    .word_bounds_at(current_screen.row, current_screen.col)
                    .unwrap_or((current_screen, current_screen));

                if Self::compare_pos(current_start, anchor_start) == Ordering::Less {
                    Selection {
                        start: self.pos_to_abs(current_start),
                        end: self.pos_to_abs(anchor_end),
                    }
                } else {
                    Selection {
                        start: self.pos_to_abs(anchor_start),
                        end: self.pos_to_abs(current_end),
                    }
                }
            }
            SelectionDragMode::Line => Selection {
                start: PageCoord { abs_row: current.abs_row.min(anchor.abs_row), col: 0 },
                end: PageCoord { abs_row: current.abs_row.max(anchor.abs_row), col: max_col },
            },
        };

        if let Some(leaf) = self.active_leaf_mut() {
            leaf.set_selection(selection);
        }
    }
}
