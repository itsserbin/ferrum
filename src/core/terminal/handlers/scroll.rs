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

#[cfg(test)]
mod tests {
    use crate::core::Cell;
    use crate::core::terminal::Terminal;

    /// Helper: fill each row with a distinct character ('A' for row 0, 'B' for row 1, etc.)
    fn filled_term(rows: usize, cols: usize) -> Terminal {
        let mut term = Terminal::new(rows, cols);
        for r in 0..rows {
            let ch = (b'A' + r as u8) as char;
            for c in 0..cols {
                term.grid.set(
                    r,
                    c,
                    Cell {
                        character: ch,
                        ..Cell::default()
                    },
                );
            }
        }
        term
    }

    /// Helper: get the character in every column of a row as a single char
    /// (assumes all cols in a row have the same char for our tests).
    fn row_char(term: &Terminal, row: usize) -> char {
        term.grid.get(row, 0).character
    }

    #[test]
    fn decstbm_set_margins() {
        // \x1b[2;5r on a 10-row grid sets scroll region rows 1..4 (0-based)
        // and moves cursor to (0,0).
        // Since scroll_top/scroll_bottom are private, verify cursor position
        // and that scroll operations respect the margins.
        let mut term = Terminal::new(10, 10);
        term.cursor_row = 5;
        term.cursor_col = 3;
        term.process(b"\x1b[2;5r");

        // Cursor must be reset to (0,0)
        assert_eq!(term.cursor_row, 0);
        assert_eq!(term.cursor_col, 0);

        // Verify margins are set correctly by filling rows and scrolling.
        // Fill rows 0..9 with distinct chars.
        for r in 0..10 {
            let ch = (b'A' + r as u8) as char;
            for c in 0..10 {
                term.grid.set(
                    r,
                    c,
                    Cell {
                        character: ch,
                        ..Cell::default()
                    },
                );
            }
        }

        // Scroll up within the region: only rows 1..4 should move
        term.process(b"\x1b[1S");

        // Row 0 (outside region, above): unchanged
        assert_eq!(row_char(&term, 0), 'A');
        // Rows 1..3: shifted up from rows 2..4
        assert_eq!(row_char(&term, 1), 'C');
        assert_eq!(row_char(&term, 2), 'D');
        assert_eq!(row_char(&term, 3), 'E');
        // Row 4: blanked (bottom of region)
        assert_eq!(row_char(&term, 4), ' ');
        // Rows 5..9 (outside region, below): unchanged
        assert_eq!(row_char(&term, 5), 'F');
        assert_eq!(row_char(&term, 9), 'J');
    }

    #[test]
    fn decstbm_default_full_screen() {
        // \x1b[r with no params resets margins to full screen.
        let mut term = Terminal::new(6, 5);
        // First set custom margins
        term.process(b"\x1b[2;4r");
        // Then reset to full screen
        term.process(b"\x1b[r");

        // Fill rows and scroll: all rows should participate
        for r in 0..6 {
            let ch = (b'A' + r as u8) as char;
            for c in 0..5 {
                term.grid.set(
                    r,
                    c,
                    Cell {
                        character: ch,
                        ..Cell::default()
                    },
                );
            }
        }
        term.process(b"\x1b[1S");

        // Full screen scroll: row 0 shifted out, rows shift up
        assert_eq!(row_char(&term, 0), 'B');
        assert_eq!(row_char(&term, 4), 'F');
        assert_eq!(row_char(&term, 5), ' ');
    }

    #[test]
    fn su_scroll_up() {
        // Fill 4 rows ['A','B','C','D'], scroll up 1 => ['B','C','D',' ']
        // Top row 'A' should go to scrollback.
        let mut term = filled_term(4, 5);
        term.process(b"\x1b[1S");

        assert_eq!(row_char(&term, 0), 'B');
        assert_eq!(row_char(&term, 1), 'C');
        assert_eq!(row_char(&term, 2), 'D');
        assert_eq!(row_char(&term, 3), ' ');

        // Row A should be in scrollback
        assert_eq!(term.scrollback.len(), 1);
        assert_eq!(term.scrollback[0][0].character, 'A');
    }

    #[test]
    fn sd_scroll_down() {
        // Fill 4 rows ['A','B','C','D'], scroll down 1 => [' ','A','B','C']
        let mut term = filled_term(4, 5);
        term.process(b"\x1b[1T");

        assert_eq!(row_char(&term, 0), ' ');
        assert_eq!(row_char(&term, 1), 'A');
        assert_eq!(row_char(&term, 2), 'B');
        assert_eq!(row_char(&term, 3), 'C');
    }

    #[test]
    fn il_insert_lines() {
        // Rows ['A','B','C','D'], cursor at row 1, insert 1 line
        // => ['A',' ','B','C'], row D lost
        let mut term = filled_term(4, 5);
        term.cursor_row = 1;
        term.process(b"\x1b[1L");

        assert_eq!(row_char(&term, 0), 'A');
        assert_eq!(row_char(&term, 1), ' ');
        assert_eq!(row_char(&term, 2), 'B');
        assert_eq!(row_char(&term, 3), 'C');
    }

    #[test]
    fn dl_delete_lines() {
        // Rows ['A','B','C','D'], cursor at row 1, delete 1 line
        // => ['A','C','D',' ']
        let mut term = filled_term(4, 5);
        term.cursor_row = 1;
        term.process(b"\x1b[1M");

        assert_eq!(row_char(&term, 0), 'A');
        assert_eq!(row_char(&term, 1), 'C');
        assert_eq!(row_char(&term, 2), 'D');
        assert_eq!(row_char(&term, 3), ' ');
    }

    #[test]
    fn scroll_within_margins() {
        // Set margins rows 1..2 (1-based: 2;3), fill rows, scroll up 1.
        // Only rows 1-2 scroll; rows 0 and 3 stay.
        let mut term = filled_term(4, 5);
        term.process(b"\x1b[2;3r");

        // Re-fill after DECSTBM (which resets cursor)
        for r in 0..4 {
            let ch = (b'A' + r as u8) as char;
            for c in 0..5 {
                term.grid.set(
                    r,
                    c,
                    Cell {
                        character: ch,
                        ..Cell::default()
                    },
                );
            }
        }

        term.process(b"\x1b[1S");

        // Row 0 (above region): unchanged
        assert_eq!(row_char(&term, 0), 'A');
        // Row 1: was row 2
        assert_eq!(row_char(&term, 1), 'C');
        // Row 2: blanked (bottom of region)
        assert_eq!(row_char(&term, 2), ' ');
        // Row 3 (below region): unchanged
        assert_eq!(row_char(&term, 3), 'D');
    }
}
