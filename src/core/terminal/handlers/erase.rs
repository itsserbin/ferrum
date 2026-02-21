use crate::core::terminal::Terminal;
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
                    let blank = term.make_blank_cell();
                    for col in term.cursor_col..term.grid.cols {
                        term.grid.set(term.cursor_row, col, blank.clone());
                    }
                    for row in (term.cursor_row + 1)..term.grid.rows {
                        for col in 0..term.grid.cols {
                            term.grid.set(row, col, blank.clone());
                        }
                    }
                }
                1 => {
                    let blank = term.make_blank_cell();
                    for row in 0..term.cursor_row {
                        for col in 0..term.grid.cols {
                            term.grid.set(row, col, blank.clone());
                        }
                    }
                    for col in 0..=term.cursor_col.min(term.grid.cols - 1) {
                        term.grid.set(term.cursor_row, col, blank.clone());
                    }
                }
                2 => {
                    let blank = term.make_blank_cell();
                    for row in 0..term.grid.rows {
                        for col in 0..term.grid.cols {
                            term.grid.set(row, col, blank.clone());
                        }
                    }
                }
                3 => {
                    let cleared = term.scrollback.len();
                    term.scrollback.clear();
                    // Keep selection anchoring in sync with absolute-row space.
                    term.scrollback_popped = term.scrollback_popped.saturating_add(cleared);
                }
                _ => {}
            }
            true
        }
        'K' => {
            match term.param(params, 0) {
                0 => {
                    let blank = term.make_blank_cell();
                    for col in term.cursor_col..term.grid.cols {
                        term.grid.set(term.cursor_row, col, blank.clone());
                    }
                }
                1 => {
                    let blank = term.make_blank_cell();
                    for col in 0..=term.cursor_col.min(term.grid.cols - 1) {
                        term.grid.set(term.cursor_row, col, blank.clone());
                    }
                }
                2 => {
                    let blank = term.make_blank_cell();
                    for col in 0..term.grid.cols {
                        term.grid.set(term.cursor_row, col, blank.clone());
                    }
                }
                _ => {}
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

    fn filled_term(rows: usize, cols: usize) -> Terminal {
        let mut term = Terminal::new(rows, cols);
        for r in 0..rows {
            for c in 0..cols {
                term.grid.set(
                    r,
                    c,
                    Cell {
                        character: 'A',
                        ..Cell::default()
                    },
                );
            }
        }
        term
    }

    #[test]
    fn ed_erase_below() {
        let mut term = filled_term(4, 10);
        term.cursor_row = 1;
        term.cursor_col = 3;
        term.process(b"\x1b[0J");
        // Before cursor on row 1: preserved
        for c in 0..3 {
            assert_eq!(
                term.grid.get_unchecked(1, c).character,
                'A',
                "row 1 col {} should be A",
                c
            );
        }
        // Row 0: fully preserved
        for c in 0..10 {
            assert_eq!(
                term.grid.get_unchecked(0, c).character,
                'A',
                "row 0 col {} should be A",
                c
            );
        }
        // From cursor to end of row 1: erased
        for c in 3..10 {
            assert_eq!(
                term.grid.get_unchecked(1, c).character,
                ' ',
                "row 1 col {} should be erased",
                c
            );
        }
        // Rows below: fully erased
        for r in 2..4 {
            for c in 0..10 {
                assert_eq!(
                    term.grid.get_unchecked(r, c).character,
                    ' ',
                    "row {} col {} should be erased",
                    r,
                    c
                );
            }
        }
    }

    #[test]
    fn ed_erase_above() {
        let mut term = filled_term(4, 10);
        term.cursor_row = 1;
        term.cursor_col = 3;
        term.process(b"\x1b[1J");
        // Row 0: fully erased
        for c in 0..10 {
            assert_eq!(
                term.grid.get_unchecked(0, c).character,
                ' ',
                "row 0 col {} should be erased",
                c
            );
        }
        // Row 1 up to and including cursor: erased
        for c in 0..=3 {
            assert_eq!(
                term.grid.get_unchecked(1, c).character,
                ' ',
                "row 1 col {} should be erased",
                c
            );
        }
        // Row 1 after cursor: preserved
        for c in 4..10 {
            assert_eq!(
                term.grid.get_unchecked(1, c).character,
                'A',
                "row 1 col {} should be A",
                c
            );
        }
        // Rows below: preserved
        for r in 2..4 {
            for c in 0..10 {
                assert_eq!(
                    term.grid.get_unchecked(r, c).character,
                    'A',
                    "row {} col {} should be A",
                    r,
                    c
                );
            }
        }
    }

    #[test]
    fn ed_erase_all() {
        let mut term = filled_term(4, 10);
        term.process(b"\x1b[2J");
        for r in 0..4 {
            for c in 0..10 {
                assert_eq!(
                    term.grid.get_unchecked(r, c).character,
                    ' ',
                    "row {} col {} should be erased",
                    r,
                    c
                );
            }
        }
    }

    #[test]
    fn ed_erase_saved_lines_only() {
        let mut term = Terminal::new(2, 4);
        term.process(b"AAAA\nBBBB\nCCCC\n");
        assert!(
            !term.scrollback.is_empty(),
            "expected scrollback before CSI 3J"
        );
        let visible_before = term.grid.get_unchecked(0, 0).character;

        term.process(b"\x1b[3J");

        assert!(term.scrollback.is_empty(), "CSI 3J should clear scrollback");
        assert_eq!(
            term.grid.get_unchecked(0, 0).character,
            visible_before,
            "CSI 3J should not clear visible grid"
        );
    }

    #[test]
    fn ed_default_is_erase_below() {
        let mut term = filled_term(4, 10);
        term.cursor_row = 1;
        term.cursor_col = 3;
        term.process(b"\x1b[J");
        // Same as 0J: from cursor to end erased
        for c in 3..10 {
            assert_eq!(
                term.grid.get_unchecked(1, c).character,
                ' ',
                "row 1 col {} should be erased",
                c
            );
        }
        for r in 2..4 {
            for c in 0..10 {
                assert_eq!(
                    term.grid.get_unchecked(r, c).character,
                    ' ',
                    "row {} col {} should be erased",
                    r,
                    c
                );
            }
        }
        // Before cursor preserved
        for c in 0..3 {
            assert_eq!(
                term.grid.get_unchecked(1, c).character,
                'A',
                "row 1 col {} should be A",
                c
            );
        }
        for c in 0..10 {
            assert_eq!(
                term.grid.get_unchecked(0, c).character,
                'A',
                "row 0 col {} should be A",
                c
            );
        }
    }

    #[test]
    fn el_erase_right() {
        let mut term = filled_term(4, 10);
        term.cursor_row = 0;
        term.cursor_col = 3;
        term.process(b"\x1b[0K");
        // Cols 0..2 preserved
        for c in 0..3 {
            assert_eq!(
                term.grid.get_unchecked(0, c).character,
                'A',
                "col {} should be A",
                c
            );
        }
        // Cols 3..9 erased
        for c in 3..10 {
            assert_eq!(
                term.grid.get_unchecked(0, c).character,
                ' ',
                "col {} should be erased",
                c
            );
        }
    }

    #[test]
    fn el_erase_left() {
        let mut term = filled_term(4, 10);
        term.cursor_row = 0;
        term.cursor_col = 3;
        term.process(b"\x1b[1K");
        // Cols 0..3 erased (inclusive)
        for c in 0..=3 {
            assert_eq!(
                term.grid.get_unchecked(0, c).character,
                ' ',
                "col {} should be erased",
                c
            );
        }
        // Cols 4..9 preserved
        for c in 4..10 {
            assert_eq!(
                term.grid.get_unchecked(0, c).character,
                'A',
                "col {} should be A",
                c
            );
        }
    }

    #[test]
    fn el_erase_whole_line() {
        let mut term = filled_term(4, 10);
        term.cursor_row = 0;
        term.cursor_col = 5;
        term.process(b"\x1b[2K");
        for c in 0..10 {
            assert_eq!(
                term.grid.get_unchecked(0, c).character,
                ' ',
                "col {} should be erased",
                c
            );
        }
    }

    #[test]
    fn el_default_is_erase_right() {
        let mut term = filled_term(4, 10);
        term.cursor_row = 0;
        term.cursor_col = 3;
        term.process(b"\x1b[K");
        // Same as 0K
        for c in 0..3 {
            assert_eq!(
                term.grid.get_unchecked(0, c).character,
                'A',
                "col {} should be A",
                c
            );
        }
        for c in 3..10 {
            assert_eq!(
                term.grid.get_unchecked(0, c).character,
                ' ',
                "col {} should be erased",
                c
            );
        }
    }

    #[test]
    fn ed_erase_inherits_current_bg() {
        use crate::core::Color;
        let mut term = Terminal::new(4, 10);
        let red = Color { r: 255, g: 0, b: 0 };
        term.process(b"\x1b[48;2;255;0;0m");
        term.process(b"\x1b[2J");
        assert_eq!(term.grid.get_unchecked(0, 0).bg, red);
        assert_eq!(term.grid.get_unchecked(3, 9).bg, red);
    }

    #[test]
    fn el_erase_right_inherits_current_bg() {
        use crate::core::Color;
        let mut term = Terminal::new(4, 10);
        let blue = Color { r: 0, g: 0, b: 255 };
        term.process(b"AAAA");
        term.cursor_col = 0;
        term.process(b"\x1b[48;2;0;0;255m\x1b[2K");
        assert_eq!(term.grid.get_unchecked(0, 0).bg, blue);
        assert_eq!(term.grid.get_unchecked(0, 9).bg, blue);
    }
}
