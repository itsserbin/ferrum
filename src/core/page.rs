use crate::core::GraphemeCell;

/// A single row of terminal cells.
#[derive(Clone, Debug)]
pub struct PageRow {
    pub cells: Vec<GraphemeCell>,
    pub wrapped: bool,
    /// Number of columns explicitly written to this row.
    ///
    /// Cells at indices `written_cols..cells.len()` are unwritten padding
    /// (never touched by the terminal's print/erase path). Used during
    /// reflow to avoid including trailing padding as logical-line content —
    /// which would otherwise silently discard inter-word spaces and leading
    /// indentation that fall at a soft-wrap boundary.
    pub written_cols: usize,
}

impl PageRow {
    /// Creates a row with `cols` default cells and `wrapped = false`.
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![GraphemeCell::default(); cols],
            wrapped: false,
            written_cols: 0,
        }
    }

    /// Creates a row from an existing cell vector and a wrap flag.
    ///
    /// All cells in `cells` are considered explicitly written (reflow
    /// computed them intentionally), so `written_cols = cells.len()`.
    pub fn from_cells(cells: Vec<GraphemeCell>, wrapped: bool) -> Self {
        let written_cols = cells.len();
        Self { cells, wrapped, written_cols }
    }
}

/// Maximum number of rows a single `Page` can hold.
pub const PAGE_SIZE: usize = 256;

/// A fixed-size block of up to [`PAGE_SIZE`] rows, heap-allocated to avoid
/// stack overflow when holding 256 × cols cells.
pub struct Page {
    rows: Box<[PageRow; PAGE_SIZE]>,
    pub len: usize,
    pub cols: usize,
}

impl Page {
    /// Allocates a new `Page` on the heap.
    ///
    /// The length is exactly `PAGE_SIZE` by construction (the iterator
    /// produces precisely that many items), so the `try_into` conversion
    /// always succeeds.
    pub fn new(cols: usize) -> Box<Self> {
        let rows: Box<[PageRow; PAGE_SIZE]> = (0..PAGE_SIZE)
            .map(|_| PageRow::new(cols))
            .collect::<Vec<_>>()
            .try_into()
            .expect("PAGE_SIZE rows");
        Box::new(Self { rows, len: 0, cols })
    }

    /// Returns `true` when no more rows can be pushed.
    pub fn is_full(&self) -> bool {
        self.len >= PAGE_SIZE
    }

    /// Appends a row. Panics in debug builds if the page is already full.
    pub fn push(&mut self, row: PageRow) {
        debug_assert!(!self.is_full(), "Page::push called on a full page");
        self.rows[self.len] = row;
        self.len += 1;
    }

    /// Returns a reference to the row at `idx`.
    pub fn get(&self, idx: usize) -> &PageRow {
        &self.rows[idx]
    }

    /// Returns a mutable reference to the row at `idx`.
    pub fn get_mut(&mut self, idx: usize) -> &mut PageRow {
        &mut self.rows[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::GraphemeCell;

    #[test]
    fn page_new_has_empty_rows() {
        let page = Page::new(80);
        assert_eq!(page.len, 0);
        assert_eq!(page.cols, 80);
    }

    #[test]
    fn page_push_and_get_row() {
        let mut page = Page::new(5);
        let mut row = PageRow::new(5);
        row.cells[0] = GraphemeCell::from_char('A');
        page.push(row);
        assert_eq!(page.len, 1);
        assert_eq!(page.get(0).cells[0].grapheme(), "A");
    }

    #[test]
    fn page_row_wrapped_flag() {
        let mut page = Page::new(5);
        let mut row = PageRow::new(5);
        row.wrapped = true;
        page.push(row);
        assert!(page.get(0).wrapped);
    }

    #[test]
    fn page_is_full_at_capacity() {
        let mut page = Page::new(5);
        for _ in 0..PAGE_SIZE {
            page.push(PageRow::new(5));
        }
        assert!(page.is_full());
    }

    #[test]
    fn page_row_new_has_default_cells() {
        let row = PageRow::new(3);
        assert_eq!(row.cells.len(), 3);
        assert!(!row.wrapped);
        for cell in &row.cells {
            assert!(cell.is_default());
        }
    }

    #[test]
    fn page_row_from_cells_stores_cells_and_wrap() {
        let cells = vec![GraphemeCell::from_char('X'); 4];
        let row = PageRow::from_cells(cells, true);
        assert_eq!(row.cells.len(), 4);
        assert!(row.wrapped);
        assert_eq!(row.cells[0].grapheme(), "X");
    }

    #[test]
    fn page_get_mut_allows_mutation() {
        let mut page = Page::new(3);
        page.push(PageRow::new(3));
        page.get_mut(0).cells[1] = GraphemeCell::from_char('Z');
        assert_eq!(page.get(0).cells[1].grapheme(), "Z");
    }
}
