use crate::core::terminal::Terminal;
use crate::core::{Cell, Grid};
use vte::Params;

pub(in super::super) fn handle_erase_csi(
    term: &mut Terminal,
    action: char,
    params: &Params,
) -> bool {
    match action {
        'J' => {
            match term.param(params, 0) {
                0 => {
                    for col in term.cursor_col..term.grid.cols {
                        term.grid.set(term.cursor_row, col, Cell::default());
                    }
                    for row in (term.cursor_row + 1)..term.grid.rows {
                        for col in 0..term.grid.cols {
                            term.grid.set(row, col, Cell::default());
                        }
                    }
                }
                1 => {
                    for row in 0..term.cursor_row {
                        for col in 0..term.grid.cols {
                            term.grid.set(row, col, Cell::default());
                        }
                    }
                    for col in 0..=term.cursor_col.min(term.grid.cols - 1) {
                        term.grid.set(term.cursor_row, col, Cell::default());
                    }
                }
                2 | 3 => term.grid = Grid::new(term.grid.rows, term.grid.cols),
                _ => {}
            }
            true
        }
        'K' => {
            match term.param(params, 0) {
                0 => {
                    for col in term.cursor_col..term.grid.cols {
                        term.grid.set(term.cursor_row, col, Cell::default());
                    }
                }
                1 => {
                    for col in 0..=term.cursor_col.min(term.grid.cols - 1) {
                        term.grid.set(term.cursor_row, col, Cell::default());
                    }
                }
                2 => {
                    for col in 0..term.grid.cols {
                        term.grid.set(term.cursor_row, col, Cell::default());
                    }
                }
                _ => {}
            }
            true
        }
        _ => false,
    }
}
