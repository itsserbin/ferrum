use std::collections::VecDeque;

use crate::core::tracked_pin::{PageCoord, TrackedPin};
use crate::core::{GraphemeCell, Page, PageRow, PAGE_SIZE};

pub struct PageList {
    /// Scrollback ring buffer: oldest row at the front, newest at the back.
    /// Stored separately from the viewport so eviction is O(1).
    scrollback: VecDeque<PageRow>,
    /// Viewport rows only.  Physical page index 0 corresponds to viewport row 0.
    pages: Vec<Page>,
    viewport_rows: usize,
    cols: usize,
    max_scrollback: usize,
}

impl PageList {
    pub fn new(viewport_rows: usize, cols: usize, max_scrollback: usize) -> Self {
        let mut list = Self {
            scrollback: VecDeque::new(),
            pages: Vec::new(),
            viewport_rows,
            cols,
            max_scrollback,
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
        self.scrollback.len()
    }

    pub fn total_rows(&self) -> usize {
        self.scrollback.len() + self.viewport_rows
    }

    /// Absolute index of the first viewport row.
    pub fn viewport_start_abs(&self) -> usize {
        self.scrollback.len()
    }

    // ── Internal: viewport row → (page_idx, row_within_page) ─────────────────

    fn vrow_to_page(vrow: usize) -> (usize, usize) {
        (vrow / PAGE_SIZE, vrow % PAGE_SIZE)
    }

    // ── Viewport access ───────────────────────────────────────────────────────

    pub fn viewport_get(&self, row: usize, col: usize) -> &GraphemeCell {
        let (pi, ri) = Self::vrow_to_page(row);
        &self.pages[pi].row(ri).cells[col]
    }

    pub fn viewport_set(&mut self, row: usize, col: usize, cell: GraphemeCell) {
        let (pi, ri) = Self::vrow_to_page(row);
        let page_row = self.pages[pi].row_mut(ri);
        page_row.cells[col] = cell;
        // Track the highest column written so reflow can distinguish real content
        // from unwritten padding at the end of soft-wrapped rows.
        if col + 1 > page_row.written_cols {
            page_row.written_cols = col + 1;
        }
    }

    pub fn viewport_row(&self, row: usize) -> &PageRow {
        let (pi, ri) = Self::vrow_to_page(row);
        self.pages[pi].row(ri)
    }

    pub fn viewport_row_mut(&mut self, row: usize) -> &mut PageRow {
        let (pi, ri) = Self::vrow_to_page(row);
        self.pages[pi].row_mut(ri)
    }

    pub fn viewport_is_wrapped(&self, row: usize) -> bool {
        self.viewport_row(row).wrapped
    }

    pub fn viewport_set_wrapped(&mut self, row: usize, wrapped: bool) {
        self.viewport_row_mut(row).wrapped = wrapped;
    }

    // ── Scrollback access ─────────────────────────────────────────────────────

    pub fn scrollback_row(&self, idx: usize) -> &PageRow {
        &self.scrollback[idx]
    }

    // ── Copy / scroll within viewport ─────────────────────────────────────────

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
        self.viewport_row_mut(bottom).clear();
        if let Some(evicted_row) = evicted {
            self.push_to_scrollback(evicted_row);
        }
    }

    /// Scroll down within a viewport region [top..=bottom].
    pub fn scroll_down_region(&mut self, top: usize, bottom: usize) {
        for row in (top..bottom).rev() {
            self.viewport_copy_row(row, row + 1);
        }
        self.viewport_row_mut(top).clear();
    }

    /// Push a row into the scrollback ring buffer.  O(1) amortized.
    ///
    /// When the buffer is at capacity the oldest row is evicted before the
    /// new row is inserted — no data is shifted.
    pub fn push_to_scrollback(&mut self, row: PageRow) {
        if self.scrollback.len() >= self.max_scrollback {
            self.scrollback.pop_front(); // evict oldest — O(1)
        }
        self.scrollback.push_back(row); // O(1) amortized
    }

    // ── Recolor ───────────────────────────────────────────────────────────────

    /// Apply a function to every cell in the current viewport.
    pub fn viewport_recolor<F: FnMut(&mut GraphemeCell)>(&mut self, mut f: F) {
        for vrow in 0..self.viewport_rows {
            let (pi, ri) = Self::vrow_to_page(vrow);
            for cell in &mut self.pages[pi].row_mut(ri).cells {
                f(cell);
            }
        }
    }

