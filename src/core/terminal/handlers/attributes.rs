use crate::core::UnderlineStyle;
use crate::core::terminal::Terminal;

pub(in super::super) fn reset_attributes(term: &mut Terminal) {
    let fg = term.default_fg;
    let bg = term.default_bg;
    term.set_fg(fg);
    term.set_bg(bg);
    term.set_bold(false);
    term.set_dim(false);
    term.set_italic(false);
    term.set_reverse(false);
    term.set_underline_style(UnderlineStyle::None);
    term.set_strikethrough(false);
}
