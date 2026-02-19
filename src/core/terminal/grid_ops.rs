//! Grid operations: scrolling, display construction, and alt-screen query.

use crate::core::{Cell, Grid, Row};

impl super::Terminal {
    /// Returns whether the terminal is in the alternate screen.
    pub fn is_alt_screen(&self) -> bool {
        self.alt_grid.is_some()
    }

    /// Builds a display grid by combining scrollback with the visible grid.
    /// Called only when `scroll_offset > 0`, so the copy cost is acceptable.
    pub fn build_display(&self, scroll_offset: usize) -> Grid {
        let scroll_offset = scroll_offset.min(self.scrollback.len());
        let mut display = Grid::new(self.grid.rows, self.grid.cols);
        for row in 0..self.grid.rows {
            for col in 0..self.grid.cols {
                let cell = if row < scroll_offset {
                    let sb_idx = self.scrollback.len().saturating_sub(scroll_offset) + row;
                    if col < self.scrollback[sb_idx].cells.len() {
                        self.scrollback[sb_idx].cells[col].clone()
                    } else {
                        Cell::default()
                    }
                } else {
                    self.grid.get_unchecked(row - scroll_offset, col).clone()
                };
                display.set(row, col, cell);
            }
        }
        display
    }

    pub(super) fn scroll_up_region(&mut self, top: usize, bottom: usize) {
        // Persist the top row to scrollback only for the main screen.
        if top == 0 && self.alt_grid.is_none() {
            let cells = self.grid.row_cells(0);
            let wrapped = self.grid.is_wrapped(0);
            self.scrollback.push_back(Row::from_cells(cells, wrapped));
            if self.scrollback.len() > self.max_scrollback {
                self.scrollback.pop_front();
                self.scrollback_popped += 1;
            }
        }

        for row in (top + 1)..=bottom {
            for col in 0..self.grid.cols {
                let cell = self.grid.get_unchecked(row, col).clone();
                self.grid.set(row - 1, col, cell);
            }
            let wrapped = self.grid.is_wrapped(row);
            self.grid.set_wrapped(row - 1, wrapped);
        }
        for col in 0..self.grid.cols {
            self.grid.set(bottom, col, Cell::default());
        }
        self.grid.set_wrapped(bottom, false);
    }

    pub(super) fn scroll_down_region(&mut self, top: usize, bottom: usize) {
        for row in (top..bottom).rev() {
            for col in 0..self.grid.cols {
                let cell = self.grid.get_unchecked(row, col).clone();
                self.grid.set(row + 1, col, cell);
            }
            let wrapped = self.grid.is_wrapped(row);
            self.grid.set_wrapped(row + 1, wrapped);
        }
        for col in 0..self.grid.cols {
            self.grid.set(top, col, Cell::default());
        }
        self.grid.set_wrapped(top, false);
    }
}

#[cfg(test)]
mod tests {
    use super::super::Terminal;

    /// Helper to collect all visible content (grid + scrollback) as a string.
    fn collect_all_content(term: &Terminal) -> String {
        let mut content = String::new();
        for row in term.scrollback.iter() {
            for cell in &row.cells {
                if cell.character != ' ' {
                    content.push(cell.character);
                }
            }
        }
        for r in 0..term.grid.rows {
            for c in 0..term.grid.cols {
                let ch = term.grid.get_unchecked(r, c).character;
                if ch != ' ' {
                    content.push(ch);
                }
            }
        }
        content
    }

    #[test]
    fn reflow_preserves_content_after_width_change() {
        let mut term = Terminal::new(4, 10);
        term.process(b"AAAAAAAAAA");
        term.process(b"\n");
        term.cursor_col = 0;
        term.process(b"BBBBBBBBBB");

        let content_before = collect_all_content(&term);

        term.resize(4, 15);

        let content_after = collect_all_content(&term);

        assert_eq!(
            content_before, content_after,
            "Content should be preserved after reflow"
        );
    }

    #[test]
    fn reflow_preserves_rows_below_cursor() {
        let mut term = Terminal::new(5, 10);
        term.process(b"TOP");
        term.process(b"\x1b[4;1HLOWER");

        term.cursor_row = 1;
        term.cursor_col = 0;

        let before = collect_all_content(&term);
        assert_eq!(before, "TOPLOWER");

        term.resize(5, 12);

        let after = collect_all_content(&term);
        assert_eq!(
            after, before,
            "Rows below cursor must survive reflow resize"
        );
    }

    #[test]
    fn reflow_preserves_trailing_spaces_before_cursor() {
        let mut term = Terminal::new(4, 10);
        term.process(b"abc   ");
        assert_eq!(term.cursor_row, 0);
        assert_eq!(term.cursor_col, 6);

        term.resize(4, 12);

        assert_eq!(term.grid.get(0, 0).unwrap().character, 'a');
        assert_eq!(term.grid.get(0, 1).unwrap().character, 'b');
        assert_eq!(term.grid.get(0, 2).unwrap().character, 'c');
        assert_eq!(term.grid.get(0, 3).unwrap().character, ' ');
        assert_eq!(term.grid.get(0, 4).unwrap().character, ' ');
        assert_eq!(term.grid.get(0, 5).unwrap().character, ' ');
    }

    #[test]
    fn reflow_resize_sets_correct_dimensions() {
        let mut term = Terminal::new(4, 10);
        term.process(b"Test content");

        term.resize(6, 20);

        assert_eq!(term.grid.rows, 6);
        assert_eq!(term.grid.cols, 20);
    }

    #[test]
    fn reflow_resize_clamps_cursor() {
        let mut term = Terminal::new(10, 20);
        term.cursor_row = 8;
        term.cursor_col = 15;

        term.resize(5, 10);

        assert!(
            term.cursor_row < term.grid.rows,
            "cursor_row {} should be < grid.rows {}",
            term.cursor_row,
            term.grid.rows
        );
        assert!(
            term.cursor_col < term.grid.cols,
            "cursor_col {} should be < grid.cols {}",
            term.cursor_col,
            term.grid.cols
        );
    }

    #[test]
    fn reflow_rewraps_long_lines_to_new_width() {
        let mut term = Terminal::new(4, 10);
        term.process(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ");

        term.resize(4, 20);

        let content = collect_all_content(&term);
        assert_eq!(
            content, "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "Content should be preserved after rewrap"
        );
    }

    #[test]
    fn simple_resize_height_only_preserves_grid() {
        let mut term = Terminal::new(4, 10);
        term.process(b"Test");

        term.resize(6, 10);

        assert_eq!(term.grid.get(0, 0).unwrap().character, 'T');
        assert_eq!(term.grid.get(0, 1).unwrap().character, 'e');
        assert_eq!(term.grid.get(0, 2).unwrap().character, 's');
        assert_eq!(term.grid.get(0, 3).unwrap().character, 't');
    }

    #[test]
    fn alt_screen_resize_does_not_reflow() {
        let mut term = Terminal::new(4, 10);
        term.process(b"Main screen");

        term.process(b"\x1b[?1049h");
        term.process(b"Alt content");

        let scrollback_before = term.scrollback.len();

        term.resize(4, 15);

        assert_eq!(term.scrollback.len(), scrollback_before);
    }
}