    /// Apply a function to every cell in the scrollback buffer.
    pub fn scrollback_recolor<F: FnMut(&mut GraphemeCell)>(&mut self, mut f: F) {
        for row in &mut self.scrollback {
            for cell in &mut row.cells {
                f(cell);
            }
        }
    }

    // ── Internal: append a viewport row at the end ────────────────────────────

    pub fn append_row(&mut self, row: PageRow) {
        if self.pages.last().map(|p| p.is_full()).unwrap_or(true) {
            self.alloc_page();
        }
        debug_assert!(!self.pages.is_empty(), "pages non-empty after alloc_page");
        if let Some(page) = self.pages.last_mut() {
            page.push(row);
        }
    }

    fn alloc_page(&mut self) {
        self.pages.push(Page::new());
    }

    // ── Simple resize ─────────────────────────────────────────────────────────

    pub fn simple_resize(&mut self, new_rows: usize, new_cols: usize) {
        if new_rows > self.viewport_rows {
            let extra = new_rows - self.viewport_rows;
            for _ in 0..extra {
                self.append_row(PageRow::new(new_cols));
            }
        }
        self.viewport_rows = new_rows;
        self.cols = new_cols;
        for vrow in 0..self.viewport_rows {
            let (pi, ri) = Self::vrow_to_page(vrow);
            let row = self.pages[pi].row_mut(ri);
            row.cells.resize(new_cols, GraphemeCell::default());
            row.written_cols = row.written_cols.min(new_cols);
        }
    }

    // ── Abs-row access (for reflow) ───────────────────────────────────────────

    fn abs_row(&self, abs: usize) -> &PageRow {
        let sb_len = self.scrollback.len();
        if abs < sb_len {
            &self.scrollback[abs]
        } else {
            let vrow = abs - sb_len;
            let (pi, ri) = Self::vrow_to_page(vrow);
            self.pages[pi].row(ri)
        }
    }

    // ── Pin management ────────────────────────────────────────────────────────

    /// Creates a new pin at the given coordinate.
    /// Cloning the returned handle shares the same underlying coordinate.
    pub fn pin_at(&self, coord: PageCoord) -> TrackedPin {
        TrackedPin::new(coord)
    }

    // ── Reflow ────────────────────────────────────────────────────────────────

