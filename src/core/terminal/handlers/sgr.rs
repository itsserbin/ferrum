use crate::core::Color;
use crate::core::terminal::Terminal;
use vte::Params;

pub(in super::super) fn handle_sgr(term: &mut Terminal, params: &Params) {
    // SGR with no params means reset all attributes.
    if params.is_empty() {
        term.reset_attributes();
        return;
    }

    let mut iter = params.iter();
    while let Some(param) = iter.next() {
        let code = param[0];
        match code {
            0 => term.reset_attributes(),
            1 => term.set_bold(true),
            4 => term.set_underline(true),
            7 => term.set_reverse(true),
            22 => term.set_bold(false),
            24 => term.set_underline(false),
            27 => term.set_reverse(false),
            30..=37 => term.set_fg(Color::ANSI[(code - 30) as usize]),
            38 => {
                // Extended foreground: 38;5;N (256-color) or 38;2;R;G;B (true color)
                if let Some(sub) = iter.next() {
                    match sub[0] {
                        5 => {
                            if let Some(n) = iter.next() {
                                term.set_fg(Color::from_256(n[0]));
                            }
                        }
                        2 => {
                            let r = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            let g = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            let b = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            term.set_fg(Color { r, g, b });
                        }
                        _ => {}
                    }
                }
            }
            39 => term.set_fg(Color::DEFAULT_FG),
            40..=47 => term.set_bg(Color::ANSI[(code - 40) as usize]),
            48 => {
                // Extended background: 48;5;N (256-color) or 48;2;R;G;B (true color)
                if let Some(sub) = iter.next() {
                    match sub[0] {
                        5 => {
                            if let Some(n) = iter.next() {
                                term.set_bg(Color::from_256(n[0]));
                            }
                        }
                        2 => {
                            let r = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            let g = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            let b = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            term.set_bg(Color { r, g, b });
                        }
                        _ => {}
                    }
                }
            }
            49 => term.set_bg(Color::DEFAULT_BG),
            90..=97 => term.set_fg(Color::ANSI[(code - 90 + 8) as usize]),
            100..=107 => term.set_bg(Color::ANSI[(code - 100 + 8) as usize]),
            _ => {}
        }
    }
}
