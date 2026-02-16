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
        let tab = self.active_tab_ref()?;
        let grid = &tab.terminal.grid;
        if row >= grid.rows || col >= grid.cols {
            return None;
        }

        let ch = grid.get(row, col).character;
        if !Self::is_word_char(ch) {
            let pos = Position { row, col };
            return Some((pos, pos));
        }

        let mut start_col = col;
        while start_col > 0 && Self::is_word_char(grid.get(row, start_col - 1).character) {
            start_col -= 1;
        }

        let mut end_col = col;
        while end_col + 1 < grid.cols && Self::is_word_char(grid.get(row, end_col + 1).character) {
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
        let tab = self.active_tab_ref()?;
        let grid = &tab.terminal.grid;
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

    pub(in crate::gui) fn select_word_at(&mut self, row: usize, col: usize) {
        let Some((start, end)) = self.word_bounds_at(row, col) else {
            return;
        };
        if let Some(tab) = self.active_tab_mut() {
            tab.selection = Some(Selection { start, end });
        }
    }

    pub(in crate::gui) fn select_line_at(&mut self, row: usize) {
        let Some((start, end)) = self.line_bounds_at(row) else {
            return;
        };
        if let Some(tab) = self.active_tab_mut() {
            tab.selection = Some(Selection { start, end });
        }
    }

    pub(in crate::gui) fn update_drag_selection(&mut self, row: usize, col: usize) {
        let (max_row, max_col, existing_selection) = match self.active_tab_ref() {
            Some(tab) => (
                tab.terminal.grid.rows.saturating_sub(1),
                tab.terminal.grid.cols.saturating_sub(1),
                tab.selection,
            ),
            None => return,
        };
        let mut anchor = self.selection_anchor.unwrap_or(Position { row, col });
        anchor.row = anchor.row.min(max_row);
        anchor.col = anchor.col.min(max_col);
        let current = Position {
            row: row.min(max_row),
            col: col.min(max_col),
        };
        let mode = self.selection_drag_mode;

        if mode == SelectionDragMode::Character && current == anchor && existing_selection.is_none()
        {
            return;
        }

        let selection = match mode {
            SelectionDragMode::Character => Selection {
                start: anchor,
                end: current,
            },
            SelectionDragMode::Word => {
                let (anchor_start, anchor_end) = self
                    .word_bounds_at(anchor.row, anchor.col)
                    .unwrap_or((anchor, anchor));
                let (current_start, current_end) = self
                    .word_bounds_at(current.row, current.col)
                    .unwrap_or((current, current));

                if Self::compare_pos(current_start, anchor_start) == Ordering::Less {
                    Selection {
                        start: current_start,
                        end: anchor_end,
                    }
                } else {
                    Selection {
                        start: anchor_start,
                        end: current_end,
                    }
                }
            }
            SelectionDragMode::Line => {
                if current.row < anchor.row {
                    Selection {
                        start: Position {
                            row: current.row,
                            col: 0,
                        },
                        end: Position {
                            row: anchor.row,
                            col: max_col,
                        },
                    }
                } else {
                    Selection {
                        start: Position {
                            row: anchor.row,
                            col: 0,
                        },
                        end: Position {
                            row: current.row,
                            col: max_col,
                        },
                    }
                }
            }
        };

        if let Some(tab) = self.active_tab_mut() {
            tab.selection = Some(selection);
        }
    }
}
