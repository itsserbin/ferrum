//! Reflow logic for terminal resize: merge soft-wrapped rows into logical
//! lines and re-wrap them to a new column width.

use crate::core::{Cell, Row};

/// A logical line collected from scrollback and grid content.
///
/// Represents one logical line of text that may span multiple physical rows
/// when soft-wrapped. The `min_len` field preserves trailing spaces before
/// the cursor position so they are not trimmed during rewrapping.
pub(super) struct LogicalLine {
    pub cells: Vec<Cell>,
    /// Minimum number of cells to preserve (for cursor-position trailing spaces).
    pub min_len: usize,
}

/// Rewrap logical lines to fit a new column width.
///
/// This is a pure function with no side effects: it takes collected logical
/// lines and a target width, and produces a flat list of physical rows with
/// correct wrap flags.
///
/// If `cursor_line_idx` is provided, also returns the index of the first
/// rewrapped row that corresponds to that logical line — used by the caller
/// to restore the cursor position after reflow without an extra traversal.
pub(super) fn rewrap_lines(
    lines: &[LogicalLine],
    new_cols: usize,
    cursor_line_idx: Option<usize>,
) -> (Vec<Row>, Option<usize>) {
    let mut rewrapped: Vec<Row> = Vec::new();
    let mut cursor_rewrapped_row: Option<usize> = None;
    for (line_idx, logical_line) in lines.iter().enumerate() {
        // Trim only untouched default cells; keep styled spaces and explicit
        // spaces before cursor in the active line.
        let len = line_content_len(logical_line);

        if cursor_line_idx == Some(line_idx) {
            cursor_rewrapped_row = Some(rewrapped.len());
        }

        if len == 0 {
            rewrapped.push(Row::new(new_cols));
            continue;
        }

        let content = &logical_line.cells[..len];
        let mut pos = 0;
        while pos < content.len() {
            let end = (pos + new_cols).min(content.len());
            let mut cells = Vec::with_capacity(new_cols);
            cells.extend_from_slice(&content[pos..end]);
            cells.resize(new_cols, Cell::default());
            let wrapped = end < content.len();
            rewrapped.push(Row::from_cells(cells, wrapped));
            pos = end;
        }
    }
    (rewrapped, cursor_rewrapped_row)
}

fn line_content_len(line: &LogicalLine) -> usize {
    let len = line
        .cells
        .iter()
        .rposition(|c| c != &Cell::default())
        .map(|i| i + 1)
        .unwrap_or(0);
    len.max(line.min_len.min(line.cells.len()))
}

impl super::Terminal {
    /// Collect scrollback and visible grid into logical lines.
    ///
    /// Also returns the index of the logical line that contains the cursor,
    /// used to correctly position the cursor after reflow.
    pub(super) fn collect_logical_lines(&self) -> (Vec<LogicalLine>, Option<usize>) {
        let content_rows = self.compute_content_rows();
        let mut lines: Vec<LogicalLine> = Vec::new();
        let mut current_cells: Vec<Cell> = Vec::new();
        let mut cursor_line_idx: Option<usize> = None;

        for row in self.scrollback.iter() {
            current_cells.extend(row.cells.iter().cloned());
            if !row.wrapped {
                lines.push(LogicalLine {
                    cells: std::mem::take(&mut current_cells),
                    min_len: 0,
                });
            }
        }

        // cursor_in_current only applies to grid rows; the cursor is never in scrollback.
        let mut cursor_in_current = false;
        let mut current_min_len = 0usize;

        for r in 0..content_rows {
            let line_start = current_cells.len();
            current_cells.extend(self.grid.row_cells(r));
            if r == self.cursor_row {
                cursor_in_current = true;
                let clamped = self.cursor_col.min(self.grid.cols);
                current_min_len = current_min_len.max(line_start + clamped);
            }
            if !self.grid.is_wrapped(r) {
                if cursor_in_current {
                    cursor_line_idx = Some(lines.len());
                    cursor_in_current = false;
                }
                lines.push(LogicalLine {
                    cells: std::mem::take(&mut current_cells),
                    min_len: current_min_len,
                });
                current_min_len = 0;
            }
        }

        if !current_cells.is_empty() {
            if cursor_in_current {
                cursor_line_idx = Some(lines.len());
            }
            lines.push(LogicalLine {
                cells: current_cells,
                min_len: current_min_len,
            });
        }

        (lines, cursor_line_idx)
    }

