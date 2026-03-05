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

            for r in 0..push_count {
                let cells = self.grid.row_cells(r);
                let wrapped = self.grid.is_wrapped(r);
                self.scrollback.push_back(Row::from_cells(cells, wrapped));
                if self.scrollback.len() > self.max_scrollback {
                    self.scrollback.pop_front();
                }
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

        // Compute cursor (row, col) in the rewrapped output.
        // `min_len` is the cursor's absolute offset within its logical line,
        // so after rewrapping: row = start + offset/new_cols, col = offset%new_cols.
        let cursor_in_rewrapped = cursor_line_idx.map(|idx| {
            let start = super::reflow::rows_before_line(&logical_lines, idx, new_cols);
            let offset = logical_lines[idx].min_len;
            (start + offset / new_cols, offset % new_cols)
        });

        let rewrapped = super::reflow::rewrap_lines(&logical_lines, new_cols);
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
        let scrollback_count = total_rows.saturating_sub(new_rows);

        self.scrollback.clear();
        self.grid = Grid::new(new_rows, new_cols);

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

        // Place cursor at its correct reflowed position.
        //
        // readline's SIGWINCH handler sends CR first (moves to col 0), then
        // redraws the full prompt from the current row — so the exact col value
        // does not affect readline redraw correctness, but it must match the
        // shell's expectation of where the cursor is so zsh/bash don't emit a
        // stray partial-line indicator (`%`).
        if let Some((row_in_rewrapped, col)) = cursor_in_rewrapped {
            self.cursor_row = row_in_rewrapped
                .saturating_sub(scrollback_count)
                .min(new_rows.saturating_sub(1));
            self.cursor_col = col.min(new_cols.saturating_sub(1));
        } else {
            self.cursor_row = new_rows.saturating_sub(1);
            self.cursor_col = 0;
        }
    }
}
