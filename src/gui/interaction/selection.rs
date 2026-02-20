use crate::core::SelectionPoint;
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
        let grid = &leaf.terminal.grid;
        if row >= grid.rows || col >= grid.cols {
            return None;
        }

        // Safe: bounds checked above
        let ch = grid.get_unchecked(row, col).character;
        if !Self::is_word_char(ch) {
            let pos = Position { row, col };
            return Some((pos, pos));
        }

        let mut start_col = col;
        // Safe: start_col - 1 >= 0, and row is in bounds
        while start_col > 0 && Self::is_word_char(grid.get_unchecked(row, start_col - 1).character)
        {
            start_col -= 1;
        }

        let mut end_col = col;
        // Safe: end_col + 1 < grid.cols, and row is in bounds
        while end_col + 1 < grid.cols
            && Self::is_word_char(grid.get_unchecked(row, end_col + 1).character)
        {
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
        let grid = &leaf.terminal.grid;
        if row >= grid.rows || grid.cols == 0 {
            return None;
        }
        Some((
            Position { row, col: 0 },
            Position {
                row,
                col: grid.cols - 1,
            },
        ))
    }

    fn viewport_start(&self) -> usize {
        match self.active_leaf_ref() {
            Some(leaf) => leaf
                .terminal
                .scrollback
                .len()
                .saturating_sub(leaf.scroll_offset),
            None => 0,
        }
    }

    pub(in crate::gui) fn screen_to_abs(&self, screen_row: usize) -> usize {
        self.viewport_start() + screen_row
    }

    fn pos_to_abs(&self, pos: Position) -> SelectionPoint {
        SelectionPoint {
            row: self.screen_to_abs(pos.row),
            col: pos.col,
        }
    }

    pub(in crate::gui) fn select_word_at(&mut self, row: usize, col: usize) {
        let Some((start, end)) = self.word_bounds_at(row, col) else {
            return;
        };
        let abs_start = self.pos_to_abs(start);
        let abs_end = self.pos_to_abs(end);
        if let Some(leaf) = self.active_leaf_mut() {
            leaf.selection = Some(Selection {
                start: abs_start,
                end: abs_end,
            });
        }
    }

    pub(in crate::gui) fn select_line_at(&mut self, row: usize) {
        let Some((start, end)) = self.line_bounds_at(row) else {
            return;
        };
        let abs_start = self.pos_to_abs(start);
        let abs_end = self.pos_to_abs(end);
        if let Some(leaf) = self.active_leaf_mut() {
            leaf.selection = Some(Selection {
                start: abs_start,
                end: abs_end,
            });
        }
    }

    pub(in crate::gui) fn update_drag_selection(&mut self, row: usize, col: usize) {
        let (max_row, max_col, existing_selection) = match self.active_leaf_ref() {
            Some(leaf) => (
                leaf.terminal.grid.rows.saturating_sub(1),
                leaf.terminal.grid.cols.saturating_sub(1),
                leaf.selection,
            ),
            None => return,
        };

        let vp = self.viewport_start();

        // Anchor is already in absolute coords
        let mut anchor = self
            .selection_anchor
            .unwrap_or(SelectionPoint { row: vp + row, col });
        // Clamp anchor col (row is absolute, no clamping to screen max_row)
        anchor.col = anchor.col.min(max_col);

        let abs_row = vp + row.min(max_row);
        let current = SelectionPoint {
            row: abs_row,
            col: col.min(max_col),
        };
        let mode = self.selection_drag_mode;

        if mode == SelectionDragMode::Character && current == anchor && existing_selection.is_none()
        {
            return;
        }

        // Convert anchor back to screen-relative for word_bounds_at/line_bounds_at
        let anchor_screen_row = anchor.row.saturating_sub(vp).min(max_row);
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
            SelectionDragMode::Line => {
                if current.row < anchor.row {
                    Selection {
                        start: SelectionPoint {
                            row: current.row,
                            col: 0,
                        },
                        end: SelectionPoint {
                            row: anchor.row,
                            col: max_col,
                        },
                    }
                } else {
                    Selection {
                        start: SelectionPoint {
                            row: anchor.row,
                            col: 0,
                        },
                        end: SelectionPoint {
                            row: current.row,
                            col: max_col,
                        },
                    }
                }
            }
        };

        if let Some(leaf) = self.active_leaf_mut() {
            leaf.selection = Some(selection);
        }
    }
}
