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
