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

    let mode = term.param(params, 0);
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
        _ => {}
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