    /// Reflow all content (scrollback + viewport) to new dimensions.
    ///
    /// After reflow, `cursor_pin` is updated to the cursor's position in the
    /// reflowed buffer.  The column is preserved so the terminal stays in sync
    /// with the shell's internal cursor tracking (readline / zsh compute relative
    /// movements from their own state; forcing col=0 would desynchronise them).
    ///
    /// **Narrow resize** (`new_cols < old_cols`):
    ///   • If the cursor's logical line carries content (shell prompt or active
    ///     input), all rewrapped rows are pushed into scrollback and the viewport
    ///     is left blank.  The blank rows preceding the cursor are flagged
    ///     `wrapped=true` so they collapse into a single logical unit on the
    ///     next reflow, preventing spurious blank logical lines from accumulating
    ///     across repeated resize cycles.  This eliminates the duplicate-prompt
    ///     glitch: the shell redraws the prompt after SIGWINCH anyway.
    ///   • If the cursor's logical line is blank (cursor is parked on an empty
    ///     row after output), pre-cursor content fills the viewport from the
    ///     bottom up, keeping visible output in view.
    ///
    /// **Wide / equal resize** (`new_cols >= old_cols`):
    ///   Reflow all logical lines with `grid_offset = 0` so content fills the
    ///   viewport from the top without unnecessary scrollback spill.
    pub fn reflow(&mut self, new_rows: usize, new_cols: usize, cursor_pin: &TrackedPin) {
        let is_narrow = new_cols < self.cols;
        let (logical_lines, cursor_info) = self.collect_logical_lines(cursor_pin);

        let cursor_new_col = cursor_info
            .as_ref()
            .map_or_else(
                || cursor_pin.coord().col.min(new_cols.saturating_sub(1)),
                |ci| ci.col_in_line.min(new_cols.saturating_sub(1)),
            );

        let cursor_line_has_content = cursor_info.as_ref().is_some_and(|ci| {
            logical_lines.get(ci.line_idx).is_some_and(|l| line_content_len(l) > 0)
        });

        if is_narrow && cursor_line_has_content {
            // Narrow resize with active content on the cursor's line.
            // Push all rewrapped rows into scrollback; leave the viewport blank.
            let rewrapped = rewrap_lines(&logical_lines, new_cols);
            let rows_for_content = new_rows.saturating_sub(1);

            // cursor_row_in_rows must be large enough to make grid_offset equal to
            // rewrapped.len(), so every row ends up in scrollback.
            let anchor = rewrapped.len() + rows_for_content.saturating_sub(1);
            self.rebuild_from_rows(rewrapped, rows_for_content, new_cols, Some(anchor));

            // Mark the blank padding rows as wrapped so they merge into one logical
            // unit with the cursor on the next reflow.  Without this they become
            // separate blank logical lines that accumulate and push real content into
            // scrollback across repeated narrow→wide resize cycles.
            for vrow in 0..rows_for_content {
                self.viewport_row_mut(vrow).wrapped = true;
            }
            self.append_row(PageRow::new(new_cols)); // cursor row (not wrapped)
            self.viewport_rows = new_rows;
        } else if !is_narrow {
            // Wide / equal resize: reflow everything and fill the viewport from
            // the top (grid_offset = 0).
            let rewrapped = rewrap_lines(&logical_lines, new_cols);
            self.rebuild_from_rows(
                rewrapped,
                new_rows,
                new_cols,
                Some(new_rows.saturating_sub(1)),
            );
            self.viewport_rows = new_rows;
        } else {
            // Narrow resize with blank cursor line: keep pre-cursor content visible.
            let mut logical_lines = logical_lines;
            if let Some(ref ci) = cursor_info {
                logical_lines.truncate(ci.line_idx);
            }
            let rewrapped = rewrap_lines(&logical_lines, new_cols);
            let rows_for_content = new_rows.saturating_sub(1);

            if rows_for_content == 0 {
                // Single viewport row: cursor occupies it directly.
                self.rebuild_from_rows(rewrapped, new_rows, new_cols, None);
            } else {
                // Anchor so content fills the content area from the bottom up.
                let anchor =
                    rewrapped.len().saturating_sub(rows_for_content).saturating_add(1);
                self.rebuild_from_rows(rewrapped, rows_for_content, new_cols, Some(anchor));
                self.append_row(PageRow::new(new_cols)); // blank cursor row
            }
            self.viewport_rows = new_rows;
        }

        let cursor_abs = self.scrollback.len() + new_rows.saturating_sub(1);
        cursor_pin.set_coord(PageCoord { abs_row: cursor_abs, col: cursor_new_col });
    }

