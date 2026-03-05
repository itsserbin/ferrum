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
        self.pages.push(page);
    }

    // ── Scroll ───────────────────────────────────────────────────────────────

    /// Push the top `count` viewport rows into scrollback, appending blank rows at the bottom.
    pub fn scroll_up(&mut self, count: usize) {
        let count = count.min(self.viewport_rows);
        let would_be = self.scrollback_count + count;
        if would_be > self.max_scrollback {
            let evict = would_be - self.max_scrollback;
            self.evict_scrollback(evict);
        }
        self.scrollback_count += count;
        let cols = self.cols;
        for _ in 0..count {
            self.append_row(PageRow::new(cols));
        }
    }

    fn evict_scrollback(&mut self, count: usize) {
        let count = count.min(self.scrollback_count);
        let first_kept_page = count / PAGE_SIZE;
        let evicted_pages: Vec<Page> = self.pages.drain(0..first_kept_page).collect();
        for p in evicted_pages {
            self.free_list.push(p);
        }
        self.scrollback_count -= first_kept_page * PAGE_SIZE;
        let remaining_evict = count - first_kept_page * PAGE_SIZE;
        self.scrollback_count = self.scrollback_count.saturating_sub(remaining_evict);
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
    fn scroll_up_grows_scrollback() {
        let mut list = PageList::new(3, 5, 100);
        list.scroll_up(1);
        assert_eq!(list.scrollback_len(), 1);
        assert_eq!(list.viewport_rows(), 3);
        assert_eq!(list.total_rows(), 4);
    }

    #[test]
    fn scroll_up_preserves_content_in_scrollback() {
        let mut list = PageList::new(3, 5, 100);
        list.viewport_set(0, 0, GraphemeCell::from_char('A'));
        list.scroll_up(1);
        // Row that was viewport row 0 is now scrollback row 0.
        assert_eq!(list.scrollback_row(0).cells[0].grapheme(), "A");
        // New viewport row 0 is blank.
        assert!(list.viewport_get(0, 0).is_default());
    }

    #[test]
    fn scrollback_respects_max_rows() {
        let mut list = PageList::new(2, 5, 3);
        for _ in 0..6 {
            list.scroll_up(1);
        }
        assert!(list.scrollback_len() <= 3);
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
}
