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

#[cfg(test)]
mod tests {
    use crate::core::terminal::Terminal;

    #[test]
    fn cup_goto_position() {
        let mut term = Terminal::new(10, 20);
        term.process(b"\x1b[3;5H");
        assert_eq!(term.cursor_row, 2);
        assert_eq!(term.cursor_col, 4);
    }

    #[test]
    fn cup_default_home() {
        let mut term = Terminal::new(10, 20);
        term.cursor_row = 5;
        term.cursor_col = 10;
        term.process(b"\x1b[H");
        assert_eq!(term.cursor_row, 0);
        assert_eq!(term.cursor_col, 0);
    }

    #[test]
    fn cup_f_variant() {
        let mut term = Terminal::new(10, 20);
        term.process(b"\x1b[2;3f");
        assert_eq!(term.cursor_row, 1);
        assert_eq!(term.cursor_col, 2);
    }

    #[test]
    fn cuu_move_up() {
        let mut term = Terminal::new(10, 20);
        term.cursor_row = 5;
        term.process(b"\x1b[3A");
        assert_eq!(term.cursor_row, 2);
    }

    #[test]
    fn cuu_clamp_top() {
        let mut term = Terminal::new(10, 20);
        term.cursor_row = 1;
        term.process(b"\x1b[10A");
        assert_eq!(term.cursor_row, 0);
    }

    #[test]
    fn cud_move_down() {
        let mut term = Terminal::new(10, 20);
        term.cursor_row = 0;
        term.process(b"\x1b[3B");
        assert_eq!(term.cursor_row, 3);
    }

    #[test]
    fn cud_clamp_bottom() {
        let mut term = Terminal::new(10, 20);
        term.cursor_row = 5;
        term.process(b"\x1b[20B");
        assert_eq!(term.cursor_row, 9);
    }

    #[test]
    fn cuf_move_right() {
        let mut term = Terminal::new(10, 20);
        term.cursor_col = 0;
        term.process(b"\x1b[5C");
        assert_eq!(term.cursor_col, 5);
    }

    #[test]
    fn cuf_clamp_right() {
        let mut term = Terminal::new(10, 20);
        term.cursor_col = 15;
        term.process(b"\x1b[30C");
        assert_eq!(term.cursor_col, 19);
    }

    #[test]
    fn cub_move_left() {
        let mut term = Terminal::new(10, 20);
        term.cursor_col = 10;
        term.process(b"\x1b[3D");
        assert_eq!(term.cursor_col, 7);
    }

    #[test]
    fn cub_clamp_left() {
        let mut term = Terminal::new(10, 20);
        term.cursor_col = 2;
        term.process(b"\x1b[10D");
        assert_eq!(term.cursor_col, 0);
    }

    #[test]
    fn cha_column_absolute() {
        let mut term = Terminal::new(10, 20);
        term.process(b"\x1b[10G");
        assert_eq!(term.cursor_col, 9);
    }

    #[test]
    fn vpa_row_absolute() {
        let mut term = Terminal::new(10, 20);
        term.process(b"\x1b[5d");
        assert_eq!(term.cursor_row, 4);
    }

    #[test]
    fn move_default_is_one() {
        let mut term = Terminal::new(10, 20);
        term.cursor_row = 5;
        term.process(b"\x1b[A");
        assert_eq!(term.cursor_row, 4);
    }
}
