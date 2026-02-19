use super::Terminal;
use crate::core::Color;

// ── Alternate screen mode ──

#[test]
fn toggles_alternate_screen_mode() {
    let mut term = Terminal::new(4, 4);
    assert!(!term.is_alt_screen());

    term.process(b"\x1b[?1049h");
    assert!(term.is_alt_screen());

    term.process(b"\x1b[?1049l");
    assert!(!term.is_alt_screen());
}

// ── SGR attributes ──

#[test]
fn applies_sgr_and_resets_attributes() {
    let mut term = Terminal::new(2, 4);
    term.process(b"\x1b[31mA\x1b[0mB");

    assert_eq!(term.grid.get(0, 0).unwrap().fg, Color::ANSI[1]);
    assert_eq!(term.grid.get(0, 1).unwrap().fg, Color::DEFAULT_FG);
}

// ── Device status reports ──

#[test]
fn reports_cursor_position_response() {
    let mut term = Terminal::new(3, 5);
    term.cursor_row = 1;
    term.cursor_col = 3;

    term.process(b"\x1b[6n");

    assert_eq!(term.drain_responses(), b"\x1b[2;4R".to_vec());
}

#[test]
fn reports_cursor_position_response_for_private_query() {
    let mut term = Terminal::new(3, 5);
    term.cursor_row = 2;
    term.cursor_col = 4;

    term.process(b"\x1b[?6n");

    assert_eq!(term.drain_responses(), b"\x1b[3;5R".to_vec());
}

// ── Perform trait: print ──

#[test]
fn print_simple_text() {
    let mut term = Terminal::new(4, 80);
    term.process(b"Hello");

    assert_eq!(term.grid.get(0, 0).unwrap().character, 'H');
    assert_eq!(term.grid.get(0, 1).unwrap().character, 'e');
    assert_eq!(term.grid.get(0, 2).unwrap().character, 'l');
    assert_eq!(term.grid.get(0, 3).unwrap().character, 'l');
    assert_eq!(term.grid.get(0, 4).unwrap().character, 'o');
    assert_eq!(term.cursor_col, 5);
}

#[test]
fn print_wraps_at_edge() {
    let mut term = Terminal::new(4, 80);
    // Print 82 characters: 80 fill row 0, 2 wrap to row 1
    let text: String = (0..82).map(|i| (b'A' + (i % 26) as u8) as char).collect();
    term.process(text.as_bytes());

    // Row 0 should be full (80 chars)
    assert_eq!(term.grid.get(0, 0).unwrap().character, 'A');
    assert_eq!(term.grid.get(0, 79).unwrap().character, 'B'); // index 79: 79%26=1 -> 'B'

    // Row 1 should have 2 chars
    assert_eq!(term.grid.get(1, 0).unwrap().character, 'C'); // index 80: 80%26=2 -> 'C'
    assert_eq!(term.grid.get(1, 1).unwrap().character, 'D'); // index 81: 81%26=3 -> 'D'
    assert_eq!(term.cursor_row, 1);
    assert_eq!(term.cursor_col, 2);
}

#[test]
fn print_wide_char() {
    let mut term = Terminal::new(4, 80);
    // CJK character is 2 columns wide
    term.process("漢".as_bytes());

    assert_eq!(term.grid.get(0, 0).unwrap().character, '漢');
    assert_eq!(term.grid.get(0, 1).unwrap().character, ' '); // placeholder
    assert_eq!(term.cursor_col, 2);
}

#[test]
fn print_combining_mark_is_not_dropped() {
    let mut term = Terminal::new(4, 80);
    term.process("e\u{0301}".as_bytes()); // e + combining acute accent

    assert_eq!(term.grid.get(0, 0).unwrap().character, 'e');
    assert_eq!(term.grid.get(0, 1).unwrap().character, '\u{0301}');
    assert_eq!(term.cursor_col, 2);
}

// ── Perform trait: execute ──

#[test]
fn execute_lf() {
    let mut term = Terminal::new(4, 80);
    term.process(b"A\nB");

    assert_eq!(term.grid.get(0, 0).unwrap().character, 'A');
    // LF moves down one row; col stays after A (col 1).
    assert_eq!(term.grid.get(1, 1).unwrap().character, 'B');
}

#[test]
fn execute_vt_and_ff_behave_like_newline() {
    let mut term = Terminal::new(6, 80);
    term.process(b"A\x0bB\x0cC");

    assert_eq!(term.grid.get(0, 0).unwrap().character, 'A');
    assert_eq!(term.grid.get(1, 1).unwrap().character, 'B');
    assert_eq!(term.grid.get(2, 2).unwrap().character, 'C');
}

#[test]
fn execute_cr() {
    let mut term = Terminal::new(4, 80);
    term.process(b"ABC\rX");

    // CR resets col to 0, X overwrites A
    assert_eq!(term.grid.get(0, 0).unwrap().character, 'X');
    assert_eq!(term.grid.get(0, 1).unwrap().character, 'B');
    assert_eq!(term.grid.get(0, 2).unwrap().character, 'C');
}

