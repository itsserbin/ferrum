use crate::core::Color;
use crate::core::terminal::Terminal;

pub(in super::super) fn reset_attributes(term: &mut Terminal) {
    term.set_fg(Color::DEFAULT_FG);
    term.set_bg(Color::DEFAULT_BG);
    term.set_bold(false);
    term.set_reverse(false);
    term.set_underline(false);
}
