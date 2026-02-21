use crate::core::terminal::Terminal;
use vte::Params;

pub(in super::super) fn handle_device_csi(
    term: &mut Terminal,
    action: char,
    params: &Params,
    intermediates: &[u8],
) -> bool {
    match action {
        'n' => {
            match term.param(params, 0) {
                6 => {
                    // CPR — Cursor Position Report (1-indexed)
                    let response =
                        format!("\x1b[{};{}R", term.cursor_row + 1, term.cursor_col + 1,);
                    term.respond(response.as_bytes());
                }
                5 => {
                    // Operating Status — "OK"
                    term.respond(b"\x1b[0n");
                }
                _ => {}
            }
            true
        }
        'c' => {
            if intermediates.is_empty() && term.param(params, 0) == 0 {
                // DA1 — Primary Device Attributes (VT220-level).
                // ?62: VT220; 22: ANSI color; 29: ANSI text locator.
                term.respond(b"\x1b[?62;22;29c");
            }
            true
        }
        _ => false,
    }
}