    pub(super) fn compute_content_rows(&self) -> usize {
        let cursor_rows = self.cursor_row.saturating_add(1).min(self.grid.rows);
        let default_cell = Cell::default();
        let last_content_row = (0..self.grid.rows).rev().find(|&row| {
            self.grid.is_wrapped(row)
                || (0..self.grid.cols)
                    .any(|col| self.grid.get_unchecked(row, col) != &default_cell)
        });
        last_content_row
            .map(|row| (row + 1).max(cursor_rows))
            .unwrap_or(cursor_rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrap_lines_empty_line_produces_blank_row() {
        let lines = vec![LogicalLine {
            cells: vec![],
            min_len: 0,
        }];
        let (result, _) = rewrap_lines(&lines, 10, None);
        assert_eq!(result.len(), 1);
        assert!(!result[0].wrapped);
        assert_eq!(result[0].cells.len(), 10);
        for cell in &result[0].cells {
            assert_eq!(*cell, Cell::default());
        }
    }

    #[test]
    fn rewrap_lines_short_line_fits_in_one_row() {
        let cells: Vec<Cell> = "Hello"
            .chars()
            .map(|c| Cell {
                character: c,
                ..Cell::default()
            })
            .collect();
        let lines = vec![LogicalLine { cells, min_len: 0 }];
        let (result, _) = rewrap_lines(&lines, 10, None);
        assert_eq!(result.len(), 1);
        assert!(!result[0].wrapped);
        assert_eq!(result[0].cells[0].character, 'H');
        assert_eq!(result[0].cells[4].character, 'o');
    }

    #[test]
    fn rewrap_lines_long_line_wraps_to_multiple_rows() {
        let cells: Vec<Cell> = "ABCDEFGHIJ"
            .chars()
            .map(|c| Cell {
                character: c,
                ..Cell::default()
            })
            .collect();
        let lines = vec![LogicalLine { cells, min_len: 0 }];
        let (result, _) = rewrap_lines(&lines, 4, None);
        assert_eq!(result.len(), 3);
        assert!(result[0].wrapped);
        assert!(result[1].wrapped);
        assert!(!result[2].wrapped);
        assert_eq!(result[0].cells[0].character, 'A');
        assert_eq!(result[0].cells[3].character, 'D');
        assert_eq!(result[1].cells[0].character, 'E');
        assert_eq!(result[2].cells[0].character, 'I');
        assert_eq!(result[2].cells[1].character, 'J');
    }

    #[test]
    fn rewrap_lines_preserves_min_len_trailing_spaces() {
        let cells: Vec<Cell> = "abc   "
            .chars()
            .map(|c| Cell {
                character: c,
                ..Cell::default()
            })
            .collect();
        let lines = vec![LogicalLine { cells, min_len: 6 }];
        let (result, _) = rewrap_lines(&lines, 10, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].cells[0].character, 'a');
        assert_eq!(result[0].cells[1].character, 'b');
        assert_eq!(result[0].cells[2].character, 'c');
        assert_eq!(result[0].cells[3].character, ' ');
        assert_eq!(result[0].cells[4].character, ' ');
        assert_eq!(result[0].cells[5].character, ' ');
    }

    #[test]
    fn rewrap_lines_trims_trailing_default_cells_without_min_len() {
        let mut cells: Vec<Cell> = "abc"
            .chars()
            .map(|c| Cell {
                character: c,
                ..Cell::default()
            })
            .collect();
        cells.resize(10, Cell::default());
        let lines = vec![LogicalLine { cells, min_len: 0 }];
        let (result, _) = rewrap_lines(&lines, 5, None);
        assert_eq!(result.len(), 1);
        assert!(!result[0].wrapped);
        assert_eq!(result[0].cells[0].character, 'a');
        assert_eq!(result[0].cells[1].character, 'b');
        assert_eq!(result[0].cells[2].character, 'c');
    }

    #[test]
    fn rewrap_lines_multiple_logical_lines() {
        let line1_cells: Vec<Cell> = "ABCD"
            .chars()
            .map(|c| Cell {
                character: c,
                ..Cell::default()
            })
            .collect();
        let line2_cells: Vec<Cell> = "EF"
            .chars()
            .map(|c| Cell {
                character: c,
                ..Cell::default()
            })
            .collect();
        let lines = vec![
            LogicalLine {
                cells: line1_cells,
                min_len: 0,
            },
            LogicalLine {
                cells: line2_cells,
                min_len: 0,
            },
        ];
        let (result, _) = rewrap_lines(&lines, 3, None);
        assert_eq!(result.len(), 3);
        assert!(result[0].wrapped);
        assert!(!result[1].wrapped);
        assert!(!result[2].wrapped);
    }
}
