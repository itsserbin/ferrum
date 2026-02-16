use crate::core::Cell;
use crate::core::terminal::Terminal;
use vte::Params;

pub(in super::super) fn handle_inline_edit_csi(
    term: &mut Terminal,
    action: char,
    params: &Params,
) -> bool {
    match action {
        'P' => {
            // DCH: delete N chars and shift remainder left.
            let n = term.param(params, 1).max(1) as usize;
            for col in term.cursor_col..term.grid.cols {
                if col + n < term.grid.cols {
                    let cell = term.grid.get(term.cursor_row, col + n).clone();
                    term.grid.set(term.cursor_row, col, cell);
                } else {
                    term.grid.set(term.cursor_row, col, Cell::default());
                }
            }
            true
        }
        '@' => {
            // ICH: insert N blank cells and shift remainder right.
            let n = term.param(params, 1).max(1) as usize;
            for col in (term.cursor_col..term.grid.cols).rev() {
                if col >= term.cursor_col + n {
                    let cell = term.grid.get(term.cursor_row, col - n).clone();
                    term.grid.set(term.cursor_row, col, cell);
                } else {
                    term.grid.set(term.cursor_row, col, Cell::default());
                }
            }
            true
        }
        'X' => {
            // ECH: clear N cells without shifting.
            let n = term.param(params, 1).max(1) as usize;
            for col in term.cursor_col..(term.cursor_col + n).min(term.grid.cols) {
                term.grid.set(term.cursor_row, col, Cell::default());
            }
            true
        }
        _ => false,
    }
}
