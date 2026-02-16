use crate::core::Cell;

pub struct Grid {
    cells: Vec<Vec<Cell>>,
    pub rows: usize,
    pub cols: usize,
}

impl Grid {
    pub fn new(rows: usize, cols: usize) -> Self {
        Grid {
            cells: vec![vec![Cell::default(); cols]; rows],
            rows,
            cols,
        }
    }

    pub fn get(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row][col]
    }

    pub fn set(&mut self, row: usize, col: usize, cell: Cell) {
        if row < self.rows && col < self.cols {
            self.cells[row][col] = cell;
        }
    }

    pub fn resized(&self, rows: usize, cols: usize) -> Grid {
        let mut new_grid = Grid::new(rows, cols);
        for row in 0..rows.min(self.rows) {
            for col in 0..cols.min(self.cols) {
                new_grid.set(row, col, self.get(row, col).clone());
            }
        }
        new_grid
    }

    /// Extract a row as a Vec<Cell>, for saving to scrollback.
    pub fn row_cells(&self, row: usize) -> Vec<Cell> {
        self.cells[row].clone()
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
                // Move row+count into row
                self.cells.swap(row, row + count);
            }
        }
        // Clear the bottom `count` rows
        for row in (self.rows - count)..self.rows {
            for col in 0..self.cols {
                self.cells[row][col] = Cell::default();
            }
        }
    }

    /// Shift all rows down by `count` positions.
    /// Rows at the bottom are lost.
    /// Rows vacated at the top are filled with defaults.
    pub fn shift_down(&mut self, count: usize) {
        if count == 0 || count >= self.rows {
            return;
        }
        for row in (0..self.rows).rev() {
            if row >= count {
                self.cells.swap(row, row - count);
            }
        }
        // Clear the top `count` rows
        for row in 0..count {
            for col in 0..self.cols {
                self.cells[row][col] = Cell::default();
            }
        }
    }

    /// Set an entire row from a Vec<Cell>, padding or truncating to fit cols.
    pub fn set_row(&mut self, row: usize, cells: Vec<Cell>) {
        for col in 0..self.cols {
            if col < cells.len() {
                self.cells[row][col] = cells[col].clone();
            } else {
                self.cells[row][col] = Cell::default();
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
