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

#[cfg(test)]
mod tests {
    use crate::core::terminal::Terminal;
    use crate::core::{Cell, Color};

    fn write_colored(seq: &[u8]) -> Terminal {
        let mut term = Terminal::new(4, 20);
        term.process(seq);
        term.process(b"X");
        term
    }

    fn cell_at(term: &Terminal, row: usize, col: usize) -> &Cell {
        term.grid.get(row, col)
    }

    #[test]
    fn sgr_reset() {
        let mut term = Terminal::new(4, 20);
        term.process(b"\x1b[1;31mA\x1b[0mB");
        let a = cell_at(&term, 0, 0);
        assert!(a.bold);
        assert_eq!(a.fg, Color::ANSI[1]);
        let b = cell_at(&term, 0, 1);
        assert!(!b.bold);
        assert_eq!(b.fg, Color::DEFAULT_FG);
    }

    #[test]
    fn sgr_bold() {
        let term = write_colored(b"\x1b[1m");
        assert!(cell_at(&term, 0, 0).bold);
    }

    #[test]
    fn sgr_underline() {
        let term = write_colored(b"\x1b[4m");
        assert!(cell_at(&term, 0, 0).underline);
    }

    #[test]
    fn sgr_reverse() {
        let term = write_colored(b"\x1b[7m");
        assert!(cell_at(&term, 0, 0).reverse);
    }

    #[test]
    fn sgr_bold_off() {
        let term = write_colored(b"\x1b[1m\x1b[22m");
        assert!(!cell_at(&term, 0, 0).bold);
    }

    #[test]
    fn sgr_underline_off() {
        let term = write_colored(b"\x1b[4m\x1b[24m");
        assert!(!cell_at(&term, 0, 0).underline);
    }

    #[test]
    fn sgr_reverse_off() {
        let term = write_colored(b"\x1b[7m\x1b[27m");
        assert!(!cell_at(&term, 0, 0).reverse);
    }

    #[test]
    fn sgr_fg_ansi_colors() {
        for i in 0u8..8 {
            let seq = format!("\x1b[{}m", 30 + i);
            let term = write_colored(seq.as_bytes());
            assert_eq!(
                cell_at(&term, 0, 0).fg,
                Color::ANSI[i as usize],
                "SGR {} should set fg to ANSI[{}]",
                30 + i,
                i
            );
        }
    }

    #[test]
    fn sgr_bg_ansi_colors() {
        for i in 0u8..8 {
            let seq = format!("\x1b[{}m", 40 + i);
            let term = write_colored(seq.as_bytes());
            assert_eq!(
                cell_at(&term, 0, 0).bg,
                Color::ANSI[i as usize],
                "SGR {} should set bg to ANSI[{}]",
                40 + i,
                i
            );
        }
    }

    #[test]
    fn sgr_bright_fg() {
        for i in 0u8..8 {
            let seq = format!("\x1b[{}m", 90 + i);
            let term = write_colored(seq.as_bytes());
            assert_eq!(
                cell_at(&term, 0, 0).fg,
                Color::ANSI[(8 + i) as usize],
                "SGR {} should set fg to ANSI[{}]",
                90 + i,
                8 + i
            );
        }
    }

    #[test]
    fn sgr_bright_bg() {
        for i in 0u8..8 {
            let seq = format!("\x1b[{}m", 100 + i);
            let term = write_colored(seq.as_bytes());
            assert_eq!(
                cell_at(&term, 0, 0).bg,
                Color::ANSI[(8 + i) as usize],
                "SGR {} should set bg to ANSI[{}]",
                100 + i,
                8 + i
            );
        }
    }

    #[test]
    fn sgr_256_fg() {
        let term = write_colored(b"\x1b[38;5;196m");
        assert_eq!(cell_at(&term, 0, 0).fg, Color::from_256(196));
    }

    #[test]
    fn sgr_256_bg() {
        let term = write_colored(b"\x1b[48;5;82m");
        assert_eq!(cell_at(&term, 0, 0).bg, Color::from_256(82));
    }

    #[test]
    fn sgr_rgb_fg() {
        let term = write_colored(b"\x1b[38;2;255;128;0m");
        assert_eq!(
            cell_at(&term, 0, 0).fg,
            Color {
                r: 255,
                g: 128,
                b: 0
            }
        );
    }

    #[test]
    fn sgr_rgb_bg() {
        let term = write_colored(b"\x1b[48;2;10;20;30m");
        assert_eq!(
            cell_at(&term, 0, 0).bg,
            Color {
                r: 10,
                g: 20,
                b: 30
            }
        );
    }

    #[test]
    fn sgr_default_fg() {
        let term = write_colored(b"\x1b[31m\x1b[39m");
        assert_eq!(cell_at(&term, 0, 0).fg, Color::DEFAULT_FG);
    }

    #[test]
    fn sgr_default_bg() {
        let term = write_colored(b"\x1b[41m\x1b[49m");
        assert_eq!(cell_at(&term, 0, 0).bg, Color::DEFAULT_BG);
    }

    #[test]
    fn sgr_combined() {
        let term = write_colored(b"\x1b[1;4;31m");
        let cell = cell_at(&term, 0, 0);
        assert!(cell.bold);
        assert!(cell.underline);
        assert_eq!(cell.fg, Color::ANSI[1]);
    }

    #[test]
    fn sgr_no_params_resets() {
        let mut term = Terminal::new(4, 20);
        term.process(b"\x1b[1;31mA\x1b[mB");
        let b = cell_at(&term, 0, 1);
        assert!(!b.bold);
        assert_eq!(b.fg, Color::DEFAULT_FG);
    }
}