    /// Returns `(logical_lines, cursor_info)` where `cursor_info` is the cursor's
    /// logical line index and column offset within that line (= cells accumulated
    /// before the cursor's physical row + cursor_col within that row).
    fn collect_logical_lines(
        &self,
        cursor_pin: &TrackedPin,
    ) -> (Vec<LogicalLine>, Option<CursorLineInfo>) {
        let cursor_abs = cursor_pin.coord().abs_row;
        let cursor_col = cursor_pin.coord().col;
        let mut lines: Vec<LogicalLine> = Vec::new();
        let mut current: Vec<GraphemeCell> = Vec::new();
        let mut cursor_info: Option<CursorLineInfo> = None;
        let mut cursor_in_current = false;
        let mut cursor_col_in_line = 0usize;
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
                // Record the cursor's column offset within this logical line so
                // `rewrap_lines` can compute the correct physical row after rewrapping.
                cursor_col_in_line = line_start + cursor_col;
                current_min_len = current_min_len.max(cursor_col_in_line);
            }
            if !row.wrapped {
                if cursor_in_current {
                    cursor_info = Some(CursorLineInfo {
                        line_idx: lines.len(),
                        col_in_line: cursor_col_in_line,
                    });
                    cursor_in_current = false;
                    cursor_col_in_line = 0;
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
                cursor_info = Some(CursorLineInfo {
                    line_idx: lines.len(),
                    col_in_line: cursor_col_in_line,
                });
            }
            lines.push(LogicalLine { cells: current, min_len: current_min_len });
        }
        (lines, cursor_info)
    }

    /// Rebuilds the buffer from `rows` for the given new dimensions.
    ///
    /// `cursor_row_in_rows`: when provided, anchors the viewport to the cursor
    /// so that the cursor lands at the last viewport row.  This prevents blank
    /// rows that trail the cursor in the rewrapped buffer from pushing real
    /// content into scrollback — which would make the content disappear from
    /// view after a narrow resize.  When `None`, falls back to end-of-content
    /// anchoring (used when no cursor info is available).
    ///
    /// Returns `skip`: the number of leading rows discarded because they are
    /// too old to fit even in scrollback.  The caller uses this to map a
    /// `rewrapped`-Vec index back to an abs index in the rebuilt buffer.
    fn rebuild_from_rows(
        &mut self,
        rows: Vec<PageRow>,
        new_rows: usize,
        new_cols: usize,
        cursor_row_in_rows: Option<usize>,
    ) -> usize {
        // Anchor the viewport to the cursor when possible so that trailing
        // blank rows (below the cursor) are dropped instead of pushed into
        // the visible area at the expense of real content above the cursor.
        let grid_offset = if let Some(cr) = cursor_row_in_rows {
            cr.saturating_sub(new_rows.saturating_sub(1))
        } else {
            rows.len().saturating_sub(new_rows)
        };
        let scrollback_count = grid_offset.min(self.max_scrollback);
        let skip = grid_offset.saturating_sub(scrollback_count);

        self.scrollback.clear();
        self.pages.clear();
        self.viewport_rows = new_rows;
        self.cols = new_cols;

        // Scrollback rows, then viewport rows — consume `rows` in one pass.
        // After skipping `skip` discarded rows, the first `scrollback_count`
        // rows become scrollback and the next `new_rows` become the viewport.
        let mut rows_iter = rows.into_iter().skip(skip);
        for row in rows_iter.by_ref().take(scrollback_count) {
            self.scrollback.push_back(row);
        }

        // Viewport rows: take exactly new_rows rows.
        // Using .take(new_rows) ensures rows that follow the cursor (e.g. blank
        // padding) are not appended beyond the viewport boundary.
        let mut placed = 0;
        for row in rows_iter.take(new_rows) {
            self.append_row(row);
            placed += 1;
        }
        // Pad viewport if content was shorter than new_rows.
        for _ in placed..new_rows {
            self.append_row(PageRow::new(new_cols));
        }
        skip
    }
}

// ── Reflow helpers ────────────────────────────────────────────────────────────

struct LogicalLine {
    cells: Vec<GraphemeCell>,
    min_len: usize,
}

/// Cursor location within the logical-line representation produced by
/// `collect_logical_lines`: `line_idx` is the index into the `Vec<LogicalLine>`,
/// `col_in_line` is the column offset within that logical line.
struct CursorLineInfo {
    line_idx: usize,
    col_in_line: usize,
}

