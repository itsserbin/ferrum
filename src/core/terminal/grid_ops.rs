use crate::core::{Cell, Grid, Row};

use super::CursorStyle;

impl super::Terminal {
    pub fn resize(&mut self, rows: usize, cols: usize) {
        if self.grid.rows == rows && self.grid.cols == cols {
            return;
        }

        // Alt grid: simple resize (no reflow)
        if let Some(ref mut alt) = self.alt_grid {
            *alt = alt.resized(rows, cols);
        }

        // Main grid resize
        // On Unix (macOS/Linux), shell handles resize via SIGWINCH - use simple resize
        // On Windows, ConPTY doesn't redraw content - use reflow to preserve it
        #[cfg(windows)]
        {
            let old_cols = self.grid.cols;
            if old_cols != cols && self.alt_grid.is_none() {
                self.reflow_resize(rows, cols);
            } else {
                self.simple_resize(rows, cols);
            }
        }
        #[cfg(not(windows))]
        {
            self.simple_resize(rows, cols);
        }

        // ── Reset scroll region to full screen ──
        self.scroll_top = 0;
        self.scroll_bottom = rows - 1;

        self.resize_at = Some(std::time::Instant::now());
    }

    /// Simple resize without reflow (height-only changes or alt screen).
    fn simple_resize(&mut self, rows: usize, cols: usize) {
        let old_rows = self.grid.rows;
        let is_alt = self.alt_grid.is_some();

        // Vertical shrink: if cursor would be outside, push top rows to scrollback
        if rows < old_rows && self.cursor_row >= rows {
            let shift = self.cursor_row - rows + 1;
            if !is_alt {
                for r in 0..shift {
                    let cells = self.grid.row_cells(r);
                    let wrapped = self.grid.is_wrapped(r);
                    self.scrollback.push_back(Row::from_cells(cells, wrapped));
                    if self.scrollback.len() > self.max_scrollback {
                        self.scrollback.pop_front();
                    }
                }
            }
            self.grid.shift_up(shift);
            self.cursor_row -= shift;
        }

        self.grid = self.grid.resized(rows, cols);
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
    }

