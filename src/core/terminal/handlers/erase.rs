use crate::core::terminal::Terminal;
use vte::Params;

pub(in super::super) fn handle_erase_csi(
    term: &mut Terminal,
    action: char,
    params: &Params,
) -> bool {
    match action {
        'J' => {
            let cr = term.cursor_row();
            let cc = term.cursor_col();
            match term.param(params, 0) {
                0 => {
                    let blank = term.make_blank_grapheme_cell();
                    for col in cc..term.screen.cols() {
                        term.screen.viewport_set(cr, col, blank.clone());
                    }
                    for row in (cr + 1)..term.screen.viewport_rows() {
                        term.screen.viewport_row_mut(row).clear_with(blank.clone());
                    }
                }
                1 => {
                    let blank = term.make_blank_grapheme_cell();
                    for row in 0..cr {
                        term.screen.viewport_row_mut(row).clear_with(blank.clone());
                    }
                    for col in 0..=cc.min(term.screen.cols().saturating_sub(1)) {
                        term.screen.viewport_set(cr, col, blank.clone());
                    }
                }
                2 => {
                    let blank = term.make_blank_grapheme_cell();
                    for row in 0..term.screen.viewport_rows() {
                        term.screen.viewport_row_mut(row).clear_with(blank.clone());
                    }
                }
                3 => {
                    // Clear page-based scrollback.
                    // Rebuild screen with empty scrollback to stay in sync.
                    let rows = term.screen.viewport_rows();
                    let cols = term.screen.cols();
                    let max_sb = term.max_scrollback;
                    let mut new_screen = crate::core::PageList::new(rows, cols, max_sb);
                    for r in 0..rows {
                        for c in 0..cols {
                            let gc = term.screen.viewport_get(r, c).clone();
                            new_screen.viewport_set(r, c, gc);
                        }
                        new_screen.viewport_set_wrapped(r, term.screen.viewport_is_wrapped(r));
                    }
                    let abs = new_screen.viewport_start_abs() + cr;
                    let new_cursor_pin =
                        crate::core::PageList::pin_at(crate::core::PageCoord { abs_row: abs, col: cc });
                    term.cursor_pin = new_cursor_pin;
                    term.screen = new_screen;
                }
                _ => {}
            }
            true
        }
        'K' => {
            let cr = term.cursor_row();
            let cc = term.cursor_col();
            match term.param(params, 0) {
                0 => {
                    let blank = term.make_blank_grapheme_cell();
                    for col in cc..term.screen.cols() {
                        term.screen.viewport_set(cr, col, blank.clone());
                    }
                }
                1 => {
                    let blank = term.make_blank_grapheme_cell();
                    for col in 0..=cc.min(term.screen.cols().saturating_sub(1)) {
                        term.screen.viewport_set(cr, col, blank.clone());
                    }
                }
                2 => {
                    let blank = term.make_blank_grapheme_cell();
                    term.screen.viewport_row_mut(cr).clear_with(blank);
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
    use crate::core::GraphemeCell;
    use crate::core::terminal::Terminal;

    fn filled_term(rows: usize, cols: usize) -> Terminal {
        let mut term = Terminal::new(rows, cols);
        let cell = GraphemeCell::from_char('A');
        for r in 0..rows {
            for c in 0..cols {
                term.screen.viewport_set(r, c, cell.clone());
            }
        }
        term
    }

    fn get_char(term: &Terminal, row: usize, col: usize) -> char {
        term.screen.viewport_get(row, col).first_char()
    }

    #[test]
    fn ed_erase_below() {
        let mut term = filled_term(4, 10);
        term.set_cursor(1, 3);
        term.process(b"\x1b[0J");
        // Before cursor on row 1: preserved
        for c in 0..3 {
            assert_eq!(
                get_char(&term, 1, c),
                'A',
                "row 1 col {} should be A",
                c
            );
        }
        // Row 0: fully preserved
        for c in 0..10 {
            assert_eq!(
                get_char(&term, 0, c),
                'A',
                "row 0 col {} should be A",
                c
            );
        }
        // From cursor to end of row 1: erased
        for c in 3..10 {
            assert_eq!(
                get_char(&term, 1, c),
                ' ',
                "row 1 col {} should be erased",
                c
            );
        }
        // Rows below: fully erased
        for r in 2..4 {
            for c in 0..10 {
                assert_eq!(
                    get_char(&term, r, c),
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
        term.set_cursor(1, 3);
        term.process(b"\x1b[1J");
        // Row 0: fully erased
        for c in 0..10 {
            assert_eq!(
                get_char(&term, 0, c),
                ' ',
                "row 0 col {} should be erased",
                c
            );
        }
        // Row 1 up to and including cursor: erased
        for c in 0..=3 {
            assert_eq!(
                get_char(&term, 1, c),
                ' ',
                "row 1 col {} should be erased",
                c
            );
        }
        // Row 1 after cursor: preserved
        for c in 4..10 {
            assert_eq!(
                get_char(&term, 1, c),
                'A',
                "row 1 col {} should be A",
                c
            );
        }
        // Rows below: preserved
        for r in 2..4 {
            for c in 0..10 {
                assert_eq!(
                    get_char(&term, r, c),
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
                    get_char(&term, r, c),
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
            term.screen.scrollback_len() > 0,
            "expected scrollback before CSI 3J"
        );
        let visible_before = get_char(&term, 0, 0);

        term.process(b"\x1b[3J");

        assert_eq!(term.screen.scrollback_len(), 0, "CSI 3J should clear scrollback");
        assert_eq!(
            get_char(&term, 0, 0),
            visible_before,
            "CSI 3J should not clear visible grid"
        );
    }

    #[test]
    fn ed_default_is_erase_below() {
        let mut term = filled_term(4, 10);
        term.set_cursor(1, 3);
        term.process(b"\x1b[J");
        // Same as 0J: from cursor to end erased
        for c in 3..10 {
            assert_eq!(
                get_char(&term, 1, c),
                ' ',
                "row 1 col {} should be erased",
                c
            );
        }
        for r in 2..4 {
            for c in 0..10 {
                assert_eq!(
                    get_char(&term, r, c),
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
                get_char(&term, 1, c),
                'A',
                "row 1 col {} should be A",
                c
            );
        }
        for c in 0..10 {
            assert_eq!(
                get_char(&term, 0, c),
                'A',
                "row 0 col {} should be A",
                c
            );
        }
    }

    #[test]
    fn el_erase_right() {
        let mut term = filled_term(4, 10);
        term.set_cursor(0, 3);
        term.process(b"\x1b[0K");
        // Cols 0..2 preserved
        for c in 0..3 {
            assert_eq!(
                get_char(&term, 0, c),
                'A',
                "col {} should be A",
                c
            );
        }
        // Cols 3..9 erased
        for c in 3..10 {
            assert_eq!(
                get_char(&term, 0, c),
                ' ',
                "col {} should be erased",
                c
            );
        }
    }

    #[test]
    fn el_erase_left() {
        let mut term = filled_term(4, 10);
        term.set_cursor(0, 3);
        term.process(b"\x1b[1K");
        // Cols 0..3 erased (inclusive)
        for c in 0..=3 {
            assert_eq!(
                get_char(&term, 0, c),
                ' ',
                "col {} should be erased",
                c
            );
        }
        // Cols 4..9 preserved
        for c in 4..10 {
            assert_eq!(
                get_char(&term, 0, c),
                'A',
                "col {} should be A",
                c
            );
        }
    }

    #[test]
    fn el_erase_whole_line() {
        let mut term = filled_term(4, 10);
        term.set_cursor(0, 5);
        term.process(b"\x1b[2K");
        for c in 0..10 {
            assert_eq!(
                get_char(&term, 0, c),
                ' ',
                "col {} should be erased",
                c
            );
        }
    }

    #[test]
    fn el_default_is_erase_right() {
        let mut term = filled_term(4, 10);
        term.set_cursor(0, 3);
        term.process(b"\x1b[K");
        // Same as 0K
        for c in 0..3 {
            assert_eq!(
                get_char(&term, 0, c),
                'A',
                "col {} should be A",
                c
            );
        }
        for c in 3..10 {
            assert_eq!(
                get_char(&term, 0, c),
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
        assert_eq!(term.screen.viewport_get(0, 0).bg, red);
        assert_eq!(term.screen.viewport_get(3, 9).bg, red);
    }

    #[test]
    fn el_erase_right_inherits_current_bg() {
        use crate::core::Color;
        let mut term = Terminal::new(4, 10);
        let blue = Color { r: 0, g: 0, b: 255 };
        term.process(b"AAAA");
        term.set_cursor_col(0);
        term.process(b"\x1b[48;2;0;0;255m\x1b[2K");
        assert_eq!(term.screen.viewport_get(0, 0).bg, blue);
        assert_eq!(term.screen.viewport_get(0, 9).bg, blue);
    }
}
