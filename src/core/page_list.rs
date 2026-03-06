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
        let page_row = self.pages[pi].get_mut(ri);
        page_row.cells[col] = cell;
        // Track the highest column written so reflow can distinguish real content
        // from unwritten padding at the end of soft-wrapped rows.
        if col + 1 > page_row.written_cols {
            page_row.written_cols = col + 1;
        }
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

    /// Copy a viewport row to another viewport row.
    pub fn viewport_copy_row(&mut self, src_row: usize, dst_row: usize) {
        debug_assert_ne!(src_row, dst_row);
        let src = self.viewport_row(src_row).clone();
        *self.viewport_row_mut(dst_row) = src;
    }

    /// Scroll up within a viewport region [top..=bottom].
    ///
    /// When `top == 0` and `to_scrollback` is `true`, the evicted top row is
    /// appended to the scrollback buffer (oldest entries are dropped when the
    /// buffer is full).  In all other cases the evicted row is discarded.
    pub fn scroll_up_region(&mut self, top: usize, bottom: usize, to_scrollback: bool) {
        let evicted = if top == 0 && to_scrollback {
            Some(self.viewport_row(top).clone())
        } else {
            None
        };
        for row in (top + 1)..=bottom {
            self.viewport_copy_row(row, row - 1);
        }
        // Clear the bottom row.
        let row_mut = self.viewport_row_mut(bottom);
        for cell in &mut row_mut.cells {
            *cell = GraphemeCell::default();
        }
        row_mut.wrapped = false;
        if let Some(evicted_row) = evicted {
            self.push_to_scrollback(evicted_row);
        }
    }

    /// Scroll down within a viewport region [top..=bottom].
    pub fn scroll_down_region(&mut self, top: usize, bottom: usize) {
        for row in (top..bottom).rev() {
            self.viewport_copy_row(row, row + 1);
        }
        let row_mut = self.viewport_row_mut(top);
        for cell in &mut row_mut.cells {
            *cell = GraphemeCell::default();
        }
        row_mut.wrapped = false;
    }

    /// Append a row to the scrollback buffer, evicting the oldest row if full.
    pub fn push_to_scrollback(&mut self, row: PageRow) {
        if self.scrollback_count >= self.max_scrollback {
            // Buffer is full: evict the oldest scrollback row (abs index 0)
            // by shifting all scrollback rows left by one slot and placing
            // the new row at the last scrollback slot.
            debug_assert!(self.scrollback_count > 0, "max_scrollback must be > 0");
            for i in 1..self.scrollback_count {
                let src = self.abs_row(i).clone();
                *self.abs_row_mut(i - 1) = src;
            }
            *self.abs_row_mut(self.scrollback_count - 1) = row;
        } else {
            // Buffer has space: insert the new row at the scrollback/viewport
            // boundary (abs index = scrollback_count) so it becomes the newest
            // scrollback row.  This requires shifting all viewport rows up by 1.
            //
            // Step 1: append a spare row at the physical end to make room.
            //         The new spare is at abs index (scrollback_count + viewport_rows).
            let spare_abs = self.scrollback_count + self.viewport_rows;
            self.append_row(PageRow::new(self.cols));
            // Step 2: rotate viewport rows one position forward (higher abs),
            //         working from the end toward the boundary.
            for i in (self.scrollback_count..spare_abs).rev() {
                let src = self.abs_row(i).clone();
                *self.abs_row_mut(i + 1) = src;
            }
            // Step 3: write the new scrollback row at the boundary slot.
            *self.abs_row_mut(self.scrollback_count) = row;
            // Step 4: increment scrollback_count; viewport_rows stays unchanged.
            self.scrollback_count += 1;
        }
    }

    /// Apply a function to every cell in the current viewport.
    pub fn viewport_recolor<F: FnMut(&mut GraphemeCell)>(&mut self, mut f: F) {
        for vrow in 0..self.viewport_rows {
            let abs = self.viewport_start_abs() + vrow;
            let (pi, ri) = self.abs_to_page(abs);
            for cell in &mut self.pages[pi].get_mut(ri).cells {
                f(cell);
            }
        }
    }

    /// Apply a function to every cell in the scrollback buffer.
    pub fn scrollback_recolor<F: FnMut(&mut GraphemeCell)>(&mut self, mut f: F) {
        for sb_idx in 0..self.scrollback_count {
            let (pi, ri) = self.abs_to_page(sb_idx);
            for cell in &mut self.pages[pi].get_mut(ri).cells {
                f(cell);
            }
        }
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
            row.written_cols = row.written_cols.min(new_cols);
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
                // For soft-wrapped rows use `written_cols` as the content boundary.
                // Cells beyond `written_cols` are unwritten padding (e.g. the last
                // cell left blank when a wide char couldn't fit). Using `written_cols`
                // here preserves intentional spaces — inter-word gaps and leading
                // indentation — that happen to fall at a wrap boundary, while still
                // excluding trailing unwritten padding.
                let take = row.written_cols.min(row.cells.len());
                current.extend_from_slice(&row.cells[..take]);
            } else {
                // Non-wrapped rows: copy all cells; `line_content_len` handles
                // trailing-default trimming at the logical-line level.
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
                        // Wide char doesn't fit — replace with '?' grapheme.
                        cells.push(GraphemeCell::from_str("?"));
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
    fn reflow_preserves_inter_word_space_at_wrap_boundary() {
        // "hello world" (11 chars) at cols=6 wraps as:
        //   Row 0: "hello " (wrapped=true)  ← trailing space is a default cell
        //   Row 1: "world " (not wrapped)
        // After reflow to cols=11 it should be "hello world", not "helloworld".
        let mut list = PageList::new(2, 6, 100);
        fill_viewport_row(&mut list, 0, "hello ");
        fill_viewport_row(&mut list, 1, "world ");
        list.viewport_set_wrapped(0, true);
        let cursor_abs = list.viewport_start_abs();
        let pin = list.register_pin(PageCoord { abs_row: cursor_abs, col: 0 });
        list.reflow(1, 11, &pin);
        let mut result = String::new();
        for col in 0..5 {
            result.push_str(list.viewport_get(0, col).grapheme());
        }
        // "hello" at cols 0-4, space at col 5, "world" at cols 6-10.
        assert_eq!(list.viewport_get(0, 4).grapheme(), "o", "col 4 should be 'o'");
        assert_eq!(list.viewport_get(0, 5).grapheme(), " ", "space between words must survive reflow");
        assert_eq!(list.viewport_get(0, 6).grapheme(), "w", "col 6 should be 'w'");
    }

    #[test]
    fn reflow_preserves_leading_indentation_at_wrap_boundary() {
        // "   ABC" (3-space indent + ABC = 6 chars) at cols=3 wraps as:
        //   Row 0: "   " (3 spaces, wrapped=true)  ← all default cells
        //   Row 1: "ABC" (not wrapped)
        // After reflow to cols=6 it should be "   ABC", not "ABC".
        let mut list = PageList::new(2, 3, 100);
        fill_viewport_row(&mut list, 0, "   ");
        fill_viewport_row(&mut list, 1, "ABC");
        list.viewport_set_wrapped(0, true);
        let cursor_abs = list.viewport_start_abs();
        let pin = list.register_pin(PageCoord { abs_row: cursor_abs, col: 0 });
        list.reflow(1, 6, &pin);
        assert_eq!(list.viewport_get(0, 0).grapheme(), " ", "leading space 0 must survive reflow");
        assert_eq!(list.viewport_get(0, 1).grapheme(), " ", "leading space 1 must survive reflow");
        assert_eq!(list.viewport_get(0, 2).grapheme(), " ", "leading space 2 must survive reflow");
        assert_eq!(list.viewport_get(0, 3).grapheme(), "A");
        assert_eq!(list.viewport_get(0, 4).grapheme(), "B");
        assert_eq!(list.viewport_get(0, 5).grapheme(), "C");
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
