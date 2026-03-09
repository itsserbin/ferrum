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

    /// `resize_no_reflow` must not alter the positions of characters that were
    /// already on screen — the content stays in the same cells even though the
    /// column count changes.  This verifies the fix for the "text shifts during
    /// window drag" artefact: when the window is resized interactively we skip
    /// reflow so the buffer stays visually stable until the shell redraws via
    /// SIGWINCH.
    #[test]
    fn resize_no_reflow_preserves_cell_positions() {
        // 4 rows × 6 cols.  Write "Hello" on row 0.
        let mut term = Terminal::new(4, 6);
        term.process(b"Hello");

        // Widen to 12 cols without reflow.
        term.resize_no_reflow(4, 12);

        // Characters must still be at their original column positions.
        assert_eq!(term.screen.viewport_rows(), 4);
        assert_eq!(term.screen.cols(), 12);
        assert_eq!(term.screen.viewport_get(0, 0).grapheme(), "H");
        assert_eq!(term.screen.viewport_get(0, 1).grapheme(), "e");
        assert_eq!(term.screen.viewport_get(0, 2).grapheme(), "l");
        assert_eq!(term.screen.viewport_get(0, 3).grapheme(), "l");
        assert_eq!(term.screen.viewport_get(0, 4).grapheme(), "o");
    }

    /// `resize` (with reflow) and `resize_no_reflow` produce a different layout
    /// when soft-wrapped content is present.  After widening the terminal,
    /// `resize` will reflow soft-wrapped lines so they occupy fewer rows;
    /// `resize_no_reflow` keeps each character at its original cell coordinate.
    #[test]
    fn resize_no_reflow_differs_from_resize_for_wrapped_content() {
        // 4 rows × 4 cols.  "ABCDE" wraps: row 0 = "ABCD", row 1 = "E".
        let mut term_reflow = Terminal::new(4, 4);
        term_reflow.process(b"ABCDE");

        let mut term_no_reflow = Terminal::new(4, 4);
        term_no_reflow.process(b"ABCDE");

        term_reflow.resize(4, 8);
        term_no_reflow.resize_no_reflow(4, 8);

        // After no-reflow: 'E' is still at (row 1, col 0) where it was before.
        assert_eq!(term_no_reflow.screen.viewport_get(1, 0).grapheme(), "E");

        // After reflow: the wrapped 'E' has moved off row 1 col 0 (merged with ABCD).
        // We verify the two paths diverge — the exact new position of 'E' is an
        // implementation detail of the reflow algorithm.
        assert_ne!(
            term_reflow.screen.viewport_get(1, 0).grapheme(),
            "E",
            "reflow should have moved 'E' off row 1 col 0"
        );
    }
}
