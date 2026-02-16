use crate::core::{Cell, Grid};

use super::CursorStyle;

impl super::Terminal {
    pub fn resize(&mut self, rows: usize, cols: usize) {
        if self.grid.rows == rows && self.grid.cols == cols {
            return;
        }

        let old_rows = self.grid.rows;
        let is_alt = self.alt_grid.is_some();

        // ── Vertical shrink: cursor would be outside new grid ──
        // Shift content up, pushing top rows into scrollback.
        if rows < old_rows && self.cursor_row >= rows {
            let shift = self.cursor_row - rows + 1;
            // Save the top `shift` rows to scrollback (main screen only)
            if !is_alt {
                for r in 0..shift {
                    let row_cells = self.grid.row_cells(r);
                    self.scrollback.push_back(row_cells);
                    if self.scrollback.len() > self.max_scrollback {
                        self.scrollback.pop_front();
                    }
                }
            }
            self.grid.shift_up(shift);
            self.cursor_row -= shift;
        }

        // ── Resize the grid (copies content, pads/truncates) ──
        self.grid = self.grid.resized(rows, cols);

        // ── Vertical grow: pull lines from scrollback to fill new top rows ──
        if rows > old_rows && !is_alt && !self.scrollback.is_empty() {
            let available = rows - old_rows; // new empty rows at bottom
            let pull = available.min(self.scrollback.len());
            // Shift existing content down to make room at top
            self.grid.shift_down(pull);
            // Fill top rows from scrollback
            for i in 0..pull {
                let sb_row = self.scrollback.pop_back().unwrap();
                // Rows are pulled in reverse: last popped goes to row 0
                self.grid.set_row(pull - 1 - i, sb_row);
            }
            self.cursor_row += pull;
        }

        // ── Alt grid: simple resize (no scrollback interaction) ──
        if let Some(ref mut alt) = self.alt_grid {
            *alt = alt.resized(rows, cols);
        }

        // ── Clamp cursor to valid bounds ──
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));

        // ── Reset scroll region to full screen ──
        self.scroll_top = 0;
        self.scroll_bottom = rows - 1;

        self.resize_at = Some(std::time::Instant::now());
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
                    let sb_idx = self.scrollback.len() - scroll_offset + row;
                    if col < self.scrollback[sb_idx].len() {
                        self.scrollback[sb_idx][col].clone()
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
            let row: Vec<Cell> = (0..self.grid.cols)
                .map(|col| self.grid.get(0, col).clone())
                .collect();
            self.scrollback.push_back(row);
            if self.scrollback.len() > self.max_scrollback {
                self.scrollback.pop_front();
            }
        }

        for row in (top + 1)..=bottom {
            for col in 0..self.grid.cols {
                let cell = self.grid.get(row, col).clone();
                self.grid.set(row - 1, col, cell);
            }
        }
        for col in 0..self.grid.cols {
            self.grid.set(bottom, col, Cell::default());
        }
    }

    pub(super) fn scroll_down_region(&mut self, top: usize, bottom: usize) {
        for row in (top..bottom).rev() {
            for col in 0..self.grid.cols {
                let cell = self.grid.get(row, col).clone();
                self.grid.set(row + 1, col, cell);
            }
        }
        for col in 0..self.grid.cols {
            self.grid.set(top, col, Cell::default());
        }
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