    /// Reflow resize: reflow content to new width, preserving cursor position.
    ///
    /// Strategy:
    /// 1. Collect scrollback + grid rows up to cursor into logical lines
    /// 2. Rewrap logical lines to new column width
    /// 3. Fill grid from top, excess goes to scrollback
    /// 4. Cursor stays at same logical position
    fn reflow_resize(&mut self, rows: usize, cols: usize) {
        // Only collect rows up to and including cursor row (not empty trailing rows)
        let content_rows = self.cursor_row + 1;

        // 1. Collect content into logical lines
        let mut lines: Vec<Vec<Cell>> = Vec::new();
        let mut current_line: Vec<Cell> = Vec::new();

        // First from scrollback
        for row in self.scrollback.iter() {
            current_line.extend(row.cells.iter().cloned());
            if !row.wrapped {
                lines.push(std::mem::take(&mut current_line));
            }
        }

        // Then from grid (only up to cursor row)
        for r in 0..content_rows {
            current_line.extend(self.grid.row_cells(r));
            if !self.grid.is_wrapped(r) {
                lines.push(std::mem::take(&mut current_line));
            }
        }

        // Handle remaining content (if last row was wrapped)
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // 2. Rewrap all lines to new width
        let mut rewrapped: Vec<Row> = Vec::new();
        for line in &lines {
            // Trim trailing spaces
            let len = line
                .iter()
                .rposition(|c| c.character != ' ')
                .map(|i| i + 1)
                .unwrap_or(0);

            if len == 0 {
                rewrapped.push(Row::new(cols));
                continue;
            }

            let content = &line[..len];
            let mut pos = 0;
            while pos < content.len() {
                let end = (pos + cols).min(content.len());
                let mut cells: Vec<Cell> = content[pos..end].to_vec();
                cells.resize(cols, Cell::default());
                let wrapped = end < content.len();
                rewrapped.push(Row::from_cells(cells, wrapped));
                pos = end;
            }
        }

        // 3. Split into scrollback and grid (content stays at top)
        let total_rows = rewrapped.len();

        self.scrollback.clear();
        self.grid = Grid::new(rows, cols);

        if total_rows <= rows {
            // All content fits in grid - fill from top
            for (i, row) in rewrapped.iter().enumerate() {
                for (col, cell) in row.cells.iter().enumerate() {
                    if col < cols {
                        self.grid.set(i, col, cell.clone());
                    }
                }
                self.grid.set_wrapped(i, row.wrapped);
            }
            // Cursor at end of content
            self.cursor_row = total_rows.saturating_sub(1);
        } else {
            // Content overflows - excess goes to scrollback
            let scrollback_count = total_rows - rows;

            for row in rewrapped.iter().take(scrollback_count) {
                self.scrollback.push_back(row.clone());
                if self.scrollback.len() > self.max_scrollback {
                    self.scrollback.pop_front();
                }
            }

            // Fill grid with remaining rows
            for (i, row) in rewrapped.iter().skip(scrollback_count).enumerate() {
                for (col, cell) in row.cells.iter().enumerate() {
                    if col < cols {
                        self.grid.set(i, col, cell.clone());
                    }
                }
                self.grid.set_wrapped(i, row.wrapped);
            }
            // Cursor at bottom (content filled the grid)
            self.cursor_row = rows.saturating_sub(1);
        }

        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
    }

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
                    // Pull row from scrollback.
                    let sb_idx = self.scrollback.len().saturating_sub(scroll_offset) + row;
                    if col < self.scrollback[sb_idx].cells.len() {
                        self.scrollback[sb_idx].cells[col].clone()
                    } else {
                        Cell::default() // Width may differ after resize.
                    }
                } else {
                    // Pull row from the live grid.
                    self.grid.get(row - scroll_offset, col).clone()
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
                let cell = self.grid.get(row, col).clone();
                self.grid.set(row - 1, col, cell);
            }
            // Also copy the wrapped flag
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
                let cell = self.grid.get(row, col).clone();
                self.grid.set(row + 1, col, cell);
            }
            // Also copy the wrapped flag
            let wrapped = self.grid.is_wrapped(row);
            self.grid.set_wrapped(row + 1, wrapped);
        }
        for col in 0..self.grid.cols {
            self.grid.set(top, col, Cell::default());
        }
        self.grid.set_wrapped(top, false);
    }

    /// Enters the alternate screen buffer (used by apps like vim/htop).
    pub(super) fn enter_alt_screen(&mut self) {
        if self.alt_grid.is_none() {
            let alt = Grid::new(self.grid.rows, self.grid.cols);
            self.alt_grid = Some(std::mem::replace(&mut self.grid, alt));
            self.alt_saved_cursor = (self.cursor_row, self.cursor_col);
            self.saved_scroll_top = self.scroll_top;
            self.saved_scroll_bottom = self.scroll_bottom;
            self.cursor_row = 0;
            self.cursor_col = 0;
            self.cursor_style = CursorStyle::BlinkingBlock;
            self.scroll_top = 0;
            self.scroll_bottom = self.grid.rows - 1;
        }
    }

    /// Leaves the alternate screen and restores the main buffer.
    pub(super) fn leave_alt_screen(&mut self) {
        if let Some(main_grid) = self.alt_grid.take() {
            self.grid = main_grid;
            self.cursor_row = self.alt_saved_cursor.0;
            self.cursor_col = self.alt_saved_cursor.1;
            self.scroll_top = self.saved_scroll_top;
            self.scroll_bottom = self.saved_scroll_bottom;
            self.cursor_style = CursorStyle::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Terminal;

    /// Helper to collect all visible content (grid + scrollback) as a string
    fn collect_all_content(term: &Terminal) -> String {
        let mut content = String::new();
        // Scrollback
        for row in term.scrollback.iter() {
            for cell in &row.cells {
                if cell.character != ' ' {
                    content.push(cell.character);
                }
            }
        }
        // Grid
        for r in 0..term.grid.rows {
            for c in 0..term.grid.cols {
                let ch = term.grid.get(r, c).character;
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
        term.process(b"AAAAAAAAAA"); // row 0: 10 A's
        term.process(b"\n");
        term.cursor_col = 0;
        term.process(b"BBBBBBBBBB"); // row 1: 10 B's

        let content_before = collect_all_content(&term);

        // Width change triggers reflow
        term.resize(4, 15);

        let content_after = collect_all_content(&term);

        // Content should be preserved
        assert_eq!(content_before, content_after,
            "Content should be preserved after reflow");
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

        // Resize to smaller dimensions
        term.resize(5, 10);

        // Cursor should be clamped to valid range
        assert!(term.cursor_row < term.grid.rows,
            "cursor_row {} should be < grid.rows {}", term.cursor_row, term.grid.rows);
        assert!(term.cursor_col < term.grid.cols,
            "cursor_col {} should be < grid.cols {}", term.cursor_col, term.grid.cols);
    }

    #[test]
    fn reflow_rewraps_long_lines_to_new_width() {
        let mut term = Terminal::new(4, 10);
        // Create a long line that will wrap: 26 chars in 10-col terminal
        term.process(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ");

        // Resize to wider terminal (20 cols)
        term.resize(4, 20);

        // Content should be rewrapped: now 2 rows instead of 3
        // Check that the full alphabet is preserved
        let content = collect_all_content(&term);
        assert_eq!(content, "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "Content should be preserved after rewrap");

        // The rewrapped content should have proper wrap flags
        // First row (20 chars) should be wrapped, second (6 chars) should not
        // Content can be in grid or scrollback depending on grid size
    }

    #[test]
    fn simple_resize_height_only_preserves_grid() {
        let mut term = Terminal::new(4, 10);
        term.process(b"Test");

        // Height-only change uses simple_resize (not reflow)
        term.resize(6, 10);

        // Content should still be in grid (not moved to scrollback)
        assert_eq!(term.grid.get(0, 0).character, 'T');
        assert_eq!(term.grid.get(0, 1).character, 'e');
        assert_eq!(term.grid.get(0, 2).character, 's');
        assert_eq!(term.grid.get(0, 3).character, 't');
    }

    #[test]
    fn alt_screen_resize_does_not_reflow() {
        let mut term = Terminal::new(4, 10);
        term.process(b"Main screen");

        // Enter alt screen
        term.process(b"\x1b[?1049h");
        term.process(b"Alt content");

        let scrollback_before = term.scrollback.len();

        // Width change on alt screen should NOT trigger reflow
        term.resize(4, 15);

        // Scrollback should not have changed (alt screen doesn't push to scrollback)
        assert_eq!(term.scrollback.len(), scrollback_before);
    }
}
