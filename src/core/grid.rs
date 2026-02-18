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
        Grid {
            rows_data: (0..rows).map(|_| Row::new(cols)).collect(),
            rows,
            cols,
        }
    }

    pub fn get(&self, row: usize, col: usize) -> &Cell {
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
                new_grid.set(row, col, self.get(row, col).clone());
            }
            new_grid.set_wrapped(row, self.is_wrapped(row));
        }
        new_grid
    }

    /// Extract a row as a Vec<Cell>, for saving to scrollback.
    pub fn row_cells(&self, row: usize) -> Vec<Cell> {
        self.rows_data[row].cells.clone()
    }

    /// Get the Row struct for a given row index.
    #[allow(dead_code)]
    pub fn get_row(&self, row: usize) -> &Row {
        &self.rows_data[row]
    }

    /// Shift all rows in range [from..to) up by `count` positions.
    /// Rows at the top are lost (caller must save them first).
    /// Rows vacated at the bottom are filled with defaults.
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn set_row(&mut self, row: usize, cells: Vec<Cell>) {
        for col in 0..self.cols {
            if col < cells.len() {
                self.rows_data[row].cells[col] = cells[col].clone();
            } else {
                self.rows_data[row].cells[col] = Cell::default();
            }
        }
    }

    /// Set an entire row from a Row struct (preserves wrapped flag).
    #[allow(dead_code)]
    pub fn set_row_data(&mut self, row: usize, row_data: Row) {
        if row < self.rows {
            // Resize cells to match grid columns
            let mut cells = row_data.cells;
            cells.resize(self.cols, Cell::default());
            self.rows_data[row] = Row {
                cells,
                wrapped: row_data.wrapped,
            };
        }
    }

    /// Get all rows as Row structs (for reflow).
    #[allow(dead_code)]
    pub fn all_rows(&self) -> &[Row] {
        &self.rows_data
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
                assert_eq!(grid.get(row, col), &default);
            }
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
        assert_eq!(grid.get(1, 2), &cell);
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
        assert_eq!(bigger.get(0, 0), &cell);
        assert_eq!(bigger.get(2, 2), &cell);
        // New cells should be default
        assert_eq!(bigger.get(3, 3), &Cell::default());
        assert_eq!(bigger.get(4, 4), &Cell::default());
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
        assert_eq!(smaller.get(0, 0), &cell_a);
        // (4,4) is outside the new 3x3 grid, so it should not be present
        // Just verify the grid is 3x3 and contains expected data
        assert_eq!(smaller.get(2, 2), &Cell::default());
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

        assert_eq!(grid.get(0, 0).character, 'B');
        assert_eq!(grid.get(1, 0).character, 'C');
        assert_eq!(grid.get(2, 0).character, ' ');
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

        assert_eq!(grid.get(0, 0).character, ' ');
        assert_eq!(grid.get(1, 0).character, 'A');
        assert_eq!(grid.get(2, 0).character, 'B');
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

        assert_eq!(grid.get(0, 0).character, 'X');
        assert_eq!(grid.get(0, 1).character, 'Y');
        // Remaining cols should be padded with defaults
        assert_eq!(grid.get(0, 2), &Cell::default());
        assert_eq!(grid.get(0, 3), &Cell::default());
        // Row 1 should be untouched
        assert_eq!(grid.get(1, 0), &Cell::default());
    }
}
