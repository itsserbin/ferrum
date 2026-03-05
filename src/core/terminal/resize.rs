//! Terminal resize logic: simple resize and reflow resize strategies.

use crate::core::{Grid, Row};

impl super::Terminal {
    pub fn resize(&mut self, rows: usize, cols: usize) {
        if self.grid.rows == rows && self.grid.cols == cols {
            return;
        }

        // Alt grid: simple resize (no reflow)
        if let Some(ref mut alt) = self.alt_grid {
            *alt = alt.resized(rows, cols);
        }

        // Reflow when cols change (main screen only).
        // For row-only changes use simple resize so we can anchor correctly.
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

    /// Resize when only the height changes (no col change, no reflow).
    ///
    /// When the terminal shrinks, rows are discarded from the top only as
    /// needed to keep the cursor visible — if there is empty space below the
    /// cursor those rows are simply dropped and the cursor stays in place.
    /// When the terminal grows, blank rows are added at the bottom.
    fn simple_resize(&mut self, rows: usize, cols: usize) {
        if rows < self.grid.rows {
            // Only push top rows to scrollback when the cursor would fall
            // outside the new grid — i.e. when there is no empty space below.
            let push_count = if self.cursor_row >= rows {
                self.cursor_row + 1 - rows
            } else {
                0
            };

            let overflow = (self.scrollback.len() + push_count).saturating_sub(self.max_scrollback);
            // Evict old scrollback rows that will be pushed out by the new rows.
            let old_evictions = overflow.saturating_sub(push_count);
            for _ in 0..old_evictions {
                self.scrollback.pop_front();
            }
            // Skip grid rows that would be pushed only to be immediately evicted.
            let skip = overflow.min(push_count);
            for r in skip..push_count {
                let cells = self.grid.row_cells(r);
                let wrapped = self.grid.is_wrapped(r);
                self.scrollback.push_back(Row::from_cells(cells, wrapped));
            }

            // Build the new grid from old rows [push_count .. push_count+rows].
            let mut new_grid = Grid::new(rows, cols);
            for new_r in 0..rows {
                let old_r = push_count + new_r;
                if old_r < self.grid.rows {
                    for c in 0..cols.min(self.grid.cols) {
                        new_grid.set(new_r, c, self.grid.get_unchecked(old_r, c).clone());
                    }
                    new_grid.set_wrapped(new_r, self.grid.is_wrapped(old_r));
                }
            }
            self.grid = new_grid;
            self.cursor_row = self.cursor_row.saturating_sub(push_count);
        } else {
            // Height increase or same: keep top rows, add blank rows at bottom.
            self.grid = self.grid.resized(rows, cols);
            self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        }

        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
    }

    /// Reflow resize: rewrap all content (scrollback + visible grid) to the new
    /// column width, then restore the cursor to the beginning of the prompt's
    /// logical line.
    ///
    /// Placing the cursor at the START of the prompt's logical line ensures
    /// that readline's SIGWINCH handler redraws the prompt starting from the
    /// same row where reflow placed it — preventing double-prompt artifacts.
    fn reflow_resize(&mut self, new_rows: usize, new_cols: usize) {
        let (logical_lines, cursor_line_idx) = self.collect_logical_lines();

        // rewrap_lines tracks the start row of cursor_line_idx in one pass,
        // avoiding a separate traversal via the old rows_before_line helper.
        let (rewrapped, cursor_line_start) =
            super::reflow::rewrap_lines(&logical_lines, new_cols, cursor_line_idx);

        // Place the cursor at the START of the prompt's logical line (col 0).
        //
        // When SIGWINCH fires, the shell (zsh, bash/readline) does:
        //   1. CR → already at col 0, no-op
        //   2. Erase from current position to end of screen
        //   3. Redraw the full prompt + any typed input
        //
        // If we placed the cursor at the ACTUAL offset within the wrapped line
        // (e.g. row start+1, col 15 for a 35-char prompt at 20 cols), the shell
        // would erase only from the MIDDLE of the reflowed prompt onward, leaving
        // the first wrapped rows visible — causing the prompt to appear doubled.
        //
        // By placing the cursor at (start, 0), the shell erases the entire
        // reflowed cursor line and redraws it cleanly.
        let cursor_in_rewrapped = cursor_line_start.map(|start| (start, 0usize));

        self.fill_grid_from_rewrapped(&rewrapped, new_rows, new_cols, cursor_in_rewrapped);
    }

    fn fill_grid_from_rewrapped(
        &mut self,
        rewrapped: &[Row],
        new_rows: usize,
        new_cols: usize,
        cursor_in_rewrapped: Option<(usize, usize)>,
    ) {
        let total_rows = rewrapped.len();
        // Rows beyond new_rows go to scrollback; the rest fill the grid.
        // Clamp to max_scrollback so we never exceed the limit, and skip
        // rows that would be evicted immediately — avoids a per-iteration
        // pop_front (scrollback was just cleared).
        let grid_offset = total_rows.saturating_sub(new_rows);
        let scrollback_count = grid_offset.min(self.max_scrollback);
        let skip_count = grid_offset.saturating_sub(scrollback_count);

        self.scrollback.clear();
        self.grid = Grid::new(new_rows, new_cols);

        for row in rewrapped.iter().skip(skip_count).take(scrollback_count) {
            self.scrollback.push_back(row.clone());
        }

        for (i, row) in rewrapped.iter().skip(grid_offset).enumerate() {
            for (col, cell) in row.cells.iter().enumerate() {
                self.grid.set(i, col, cell.clone());
            }
            self.grid.set_wrapped(i, row.wrapped);
        }

        // Place cursor at its correct reflowed position.
        //
        // readline's SIGWINCH handler sends CR first (moves to col 0), then
        // redraws the full prompt from the current row — so the exact col value
        // does not affect readline redraw correctness, but it must match the
        // shell's expectation of where the cursor is so zsh/bash don't emit a
        // stray partial-line indicator (`%`).
        if let Some((row_in_rewrapped, col)) = cursor_in_rewrapped {
            self.cursor_row = row_in_rewrapped
                .saturating_sub(grid_offset)
                .min(new_rows.saturating_sub(1));
            self.cursor_col = col.min(new_cols.saturating_sub(1));
        } else {
            self.cursor_row = new_rows.saturating_sub(1);
            self.cursor_col = 0;
        }
    }
}
