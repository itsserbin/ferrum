use crate::core::terminal::Terminal;
use crate::core::{CursorStyle, MouseMode};
use vte::Params;

pub(in super::super) fn handle_private_mode(
    term: &mut Terminal,
    params: &Params,
    intermediates: &[u8],
    action: char,
) -> bool {
    if intermediates != [b'?'] {
        return false;
    }
    if action != 'h' && action != 'l' {
        return false;
    }

    // DEC private modes may contain multiple semicolon-separated params (e.g. ?1002;1006h).
    for param in params.iter() {
        let Some(mode) = param.first().copied() else {
            continue;
        };
        match (action, mode) {
            ('h', 1) => term.set_decckm(true), // DECCKM: application cursor keys
            ('l', 1) => term.set_decckm(false), // DECCKM: normal cursor keys
            ('h', 25) => term.set_cursor_visible(true),
            ('l', 25) => term.set_cursor_visible(false),
            ('h', 1049) => term.enter_alt_screen(),
            ('l', 1049) => term.leave_alt_screen(),
            // Mouse tracking modes
            ('h', 1000) => term.set_mouse_mode(MouseMode::Normal),
            ('l', 1000) => term.set_mouse_mode(MouseMode::Off),
            ('h', 1002) => term.set_mouse_mode(MouseMode::ButtonEvent),
            ('l', 1002) => term.set_mouse_mode(MouseMode::Off),
            ('h', 1003) => term.set_mouse_mode(MouseMode::AnyEvent),
            ('l', 1003) => term.set_mouse_mode(MouseMode::Off),
            // SGR extended mouse format
            ('h', 1006) => term.set_sgr_mouse(true),
            ('l', 1006) => term.set_sgr_mouse(false),
            // Bracketed paste mode
            ('h', 2004) => term.set_bracketed_paste(true),
            ('l', 2004) => term.set_bracketed_paste(false),
            _ => {}
        }
    }
    true
}

pub(in super::super) fn handle_cursor_style_csi(
    term: &mut Terminal,
    params: &Params,
    intermediates: &[u8],
    action: char,
) -> bool {
    if intermediates != [b' '] || action != 'q' {
        return false;
    }

    // DECSCUSR — Set Cursor Style: CSI Ps SP q
    let style = term.param(params, 0);
    term.set_cursor_style(match style {
        0 | 1 => CursorStyle::BlinkingBlock,
        2 => CursorStyle::SteadyBlock,
        3 => CursorStyle::BlinkingUnderline,
        4 => CursorStyle::SteadyUnderline,
        5 => CursorStyle::BlinkingBar,
        6 => CursorStyle::SteadyBar,
        _ => CursorStyle::BlinkingBlock,
    });
    true
}

#[cfg(test)]
mod tests {
    use crate::core::terminal::Terminal;
    use crate::core::{CursorStyle, MouseMode};

    #[test]
    fn decckm_application_mode() {
        let mut term = Terminal::new(4, 10);
        term.process(b"\x1b[?1h");
        assert!(term.decckm);
    }

    #[test]
    fn decckm_normal_mode() {
        let mut term = Terminal::new(4, 10);
        // First enable, then disable
        term.process(b"\x1b[?1h");
        term.process(b"\x1b[?1l");
        assert!(!term.decckm);
    }

    #[test]
    fn cursor_hide() {
        let mut term = Terminal::new(4, 10);
        assert!(term.cursor_visible);
        term.process(b"\x1b[?25l");
        assert!(!term.cursor_visible);
    }

    #[test]
    fn cursor_show() {
        let mut term = Terminal::new(4, 10);
        term.process(b"\x1b[?25l");
        term.process(b"\x1b[?25h");
        assert!(term.cursor_visible);
    }

