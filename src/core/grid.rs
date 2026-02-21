use crate::core::Cell;

/// A single row with its cells and a wrapped flag.
/// `wrapped = true` means this row continues on the next row (soft wrap).
/// `wrapped = false` means this row ends with a logical line break (hard wrap / newline).
#[derive(Clone)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub wrapped: bool,
}

impl Row {
    pub fn new(cols: usize) -> Self {
        assert!(cols > 0, "Row cols must be positive, got {}", cols);
        Row {
            cells: vec![Cell::default(); cols],
            wrapped: false,
        }
    }

    /// Create a row from cells (for scrollback restoration).
    pub fn from_cells(cells: Vec<Cell>, wrapped: bool) -> Self {
        Row { cells, wrapped }
    }
}

pub struct Grid {
    rows_data: Vec<Row>,
    pub rows: usize,
    pub cols: usize,
}

impl Grid {
    pub fn new(rows: usize, cols: usize) -> Self {
        assert!(rows > 0, "Grid rows must be positive, got {}", rows);
        assert!(cols > 0, "Grid cols must be positive, got {}", cols);
        Grid {
            rows_data: (0..rows).map(|_| Row::new(cols)).collect(),
            rows,
            cols,
        }
    }

    /// Returns a reference to the cell at (row, col), or None if out of bounds.
    pub fn get(&self, row: usize, col: usize) -> Option<&Cell> {
        if row < self.rows && col < self.cols {
            Some(&self.rows_data[row].cells[col])
        } else {
            None
        }
    }

    /// Returns a reference to the cell at (row, col) without bounds checking.
    ///
    /// This is safe to call in performance-critical loops where bounds are already verified
    /// (e.g., iterating `0..grid.rows` and `0..grid.cols`).
    ///
    /// # Panics
    /// Panics if row >= self.rows or col >= self.cols.
    #[inline]
    pub fn get_unchecked(&self, row: usize, col: usize) -> &Cell {
        &self.rows_data[row].cells[col]
    }

    pub fn set(&mut self, row: usize, col: usize, cell: Cell) {
        if row < self.rows && col < self.cols {
            self.rows_data[row].cells[col] = cell;
        }
    }

    /// Check if a row is soft-wrapped (continues on next row).
    pub fn is_wrapped(&self, row: usize) -> bool {
        if row < self.rows {
            self.rows_data[row].wrapped
        } else {
            false
        }
    }

    /// Mark a row as soft-wrapped (true) or hard-wrapped/newline (false).
    pub fn set_wrapped(&mut self, row: usize, wrapped: bool) {
        if row < self.rows {
            self.rows_data[row].wrapped = wrapped;
        }
    }

    /// Simple resize without reflow (used for alt screen).
    pub fn resized(&self, rows: usize, cols: usize) -> Grid {
        let mut new_grid = Grid::new(rows, cols);
        for row in 0..rows.min(self.rows) {
            for col in 0..cols.min(self.cols) {
                // Safe: iterating within both grids' bounds
                new_grid.set(row, col, self.get_unchecked(row, col).clone());
            }
            new_grid.set_wrapped(row, self.is_wrapped(row));
        }
        new_grid
    }

    /// Extract a row as a Vec<Cell>, for saving to scrollback.
    pub fn row_cells(&self, row: usize) -> Vec<Cell> {
        self.rows_data[row].cells.clone()
    }

    /// Applies a function to every cell in the grid.
    pub fn recolor_cells(&mut self, mut map_fn: impl FnMut(&mut Cell)) {
        for row in &mut self.rows_data {
            for cell in &mut row.cells {
                map_fn(cell);
            }
        }
    }

    /// Get the Row struct for a given row index.
    ///
    /// Useful for inspecting row metadata (e.g., wrapped flag) without copying cells.
    /// Currently only used in tests; kept for future reflow/scrollback features.
    #[cfg(test)]
    pub fn get_row(&self, row: usize) -> &Row {
        &self.rows_data[row]
    }

