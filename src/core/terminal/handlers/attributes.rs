use crate::core::terminal::Terminal;

pub(in super::super) fn reset_attributes(term: &mut Terminal) {
    let fg = term.default_fg;
    let bg = term.default_bg;
    term.set_fg(fg);
    term.set_bg(bg);
    term.set_bold(false);
    term.set_reverse(false);
    term.set_underline(false);
}
