//! Grid operations: scrolling, display construction, and alt-screen query.

use crate::core::{Cell, Grid, Row};

impl super::Terminal {
    /// Returns whether the terminal is in the alternate screen.
    pub fn is_alt_screen(&self) -> bool {
        self.alt_screen.is_some()
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
        let to_scrollback = top == 0 && self.alt_screen.is_none();

        // ── screen (PageList) ────────────────────────────────────────────────
        self.screen.scroll_up_region(top, bottom, to_scrollback);
        // Apply current blank cell colours to the newly cleared bottom row.
        let blank_gc = self.make_blank_grapheme_cell();
        let cols = self.screen.cols();
        for col in 0..cols {
            self.screen.viewport_set(bottom, col, blank_gc.clone());
        }
        self.screen.viewport_set_wrapped(bottom, false);

        // ── grid (display cache) ─────────────────────────────────────────────
        if top == 0 && self.alt_grid.is_none() {
            self.scrollback.push_back(Row::from_cells(
                self.grid.row_slice(0).to_vec(),
                self.grid.is_wrapped(0),
            ));
            if self.scrollback.len() > self.max_scrollback {
                self.scrollback.pop_front();
            }
        }
        for row in (top + 1)..=bottom {
            self.grid.copy_row_within(row, row - 1);
        }
        let blank = self.make_blank_cell();
        for col in 0..self.grid.cols {
            self.grid.set(bottom, col, blank.clone());
        }
        self.grid.set_wrapped(bottom, false);
    }

    pub(super) fn scroll_down_region(&mut self, top: usize, bottom: usize) {
        // ── screen (PageList) ────────────────────────────────────────────────
        self.screen.scroll_down_region(top, bottom);
        let blank_gc = self.make_blank_grapheme_cell();
        let cols = self.screen.cols();
        for col in 0..cols {
            self.screen.viewport_set(top, col, blank_gc.clone());
        }
        self.screen.viewport_set_wrapped(top, false);

        // ── grid (display cache) ─────────────────────────────────────────────
        for row in (top..bottom).rev() {
            self.grid.copy_row_within(row, row + 1);
        }
        let blank = self.make_blank_cell();
        for col in 0..self.grid.cols {
            self.grid.set(top, col, blank.clone());
        }
        self.grid.set_wrapped(top, false);
    }
}

#[cfg(test)]
mod tests {
    use super::super::Terminal;

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
    fn resize_sets_correct_dimensions() {
        let mut term = Terminal::new(4, 10);
        term.process(b"Test content");

        term.resize(6, 20);

        assert_eq!(term.grid.rows, 6);
        assert_eq!(term.grid.cols, 20);
    }

    #[test]
    fn resize_clamps_cursor() {
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
    fn alt_screen_resize_does_not_grow_scrollback() {
        let mut term = Terminal::new(4, 10);
        term.process(b"Main screen");

        term.process(b"\x1b[?1049h");
        term.process(b"Alt content");

        let scrollback_before = term.scrollback.len();

        term.resize(4, 15);

        assert_eq!(term.scrollback.len(), scrollback_before);
    }
}