#[test]
fn execute_backspace() {
    let mut term = Terminal::new(4, 80);
    term.process(b"AB\x08X");

    // Backspace moves cursor back one; X overwrites B
    assert_eq!(term.grid.get(0, 0).unwrap().character, 'A');
    assert_eq!(term.grid.get(0, 1).unwrap().character, 'X');
}

#[test]
fn execute_tab() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\tX");

    // Tab stops every 8 columns: col 0 -> col 8
    assert_eq!(term.grid.get(0, 8).unwrap().character, 'X');
    assert_eq!(term.cursor_col, 9);
}

// ── Perform trait: esc_dispatch ──

#[test]
fn esc_save_restore_cursor() {
    let mut term = Terminal::new(10, 80);
    // Move cursor to (3, 5) using CUP
    term.process(b"\x1b[4;6H"); // 1-based: row 4, col 6 -> 0-based: (3, 5)
    assert_eq!(term.cursor_row, 3);
    assert_eq!(term.cursor_col, 5);

    // ESC 7 = save cursor
    term.process(b"\x1b7");

    // Move somewhere else
    term.process(b"\x1b[1;1H"); // move to (0, 0)
    assert_eq!(term.cursor_row, 0);
    assert_eq!(term.cursor_col, 0);

    // ESC 8 = restore cursor
    term.process(b"\x1b8");
    assert_eq!(term.cursor_row, 3);
    assert_eq!(term.cursor_col, 5);
}

#[test]
fn esc_reverse_index_at_top() {
    let mut term = Terminal::new(4, 10);
    // Fill rows with identifiable content
    term.process(b"AAAAAAAAAA"); // row 0
    term.process(b"\n");
    term.cursor_col = 0;
    term.process(b"BBBBBBBBBB"); // row 1

    // Move cursor to row 0
    term.process(b"\x1b[1;1H"); // CUP to (0, 0)
    assert_eq!(term.cursor_row, 0);

    // ESC M = Reverse Index at top => scroll_down_region
    term.process(b"\x1bM");

    // Row 0 should now be blank (scroll_down inserts blank at top)
    assert_eq!(term.grid.get(0, 0).unwrap().character, ' ');
    // Old row 0 ('A') should have moved to row 1
    assert_eq!(term.grid.get(1, 0).unwrap().character, 'A');
    // Old row 1 ('B') should have moved to row 2
    assert_eq!(term.grid.get(2, 0).unwrap().character, 'B');
}

#[test]
fn esc_ris_full_reset() {
    let mut term = Terminal::new(4, 80);
    // Set some attributes and move cursor
    term.process(b"\x1b[1;31mHello");
    assert!(term.grid.get(0, 0).unwrap().bold);
    assert_eq!(term.cursor_col, 5);

    // ESC c = RIS (full reset)
    term.process(b"\x1bc");

    assert_eq!(term.cursor_row, 0);
    assert_eq!(term.cursor_col, 0);
    assert_eq!(term.grid.get(0, 0).unwrap().character, ' '); // grid cleared
    assert!(term.scrollback.is_empty());
    assert!(term.cursor_visible);
    assert!(!term.decckm);
}

// ── Scrolling behavior ──

#[test]
fn lf_at_bottom_scrolls() {
    let mut term = Terminal::new(4, 10);
    // Fill all 4 rows
    for row_char in b"ABCD" {
        let line: Vec<u8> = vec![*row_char; 10];
        term.process(&line);
        if *row_char != b'D' {
            term.process(b"\n");
            term.cursor_col = 0;
        }
    }
    // Cursor should be at row 3 (bottom)
    assert_eq!(term.cursor_row, 3);
    assert_eq!(term.grid.get(0, 0).unwrap().character, 'A');

    // LF at bottom triggers scroll
    term.process(b"\n");

    // Row A should go to scrollback
    assert_eq!(term.scrollback.len(), 1);
    assert_eq!(term.scrollback[0].cells[0].character, 'A');

    // Content shifts up: B->row0, C->row1, D->row2, blank->row3
    assert_eq!(term.grid.get(0, 0).unwrap().character, 'B');
    assert_eq!(term.grid.get(1, 0).unwrap().character, 'C');
    assert_eq!(term.grid.get(2, 0).unwrap().character, 'D');
    assert_eq!(term.grid.get(3, 0).unwrap().character, ' ');
}

#[test]
fn scrollback_preserved() {
    let mut term = Terminal::new(4, 10);
    // Produce enough LFs to scroll many times
    // Each LF at the bottom will push a row to scrollback
    for i in 0..20u8 {
        let ch = b'A' + (i % 26);
        let line: Vec<u8> = vec![ch; 10];
        term.process(&line);
        term.process(b"\n");
        term.cursor_col = 0;
    }

    // We scrolled 17 times (20 lines - 4 visible + 1 for the last \n)
    // Scrollback should have grown
    assert!(!term.scrollback.is_empty());
    // Scrollback should not exceed max (1000)
    assert!(term.scrollback.len() <= 1000);
}