    #[test]
    fn alt_screen_enter() {
        let mut term = Terminal::new(4, 10);
        term.cursor_row = 2;
        term.cursor_col = 5;
        term.process(b"\x1b[?1049h");

        assert!(term.is_alt_screen());
        assert_eq!(term.cursor_row, 0);
        assert_eq!(term.cursor_col, 0);
    }

    #[test]
    fn alt_screen_leave_restores() {
        let mut term = Terminal::new(4, 10);
        // Write something on main screen
        term.process(b"Hello");
        let saved_row = term.cursor_row;
        let saved_col = term.cursor_col;

        // Enter alt screen, write something different
        term.process(b"\x1b[?1049h");
        assert!(term.is_alt_screen());
        term.process(b"Alt");

        // Leave alt screen
        term.process(b"\x1b[?1049l");
        assert!(!term.is_alt_screen());

        // Cursor restored
        assert_eq!(term.cursor_row, saved_row);
        assert_eq!(term.cursor_col, saved_col);

        // Original grid restored: "Hello" should still be there
        assert_eq!(term.grid.get_unchecked(0, 0).character, 'H');
        assert_eq!(term.grid.get_unchecked(0, 4).character, 'o');
    }

    #[test]
    fn mouse_normal_mode() {
        let mut term = Terminal::new(4, 10);
        term.process(b"\x1b[?1000h");
        assert!(term.mouse_mode == MouseMode::Normal);
    }

    #[test]
    fn mouse_button_event() {
        let mut term = Terminal::new(4, 10);
        term.process(b"\x1b[?1002h");
        assert!(term.mouse_mode == MouseMode::ButtonEvent);
    }

    #[test]
    fn mouse_any_event() {
        let mut term = Terminal::new(4, 10);
        term.process(b"\x1b[?1003h");
        assert!(term.mouse_mode == MouseMode::AnyEvent);
    }

    #[test]
    fn private_mode_multi_param_enables_all_modes() {
        let mut term = Terminal::new(4, 10);
        term.process(b"\x1b[?1002;1006h");
        assert!(term.mouse_mode == MouseMode::ButtonEvent);
        assert!(term.sgr_mouse);

        term.process(b"\x1b[?1002;1006l");
        assert!(term.mouse_mode == MouseMode::Off);
        assert!(!term.sgr_mouse);
    }

    #[test]
    fn private_query_not_swallowed_by_mode_handler() {
        let mut term = Terminal::new(4, 10);
        term.cursor_row = 1;
        term.cursor_col = 2;

        term.process(b"\x1b[?6n");
        assert_eq!(term.drain_responses(), b"\x1b[2;3R".to_vec());
    }

    #[test]
    fn mouse_off() {
        let mut term = Terminal::new(4, 10);
        term.process(b"\x1b[?1000h");
        assert!(term.mouse_mode == MouseMode::Normal);
        term.process(b"\x1b[?1000l");
        assert!(term.mouse_mode == MouseMode::Off);
    }

    #[test]
    fn sgr_mouse_on_off() {
        let mut term = Terminal::new(4, 10);
        term.process(b"\x1b[?1006h");
        assert!(term.sgr_mouse);
        term.process(b"\x1b[?1006l");
        assert!(!term.sgr_mouse);
    }

    #[test]
    fn bracketed_paste_on_off() {
        let mut term = Terminal::new(4, 10);
        assert!(!term.bracketed_paste);
        term.process(b"\x1b[?2004h");
        assert!(term.bracketed_paste);
        term.process(b"\x1b[?2004l");
        assert!(!term.bracketed_paste);
    }

    #[test]
    fn cursor_style_steady_block() {
        let mut term = Terminal::new(4, 10);
        term.process(b"\x1b[2 q");
        assert!(term.cursor_style == CursorStyle::SteadyBlock);
    }

    #[test]
    fn cursor_style_blinking_bar() {
        let mut term = Terminal::new(4, 10);
        term.process(b"\x1b[5 q");
        assert!(term.cursor_style == CursorStyle::BlinkingBar);
    }
}
