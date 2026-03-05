use crate::core::tracked_pin::{PageCoord, TrackedPin};
use crate::core::{GraphemeCell, Page, PageRow, PAGE_SIZE};

pub struct PageList {
    pages: Vec<Page>,
    free_list: Vec<Page>,
    pub scrollback_count: usize,
    viewport_rows: usize,
    cols: usize,
    max_scrollback: usize,
    pins: Vec<TrackedPin>,
}

impl PageList {
    pub fn new(viewport_rows: usize, cols: usize, max_scrollback: usize) -> Self {
        let mut list = Self {
            pages: Vec::new(),
            free_list: Vec::new(),
            scrollback_count: 0,
            viewport_rows,
            cols,
            max_scrollback,
            pins: Vec::new(),
        };
        for _ in 0..viewport_rows {
            list.append_row(PageRow::new(cols));
        }
        list
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn viewport_rows(&self) -> usize {
        self.viewport_rows
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback_count
    }

    pub fn total_rows(&self) -> usize {
        self.scrollback_count + self.viewport_rows
    }

    /// Create a blank row with the current column count.
    pub fn make_row(&self) -> PageRow {
        PageRow::new(self.cols)
    }

    /// Convert absolute row index to (page_idx, row_within_page).
    fn abs_to_page(&self, abs_row: usize) -> (usize, usize) {
        (abs_row / PAGE_SIZE, abs_row % PAGE_SIZE)
    }

    /// Absolute index of the first viewport row.
    pub fn viewport_start_abs(&self) -> usize {
        self.scrollback_count
    }

    // ── Viewport access ──────────────────────────────────────────────────────

    pub fn viewport_get(&self, row: usize, col: usize) -> &GraphemeCell {
        let abs = self.viewport_start_abs() + row;
        let (pi, ri) = self.abs_to_page(abs);
        &self.pages[pi].get(ri).cells[col]
    }

    pub fn viewport_set(&mut self, row: usize, col: usize, cell: GraphemeCell) {
        let abs = self.viewport_start_abs() + row;
        let (pi, ri) = self.abs_to_page(abs);
        self.pages[pi].get_mut(ri).cells[col] = cell;
    }

    pub fn viewport_row(&self, row: usize) -> &PageRow {
        let abs = self.viewport_start_abs() + row;
        let (pi, ri) = self.abs_to_page(abs);
        self.pages[pi].get(ri)
    }

    pub fn viewport_row_mut(&mut self, row: usize) -> &mut PageRow {
        let abs = self.viewport_start_abs() + row;
        let (pi, ri) = self.abs_to_page(abs);
        self.pages[pi].get_mut(ri)
    }

    pub fn viewport_set_row(&mut self, row: usize, src: PageRow) {
        let abs = self.viewport_start_abs() + row;
        let (pi, ri) = self.abs_to_page(abs);
        *self.pages[pi].get_mut(ri) = src;
    }

    pub fn viewport_is_wrapped(&self, row: usize) -> bool {
        self.viewport_row(row).wrapped
    }

    pub fn viewport_set_wrapped(&mut self, row: usize, wrapped: bool) {
        self.viewport_row_mut(row).wrapped = wrapped;
    }

    // ── Scrollback access ────────────────────────────────────────────────────

    pub fn scrollback_row(&self, idx: usize) -> &PageRow {
        let (pi, ri) = self.abs_to_page(idx);
        self.pages[pi].get(ri)
    }

    // ── Internal: append a row to the end ───────────────────────────────────

    pub fn append_row(&mut self, row: PageRow) {
        if self.pages.last().map(|p| p.is_full()).unwrap_or(true) {
            self.alloc_page();
        }
        self.pages
            .last_mut()
            .expect("pages non-empty after alloc")
            .push(row);
    }

    fn alloc_page(&mut self) {
        let page = self
            .free_list
            .pop()
            .unwrap_or_else(|| *Page::new(self.cols));
        // Verify the recycled or new page was allocated for the current column width.
        debug_assert_eq!(page.cols, self.cols, "page column mismatch on alloc");
        self.pages.push(page);
    }

    // ── Simple resize ────────────────────────────────────────────────────────

    pub fn simple_resize(&mut self, new_rows: usize, new_cols: usize) {
        if new_rows > self.viewport_rows {
            let cols = new_cols;
            let extra = new_rows - self.viewport_rows;
            for _ in 0..extra {
                self.append_row(PageRow::new(cols));
            }
        }
        self.viewport_rows = new_rows;
        self.cols = new_cols;
        for vrow in 0..self.viewport_rows {
            let abs = self.viewport_start_abs() + vrow;
            let (pi, ri) = self.abs_to_page(abs);
            let row = self.pages[pi].get_mut(ri);
            row.cells.resize(new_cols, GraphemeCell::default());
        }
    }

    // ── Abs-row access (for reflow, used by Task 5) ──────────────────────────

    pub fn abs_row(&self, abs: usize) -> &PageRow {
        let (pi, ri) = self.abs_to_page(abs);
        self.pages[pi].get(ri)
    }

    pub fn abs_row_mut(&mut self, abs: usize) -> &mut PageRow {
        let (pi, ri) = self.abs_to_page(abs);
        self.pages[pi].get_mut(ri)
    }

    // ── Pin management ───────────────────────────────────────────────────────

    /// Register a new tracked pin at the given coordinate.
    /// Returns a handle — cloning shares the same underlying coordinate.
    pub fn register_pin(&mut self, coord: PageCoord) -> TrackedPin {
        let pin = TrackedPin::new(coord);
        self.pins.push(pin.clone());
        pin
    }

    /// Read the current coordinate of a pin.
    pub fn pin_coord(&self, pin: &TrackedPin) -> PageCoord {
        pin.coord()
    }

    /// Set the column of a pin.
    pub fn set_pin_col(&self, pin: &TrackedPin, col: usize) {
        pin.set_col(col);
    }

    /// Set the absolute row of a pin (used during reflow to update cursor).
    pub fn set_pin_abs_row(&self, pin: &TrackedPin, abs_row: usize) {
        pin.set_abs_row(abs_row);
    }

    // ── Reflow ───────────────────────────────────────────────────────────────

    /// Reflow all content (scrollback + viewport) to new dimensions.
    ///
    /// After reflow, `cursor_pin.col` is set to 0 for SIGWINCH compatibility:
    /// readline/zsh sends CR then redraws from the cursor row, so placing the
    /// cursor at col=0 ensures the shell erases the full reflowed line cleanly.
    pub fn reflow(&mut self, new_rows: usize, new_cols: usize, cursor_pin: &TrackedPin) {
        let (logical_lines, cursor_line_idx) = self.collect_logical_lines(cursor_pin);
        let (rewrapped, cursor_row_in_rewrapped) =
            rewrap_lines(&logical_lines, new_cols, cursor_line_idx);
        self.rebuild_from_rows(rewrapped, new_rows, new_cols);
        if let Some(vrow) = cursor_row_in_rewrapped {
            let abs = self.viewport_start_abs() + vrow;
            cursor_pin.set_coord(PageCoord { abs_row: abs, col: 0 });
        }
    }

    fn collect_logical_lines(&self, cursor_pin: &TrackedPin) -> (Vec<LogicalLine>, Option<usize>) {
        let cursor_abs = cursor_pin.coord().abs_row;
        let cursor_col = cursor_pin.coord().col;
        let mut lines: Vec<LogicalLine> = Vec::new();
        let mut current: Vec<GraphemeCell> = Vec::new();
        let mut cursor_line_idx: Option<usize> = None;
        let mut cursor_in_current = false;
        let mut current_min_len = 0usize;

        for abs_row in 0..self.total_rows() {
            let row = self.abs_row(abs_row);
            let line_start = current.len();
            if row.wrapped {
                // Strip trailing default cells from wrapped rows — they are padding
                // to fill the terminal width, not content. Only retain real content
                // so that rewrapping doesn't re-insert blank physical rows.
                let content_end = row
                    .cells
                    .iter()
                    .rposition(|c| !c.is_default())
                    .map(|i| i + 1)
                    .unwrap_or(0);
                current.extend_from_slice(&row.cells[..content_end]);
            } else {
                current.extend_from_slice(&row.cells);
            }
            if abs_row == cursor_abs {
                cursor_in_current = true;
                current_min_len = current_min_len.max(line_start + cursor_col);
            }
            if !row.wrapped {
                if cursor_in_current {
                    cursor_line_idx = Some(lines.len());
                    cursor_in_current = false;
                }
                lines.push(LogicalLine {
                    cells: std::mem::take(&mut current),
                    min_len: current_min_len,
                });
                current_min_len = 0;
            }
        }
        if !current.is_empty() {
            if cursor_in_current {
                cursor_line_idx = Some(lines.len());
            }
            lines.push(LogicalLine { cells: current, min_len: current_min_len });
        }
        (lines, cursor_line_idx)
    }

    fn rebuild_from_rows(&mut self, rows: Vec<PageRow>, new_rows: usize, new_cols: usize) {
        let total = rows.len();
        let grid_offset = total.saturating_sub(new_rows);
        let scrollback_count = grid_offset.min(self.max_scrollback);
        let skip = grid_offset.saturating_sub(scrollback_count);

        self.pages.clear();
        self.free_list.clear();
        self.scrollback_count = 0;
        self.viewport_rows = new_rows;
        self.cols = new_cols;

        // Scrollback rows.
        for row in rows.iter().skip(skip).take(scrollback_count) {
            self.append_row(row.clone());
            self.scrollback_count += 1;
        }

        // Viewport rows.
        let viewport_start_idx = skip + scrollback_count;
        for row in rows.iter().skip(viewport_start_idx) {
            self.append_row(row.clone());
        }
        // Pad viewport if content was shorter than new_rows.
        let placed = rows.len().saturating_sub(viewport_start_idx);
        for _ in placed..new_rows {
            self.append_row(PageRow::new(new_cols));
        }
    }
}

// ── Reflow helpers ───────────────────────────────────────────────────────────

struct LogicalLine {
    cells: Vec<GraphemeCell>,
    min_len: usize,
}

fn rewrap_lines(
    lines: &[LogicalLine],
    new_cols: usize,
    cursor_line_idx: Option<usize>,
) -> (Vec<PageRow>, Option<usize>) {
    let mut rewrapped: Vec<PageRow> = Vec::new();
    let mut cursor_rewrapped_row: Option<usize> = None;

    for (line_idx, line) in lines.iter().enumerate() {
        let len = line_content_len(line);
        if cursor_line_idx == Some(line_idx) {
            cursor_rewrapped_row = Some(rewrapped.len());
        }
        if len == 0 {
            rewrapped.push(PageRow::new(new_cols));
            continue;
        }

        let content = &line.cells[..len];
        let mut pos = 0;
        while pos < content.len() {
            let mut col_count = 0usize;
            let mut end = pos;
            // Advance until we'd exceed new_cols, respecting wide char boundaries.
            while end < content.len() {
                let w = content[end].width as usize;
                if w == 0 {
                    // Spacer: belongs to previous wide char, skip counting.
                    end += 1;
                    continue;
                }
                if col_count + w > new_cols {
                    break; // wraps here
                }
                col_count += w;
                end += 1;
            }
            // Build the physical row for this slice.
            let mut cells: Vec<GraphemeCell> = Vec::with_capacity(new_cols);
            let mut placed_cols = 0usize;
            for cell in &content[pos..end] {
                if cell.width == 0 {
                    continue; // skip stale spacers
                }
                if cell.width == 2 {
                    if placed_cols + 2 > new_cols {
                        // Wide char doesn't fit — replace with '?'.
                        cells.push(GraphemeCell::from_char('?'));
                        placed_cols += 1;
                    } else {
                        cells.push(cell.clone());
                        cells.push(GraphemeCell::spacer());
                        placed_cols += 2;
                    }
                } else {
                    cells.push(cell.clone());
                    placed_cols += 1;
                }
            }
            cells.resize(new_cols, GraphemeCell::default());
            let wrapped = end < content.len();
            rewrapped.push(PageRow::from_cells(cells, wrapped));
            // Advance pos, skipping any trailing spacers.
            pos = end;
            while pos < content.len() && content[pos].width == 0 {
                pos += 1;
            }
        }
    }
    (rewrapped, cursor_rewrapped_row)
}

fn line_content_len(line: &LogicalLine) -> usize {
    let len = line
        .cells
        .iter()
        .rposition(|c| !c.is_default())
        .map(|i| i + 1)
        .unwrap_or(0);
    len.max(line.min_len.min(line.cells.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::GraphemeCell;

    #[test]
    fn new_list_has_empty_viewport() {
        let list = PageList::new(24, 80, 1000);
        assert_eq!(list.viewport_rows(), 24);
        assert_eq!(list.cols(), 80);
        assert_eq!(list.scrollback_len(), 0);
        assert!(list.viewport_get(0, 0).is_default());
    }

    #[test]
    fn viewport_set_and_get_roundtrip() {
        let mut list = PageList::new(24, 80, 1000);
        let cell = GraphemeCell::from_char('X');
        list.viewport_set(0, 0, cell);
        assert_eq!(list.viewport_get(0, 0).grapheme(), "X");
    }

    #[test]
    fn viewport_wrapped_flag() {
        let mut list = PageList::new(3, 5, 100);
        list.viewport_set_wrapped(1, true);
        assert!(list.viewport_is_wrapped(1));
        assert!(!list.viewport_is_wrapped(0));
    }

    #[test]
    fn simple_resize_grows_viewport() {
        let mut list = PageList::new(3, 5, 100);
        list.simple_resize(5, 5);
        assert_eq!(list.viewport_rows(), 5);
    }

    #[test]
    fn simple_resize_pads_cols() {
        let mut list = PageList::new(3, 5, 100);
        list.simple_resize(3, 8);
        assert_eq!(list.cols(), 8);
        assert_eq!(list.viewport_row(0).cells.len(), 8);
    }

    // ── Reflow tests ─────────────────────────────────────────────────────

    fn fill_viewport_row(list: &mut PageList, vrow: usize, text: &str) {
        for (col, ch) in text.chars().enumerate() {
            list.viewport_set(vrow, col, GraphemeCell::from_char(ch));
        }
    }

    #[test]
    fn reflow_wraps_long_line_to_narrower_width() {
        // 10-char logical line, reflow from 10 cols to 5 cols.
        // Rows 0+1 form one logical line (row 0 wrapped=true).
        let mut list = PageList::new(3, 10, 100);
        fill_viewport_row(&mut list, 0, "ABCDE");
        fill_viewport_row(&mut list, 1, "FGHIJ");
        list.viewport_set_wrapped(0, true); // row 0 continues on row 1
        let cursor_abs = list.viewport_start_abs(); // row 0 of viewport
        let pin = list.register_pin(PageCoord { abs_row: cursor_abs, col: 0 });
        list.reflow(3, 5, &pin);
        assert_eq!(list.viewport_get(0, 0).grapheme(), "A");
        assert_eq!(list.viewport_get(0, 4).grapheme(), "E");
        assert_eq!(list.viewport_get(1, 0).grapheme(), "F");
        assert_eq!(list.viewport_get(1, 4).grapheme(), "J");
    }

    #[test]
    fn reflow_wide_char_not_split_across_boundary() {
        // Place "AB日" in a 4-col terminal (日 = width 2, fits exactly at cols 2-3).
        // Reflow to 3 cols: "AB" on row 0, "日" (+ spacer) on row 1.
        let mut list = PageList::new(3, 4, 100);
        list.viewport_set(0, 0, GraphemeCell::from_char('A'));
        list.viewport_set(0, 1, GraphemeCell::from_char('B'));
        list.viewport_set(0, 2, GraphemeCell::from_char('日'));
        list.viewport_set(0, 3, GraphemeCell::spacer());
        list.viewport_set_wrapped(0, true);
        let cursor_abs = list.viewport_start_abs();
        let pin = list.register_pin(PageCoord { abs_row: cursor_abs, col: 0 });
        list.reflow(3, 3, &pin);
        assert_eq!(list.viewport_get(0, 0).grapheme(), "A");
        assert_eq!(list.viewport_get(0, 1).grapheme(), "B");
        assert_eq!(list.viewport_get(1, 0).grapheme(), "日");
        assert_eq!(list.viewport_get(1, 1).width, 0); // spacer
    }

    #[test]
    fn reflow_cursor_pin_col_is_zero_after_reflow() {
        let mut list = PageList::new(3, 10, 100);
        fill_viewport_row(&mut list, 0, "Hello");
        let cursor_abs = list.viewport_start_abs();
        let pin = list.register_pin(PageCoord { abs_row: cursor_abs, col: 5 });
        list.reflow(3, 5, &pin);
        assert_eq!(list.pin_coord(&pin).col, 0);
    }

    #[test]
    fn reflow_empty_line_produces_blank_row() {
        let mut list = PageList::new(3, 10, 100);
        let cursor_abs = list.viewport_start_abs();
        let pin = list.register_pin(PageCoord { abs_row: cursor_abs, col: 0 });
        list.reflow(3, 5, &pin);
        assert_eq!(list.viewport_rows(), 3);
        assert!(list.viewport_get(0, 0).is_default());
    }
}
