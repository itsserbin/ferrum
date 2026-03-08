//! Grid operations: scrolling, display construction, and alt-screen query.

impl super::Terminal {
    /// Returns whether the terminal is in the alternate screen.
    pub fn is_alt_screen(&self) -> bool {
        self.alt_screen.is_some()
    }

    pub(super) fn scroll_up_region(&mut self, top: usize, bottom: usize) {
        let to_scrollback = top == 0 && self.alt_screen.is_none();

        self.screen.scroll_up_region(top, bottom, to_scrollback);
        // Apply current blank cell colours to the newly cleared bottom row.
        let blank_gc = self.make_blank_grapheme_cell();
        let cols = self.screen.cols();
        for col in 0..cols {
            self.screen.viewport_set(bottom, col, blank_gc.clone());
        }
        self.screen.viewport_set_wrapped(bottom, false);
    }

    pub(super) fn scroll_down_region(&mut self, top: usize, bottom: usize) {
        self.screen.scroll_down_region(top, bottom);
        let blank_gc = self.make_blank_grapheme_cell();
        let cols = self.screen.cols();
        for col in 0..cols {
            self.screen.viewport_set(top, col, blank_gc.clone());
        }
        self.screen.viewport_set_wrapped(top, false);
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

        assert_eq!(term.screen.viewport_get(0, 0).grapheme(), "T");
        assert_eq!(term.screen.viewport_get(0, 1).grapheme(), "e");
        assert_eq!(term.screen.viewport_get(0, 2).grapheme(), "s");
        assert_eq!(term.screen.viewport_get(0, 3).grapheme(), "t");
    }

    #[test]
    fn resize_sets_correct_dimensions() {
        let mut term = Terminal::new(4, 10);
        term.process(b"Test content");

        term.resize(6, 20);

        assert_eq!(term.screen.viewport_rows(), 6);
        assert_eq!(term.screen.cols(), 20);
    }

    #[test]
    fn resize_clamps_cursor() {
        let mut term = Terminal::new(10, 20);
        term.set_cursor(8, 15);

        term.resize(5, 10);

        assert!(
            term.cursor_row() < term.screen.viewport_rows(),
            "cursor_row {} should be < viewport_rows {}",
            term.cursor_row(),
            term.screen.viewport_rows()
        );
        assert!(
            term.cursor_col() < term.screen.cols(),
            "cursor_col {} should be < cols {}",
            term.cursor_col(),
            term.screen.cols()
        );
    }

    #[test]
    fn alt_screen_resize_does_not_grow_scrollback() {
        let mut term = Terminal::new(4, 10);
        term.process(b"Main screen");

        term.process(b"\x1b[?1049h");
        term.process(b"Alt content");

        let scrollback_before = term.screen.scrollback_len();

        term.resize(4, 15);

        assert_eq!(term.screen.scrollback_len(), scrollback_before);
    }
}
