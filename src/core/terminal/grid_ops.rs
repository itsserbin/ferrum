use crate::core::{Cell, Grid, Row};

use super::CursorStyle;

/// A logical line collected from scrollback and grid content.
///
/// Represents one logical line of text that may span multiple physical rows
/// when soft-wrapped. The `min_len` field preserves trailing spaces before
/// the cursor position so they are not trimmed during rewrapping.
struct LogicalLine {
    cells: Vec<Cell>,
    /// Minimum number of cells to preserve (for cursor-position trailing spaces).
    min_len: usize,
}

/// Rewrap logical lines to fit a new column width.
///
/// This is a pure function with no side effects: it takes collected logical lines
/// and a target width, and produces a flat list of physical rows with correct
/// wrap flags.
fn rewrap_lines(lines: &[LogicalLine], new_cols: usize) -> Vec<Row> {
    let mut rewrapped: Vec<Row> = Vec::new();
    for logical_line in lines {
        // Trim only untouched default cells; keep styled spaces and explicit
        // spaces before cursor in the active line.
        let len = logical_line
            .cells
            .iter()
            .rposition(|c| c != &Cell::default())
            .map(|i| i + 1)
            .unwrap_or(0);
        let len = len.max(logical_line.min_len.min(logical_line.cells.len()));

        if len == 0 {
            rewrapped.push(Row::new(new_cols));
            continue;
        }

        let content = &logical_line.cells[..len];
        let mut pos = 0;
        while pos < content.len() {
            let end = (pos + new_cols).min(content.len());
            let mut cells: Vec<Cell> = content[pos..end].to_vec();
            cells.resize(new_cols, Cell::default());
            let wrapped = end < content.len();
            rewrapped.push(Row::from_cells(cells, wrapped));
            pos = end;
        }
    }
    rewrapped
}

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

        // ── Reset scroll region to full screen ──
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
        // Stage 1: collect all logical lines from scrollback + grid
        let logical_lines = self.collect_logical_lines();

        // Stage 2: rewrap to new width (pure function)
        let rewrapped = rewrap_lines(&logical_lines, new_cols);

        // Stage 3: fill new grid and restore cursor
        self.fill_grid_from_rewrapped(&rewrapped, new_rows, new_cols);
    }

    /// Collect scrollback and meaningful grid rows into logical lines.
    ///
    /// Merges consecutive soft-wrapped physical rows into single logical lines.
    /// Tracks `min_len` to preserve trailing spaces before the cursor position.
    fn collect_logical_lines(&self) -> Vec<LogicalLine> {
        // Keep explicit blank lines up to cursor row, but also preserve any visible
        // content that may exist below the cursor (after cursor-addressing sequences).
        let content_rows = self.compute_content_rows();

        let mut lines: Vec<LogicalLine> = Vec::new();
        let mut current_cells: Vec<Cell> = Vec::new();
        let mut current_min_len = 0usize;

        // First from scrollback
        for row in self.scrollback.iter() {
            current_cells.extend(row.cells.iter().cloned());
            if !row.wrapped {
                lines.push(LogicalLine {
                    cells: std::mem::take(&mut current_cells),
                    min_len: current_min_len,
                });
                current_min_len = 0;
            }
        }

        // Then from grid up to the computed content boundary.
        for r in 0..content_rows {
            let line_start = current_cells.len();
            current_cells.extend(self.grid.row_cells(r));
            if r == self.cursor_row {
                let clamped_cursor_col = self.cursor_col.min(self.grid.cols);
                current_min_len = current_min_len.max(line_start + clamped_cursor_col);
            }
            if !self.grid.is_wrapped(r) {
                lines.push(LogicalLine {
                    cells: std::mem::take(&mut current_cells),
                    min_len: current_min_len,
                });
                current_min_len = 0;
            }
        }

        // Handle remaining content (if last row was wrapped)
        if !current_cells.is_empty() {
            lines.push(LogicalLine {
                cells: current_cells,
                min_len: current_min_len,
            });
        }

        lines
    }

    /// Compute the number of grid rows that contain meaningful content.
    ///
    /// Includes all rows up to the cursor position, plus any rows below the cursor
    /// that contain non-default content (e.g., after cursor-addressing sequences).
    fn compute_content_rows(&self) -> usize {
        let cursor_rows = self.cursor_row.saturating_add(1).min(self.grid.rows);
        let default_cell = Cell::default();
        let last_content_row = (0..self.grid.rows).rev().find(|&row| {
            self.grid.is_wrapped(row)
                || (0..self.grid.cols).any(|col| {
                    // Safe: row/col come from bounded loops.
                    self.grid.get_unchecked(row, col) != &default_cell
                })
        });
        last_content_row
            .map(|row| (row + 1).max(cursor_rows))
            .unwrap_or(cursor_rows)
    }

    /// Fill the grid and scrollback from rewrapped rows, and restore cursor position.
    ///
    /// If all rewrapped content fits in the grid, it is placed at the top with the
    /// cursor at the last content row. Otherwise, excess rows go to scrollback and
    /// the cursor is placed at the bottom.
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
            // Cursor at end of content
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

            // Fill grid with remaining rows
            for (i, row) in rewrapped.iter().skip(scrollback_count).enumerate() {
                for (col, cell) in row.cells.iter().enumerate() {
                    if col < new_cols {
                        self.grid.set(i, col, cell.clone());
                    }
                }
                self.grid.set_wrapped(i, row.wrapped);
            }
            // Cursor at bottom (content filled the grid)
            self.cursor_row = new_rows.saturating_sub(1);
        }

        self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));
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
                    // Safe: row - scroll_offset is in bounds since row >= scroll_offset
                    // and row < self.grid.rows, col < self.grid.cols
                    self.grid.get_unchecked(row - scroll_offset, col).clone()
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
                // Safe: row and col are within grid bounds (row <= bottom < grid.rows)
                let cell = self.grid.get_unchecked(row, col).clone();
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
                // Safe: row and col are within grid bounds (row < bottom <= grid.rows)
                let cell = self.grid.get_unchecked(row, col).clone();
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
    use super::*;

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
                // Safe: iterating within grid bounds
                let ch = term.grid.get_unchecked(r, c).character;
                if ch != ' ' {
                    content.push(ch);
                }
            }
        }
        content
    }

    // ── Tests for the rewrap_lines pure function ──

    #[test]
    fn rewrap_lines_empty_line_produces_blank_row() {
        let lines = vec![LogicalLine {
            cells: vec![],
            min_len: 0,
        }];
        let result = rewrap_lines(&lines, 10);
        assert_eq!(result.len(), 1);
        assert!(!result[0].wrapped);
        assert_eq!(result[0].cells.len(), 10);
        for cell in &result[0].cells {
            assert_eq!(*cell, Cell::default());
        }
    }

    #[test]
    fn rewrap_lines_short_line_fits_in_one_row() {
        let cells: Vec<Cell> = "Hello"
            .chars()
            .map(|c| Cell {
                character: c,
                ..Cell::default()
            })
            .collect();
        let lines = vec![LogicalLine {
            cells,
            min_len: 0,
        }];
        let result = rewrap_lines(&lines, 10);
        assert_eq!(result.len(), 1);
        assert!(!result[0].wrapped);
        assert_eq!(result[0].cells[0].character, 'H');
        assert_eq!(result[0].cells[4].character, 'o');
    }

    #[test]
    fn rewrap_lines_long_line_wraps_to_multiple_rows() {
        let cells: Vec<Cell> = "ABCDEFGHIJ"
            .chars()
            .map(|c| Cell {
                character: c,
                ..Cell::default()
            })
            .collect();
        let lines = vec![LogicalLine {
            cells,
            min_len: 0,
        }];
        let result = rewrap_lines(&lines, 4);
        assert_eq!(result.len(), 3);
        assert!(result[0].wrapped);
        assert!(result[1].wrapped);
        assert!(!result[2].wrapped);
        assert_eq!(result[0].cells[0].character, 'A');
        assert_eq!(result[0].cells[3].character, 'D');
        assert_eq!(result[1].cells[0].character, 'E');
        assert_eq!(result[2].cells[0].character, 'I');
        assert_eq!(result[2].cells[1].character, 'J');
    }

    #[test]
    fn rewrap_lines_preserves_min_len_trailing_spaces() {
        let cells: Vec<Cell> = "abc   "
            .chars()
            .map(|c| Cell {
                character: c,
                ..Cell::default()
            })
            .collect();
        let lines = vec![LogicalLine {
            cells,
            min_len: 6,
        }];
        let result = rewrap_lines(&lines, 10);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].cells[0].character, 'a');
        assert_eq!(result[0].cells[1].character, 'b');
        assert_eq!(result[0].cells[2].character, 'c');
        assert_eq!(result[0].cells[3].character, ' ');
        assert_eq!(result[0].cells[4].character, ' ');
        assert_eq!(result[0].cells[5].character, ' ');
    }

    #[test]
    fn rewrap_lines_trims_trailing_default_cells_without_min_len() {
        let mut cells: Vec<Cell> = "abc"
            .chars()
            .map(|c| Cell {
                character: c,
                ..Cell::default()
            })
            .collect();
        cells.resize(10, Cell::default());
        let lines = vec![LogicalLine {
            cells,
            min_len: 0,
        }];
        let result = rewrap_lines(&lines, 5);
        assert_eq!(result.len(), 1);
        assert!(!result[0].wrapped);
        assert_eq!(result[0].cells[0].character, 'a');
        assert_eq!(result[0].cells[1].character, 'b');
        assert_eq!(result[0].cells[2].character, 'c');
    }

    #[test]
    fn rewrap_lines_multiple_logical_lines() {
        let line1_cells: Vec<Cell> = "ABCD"
            .chars()
            .map(|c| Cell {
                character: c,
                ..Cell::default()
            })
            .collect();
        let line2_cells: Vec<Cell> = "EF"
            .chars()
            .map(|c| Cell {
                character: c,
                ..Cell::default()
            })
            .collect();
        let lines = vec![
            LogicalLine {
                cells: line1_cells,
                min_len: 0,
            },
            LogicalLine {
                cells: line2_cells,
                min_len: 0,
            },
        ];
        let result = rewrap_lines(&lines, 3);
        assert_eq!(result.len(), 3);
        assert!(result[0].wrapped);
        assert!(!result[1].wrapped);
        assert!(!result[2].wrapped);
    }

    // ── Integration tests (existing, preserved exactly) ──

    #[test]
    fn reflow_preserves_content_after_width_change() {
        let mut term = Terminal::new(4, 10);
        term.process(b"AAAAAAAAAA");
        term.process(b"\n");
        term.cursor_col = 0;
        term.process(b"BBBBBBBBBB");

        let content_before = collect_all_content(&term);

        term.resize(4, 15);

        let content_after = collect_all_content(&term);

        assert_eq!(
            content_before, content_after,
            "Content should be preserved after reflow"
        );
    }

    #[test]
    fn reflow_preserves_rows_below_cursor() {
        let mut term = Terminal::new(5, 10);
        term.process(b"TOP");
        term.process(b"\x1b[4;1HLOWER");

        term.cursor_row = 1;
        term.cursor_col = 0;

        let before = collect_all_content(&term);
        assert_eq!(before, "TOPLOWER");

        term.resize(5, 12);

        let after = collect_all_content(&term);
        assert_eq!(
            after, before,
            "Rows below cursor must survive reflow resize"
        );
    }

    #[test]
    fn reflow_preserves_trailing_spaces_before_cursor() {
        let mut term = Terminal::new(4, 10);
        term.process(b"abc   ");
        assert_eq!(term.cursor_row, 0);
        assert_eq!(term.cursor_col, 6);

        term.resize(4, 12);

        assert_eq!(term.grid.get(0, 0).unwrap().character, 'a');
        assert_eq!(term.grid.get(0, 1).unwrap().character, 'b');
        assert_eq!(term.grid.get(0, 2).unwrap().character, 'c');
        assert_eq!(term.grid.get(0, 3).unwrap().character, ' ');
        assert_eq!(term.grid.get(0, 4).unwrap().character, ' ');
        assert_eq!(term.grid.get(0, 5).unwrap().character, ' ');
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

        term.resize(5, 10);

        assert!(
            term.cursor_row < term.grid.rows,
            "cursor_row {} should be < grid.rows {}",
            term.cursor_row,
            term.grid.rows
        );
        assert!(
            term.cursor_col < term.grid.cols,
            "cursor_col {} should be < grid.cols {}",
            term.cursor_col,
            term.grid.cols
        );
    }

    #[test]
    fn reflow_rewraps_long_lines_to_new_width() {
        let mut term = Terminal::new(4, 10);
        term.process(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ");

        term.resize(4, 20);

        let content = collect_all_content(&term);
        assert_eq!(
            content, "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "Content should be preserved after rewrap"
        );
    }

    #[test]
    fn simple_resize_height_only_preserves_grid() {
        let mut term = Terminal::new(4, 10);
        term.process(b"Test");

        term.resize(6, 10);

        assert_eq!(term.grid.get(0, 0).unwrap().character, 'T');
        assert_eq!(term.grid.get(0, 1).unwrap().character, 'e');
        assert_eq!(term.grid.get(0, 2).unwrap().character, 's');
        assert_eq!(term.grid.get(0, 3).unwrap().character, 't');
    }

    #[test]
    fn alt_screen_resize_does_not_reflow() {
        let mut term = Terminal::new(4, 10);
        term.process(b"Main screen");

        term.process(b"\x1b[?1049h");
        term.process(b"Alt content");

        let scrollback_before = term.scrollback.len();

        term.resize(4, 15);

        assert_eq!(term.scrollback.len(), scrollback_before);
    }
}
