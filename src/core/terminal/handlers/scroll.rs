use crate::core::terminal::Terminal;
use vte::Params;

pub(in super::super) fn handle_scroll_csi(
    term: &mut Terminal,
    action: char,
    params: &Params,
) -> bool {
    match action {
        'r' => {
            // DECSTBM — Set Top and Bottom Margins
            let mut iter = params.iter();
            let top = iter.next().and_then(|p| p.first().copied()).unwrap_or(1);
            let bottom = iter
                .next()
                .and_then(|p| p.first().copied())
                .unwrap_or(term.grid.rows as u16);
            term.scroll_top = (top as usize).saturating_sub(1);
            term.scroll_bottom = (bottom as usize).saturating_sub(1).min(term.grid.rows - 1);
            term.cursor_row = 0;
            term.cursor_col = 0;
            true
        }
        'S' => {
            // Scroll Up
            let n = term.param(params, 1).max(1) as usize;
            for _ in 0..n {
                term.scroll_up_region(term.scroll_top, term.scroll_bottom);
            }
            true
        }
        'T' => {
            // Scroll Down
            let n = term.param(params, 1).max(1) as usize;
            for _ in 0..n {
                term.scroll_down_region(term.scroll_top, term.scroll_bottom);
            }
            true
        }
        'L' => {
            // Insert Lines
            let n = term.param(params, 1).max(1) as usize;
            for _ in 0..n {
                term.scroll_down_region(term.cursor_row, term.scroll_bottom);
            }
            true
        }
        'M' => {
            // Delete Lines
            let n = term.param(params, 1).max(1) as usize;
            for _ in 0..n {
                term.scroll_up_region(term.cursor_row, term.scroll_bottom);
            }
            true
        }
        _ => false,
    }
}
