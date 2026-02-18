use crate::gui::*;

impl FerrumWindow {
    /// Approximates click-to-cursor by sending arrow-key sequences.
    /// Shell mode only supports horizontal moves on the current cursor row.
    /// Alt-screen mode allows vertical moves, but horizontal deltas are safe only on the same row.
    pub(in crate::gui) fn move_cursor_to(&mut self, target_row: usize, target_col: usize) {
        let tab = match self.active_tab_mut() {
            Some(t) => t,
            None => return,
        };

        // Don't move cursor if scrolled - user is viewing scrollback, not live content
        if tab.scroll_offset > 0 {
            return;
        }

        // Don't move cursor shortly after resize - cursor position may not be synced with shell
        // Use longer timeout (2 seconds) until reflow sync is properly fixed
        if tab
            .terminal
            .resize_at
            .is_some_and(|t| t.elapsed().as_millis() < 2000)
        {
            return;
        }

        let cur_row = tab.terminal.cursor_row;
        let cur_col = tab.terminal.cursor_col;
        let alt_screen = tab.terminal.is_alt_screen();

        let mut bytes = Vec::new();

        if alt_screen {
            // Vertical move is safe in alt-screen apps.
            if target_row < cur_row {
                for _ in 0..(cur_row - target_row) {
                    bytes.extend_from_slice(b"\x1b[A");
                }
            } else if target_row > cur_row {
                for _ in 0..(target_row - cur_row) {
                    bytes.extend_from_slice(b"\x1b[B");
                }
            }

            // Horizontal delta is only reliable on the same visible row.
            if target_row == cur_row {
                let last_content = (0..tab.terminal.grid.cols)
                    .rev()
                    .find(|&c| tab.terminal.grid.get(target_row, c).character != ' ');
                if let Some(last_col) = last_content {
                    let safe_col = target_col.min(last_col + 1);
                    if safe_col < cur_col {
                        for _ in 0..(cur_col - safe_col) {
                            bytes.extend_from_slice(b"\x1b[D");
                        }
                    } else if safe_col > cur_col {
                        for _ in 0..(safe_col - cur_col) {
                            bytes.extend_from_slice(b"\x1b[C");
                        }
                    }
                }
            }
            // Skip horizontal move across rows: grid coords may not map to app text coords.
        } else {
            // In shell mode, avoid synthesizing vertical history navigation.
            if target_row != cur_row {
                return;
            }

            // Find the last non-space column on this row to avoid sending arrows
            // past the actual content (cmd.exe interprets RIGHT on empty input
            // as "copy from previous command").
            let last_content = (0..tab.terminal.grid.cols)
                .rev()
                .find(|&c| tab.terminal.grid.get(cur_row, c).character != ' ');

            // Only allow movement within content bounds
            let max_col = last_content.map(|c| c + 1).unwrap_or(0);
            let safe_target = target_col.min(max_col);

            if safe_target < cur_col {
                for _ in 0..(cur_col - safe_target) {
                    bytes.extend_from_slice(b"\x1b[D");
                }
            } else if safe_target > cur_col {
                for _ in 0..(safe_target - cur_col) {
                    bytes.extend_from_slice(b"\x1b[C");
                }
            }
        }

        if !bytes.is_empty() {
            let _ = tab.pty_writer.write_all(&bytes);
            let _ = tab.pty_writer.flush();
        }
    }
}
