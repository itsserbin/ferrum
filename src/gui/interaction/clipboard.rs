#[cfg(test)]
use crate::core::Grid;
#[cfg(test)]
use crate::core::Row;
use crate::core::terminal::Terminal;
use crate::core::{Selection, SelectionPoint};
use crate::gui::*;

const BRACKETED_PASTE_START: &[u8] = b"\x1b[200~";
const BRACKETED_PASTE_END: &[u8] = b"\x1b[201~";

impl FerrumWindow {
    fn selected_text_from_terminal(terminal: &Terminal, selection: Selection) -> String {
        if terminal.grid.cols == 0 {
            return String::new();
        }
        let total_rows = terminal.scrollback.len() + terminal.grid.rows;
        if total_rows == 0 {
            return String::new();
        }
        let max_row = total_rows - 1;
        let max_col = terminal.grid.cols - 1;

        let clamped = Selection {
            start: SelectionPoint {
                row: selection.start.row.min(max_row),
                col: selection.start.col.min(max_col),
            },
            end: SelectionPoint {
                row: selection.end.row.min(max_row),
                col: selection.end.col.min(max_col),
            },
        };
        let (start, end) = clamped.normalized();

        let mut text = String::new();
        for row in start.row..=end.row {
            let col_start = if row == start.row { start.col } else { 0 };
            let col_end = if row == end.row { end.col } else { max_col };

            for col in col_start..=col_end {
                let ch = if row < terminal.scrollback.len() {
                    terminal.scrollback[row]
                        .cells
                        .get(col)
                        .map_or(' ', |cell| cell.character)
                } else {
                    // Safe: clamped above, and row is in the visible grid range here.
                    terminal
                        .grid
                        .get_unchecked(row - terminal.scrollback.len(), col)
                        .character
                };
                text.push(ch);
            }
            if row < end.row {
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
            let _ = self.delete_terminal_selection(false);
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
    use crate::core::Cell;

    fn set_row(grid: &mut Grid, row: usize, text: &str) {
        for (col, ch) in text.chars().take(grid.cols).enumerate() {
            let cell = Cell {
                character: ch,
                ..Cell::default()
            };
            grid.set(row, col, cell);
        }
    }

    fn row_cells(text: &str, cols: usize) -> Vec<Cell> {
        let mut row = vec![Cell::default(); cols];
        for (i, ch) in text.chars().take(cols).enumerate() {
            row[i].character = ch;
        }
        row
    }

    #[test]
    fn selected_text_uses_visible_scrollback_when_offset_non_zero() {
        let mut terminal = Terminal::new(3, 5);
        set_row(&mut terminal.grid, 0, "LIVE0");
        set_row(&mut terminal.grid, 1, "LIVE1");
        set_row(&mut terminal.grid, 2, "LIVE2");
        terminal
            .scrollback
            .push_back(Row::from_cells(row_cells("SB000", 5), false));
        terminal
            .scrollback
            .push_back(Row::from_cells(row_cells("SB001", 5), false));

        // Scrollback has 2 entries. Live grid row 0 is absolute row 2.
        let selection = Selection {
            start: SelectionPoint { row: 2, col: 0 },
            end: SelectionPoint { row: 2, col: 4 },
        };

        let live_text = FerrumWindow::selected_text_from_terminal(&terminal, selection);
        assert_eq!(live_text, "LIVE0");

        // Absolute row 1 is the second scrollback line.
        let scrollback_selection = Selection {
            start: SelectionPoint { row: 1, col: 0 },
            end: SelectionPoint { row: 1, col: 4 },
        };
        let scrollback_text =
            FerrumWindow::selected_text_from_terminal(&terminal, scrollback_selection);
        assert_eq!(scrollback_text, "SB001");
    }
}
