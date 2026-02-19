use crate::gui::state::TabState;
use crate::gui::FerrumWindow;

#[derive(Clone, Copy)]
pub(super) enum HorizontalMotion {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MotionClass {
    Word,
    Symbol,
    Whitespace,
}

impl FerrumWindow {
    pub(super) fn build_horizontal_cursor_move_bytes(
        cursor_col: usize,
        target_col: usize,
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        if target_col < cursor_col {
            for _ in 0..(cursor_col - target_col) {
                bytes.extend_from_slice(b"\x1b[D");
            }
        } else if target_col > cursor_col {
            for _ in 0..(target_col - cursor_col) {
                bytes.extend_from_slice(b"\x1b[C");
            }
        }
        bytes
    }

    fn is_word_char_for_motion(ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_'
    }

    fn motion_class(ch: char) -> MotionClass {
        if ch.is_whitespace() {
            MotionClass::Whitespace
        } else if Self::is_word_char_for_motion(ch) {
            MotionClass::Word
        } else {
            MotionClass::Symbol
        }
    }

    pub(super) fn word_motion_target_col_for_line(
        line: &[char],
        cursor_col: usize,
        motion: HorizontalMotion,
    ) -> usize {
        let cols = line.len();
        if cols == 0 {
            return 0;
        }

        match motion {
            HorizontalMotion::Left => {
                if cursor_col == 0 {
                    return 0;
                }

                let mut idx = cursor_col.min(cols).saturating_sub(1);

                while idx > 0 && Self::motion_class(line[idx]) == MotionClass::Whitespace {
                    idx -= 1;
                }

                if idx == 0 && Self::motion_class(line[idx]) == MotionClass::Whitespace {
                    return 0;
                }

                let class = Self::motion_class(line[idx]);
                while idx > 0 && Self::motion_class(line[idx - 1]) == class {
                    idx -= 1;
                }

                idx
            }
            HorizontalMotion::Right => {
                let mut idx = cursor_col.min(cols);

                while idx < cols && Self::motion_class(line[idx]) == MotionClass::Whitespace {
                    idx += 1;
                }

                if idx >= cols {
                    return cols;
                }

                let class = Self::motion_class(line[idx]);
                while idx < cols && Self::motion_class(line[idx]) == class {
                    idx += 1;
                }

                idx
            }
        }
    }

    pub(super) fn word_motion_target_col(
        tab: &TabState,
        cursor_col: usize,
        motion: HorizontalMotion,
    ) -> usize {
        let rows = tab.terminal.grid.rows;
        let cols = tab.terminal.grid.cols;
        if rows == 0 || cols == 0 {
            return 0;
        }

        let row = tab.terminal.cursor_row.min(rows.saturating_sub(1));
        let mut line = Vec::with_capacity(cols);
        for col in 0..cols {
            // Safe: col < cols and row < rows
            line.push(tab.terminal.grid.get_unchecked(row, col).character);
        }

        Self::word_motion_target_col_for_line(&line, cursor_col, motion)
    }
}

#[cfg(test)]
mod tests {
    use super::HorizontalMotion;
    use crate::gui::FerrumWindow;

    #[test]
    fn word_motion_left_skips_delimiters_then_word() {
        let line: Vec<char> = "alpha  beta".chars().collect();
        let target =
            FerrumWindow::word_motion_target_col_for_line(&line, 11, HorizontalMotion::Left);
        assert_eq!(target, 7);
    }

    #[test]
    fn word_motion_right_moves_to_end_of_next_word() {
        let line: Vec<char> = "alpha  beta".chars().collect();
        let target =
            FerrumWindow::word_motion_target_col_for_line(&line, 0, HorizontalMotion::Right);
        assert_eq!(target, 5);
    }

    #[test]
    fn word_motion_right_stops_at_symbol_boundaries() {
        let line: Vec<char> = "foo/bar-baz".chars().collect();
        let first =
            FerrumWindow::word_motion_target_col_for_line(&line, 0, HorizontalMotion::Right);
        let second =
            FerrumWindow::word_motion_target_col_for_line(&line, first, HorizontalMotion::Right);
        let third =
            FerrumWindow::word_motion_target_col_for_line(&line, second, HorizontalMotion::Right);

        assert_eq!(first, 3); // "foo"
        assert_eq!(second, 4); // "/"
        assert_eq!(third, 7); // "bar"
    }

    #[test]
    fn word_motion_left_stops_at_symbol_boundaries() {
        let line: Vec<char> = "foo/bar-baz".chars().collect();
        let target =
            FerrumWindow::word_motion_target_col_for_line(&line, 8, HorizontalMotion::Left);
        assert_eq!(target, 7); // stops on '-' group, not whole left side
    }
}
