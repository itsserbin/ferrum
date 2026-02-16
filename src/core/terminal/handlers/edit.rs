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

#[cfg(test)]
mod tests {
    use crate::core::terminal::Terminal;
    use crate::core::Cell;

    /// Helper: write a string into row 0 starting at col 0.
    fn write_row(term: &mut Terminal, text: &str) {
        for (i, ch) in text.chars().enumerate() {
            term.grid.set(0, i, Cell { character: ch, ..Cell::default() });
        }
    }

    /// Helper: read row 0 as a String (all cols).
    fn read_row(term: &Terminal) -> String {
        (0..term.grid.cols)
            .map(|c| term.grid.get(0, c).character)
            .collect()
    }

    #[test]
    fn dch_delete_chars() {
        // Row "ABCDE", cursor at col 1, delete 2 chars => "ADE  "
        let mut term = Terminal::new(4, 5);
        write_row(&mut term, "ABCDE");
        term.cursor_col = 1;
        term.process(b"\x1b[2P");

        assert_eq!(read_row(&term), "ADE  ");
    }

    #[test]
    fn ich_insert_chars() {
        // Row "ABCDE", cursor at col 1, insert 2 blank chars => "A  BC"
        // D and E are pushed off the right edge.
        let mut term = Terminal::new(4, 5);
        write_row(&mut term, "ABCDE");
        term.cursor_col = 1;
        term.process(b"\x1b[2@");

        assert_eq!(read_row(&term), "A  BC");
    }

    #[test]
    fn ech_erase_chars() {
        // Row "ABCDE", cursor at col 1, erase 2 chars => "A  DE" (no shift)
        let mut term = Terminal::new(4, 5);
        write_row(&mut term, "ABCDE");
        term.cursor_col = 1;
        term.process(b"\x1b[2X");

        assert_eq!(read_row(&term), "A  DE");
    }

    #[test]
    fn dch_default_one() {
        // \x1b[P without param deletes 1 char.
        // Row "ABCDE", cursor at col 1 => "ACDE "
        let mut term = Terminal::new(4, 5);
        write_row(&mut term, "ABCDE");
        term.cursor_col = 1;
        term.process(b"\x1b[P");

        assert_eq!(read_row(&term), "ACDE ");
    }

    #[test]
    fn ich_at_end_of_line() {
        // Cursor at last col, insert 1 blank => last cell becomes blank.
        // Row "ABCDE", cursor at col 4 (last), insert 1 => "ABCD "
        let mut term = Terminal::new(4, 5);
        write_row(&mut term, "ABCDE");
        term.cursor_col = 4;
        term.process(b"\x1b[1@");

        assert_eq!(read_row(&term), "ABCD ");
    }
}