    /// Shift all rows up by `count` positions.
    /// Rows at the top are lost (caller must save them first).
    /// Rows vacated at the bottom are filled with defaults.
    ///
    /// Currently only used in tests; kept for potential scrolling optimizations.
    #[cfg(test)]
    pub fn shift_up(&mut self, count: usize) {
        if count == 0 || count >= self.rows {
            return;
        }
        for row in 0..self.rows {
            if row + count < self.rows {
                self.rows_data.swap(row, row + count);
            }
        }
        // Clear the bottom `count` rows
        for row in (self.rows - count)..self.rows {
            self.rows_data[row] = Row::new(self.cols);
        }
    }

    /// Shift all rows down by `count` positions.
    /// Rows at the bottom are lost.
    /// Rows vacated at the top are filled with defaults.
    ///
    /// Currently only used in tests; kept for potential scrolling optimizations.
    #[cfg(test)]
    pub fn shift_down(&mut self, count: usize) {
        if count == 0 || count >= self.rows {
            return;
        }
        for row in (0..self.rows).rev() {
            if row >= count {
                self.rows_data.swap(row, row - count);
            }
        }
        // Clear the top `count` rows
        for row in 0..count {
            self.rows_data[row] = Row::new(self.cols);
        }
    }

    /// Set an entire row from a Vec<Cell>, padding or truncating to fit cols.
    ///
    /// Currently only used in tests; kept for potential scrollback restoration features.
    #[cfg(test)]
    pub fn set_row(&mut self, row: usize, cells: Vec<Cell>) {
        for col in 0..self.cols {
            if col < cells.len() {
                self.rows_data[row].cells[col] = cells[col].clone();
            } else {
                self.rows_data[row].cells[col] = Cell::default();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Cell;

    #[test]
    fn new_grid_is_empty() {
        let grid = Grid::new(3, 5);
        let default = Cell::default();
        for row in 0..3 {
            for col in 0..5 {
                assert_eq!(grid.get(row, col), Some(&default));
            }
            assert!(!grid.get_row(row).wrapped);
        }
    }

    #[test]
    fn set_get_roundtrip() {
        let mut grid = Grid::new(3, 5);
        let cell = Cell {
            character: 'Z',
            ..Cell::default()
        };
        grid.set(1, 2, cell.clone());
        assert_eq!(grid.get(1, 2), Some(&cell));
    }

    #[test]
    fn get_out_of_bounds_returns_none() {
        let grid = Grid::new(3, 5);
        assert_eq!(grid.get(10, 0), None);
        assert_eq!(grid.get(0, 10), None);
        assert_eq!(grid.get(100, 100), None);
    }

    #[test]
    fn set_out_of_bounds_ignored() {
        let mut grid = Grid::new(3, 5);
        let cell = Cell {
            character: 'X',
            ..Cell::default()
        };
        // Should not panic
        grid.set(100, 100, cell);
    }

    #[test]
    fn resized_preserves_content() {
        let mut grid = Grid::new(3, 3);
        let cell = Cell {
            character: 'Q',
            ..Cell::default()
        };
        grid.set(0, 0, cell.clone());
        grid.set(2, 2, cell.clone());

        let bigger = grid.resized(5, 5);
        assert_eq!(bigger.get(0, 0), Some(&cell));
        assert_eq!(bigger.get(2, 2), Some(&cell));
        // New cells should be default
        assert_eq!(bigger.get(3, 3), Some(&Cell::default()));
        assert_eq!(bigger.get(4, 4), Some(&Cell::default()));
    }

    #[test]
    fn resized_truncates() {
        let mut grid = Grid::new(5, 5);
        let cell_a = Cell {
            character: 'A',
            ..Cell::default()
        };
        let cell_d = Cell {
            character: 'D',
            ..Cell::default()
        };
        grid.set(0, 0, cell_a.clone());
        grid.set(4, 4, cell_d.clone());

        let smaller = grid.resized(3, 3);
        assert_eq!(smaller.rows, 3);
        assert_eq!(smaller.cols, 3);
        assert_eq!(smaller.get(0, 0), Some(&cell_a));
        // (4,4) is outside the new 3x3 grid, so it should not be present
        // Just verify the grid is 3x3 and contains expected data
        assert_eq!(smaller.get(2, 2), Some(&Cell::default()));
    }

    #[test]
    fn shift_up_moves_rows() {
        let mut grid = Grid::new(3, 1);
        grid.set(
            0,
            0,
            Cell {
                character: 'A',
                ..Cell::default()
            },
        );
        grid.set(
            1,
            0,
            Cell {
                character: 'B',
                ..Cell::default()
            },
        );
        grid.set(
            2,
            0,
            Cell {
                character: 'C',
                ..Cell::default()
            },
        );

        grid.shift_up(1);

        assert_eq!(grid.get(0, 0).unwrap().character, 'B');
        assert_eq!(grid.get(1, 0).unwrap().character, 'C');
        assert_eq!(grid.get(2, 0).unwrap().character, ' ');
    }

    #[test]
    fn shift_down_moves_rows() {
        let mut grid = Grid::new(3, 1);
        grid.set(
            0,
            0,
            Cell {
                character: 'A',
                ..Cell::default()
            },
        );
        grid.set(
            1,
            0,
            Cell {
                character: 'B',
                ..Cell::default()
            },
        );
        grid.set(
            2,
            0,
            Cell {
                character: 'C',
                ..Cell::default()
            },
        );

        grid.shift_down(1);

        assert_eq!(grid.get(0, 0).unwrap().character, ' ');
        assert_eq!(grid.get(1, 0).unwrap().character, 'A');
        assert_eq!(grid.get(2, 0).unwrap().character, 'B');
    }

    #[test]
    fn set_row_replaces() {
        let mut grid = Grid::new(2, 4);
        let cells = vec![
            Cell {
                character: 'X',
                ..Cell::default()
            },
            Cell {
                character: 'Y',
                ..Cell::default()
            },
        ];
        grid.set_row(0, cells);

        assert_eq!(grid.get(0, 0).unwrap().character, 'X');
        assert_eq!(grid.get(0, 1).unwrap().character, 'Y');
        // Remaining cols should be padded with defaults
        assert_eq!(grid.get(0, 2), Some(&Cell::default()));
        assert_eq!(grid.get(0, 3), Some(&Cell::default()));
        // Row 1 should be untouched
        assert_eq!(grid.get(1, 0), Some(&Cell::default()));
    }

    #[test]
    #[should_panic(expected = "Grid rows must be positive")]
    fn grid_new_zero_rows_panics() {
        Grid::new(0, 5);
    }

    #[test]
    #[should_panic(expected = "Grid cols must be positive")]
    fn grid_new_zero_cols_panics() {
        Grid::new(5, 0);
    }

    #[test]
    #[should_panic(expected = "Row cols must be positive")]
    fn row_new_zero_cols_panics() {
        Row::new(0);
    }

    #[test]
    #[should_panic(expected = "Grid rows must be positive")]
    fn grid_resized_zero_rows_panics() {
        let grid = Grid::new(3, 3);
        grid.resized(0, 3);
    }

    #[test]
    #[should_panic(expected = "Grid cols must be positive")]
    fn grid_resized_zero_cols_panics() {
        let grid = Grid::new(3, 3);
        grid.resized(3, 0);
    }

    #[test]
    fn recolor_cells_transforms_all_cells() {
        use crate::core::Color;
        let mut grid = Grid::new(2, 3);
        let old_fg = Color::SENTINEL_FG;
        let new_fg = Color { r: 1, g: 2, b: 3 };

        grid.recolor_cells(|cell| {
            if cell.fg == old_fg {
                cell.fg = new_fg;
            }
        });

        for row in 0..2 {
            for col in 0..3 {
                assert_eq!(grid.get(row, col).unwrap().fg, new_fg);
            }
        }
    }
}
