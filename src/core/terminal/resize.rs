//! Terminal resize logic: simple resize and reflow resize strategies.

use crate::core::{Grid, Row};

use super::reflow::rewrap_lines;

impl super::Terminal {
    pub fn resize(&mut self, rows: usize, cols: usize) {
        if self.grid.rows == rows && self.grid.cols == cols {
            return;
        }

        // Alt grid: simple resize (no reflow)
        if let Some(ref mut alt) = self.alt_grid {
            *alt = alt.resized(rows, cols);
        }

        // Main grid resize:
        // - Reflow on width changes in the main grid to preserve wrapped content
        // - Fallback to simple resize for alt grid or row-only changes
        let old_cols = self.grid.cols;
        if old_cols != cols && self.alt_grid.is_none() {
            self.reflow_resize(rows, cols);
        } else {
            self.simple_resize(rows, cols);
        }

        // Reset scroll region to full screen.
        self.scroll_top = 0;
        self.scroll_bottom = rows - 1;

        self.resize_at = Some(std::time::Instant::now());
    }

    /// Simple resize - just resize the grid, let shell handle content via SIGWINCH.
    fn simple_resize(&mut self, rows: usize, cols: usize) {
        self.grid = self.grid.resized(rows, cols);
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
    }

    /// Reflow resize: reflow content to new width, preserving cursor position.
    ///
    /// Strategy:
    /// 1. Collect scrollback + meaningful grid rows into logical lines
    /// 2. Rewrap logical lines to new column width
    /// 3. Fill grid from top, excess goes to scrollback
    /// 4. Cursor stays at same logical position
    fn reflow_resize(&mut self, new_rows: usize, new_cols: usize) {
        let logical_lines = self.collect_logical_lines();
        let rewrapped = rewrap_lines(&logical_lines, new_cols);
        self.fill_grid_from_rewrapped(&rewrapped, new_rows, new_cols);
    }

    /// Fill the grid and scrollback from rewrapped rows, and restore cursor.
    ///
    /// If all rewrapped content fits in the grid, it is placed at the top with
    /// the cursor at the last content row. Otherwise, excess rows go to
    /// scrollback and the cursor is placed at the bottom.
    fn fill_grid_from_rewrapped(
        &mut self,
        rewrapped: &[Row],
        new_rows: usize,
        new_cols: usize,
    ) {
        let total_rows = rewrapped.len();

        self.scrollback.clear();
        self.grid = Grid::new(new_rows, new_cols);

        if total_rows <= new_rows {
            // All content fits in grid - fill from top
            for (i, row) in rewrapped.iter().enumerate() {
                for (col, cell) in row.cells.iter().enumerate() {
                    if col < new_cols {
                        self.grid.set(i, col, cell.clone());
                    }
                }
                self.grid.set_wrapped(i, row.wrapped);
            }
            self.cursor_row = total_rows.saturating_sub(1);
        } else {
            // Content overflows - excess goes to scrollback
            let scrollback_count = total_rows - new_rows;

            for row in rewrapped.iter().take(scrollback_count) {
                self.scrollback.push_back(row.clone());
                if self.scrollback.len() > self.max_scrollback {
                    self.scrollback.pop_front();
                }
            }

            for (i, row) in rewrapped.iter().skip(scrollback_count).enumerate() {
                for (col, cell) in row.cells.iter().enumerate() {
                    if col < new_cols {
                        self.grid.set(i, col, cell.clone());
                    }
                }
                self.grid.set_wrapped(i, row.wrapped);
            }
            self.cursor_row = new_rows.saturating_sub(1);
        }

        self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));
    }
}
