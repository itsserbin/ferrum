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
            // Sync cursor_pin to the current cursor position before reflow.
            // The pin is not updated during ordinary scrolling or cursor
            // movement — only at resize boundaries — so it may be stale.
            // Without this sync, reflow finds the cursor at the wrong row and
            // the shell redraws its prompt at the wrong position after SIGWINCH.
            let cur_abs = self.screen.viewport_start_abs() + self.cursor_row;
            self.screen.set_pin_abs_row(&self.cursor_pin, cur_abs);
            self.screen.set_pin_col(&self.cursor_pin, self.cursor_col);

            // Reflow resize: run grapheme-aware reflow on the PageList directly.
            self.screen.reflow(rows, cols, &self.cursor_pin);
            self.update_cursor_after_resize(rows, cols);
        } else {
            // Height-only resize or alt-screen present: use simple resize logic.
            self.simple_resize(rows, cols);
        }

        // Reset scroll region to full screen.
        self.scroll_top = 0;
        self.scroll_bottom = rows - 1;

        self.resize_at = Some(std::time::Instant::now());
    }

    /// Update `cursor_row` and `cursor_col` from the cursor pin after a reflow resize.
    fn update_cursor_after_resize(&mut self, new_rows: usize, new_cols: usize) {
        let coord = self.screen.pin_coord(&self.cursor_pin);
        let vstart = self.screen.viewport_start_abs();
        self.cursor_row =
            coord.abs_row.saturating_sub(vstart).min(new_rows.saturating_sub(1));
        self.cursor_col = coord.col.min(new_cols.saturating_sub(1));
        let new_abs = vstart + self.cursor_row;
        self.screen.set_pin_abs_row(&self.cursor_pin, new_abs);
        self.screen.set_pin_col(&self.cursor_pin, self.cursor_col);
    }

    /// Resize when only the height changes (no col change, no reflow).
    ///
    /// When the terminal shrinks, rows are discarded from the top only as
    /// needed to keep the cursor visible — if there is empty space below the
    /// cursor those rows are simply dropped and the cursor stays in place.
    /// When the terminal grows, blank rows are added at the bottom.
    fn simple_resize(&mut self, rows: usize, cols: usize) {
        self.screen.simple_resize(rows, cols);
        // Update cursor position to stay within bounds.
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
        // Update cursor pin.
        let abs = self.screen.viewport_start_abs() + self.cursor_row;
        self.screen.set_pin_abs_row(&self.cursor_pin, abs);
        self.screen.set_pin_col(&self.cursor_pin, self.cursor_col);
    }
}
