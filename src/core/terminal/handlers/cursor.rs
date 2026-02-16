use crate::core::terminal::Terminal;
use vte::Params;

pub(in super::super) fn handle_cursor_csi(
    term: &mut Terminal,
    action: char,
    params: &Params,
) -> bool {
    match action {
        'H' | 'f' => {
            let mut iter = params.iter();
            let row = iter.next().and_then(|p| p.first().copied()).unwrap_or(1);
            let col = iter.next().and_then(|p| p.first().copied()).unwrap_or(1);
            term.cursor_row = (row as usize).saturating_sub(1).min(term.grid.rows - 1);
            term.cursor_col = (col as usize).saturating_sub(1).min(term.grid.cols - 1);
            true
        }
        'A' => {
            let n = term.param(params, 1).max(1) as usize;
            term.cursor_row = term.cursor_row.saturating_sub(n);
            true
        }
        'B' => {
            let n = term.param(params, 1).max(1) as usize;
            term.cursor_row = (term.cursor_row + n).min(term.grid.rows - 1);
            true
        }
        'C' => {
            let n = term.param(params, 1).max(1) as usize;
            term.cursor_col = (term.cursor_col + n).min(term.grid.cols - 1);
            true
        }
        'D' => {
            let n = term.param(params, 1).max(1) as usize;
            term.cursor_col = term.cursor_col.saturating_sub(n);
            true
        }
        'G' => {
            let col = term.param(params, 1) as usize;
            term.cursor_col = col.saturating_sub(1).min(term.grid.cols - 1);
            true
        }
        'd' => {
            let row = term.param(params, 1) as usize;
            term.cursor_row = row.saturating_sub(1).min(term.grid.rows - 1);
            true
        }
        _ => false,
    }
}
