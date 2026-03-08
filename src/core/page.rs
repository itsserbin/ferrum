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
    ///
    /// Intentionally not `pub`: all writes to `cells` must go through
    /// `PageList::viewport_set`, which keeps this field in sync.
    pub(crate) written_cols: usize,
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

    /// Resets all cells to default, clears the wrapped flag, and zeros `written_cols`.
    pub fn clear(&mut self) {
        self.clear_with(GraphemeCell::default());
    }

    /// Fills every cell with `blank`, clears the wrapped flag, and zeros `written_cols`.
    ///
    /// Unlike [`clear`], this preserves the blank's SGR attributes (e.g. background
    /// color set by `\x1b[48;…m` before an erase sequence).
    pub fn clear_with(&mut self, blank: GraphemeCell) {
        self.cells.fill(blank);
        self.wrapped = false;
        self.written_cols = 0;
    }
}

/// Maximum number of rows a single `Page` can hold.
pub const PAGE_SIZE: usize = 256;

/// A lazily-populated block of up to [`PAGE_SIZE`] rows.
///
/// Rows are allocated on demand via [`push`] rather than upfront, so a freshly
/// created page consumes only a small Vec header until rows are actually added.
pub struct Page {
    rows: Vec<PageRow>,
}

impl Page {
    /// Creates an empty page.
    ///
    /// No rows are allocated until [`push`] is called.
    pub fn new() -> Self {
        Self { rows: Vec::with_capacity(PAGE_SIZE) }
    }

    /// Returns `true` when no more rows can be pushed.
    pub fn is_full(&self) -> bool {
        self.rows.len() >= PAGE_SIZE
    }

    /// Appends a row. Panics in debug builds if the page is already full.
    pub fn push(&mut self, row: PageRow) {
        debug_assert!(!self.is_full(), "Page::push called on a full page");
        self.rows.push(row);
    }

    /// Returns a reference to the row at `idx`.
    pub fn row(&self, idx: usize) -> &PageRow {
        &self.rows[idx]
    }

    /// Returns a mutable reference to the row at `idx`.
    pub fn row_mut(&mut self, idx: usize) -> &mut PageRow {
        &mut self.rows[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::GraphemeCell;

    #[test]
    fn page_new_has_empty_rows() {
        let page = Page::new();
        assert!(!page.is_full());
    }

    #[test]
    fn page_push_and_get_row() {
        let mut page = Page::new();
        let mut row = PageRow::new(5);
        row.cells[0] = GraphemeCell::from_char('A');
        page.push(row);
        assert_eq!(page.rows.len(), 1);
        assert_eq!(page.row(0).cells[0].grapheme(), "A");
    }

    #[test]
    fn page_row_wrapped_flag() {
        let mut page = Page::new();
        let mut row = PageRow::new(5);
        row.wrapped = true;
        page.push(row);
        assert!(page.row(0).wrapped);
    }

    #[test]
    fn page_is_full_at_capacity() {
        let mut page = Page::new();
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
    fn page_row_mut_allows_mutation() {
        let mut page = Page::new();
        page.push(PageRow::new(3));
        page.row_mut(0).cells[1] = GraphemeCell::from_char('Z');
        assert_eq!(page.row(0).cells[1].grapheme(), "Z");
    }
}
