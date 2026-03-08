use crate::core::terminal::Terminal;
use crate::core::{PageCoord, Selection};
use crate::gui::*;

const BRACKETED_PASTE_START: &[u8] = b"\x1b[200~";
const BRACKETED_PASTE_END: &[u8] = b"\x1b[201~";

impl FerrumWindow {
    fn selected_text_from_terminal(terminal: &Terminal, selection: Selection) -> String {
        let cols = terminal.screen.cols();
        if cols == 0 {
            return String::new();
        }
        let scrollback_len = terminal.screen.scrollback_len();
        let total_rows = scrollback_len + terminal.screen.viewport_rows();
        if total_rows == 0 {
            return String::new();
        }
        let max_row = total_rows - 1;
        let max_col = cols - 1;

        let clamped = Selection {
            start: PageCoord {
                abs_row: selection.start.abs_row.min(max_row),
                col: selection.start.col.min(max_col),
            },
            end: PageCoord {
                abs_row: selection.end.abs_row.min(max_row),
                col: selection.end.col.min(max_col),
            },
        };
        let (start, end) = clamped.normalized();

        let mut text = String::new();
        for row in start.abs_row..=end.abs_row {
            let col_start = if row == start.abs_row { start.col } else { 0 };
            let col_end = if row == end.abs_row { end.col } else { max_col };

            for col in col_start..=col_end {
                let ch = if row < scrollback_len {
                    terminal.screen.scrollback_row(row)
                        .cells
                        .get(col)
                        .map_or(' ', |cell| cell.first_char())
                } else {
                    terminal.screen.viewport_get(row - scrollback_len, col).first_char()
                };
                text.push(ch);
            }
            if row < end.abs_row {
                text.push('\n');
            }
        }

        text.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn wrap_bracketed_paste(text: &str) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            BRACKETED_PASTE_START.len() + text.len() + BRACKETED_PASTE_END.len(),
        );
        bytes.extend_from_slice(BRACKETED_PASTE_START);
        bytes.extend_from_slice(text.as_bytes());
        bytes.extend_from_slice(BRACKETED_PASTE_END);
        bytes
    }

    pub(in crate::gui) fn copy_selection(&mut self) {
        let text = {
            let leaf = match self.active_leaf_ref() {
                Some(l) => l,
                None => return,
            };
            let Some(sel) = leaf.selection else { return };
            Self::selected_text_from_terminal(&leaf.terminal, sel)
        };

        if let Some(ref mut clipboard) = self.clipboard
            && let Err(e) = clipboard.set_text(&text)
        {
            eprintln!("Failed to copy to clipboard: {}", e);
        }
    }

    pub(in crate::gui) fn paste_clipboard(&mut self) {
        let text = match self.clipboard.as_mut() {
            Some(cb) => match cb.get_text() {
                Ok(text) => text,
                Err(e) => {
                    eprintln!("Failed to read from clipboard: {}", e);
                    return;
                }
            },
            None => return,
        };

        if text.is_empty() {
            return;
        }

        if self
            .active_leaf_ref()
            .is_some_and(|leaf| leaf.selection.is_some())
        {
            self.delete_terminal_selection(false);
        }

        let bytes = {
            let Some(leaf) = self.active_leaf_mut() else {
                return;
            };
            leaf.security.check_paste_payload(&text);
            if leaf.terminal.bracketed_paste {
                Self::wrap_bracketed_paste(&text)
            } else {
                text.as_bytes().to_vec()
            }
        };

        self.write_pty_bytes(&bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_text_reads_viewport_and_scrollback() {
        // 3-row, 5-col terminal. Process 5 lines: the first rows scroll into scrollback.
        let mut terminal = Terminal::new(3, 5);
        terminal.process(b"AAAAA\nBBBBB\nCCCCC\nDDDDD\nEEEEE");

        // After processing 5 lines into a 3-row terminal, some rows are in scrollback
        // and the rest are in the viewport.
        let scrollback_len = terminal.screen.scrollback_len();
        assert!(scrollback_len > 0, "should have scrollback rows");

        // Verify viewport row 0 (abs_row == scrollback_len) is readable
        let viewport_selection = Selection {
            start: PageCoord { abs_row: scrollback_len, col: 0 },
            end: PageCoord { abs_row: scrollback_len, col: 4 },
        };
        let viewport_text = FerrumWindow::selected_text_from_terminal(&terminal, viewport_selection);
        // Viewport first row is readable (non-empty content)
        assert!(!viewport_text.is_empty());

        // Verify a scrollback row is also readable
        let scrollback_selection = Selection {
            start: PageCoord { abs_row: 0, col: 0 },
            end: PageCoord { abs_row: 0, col: 4 },
        };
        let scrollback_text =
            FerrumWindow::selected_text_from_terminal(&terminal, scrollback_selection);
        assert!(!scrollback_text.is_empty());

        // Verify that scrollback and viewport content differ
        assert_ne!(viewport_text, scrollback_text);
    }
}
