use super::Terminal;
use crate::core::Color;

#[test]
fn toggles_alternate_screen_mode() {
    let mut term = Terminal::new(4, 4);
    assert!(!term.is_alt_screen());

    term.process(b"\x1b[?1049h");
    assert!(term.is_alt_screen());

    term.process(b"\x1b[?1049l");
    assert!(!term.is_alt_screen());
}

#[test]
fn applies_sgr_and_resets_attributes() {
    let mut term = Terminal::new(2, 4);
    term.process(b"\x1b[31mA\x1b[0mB");

    assert_eq!(term.grid.get(0, 0).fg, Color::ANSI[1]);
    assert_eq!(term.grid.get(0, 1).fg, Color::DEFAULT_FG);
}

#[test]
fn reports_cursor_position_response() {
    let mut term = Terminal::new(3, 5);
    term.cursor_row = 1;
    term.cursor_col = 3;

    term.process(b"\x1b[6n");

    assert_eq!(term.drain_responses(), b"\x1b[2;4R".to_vec());
}
