use crate::core::terminal::Terminal;
use crate::core::{Grid, Selection, SelectionPoint};
#[cfg(test)]
use crate::core::Row;
use crate::gui::*;

const BRACKETED_PASTE_START: &[u8] = b"\x1b[200~";
const BRACKETED_PASTE_END: &[u8] = b"\x1b[201~";

impl FerrumWindow {
    fn selected_text_from_terminal(
        terminal: &Terminal,
        selection: Selection,
        scroll_offset: usize,
    ) -> String {
        let viewport_start = terminal.scrollback.len().saturating_sub(scroll_offset);

        // Convert absolute selection to viewport-relative
        let rel_selection = Selection {
            start: SelectionPoint {
                row: selection.start.row.saturating_sub(viewport_start),
                col: selection.start.col,
            },
            end: SelectionPoint {
                row: selection.end.row.saturating_sub(viewport_start),
                col: selection.end.col,
            },
        };

        if scroll_offset == 0 {
            return Self::selected_text_from_grid(&terminal.grid, rel_selection);
        }

        let display = terminal.build_display(scroll_offset);
        Self::selected_text_from_grid(&display, rel_selection)
    }

    fn selected_text_from_grid(grid: &Grid, selection: Selection) -> String {
        if grid.rows == 0 || grid.cols == 0 {
            return String::new();
        }

        let clamped = Selection {
            start: SelectionPoint {
                row: selection.start.row.min(grid.rows - 1),
                col: selection.start.col.min(grid.cols - 1),
            },
            end: SelectionPoint {
                row: selection.end.row.min(grid.rows - 1),
                col: selection.end.col.min(grid.cols - 1),
            },
        };
        let (start, end) = clamped.normalized();

        let mut text = String::new();
        for row in start.row..=end.row {
            let col_start = if row == start.row { start.col } else { 0 };
            let col_end = if row == end.row {
                end.col
            } else {
                grid.cols - 1
            };

            for col in col_start..=col_end {
                // Safe: selection was clamped to grid bounds above
                text.push(grid.get_unchecked(row, col).character);
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
            let tab = match self.active_tab_ref() {
                Some(t) => t,
                None => return,
            };
            let Some(sel) = tab.selection else { return };
            Self::selected_text_from_terminal(&tab.terminal, sel, tab.scroll_offset)
        };

        if let Some(ref mut clipboard) = self.clipboard {
            if let Err(e) = clipboard.set_text(&text) {
                eprintln!("Failed to copy to clipboard: {}", e);
            }
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

        if self.active_tab_ref().is_some_and(|tab| tab.selection.is_some()) {
            let _ = self.delete_terminal_selection(false);
        }

        let bytes = {
            let Some(tab) = self.active_tab_mut() else {
                return;
            };
            tab.security.check_paste_payload(&text);
            if tab.security.should_wrap_paste() {
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
            let mut cell = Cell::default();
            cell.character = ch;
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
        terminal.scrollback.push_back(Row::from_cells(row_cells("SB000", 5), false));
        terminal.scrollback.push_back(Row::from_cells(row_cells("SB001", 5), false));

        // scrollback has 2 entries. With scroll_offset=0, viewport_start=2.
        // Live grid row 0 is absolute row 2.
        let selection = Selection {
            start: SelectionPoint { row: 2, col: 0 },
            end: SelectionPoint { row: 2, col: 4 },
        };

        let live_text = FerrumWindow::selected_text_from_terminal(&terminal, selection, 0);
        assert_eq!(live_text, "LIVE0");

        // With scroll_offset=1, viewport_start=1. Viewport row 0 is SB001 (absolute row 1).
        let scrollback_selection = Selection {
            start: SelectionPoint { row: 1, col: 0 },
            end: SelectionPoint { row: 1, col: 4 },
        };
        let scrollback_text =
            FerrumWindow::selected_text_from_terminal(&terminal, scrollback_selection, 1);
        assert_eq!(scrollback_text, "SB001");
    }
}