fn rewrap_lines(lines: &[LogicalLine], new_cols: usize) -> Vec<PageRow> {
    let mut rewrapped: Vec<PageRow> = Vec::new();

    for line in lines.iter() {
        let len = line_content_len(line);

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
            let wrapped = end < content.len();

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
            rewrapped.push(PageRow::from_cells(cells, wrapped));

            // Advance pos, skipping any trailing spacers.
            pos = end;
            while pos < content.len() && content[pos].width == 0 {
                pos += 1;
            }
        }
    }
    rewrapped
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

    #[test]
    fn push_to_scrollback_stays_within_max() {
        let mut list = PageList::new(3, 5, 3);
        // Push 5 rows into a scrollback capped at 3.
        for i in 0..5u8 {
            let mut row = PageRow::new(5);
            row.cells[0] = GraphemeCell::from_char(char::from(b'A' + i));
            list.push_to_scrollback(row);
        }
        assert_eq!(list.scrollback_len(), 3);
        // Oldest 2 rows were evicted; only C, D, E remain.
        assert_eq!(list.scrollback_row(0).cells[0].grapheme(), "C");
        assert_eq!(list.scrollback_row(1).cells[0].grapheme(), "D");
        assert_eq!(list.scrollback_row(2).cells[0].grapheme(), "E");
    }

    #[test]
    fn scroll_up_region_pushes_to_scrollback() {
        let mut list = PageList::new(3, 5, 100);
        list.viewport_set(0, 0, GraphemeCell::from_char('X'));
        list.scroll_up_region(0, 2, true);
        assert_eq!(list.scrollback_len(), 1);
        assert_eq!(list.scrollback_row(0).cells[0].grapheme(), "X");
    }

    // ── Reflow tests ──────────────────────────────────────────────────────────

    fn fill_viewport_row(list: &mut PageList, vrow: usize, text: &str) {
        for (col, ch) in text.chars().enumerate() {
            list.viewport_set(vrow, col, GraphemeCell::from_char(ch));
        }
    }

    #[test]
    fn reflow_wraps_long_line_to_narrower_width() {
        // 10-char logical line (rows 0+1 form one wrapped logical line),
        // reflow from 10 cols to 5 cols.
        let mut list = PageList::new(3, 10, 100);
        fill_viewport_row(&mut list, 0, "ABCDE");
        fill_viewport_row(&mut list, 1, "FGHIJ");
        list.viewport_set_wrapped(0, true);
        // Cursor on blank row 2 (after the content) — realistic shell position.
        let cursor_abs = list.viewport_start_abs() + 2;
        let pin = list.pin_at(PageCoord { abs_row: cursor_abs, col: 0 });
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
        // Cursor on blank row 2 (after the content).
        let cursor_abs = list.viewport_start_abs() + 2;
        let pin = list.pin_at(PageCoord { abs_row: cursor_abs, col: 0 });
        list.reflow(3, 3, &pin);
        assert_eq!(list.viewport_get(0, 0).grapheme(), "A");
        assert_eq!(list.viewport_get(0, 1).grapheme(), "B");
        assert_eq!(list.viewport_get(1, 0).grapheme(), "日");
        assert_eq!(list.viewport_get(1, 1).width, 0);
    }

    #[test]
    fn reflow_cursor_pin_preserves_col_after_reflow() {
        // "Hello" (5 chars) in a 10-col terminal.
        // Cursor is on the blank row after content, at col 5.
        // After reflow to 5 cols: cursor col is clamped to new_cols-1 = 4.
        let mut list = PageList::new(3, 10, 100);
        fill_viewport_row(&mut list, 0, "Hello");
        let cursor_abs = list.viewport_start_abs() + 2; // blank row after content
        let pin = list.pin_at(PageCoord { abs_row: cursor_abs, col: 5 });
        list.reflow(3, 5, &pin);
        assert_eq!(pin.coord().col, 4);
    }

    #[test]
    fn reflow_preserves_inter_word_space_at_wrap_boundary() {
        // "hello world" (11 chars) at cols=6 wraps as:
        //   Row 0: "hello " (wrapped=true)  ← trailing space is a default cell
        //   Row 1: "world " (not wrapped)
        // After reflow to cols=11 it should be "hello world", not "helloworld".
        let mut list = PageList::new(3, 6, 100); // 3 rows: content + content + cursor
        fill_viewport_row(&mut list, 0, "hello ");
        fill_viewport_row(&mut list, 1, "world ");
        list.viewport_set_wrapped(0, true);
        let cursor_abs = list.viewport_start_abs() + 2; // blank cursor row
        let pin = list.pin_at(PageCoord { abs_row: cursor_abs, col: 0 });
        list.reflow(1, 11, &pin);
        assert_eq!(list.viewport_get(0, 4).grapheme(), "o", "col 4 should be 'o'");
        assert_eq!(
            list.viewport_get(0, 5).grapheme(),
            " ",
            "space between words must survive reflow"
        );
        assert_eq!(list.viewport_get(0, 6).grapheme(), "w", "col 6 should be 'w'");
    }

    #[test]
    fn reflow_preserves_leading_indentation_at_wrap_boundary() {
        // "   ABC" (3-space indent + ABC = 6 chars) at cols=3 wraps as:
        //   Row 0: "   " (3 spaces, wrapped=true)  ← all default cells
        //   Row 1: "ABC" (not wrapped)
        // After reflow to cols=6 it should be "   ABC", not "ABC".
        let mut list = PageList::new(3, 3, 100); // 3 rows: content + content + cursor
        fill_viewport_row(&mut list, 0, "   ");
        fill_viewport_row(&mut list, 1, "ABC");
        list.viewport_set_wrapped(0, true);
        let cursor_abs = list.viewport_start_abs() + 2; // blank cursor row
        let pin = list.pin_at(PageCoord { abs_row: cursor_abs, col: 0 });
        list.reflow(1, 6, &pin);
        assert_eq!(
            list.viewport_get(0, 0).grapheme(),
            " ",
            "leading space 0 must survive reflow"
        );
        assert_eq!(
            list.viewport_get(0, 1).grapheme(),
            " ",
            "leading space 1 must survive reflow"
        );
        assert_eq!(
            list.viewport_get(0, 2).grapheme(),
            " ",
            "leading space 2 must survive reflow"
        );
        assert_eq!(list.viewport_get(0, 3).grapheme(), "A");
        assert_eq!(list.viewport_get(0, 4).grapheme(), "B");
        assert_eq!(list.viewport_get(0, 5).grapheme(), "C");
    }

    #[test]
    fn reflow_empty_line_produces_blank_row() {
        let mut list = PageList::new(3, 10, 100);
        let cursor_abs = list.viewport_start_abs();
        let pin = list.pin_at(PageCoord { abs_row: cursor_abs, col: 0 });
        list.reflow(3, 5, &pin);
        assert_eq!(list.viewport_rows(), 3);
        assert!(list.viewport_get(2, 0).is_default()); // cursor row is last row, blank
    }

    #[test]
    fn reflow_narrow_active_line_is_cleared() {
        // 5-col terminal, 4 rows. Row 2 has "ABCDE" (content above cursor).
        // Cursor on row 3 (blank). After reflow to 2 cols:
        // "ABCDE" must be reflowed into rows above the cursor.
        // The last row (cursor row) must be blank.
        let mut list = PageList::new(4, 5, 100);
        fill_viewport_row(&mut list, 2, "ABCDE");
        let cursor_abs = list.viewport_start_abs() + 3; // blank row after content
        let pin = list.pin_at(PageCoord { abs_row: cursor_abs, col: 0 });
        list.reflow(4, 2, &pin);
        // Cursor row (last row) must be blank.
        assert!(list.viewport_get(3, 0).is_default(), "cursor row must be blank after reflow");
        // Content above cursor must have been reflowed.
        assert_eq!(list.viewport_get(1, 0).grapheme(), "A", "reflowed content row 1 col 0");
        assert_eq!(list.viewport_get(2, 0).grapheme(), "C", "reflowed content row 2 col 0");
    }

    #[test]
    fn reflow_narrow_cursor_always_at_last_viewport_row() {
        // 4-row, 10-col terminal. Content at rows 0-1 (wrapped), cursor at row 3 (blank).
        let mut list = PageList::new(4, 10, 100);
        fill_viewport_row(&mut list, 0, "hello");
        fill_viewport_row(&mut list, 1, "world");
        list.viewport_set_wrapped(0, true);
        let cursor_abs = list.viewport_start_abs() + 3;
        let pin = list.pin_at(PageCoord { abs_row: cursor_abs, col: 3 });
        list.reflow(4, 3, &pin);
        let expected_last_row_abs = list.viewport_start_abs() + list.viewport_rows() - 1;
        assert_eq!(
            pin.coord().abs_row,
            expected_last_row_abs,
            "cursor must be at last viewport row after reflow"
        );
    }

    #[test]
    fn reflow_narrow_does_not_duplicate_prompt() {
        // Simulates narrow resize: "PROMPT" is on the last row (cursor here).
        // After reflow to 2 cols the prompt must NOT appear in the viewport —
        // the shell redraws it after SIGWINCH, so any duplicate causes visual
        // corruption.
        let mut list = PageList::new(4, 6, 100);
        fill_viewport_row(&mut list, 3, "PROMPT");
        let cursor_abs = list.viewport_start_abs() + 3;
        let pin = list.pin_at(PageCoord { abs_row: cursor_abs, col: 6 });
        list.reflow(4, 2, &pin);
        // Every viewport row must be blank — no prompt content duplicated.
        for vrow in 0..list.viewport_rows() {
            assert!(
                list.viewport_get(vrow, 0).is_default(),
                "row {vrow} col 0 must be blank — no duplicate prompt content"
            );
        }
        // Cursor at last viewport row.
        assert_eq!(
            pin.coord().abs_row,
            list.viewport_start_abs() + 3,
            "cursor must be at last viewport row"
        );
    }
}
