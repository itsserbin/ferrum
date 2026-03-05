//! Terminal resize logic: uses `PageList` reflow directly (no bridge methods).

use crate::core::{Grid, Row};

impl super::Terminal {
    pub fn resize(&mut self, rows: usize, cols: usize) {
        if self.grid.rows == rows && self.grid.cols == cols {
            return;
        }

        // Alt screen: simple resize (no reflow).
        if let Some(ref mut alt) = self.alt_screen {
            alt.simple_resize(rows, cols);
        }
        if let Some(ref mut alt) = self.alt_grid {
            *alt = alt.resized(rows, cols);
        }

        let old_cols = self.grid.cols;
        if old_cols != cols && self.alt_screen.is_none() {
            // Reflow resize: run grapheme-aware reflow on the PageList directly,
            // then rebuild the display cache (grid + scrollback) from the result.
            self.screen.reflow(rows, cols, &self.cursor_pin);
            self.rebuild_display_cache_after_resize(rows, cols);
        } else {
            // Height-only resize or alt-screen present: use simple resize logic.
            self.simple_resize(rows, cols);
        }

        // Reset scroll region to full screen.
        self.scroll_top = 0;
        self.scroll_bottom = rows - 1;

        self.resize_at = Some(std::time::Instant::now());
    }

    /// Rebuild `self.grid`, `self.scrollback`, `self.cursor_row`, `self.cursor_col`
    /// from `self.screen` after a reflow resize.  Called only when cols changed.
    fn rebuild_display_cache_after_resize(&mut self, new_rows: usize, new_cols: usize) {
        let sb_len = self.screen.scrollback_len();

        // Rebuild scrollback from screen.
        self.scrollback.clear();
        for i in 0..sb_len {
            let pr = self.screen.scrollback_row(i);
            let cells: Vec<crate::core::Cell> =
                pr.cells.iter().map(super::Terminal::grapheme_to_cell).collect();
            self.scrollback.push_back(Row::from_cells(cells, pr.wrapped));
        }

        // Rebuild grid from screen viewport.
        self.grid = Grid::new(new_rows, new_cols);
        let vrows = self.screen.viewport_rows();
        let vcols = self.screen.cols();
        for r in 0..vrows.min(new_rows) {
            let wrapped = self.screen.viewport_is_wrapped(r);
            for c in 0..vcols.min(new_cols) {
                let gc = self.screen.viewport_get(r, c);
                self.grid.set(r, c, super::Terminal::grapheme_to_cell(gc));
            }
            self.grid.set_wrapped(r, wrapped);
        }

        // Update cursor from pin.
        let coord = self.screen.pin_coord(&self.cursor_pin);
        let vstart = self.screen.viewport_start_abs();
        self.cursor_row =
            coord.abs_row.saturating_sub(vstart).min(new_rows.saturating_sub(1));
        self.cursor_col = coord.col.min(new_cols.saturating_sub(1));
        // Sync the display cache's cursor_row/cursor_col back to the pin.
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
        if rows < self.grid.rows {
            // Only push top rows to scrollback when the cursor would fall
            // outside the new grid — i.e. when there is no empty space below.
            let push_count = if self.cursor_row >= rows {
                self.cursor_row + 1 - rows
            } else {
                0
            };

            let overflow =
                (self.scrollback.len() + push_count).saturating_sub(self.max_scrollback);
            // Evict old scrollback rows that will be pushed out by the new rows.
            let old_evictions = overflow.saturating_sub(push_count);
            for _ in 0..old_evictions {
                self.scrollback.pop_front();
            }
            // Skip grid rows that would be pushed only to be immediately evicted.
            let skip = overflow.min(push_count);
            for r in skip..push_count {
                self.scrollback.push_back(Row::from_cells(
                    self.grid.row_slice(r).to_vec(),
                    self.grid.is_wrapped(r),
                ));
            }

            self.grid = self.grid.resized_from_offset(rows, cols, push_count);
            self.cursor_row = self.cursor_row.saturating_sub(push_count);
        } else {
            // Height increase or same: keep top rows, add blank rows at bottom.
            self.grid = self.grid.resized(rows, cols);
            self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        }

        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));

        // Keep screen in sync with simple resize.
        self.screen.simple_resize(rows, cols);
        // Rebuild the viewport from the grid so screen and grid agree.
        for r in 0..rows {
            for c in 0..cols {
                if r < self.grid.rows && c < self.grid.cols {
                    let gc = super::Terminal::cell_to_grapheme(self.grid.get_unchecked(r, c));
                    self.screen.viewport_set(r, c, gc);
                }
            }
            if r < self.grid.rows {
                self.screen.viewport_set_wrapped(r, self.grid.is_wrapped(r));
            }
        }
        // Update cursor pin.
        let abs = self.screen.viewport_start_abs() + self.cursor_row;
        self.screen.set_pin_abs_row(&self.cursor_pin, abs);
        self.screen.set_pin_col(&self.cursor_pin, self.cursor_col);
    }
}
