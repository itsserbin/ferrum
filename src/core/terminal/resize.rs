//! Terminal resize logic: uses `PageList` reflow directly.

impl super::Terminal {
    pub fn resize(&mut self, rows: usize, cols: usize) {
        if self.screen.viewport_rows() == rows && self.screen.cols() == cols {
            return;
        }

        // Alt screen: simple resize (no reflow).
        if let Some(ref mut alt) = self.alt_screen {
            alt.simple_resize(rows, cols);
        }

        let old_cols = self.screen.cols();
        if old_cols != cols && self.alt_screen.is_none() {
            // cursor_pin is always current (every cursor movement updates it),
            // so reflow can use it directly without a prior sync.
            // reflow() places the cursor at the last viewport row with the column
            // clamped to new_cols-1, so no post-reflow cursor adjustment is needed.
            self.screen.reflow(rows, cols, &self.cursor_pin);
        } else {
            // Height-only resize or alt-screen present: use simple resize logic.
            self.simple_resize(rows, cols);
        }

        // Reset scroll region to full screen.
        self.scroll_top = 0;
        self.scroll_bottom = rows - 1;

        self.resize_at = Some(std::time::Instant::now());
    }

    /// Resize when only the height changes (no col change, no reflow).
    ///
    /// When the terminal shrinks, rows are pushed from the top of the viewport
    /// into scrollback until the cursor is within the new height.  This keeps
    /// the cursor at its logical position in the content rather than clamping
    /// it, which would cause the shell to overwrite content rows on the next
    /// prompt redraw.  When the terminal grows, blank rows are added at the
    /// bottom.
    fn simple_resize(&mut self, rows: usize, cols: usize) {
        let old_rows = self.screen.viewport_rows();
        if self.alt_screen.is_none() && rows < old_rows && self.cursor_row() >= rows {
            // Push enough rows from the top to scrollback so the cursor fits.
            let excess = self.cursor_row() + 1 - rows;
            for _ in 0..excess {
                self.screen.scroll_up_region(0, old_rows - 1, true);
            }
            // When scrollback has space, each scroll decreases cursor_row() by 1
            // automatically (viewport_start_abs increases). When scrollback is full,
            // cursor_row() does not decrease — clamp explicitly.
            if self.cursor_row() >= rows {
                self.set_cursor_row(rows.saturating_sub(1));
            }
        }
        self.screen.simple_resize(rows, cols);
        let new_row = self.cursor_row().min(rows.saturating_sub(1));
        let new_col = self.cursor_col().min(cols.saturating_sub(1));
        self.set_cursor(new_row, new_col);
    }
}
