# Page-Based GraphemeCell Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace `Cell`/`Grid`/`VecDeque<Row>` with `GraphemeCell`/`PageList` — a unified
page-based grapheme-aware terminal buffer with tracked pins for cursor and selection.

**Architecture:** `PageList` holds all content (scrollback + visible area) as `Vec<Box<Page>>`
where each `Page` = 256 rows. `GraphemeCell` stores grapheme clusters inline (up to 8 bytes)
with an explicit `width` field for wide chars. `TrackedPin` is a registered coordinate that
auto-updates on reflow. Cursor goes to `col=0` after reflow (SIGWINCH compat preserved).

**Tech Stack:** Rust, `unicode-width` (already in Cargo.toml), `unicode-segmentation` (to add).

---

## Task 1: Add `unicode-segmentation` + `GraphemeCell` type

**Files:**
- Modify: `Cargo.toml`
- Create: `src/core/grapheme_cell.rs`
- Modify: `src/core/mod.rs` (add pub mod, keep old Cell export for now)

**Step 1: Add dependency to Cargo.toml**

In the `[dependencies]` section, after `unicode-width = "0.2"` add:
```toml
unicode-segmentation = "1"
```

Run `cargo fetch` to confirm it resolves.

**Step 2: Write failing tests for GraphemeCell**

Create `src/core/grapheme_cell.rs` with only the tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_char_has_width_1() {
        let cell = GraphemeCell::from_char('A');
        assert_eq!(cell.width, 1);
        assert_eq!(cell.grapheme(), "A");
    }

    #[test]
    fn wide_char_has_width_2() {
        let cell = GraphemeCell::from_char('日');
        assert_eq!(cell.width, 2);
        assert_eq!(cell.grapheme(), "日");
    }

    #[test]
    fn spacer_cell_has_width_0() {
        let cell = GraphemeCell::spacer();
        assert_eq!(cell.width, 0);
    }

    #[test]
    fn default_cell_is_space_width_1() {
        let cell = GraphemeCell::default();
        assert_eq!(cell.grapheme(), " ");
        assert_eq!(cell.width, 1);
    }

    #[test]
    fn is_default_detects_empty_cell() {
        let cell = GraphemeCell::default();
        assert!(cell.is_default());

        let mut styled = GraphemeCell::default();
        styled.bold = true;
        assert!(!styled.is_default());
    }

    #[test]
    fn grapheme_cluster_stores_correctly() {
        // Family emoji: man+woman+girl (ZWJ sequence, >8 bytes UTF-8)
        let family = "👨‍👩‍👧";
        let cell = GraphemeCell::from_str(family);
        assert_eq!(cell.grapheme(), family);
        assert_eq!(cell.width, 2);
    }

    #[test]
    fn from_char_emoji_is_wide() {
        let cell = GraphemeCell::from_char('🚀');
        assert_eq!(cell.width, 2);
    }
}
```

**Step 3: Run test to confirm it fails**

```bash
cargo test grapheme_cell 2>&1 | head -20
```
Expected: compile error — `GraphemeCell` not defined.

**Step 4: Implement `GraphemeCell`**

Replace the test-only file with the full implementation:

```rust
use crate::core::{Color, UnderlineStyle};
use unicode_width::UnicodeWidthChar;

/// Storage for a grapheme cluster — inline (≤8 UTF-8 bytes) or heap (rare ZWJ sequences).
#[derive(Clone, PartialEq, Eq, Debug)]
enum GraphemeStr {
    /// Inline: ASCII or short Unicode (covers CJK, most emoji).
    Inline { bytes: [u8; 8], len: u8 },
    /// Heap: complex ZWJ sequences exceeding 8 bytes.
    Heap(Box<str>),
}

impl GraphemeStr {
    fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        if bytes.len() <= 8 {
            let mut buf = [0u8; 8];
            buf[..bytes.len()].copy_from_slice(bytes);
            Self::Inline { bytes: buf, len: bytes.len() as u8 }
        } else {
            Self::Heap(s.into())
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Inline { bytes, len } => {
                // SAFETY: constructed only from valid UTF-8 via from_str()
                unsafe { std::str::from_utf8_unchecked(&bytes[..*len as usize]) }
            }
            Self::Heap(s) => s,
        }
    }
}

impl Default for GraphemeStr {
    fn default() -> Self {
        // Space character: 0x20
        let mut bytes = [0u8; 8];
        bytes[0] = b' ';
        Self::Inline { bytes, len: 1 }
    }
}

/// A single terminal cell with grapheme cluster support and explicit display width.
///
/// `width = 1` — normal character (ASCII, most Unicode)
/// `width = 2` — wide character (CJK, many emoji); occupies two columns
/// `width = 0` — spacer cell (right half of a wide char); has no visible content
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GraphemeCell {
    grapheme: GraphemeStr,
    pub width: u8,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub reverse: bool,
    pub underline_style: UnderlineStyle,
    pub strikethrough: bool,
}

impl Default for GraphemeCell {
    fn default() -> Self {
        Self {
            grapheme: GraphemeStr::default(),
            width: 1,
            fg: Color::SENTINEL_FG,
            bg: Color::SENTINEL_BG,
            bold: false,
            dim: false,
            italic: false,
            reverse: false,
            underline_style: UnderlineStyle::None,
            strikethrough: false,
        }
    }
}

impl GraphemeCell {
    /// Create a cell from a single `char`, computing display width automatically.
    pub fn from_char(c: char) -> Self {
        let width = UnicodeWidthChar::width(c).unwrap_or(1).min(2) as u8;
        let mut s = [0u8; 4];
        let encoded = c.encode_utf8(&mut s);
        Self {
            grapheme: GraphemeStr::from_str(encoded),
            width,
            ..Self::default()
        }
    }

    /// Create a cell from a grapheme cluster string (may be multi-codepoint).
    pub fn from_str(s: &str) -> Self {
        use unicode_width::UnicodeWidthStr;
        let width = unicode_width::UnicodeWidthStr::width(s).min(2) as u8;
        Self {
            grapheme: GraphemeStr::from_str(s),
            width,
            ..Self::default()
        }
    }

    /// A spacer cell: the right column of a wide character. Width = 0.
    pub fn spacer() -> Self {
        let mut bytes = [0u8; 8];
        bytes[0] = b' ';
        Self {
            grapheme: GraphemeStr::Inline { bytes, len: 1 },
            width: 0,
            ..Self::default()
        }
    }

    /// Returns the grapheme cluster as a `&str`.
    pub fn grapheme(&self) -> &str {
        self.grapheme.as_str()
    }

    /// Returns true if this cell is visually and semantically identical to the default.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    // ... (tests from Step 2)
}
```

**Step 5: Register in `src/core/mod.rs`**

Add alongside existing exports (do NOT remove `Cell` yet):
```rust
mod grapheme_cell;
pub use grapheme_cell::GraphemeCell;
```

**Step 6: Run tests**

```bash
cargo test grapheme_cell -- --nocapture
```
Expected: all 7 tests pass.

**Step 7: Verify no clippy warnings**

```bash
cargo clippy 2>&1 | grep -E "^error|^warning"
```
Expected: zero errors, zero warnings.

**Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock src/core/grapheme_cell.rs src/core/mod.rs
git commit -m "feat(core): add GraphemeCell with grapheme cluster and wide char support"
```

---

## Task 2: `Page` + `Row` types

**Files:**
- Create: `src/core/page.rs`

**Step 1: Write failing tests**

Create `src/core/page.rs` with tests only:

```rust
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
}
```

**Step 2: Run to confirm failure**

```bash
cargo test page -- --nocapture 2>&1 | head -10
```
Expected: compile error.

**Step 3: Implement**

```rust
use crate::core::GraphemeCell;

/// Number of rows stored in a single Page.
pub const PAGE_SIZE: usize = 256;

/// A single row in a Page.
#[derive(Clone)]
pub struct PageRow {
    pub cells: Vec<GraphemeCell>,
    /// True if this row continues on the next row (soft-wrap).
    pub wrapped: bool,
}

impl PageRow {
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![GraphemeCell::default(); cols],
            wrapped: false,
        }
    }

    pub fn from_cells(cells: Vec<GraphemeCell>, wrapped: bool) -> Self {
        Self { cells, wrapped }
    }
}

/// A fixed-capacity block of terminal rows.
pub struct Page {
    rows: Box<[PageRow; PAGE_SIZE]>,
    pub len: usize,
    pub cols: usize,
}

impl Page {
    pub fn new(cols: usize) -> Box<Self> {
        // Allocate on heap directly to avoid stack overflow for large PAGE_SIZE.
        let rows: Box<[PageRow; PAGE_SIZE]> = (0..PAGE_SIZE)
            .map(|_| PageRow::new(cols))
            .collect::<Vec<_>>()
            .try_into()
            .expect("PAGE_SIZE rows");
        Box::new(Self { rows, len: 0, cols })
    }

    pub fn is_full(&self) -> bool {
        self.len >= PAGE_SIZE
    }

    pub fn push(&mut self, row: PageRow) {
        debug_assert!(!self.is_full(), "pushed to full Page");
        self.rows[self.len] = row;
        self.len += 1;
    }

    pub fn get(&self, idx: usize) -> &PageRow {
        &self.rows[idx]
    }

    pub fn get_mut(&mut self, idx: usize) -> &mut PageRow {
        &mut self.rows[idx]
    }
}
```

**Step 4: Register in `src/core/mod.rs`**

```rust
mod page;
pub use page::{Page, PageRow, PAGE_SIZE};
```

**Step 5: Run tests**

```bash
cargo test page -- --nocapture
```
Expected: all tests pass.

**Step 6: Clippy**

```bash
cargo clippy 2>&1 | grep -E "^error|^warning"
```

**Step 7: Commit**

```bash
git add src/core/page.rs src/core/mod.rs
git commit -m "feat(core): add Page and PageRow fixed-size block types"
```

---

## Task 3: `PageList` — unified scrollback + viewport storage

**Files:**
- Create: `src/core/page_list.rs`

This is the core data structure. It holds all terminal content as a sequence of Pages.
The "viewport" is the last `rows` rows of the total content.

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::GraphemeCell;

    fn make_list(rows: usize, cols: usize) -> PageList {
        PageList::new(rows, cols, 1000)
    }

    #[test]
    fn new_list_has_empty_viewport() {
        let list = make_list(24, 80);
        assert_eq!(list.viewport_rows(), 24);
        assert_eq!(list.cols(), 80);
        // All cells default
        assert!(list.viewport_get(0, 0).is_default());
    }

    #[test]
    fn viewport_set_and_get() {
        let mut list = make_list(24, 80);
        let cell = GraphemeCell::from_char('X');
        list.viewport_set(0, 0, cell.clone());
        assert_eq!(list.viewport_get(0, 0).grapheme(), "X");
    }

    #[test]
    fn scroll_up_moves_row_to_scrollback() {
        let mut list = make_list(3, 5);
        let mut row = list.make_row();
        row.cells[0] = GraphemeCell::from_char('A');
        list.viewport_set_row(0, row);
        list.scroll_up(1); // push row 0 to scrollback
        // Scrollback now has 1 row
        assert_eq!(list.scrollback_len(), 1);
        // Row 0 in viewport should be fresh
        assert!(list.viewport_get(0, 0).is_default());
    }

    #[test]
    fn scrollback_respects_max_rows() {
        let mut list = PageList::new(2, 5, 3); // max 3 scrollback rows
        for _ in 0..5 {
            list.scroll_up(1);
        }
        assert!(list.scrollback_len() <= 3);
    }

    #[test]
    fn total_row_count_is_scrollback_plus_viewport() {
        let mut list = PageList::new(3, 5, 100);
        list.scroll_up(2);
        assert_eq!(list.total_rows(), 2 + 3);
    }
}
```

**Step 2: Run to confirm failure**

```bash
cargo test page_list -- --nocapture 2>&1 | head -10
```

**Step 3: Implement `PageList`**

```rust
use crate::core::{Page, PageRow, PAGE_SIZE, GraphemeCell};

/// Unified terminal buffer: scrollback + visible area in a single page list.
///
/// Content is stored as a sequence of Pages. The last `viewport_rows` rows
/// form the visible screen. Everything before that is scrollback.
pub struct PageList {
    /// All pages. The last one may be partially filled.
    pages: Vec<Box<Page>>,
    /// Free-list: indices of pages that were evicted and can be reused.
    free_list: Vec<usize>,
    /// Total number of scrollback rows (excluding viewport).
    scrollback_count: usize,
    /// Number of visible rows (terminal height).
    viewport_rows: usize,
    /// Terminal width (columns).
    cols: usize,
    /// Maximum scrollback rows allowed.
    max_scrollback: usize,
}

impl PageList {
    pub fn new(viewport_rows: usize, cols: usize, max_scrollback: usize) -> Self {
        // Pre-allocate enough pages for the viewport.
        let needed_pages = viewport_rows.div_ceil(PAGE_SIZE).max(1);
        let mut pages: Vec<Box<Page>> = (0..needed_pages).map(|_| Page::new(cols)).collect();
        // Fill viewport with empty rows.
        let mut remaining = viewport_rows;
        for page in &mut pages {
            let fill = remaining.min(PAGE_SIZE);
            for _ in 0..fill {
                page.push(PageRow::new(cols));
            }
            remaining = remaining.saturating_sub(fill);
        }
        Self {
            pages,
            free_list: Vec::new(),
            scrollback_count: 0,
            viewport_rows,
            cols,
            max_scrollback,
        }
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

    /// Create a blank row for this terminal's column count.
    pub fn make_row(&self) -> PageRow {
        PageRow::new(self.cols)
    }

    /// Absolute row index → (page_idx, row_in_page).
    fn abs_to_page(&self, abs_row: usize) -> (usize, usize) {
        (abs_row / PAGE_SIZE, abs_row % PAGE_SIZE)
    }

    /// Absolute row index for the first viewport row.
    fn viewport_start_abs(&self) -> usize {
        self.scrollback_count
    }

    /// Get a cell in the viewport by viewport-relative (row, col).
    pub fn viewport_get(&self, row: usize, col: usize) -> &GraphemeCell {
        let abs = self.viewport_start_abs() + row;
        let (pi, ri) = self.abs_to_page(abs);
        &self.pages[pi].get(ri).cells[col]
    }

    /// Set a cell in the viewport by viewport-relative (row, col).
    pub fn viewport_set(&mut self, row: usize, col: usize, cell: GraphemeCell) {
        let abs = self.viewport_start_abs() + row;
        let (pi, ri) = self.abs_to_page(abs);
        self.pages[pi].get_mut(ri).cells[col] = cell;
    }

    /// Replace an entire viewport row.
    pub fn viewport_set_row(&mut self, row: usize, src: PageRow) {
        let abs = self.viewport_start_abs() + row;
        let (pi, ri) = self.abs_to_page(abs);
        *self.pages[pi].get_mut(ri) = src;
    }

    /// Get a viewport row reference.
    pub fn viewport_row(&self, row: usize) -> &PageRow {
        let abs = self.viewport_start_abs() + row;
        let (pi, ri) = self.abs_to_page(abs);
        self.pages[pi].get(ri)
    }

    /// Get a viewport row as mutable reference.
    pub fn viewport_row_mut(&mut self, row: usize) -> &mut PageRow {
        let abs = self.viewport_start_abs() + row;
        let (pi, ri) = self.abs_to_page(abs);
        self.pages[pi].get_mut(ri)
    }

    /// Get a scrollback row by scrollback-relative index (0 = oldest).
    pub fn scrollback_row(&self, idx: usize) -> &PageRow {
        let (pi, ri) = self.abs_to_page(idx);
        self.pages[pi].get(ri)
    }

    /// Push the top N viewport rows to scrollback, evicting oldest if over limit.
    pub fn scroll_up(&mut self, count: usize) {
        let count = count.min(self.viewport_rows);
        // Evict oldest scrollback rows if we'd exceed max_scrollback.
        let would_be = self.scrollback_count + count;
        if would_be > self.max_scrollback {
            let evict = would_be - self.max_scrollback;
            self.evict_scrollback(evict);
        }
        // The viewport rows being scrolled off are already at the right absolute
        // positions — we just advance viewport_start by marking them as scrollback.
        self.scrollback_count += count;
        // Append new blank rows for the bottom of the viewport.
        for _ in 0..count {
            self.append_row(PageRow::new(self.cols));
        }
    }

    /// Append a raw row to the end of the page list (used for scroll_up and reflow).
    pub fn append_row(&mut self, row: PageRow) {
        if self.pages.last().map(|p| p.is_full()).unwrap_or(true) {
            self.alloc_page();
        }
        self.pages.last_mut().unwrap().push(row);
    }

    /// Allocate a new page (from free-list or fresh).
    fn alloc_page(&mut self) {
        // Reuse an evicted page if available (reset its content).
        // For now: always allocate fresh. Free-list optimization is additive.
        self.pages.push(Page::new(self.cols));
    }

    /// Evict the oldest `count` scrollback rows.
    fn evict_scrollback(&mut self, count: usize) {
        let count = count.min(self.scrollback_count);
        // Pages that are entirely evicted can be recycled.
        let full_pages = count / PAGE_SIZE;
        for _ in 0..full_pages {
            self.free_list.push(0); // simplified; real impl would track page indices
        }
        self.scrollback_count -= count;
        // Remove leading rows by rebasing: conceptually shift scrollback_start forward.
        // For correctness we track how many pages are "dead" at the start.
        // Simplified: just drain dead pages.
        let dead_pages = self.scrollback_count / PAGE_SIZE;
        if dead_pages > 0 && dead_pages < full_pages {
            self.pages.drain(0..dead_pages);
        }
    }

    /// Check if a viewport row is soft-wrapped.
    pub fn viewport_is_wrapped(&self, row: usize) -> bool {
        self.viewport_row(row).wrapped
    }

    /// Set soft-wrap flag on a viewport row.
    pub fn viewport_set_wrapped(&mut self, row: usize, wrapped: bool) {
        self.viewport_row_mut(row).wrapped = wrapped;
    }
}
```

**Step 4: Register in `src/core/mod.rs`**

```rust
mod page_list;
pub use page_list::PageList;
```

**Step 5: Run tests**

```bash
cargo test page_list -- --nocapture
```
Expected: all tests pass.

**Step 6: Clippy**

```bash
cargo clippy 2>&1 | grep -E "^error|^warning"
```

**Step 7: Commit**

```bash
git add src/core/page_list.rs src/core/mod.rs
git commit -m "feat(core): add PageList unified scrollback+viewport buffer"
```

---

## Task 4: `TrackedPin` — auto-updating cursor position

**Files:**
- Create: `src/core/tracked_pin.rs`
- Modify: `src/core/page_list.rs` (add pin registration + update on mutations)

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PageList;

    #[test]
    fn pin_tracks_viewport_position() {
        let mut list = PageList::new(24, 80, 1000);
        let pin = list.register_pin(PageCoord { abs_row: 23, col: 40 });
        assert_eq!(list.pin_coord(&pin).abs_row, 23);
        assert_eq!(list.pin_coord(&pin).col, 40);
    }

    #[test]
    fn scroll_up_shifts_pin_abs_row() {
        let mut list = PageList::new(24, 80, 1000);
        // Pin at viewport row 23 = abs_row 23
        let pin = list.register_pin(PageCoord { abs_row: 23, col: 0 });
        list.scroll_up(1);
        // abs_row stays 23 (scrollback grew by 1, viewport_start is now 1)
        // but viewport-relative row is now 22
        let coord = list.pin_coord(&pin);
        assert_eq!(coord.abs_row, 23);
    }

    #[test]
    fn pin_col_can_be_set() {
        let mut list = PageList::new(24, 80, 1000);
        let pin = list.register_pin(PageCoord { abs_row: 10, col: 5 });
        list.set_pin_col(&pin, 0);
        assert_eq!(list.pin_coord(&pin).col, 0);
    }
}
```

**Step 2: Run to confirm failure**

```bash
cargo test tracked_pin -- --nocapture 2>&1 | head -10
```

**Step 3: Implement**

Create `src/core/tracked_pin.rs`:

```rust
use std::sync::{Arc, Mutex};

/// Absolute position in PageList: absolute row index + column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PageCoord {
    pub abs_row: usize,
    pub col: usize,
}

/// A handle to a registered pin. The pin's coordinate is owned by PageList.
/// Clone the handle to share access; PageList updates coordinates on all clones.
#[derive(Clone, Debug)]
pub struct TrackedPin {
    inner: Arc<Mutex<PageCoord>>,
}

impl TrackedPin {
    pub(crate) fn new(coord: PageCoord) -> Self {
        Self { inner: Arc::new(Mutex::new(coord)) }
    }

    pub(crate) fn get(&self) -> PageCoord {
        *self.inner.lock().unwrap()
    }

    pub(crate) fn set(&self, coord: PageCoord) {
        *self.inner.lock().unwrap() = coord;
    }

    pub(crate) fn set_col(&self, col: usize) {
        self.inner.lock().unwrap().col = col;
    }

    pub(crate) fn set_abs_row(&self, abs_row: usize) {
        self.inner.lock().unwrap().abs_row = abs_row;
    }
}
```

Add pin management methods to `PageList` in `src/core/page_list.rs`:

```rust
use crate::core::tracked_pin::{PageCoord, TrackedPin};

// Inside PageList struct:
//   pins: Vec<TrackedPin>,

impl PageList {
    /// Register a new tracked pin at the given coordinate.
    /// Returns a handle; PageList updates the coordinate on reflow/scroll.
    pub fn register_pin(&mut self, coord: PageCoord) -> TrackedPin {
        let pin = TrackedPin::new(coord);
        self.pins.push(pin.clone());
        pin
    }

    /// Read the current coordinate of a pin.
    pub fn pin_coord(&self, pin: &TrackedPin) -> PageCoord {
        pin.get()
    }

    /// Update the column of a pin (e.g., reset to 0 after reflow).
    pub fn set_pin_col(&self, pin: &TrackedPin, col: usize) {
        pin.set_col(col);
    }
}
```

**Step 4: Register in `src/core/mod.rs`**

```rust
mod tracked_pin;
pub use tracked_pin::{PageCoord, TrackedPin};
```

**Step 5: Run tests**

```bash
cargo test tracked_pin -- --nocapture
cargo test page_list -- --nocapture
```
Expected: all pass.

**Step 6: Clippy**

```bash
cargo clippy 2>&1 | grep -E "^error|^warning"
```

**Step 7: Commit**

```bash
git add src/core/tracked_pin.rs src/core/page_list.rs src/core/mod.rs
git commit -m "feat(core): add TrackedPin and pin registration on PageList"
```

---

## Task 5: Grapheme-aware reflow in `PageList`

**Files:**
- Create: `src/core/page_list/reflow.rs` (or inline in `page_list.rs`)

**Step 1: Write failing tests**

Add to `src/core/page_list.rs` (or a new test file):

```rust
#[cfg(test)]
mod reflow_tests {
    use super::*;
    use crate::core::GraphemeCell;

    fn fill_row(list: &mut PageList, vrow: usize, text: &str) {
        for (col, ch) in text.chars().enumerate() {
            list.viewport_set(vrow, col, GraphemeCell::from_char(ch));
        }
    }

    #[test]
    fn reflow_wraps_long_line_to_narrower_width() {
        // 10-char line in 10-col terminal, reflow to 5 cols
        let mut list = PageList::new(5, 10, 100);
        fill_row(&mut list, 0, "ABCDEFGHIJ");
        list.viewport_set_wrapped(0, true); // force wrap continuation
        // Actually set second row as continuation for a 10-char logical line
        // Mark row 0 as wrapped so collect_logical_lines merges 0+1
        let pin = list.register_pin(PageCoord { abs_row: 0, col: 0 });
        list.reflow(5, 5, &pin);
        // After reflow: "ABCDE" on row 0, "FGHIJ" on row 1
        assert_eq!(list.viewport_rows(), 5);
        assert_eq!(list.viewport_get(0, 0).grapheme(), "A");
        assert_eq!(list.viewport_get(0, 4).grapheme(), "E");
        assert_eq!(list.viewport_get(1, 0).grapheme(), "F");
    }

    #[test]
    fn reflow_wide_char_not_split() {
        // Wide char at boundary must wrap to next line, not split
        let mut list = PageList::new(3, 4, 100);
        // Place wide char at col 3 (would need cols 3 and 4, but max is 4)
        // Logical line: "AB日" (A=1, B=1, 日=2 → total 4 cols, fits exactly)
        list.viewport_set(0, 0, GraphemeCell::from_char('A'));
        list.viewport_set(0, 1, GraphemeCell::from_char('B'));
        list.viewport_set(0, 2, GraphemeCell::from_char('日'));
        list.viewport_set(0, 3, GraphemeCell::spacer()); // right half of 日
        let pin = list.register_pin(PageCoord { abs_row: 0, col: 0 });
        list.reflow(3, 3, &pin); // reflow to 3 cols: "AB" + "日" on next row
        assert_eq!(list.viewport_get(0, 0).grapheme(), "A");
        assert_eq!(list.viewport_get(0, 1).grapheme(), "B");
        assert_eq!(list.viewport_get(1, 0).grapheme(), "日");
    }

    #[test]
    fn reflow_cursor_pin_col_is_zero_after_reflow() {
        let mut list = PageList::new(3, 10, 100);
        fill_row(&mut list, 0, "Hello");
        let pin = list.register_pin(PageCoord { abs_row: 0, col: 5 });
        list.reflow(3, 5, &pin);
        // SIGWINCH compat: col must be 0 after reflow
        assert_eq!(list.pin_coord(&pin).col, 0);
    }
}
```

**Step 2: Run to confirm failure**

```bash
cargo test reflow_tests -- --nocapture 2>&1 | head -20
```

**Step 3: Implement `reflow()` on `PageList`**

Add to `src/core/page_list.rs`:

```rust
impl PageList {
    /// Reflow all content (scrollback + viewport) to a new column width.
    ///
    /// After reflow, `cursor_pin.col` is set to 0 (SIGWINCH compatibility —
    /// readline/zsh redraws the full prompt from the cursor row on SIGWINCH).
    pub fn reflow(&mut self, new_rows: usize, new_cols: usize, cursor_pin: &TrackedPin) {
        // 1. Collect logical lines from all content.
        let (logical_lines, cursor_line_idx) = self.collect_logical_lines(cursor_pin);

        // 2. Rewrap to new column width.
        let (rewrapped, cursor_row_in_rewrapped) =
            rewrap_lines(&logical_lines, new_cols, cursor_line_idx);

        // 3. Rebuild PageList from rewrapped rows.
        self.rebuild_from_rows(rewrapped, new_rows, new_cols);

        // 4. Restore cursor pin — col=0 for SIGWINCH compat.
        if let Some(vrow) = cursor_row_in_rewrapped {
            let abs = self.viewport_start_abs() + vrow;
            cursor_pin.set(PageCoord { abs_row: abs, col: 0 });
        }
    }

    fn collect_logical_lines(&self, cursor_pin: &TrackedPin) -> (Vec<LogicalLine>, Option<usize>) {
        let cursor_abs = cursor_pin.get().abs_row;
        let mut lines: Vec<LogicalLine> = Vec::new();
        let mut current: Vec<GraphemeCell> = Vec::new();
        let mut cursor_line_idx: Option<usize> = None;
        let mut cursor_in_current = false;
        let mut current_min_len = 0usize;

        for abs_row in 0..self.total_rows() {
            let row = self.abs_row(abs_row);
            let line_start = current.len();
            current.extend_from_slice(&row.cells);
            if abs_row == cursor_abs {
                cursor_in_current = true;
                let col = cursor_pin.get().col;
                current_min_len = current_min_len.max(line_start + col);
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

    /// Get a row by absolute index (spans scrollback + viewport).
    fn abs_row(&self, abs: usize) -> &PageRow {
        let (pi, ri) = self.abs_to_page(abs);
        self.pages[pi].get(ri)
    }

    fn rebuild_from_rows(&mut self, rows: Vec<PageRow>, new_rows: usize, new_cols: usize) {
        let total = rows.len();
        let grid_offset = total.saturating_sub(new_rows);
        let scrollback_count = grid_offset.min(self.max_scrollback);
        let skip = grid_offset.saturating_sub(scrollback_count);

        // Reset.
        self.pages.clear();
        self.free_list.clear();
        self.scrollback_count = 0;
        self.viewport_rows = new_rows;
        self.cols = new_cols;

        // Append scrollback rows.
        for row in rows.iter().skip(skip).take(scrollback_count) {
            self.append_row(row.clone());
            self.scrollback_count += 1;
        }

        // Append viewport rows.
        let viewport_start = skip + scrollback_count;
        for row in rows.iter().skip(viewport_start) {
            self.append_row(row.clone());
        }
        // Pad viewport to new_rows if content was shorter.
        let actual_viewport = rows.len().saturating_sub(viewport_start);
        for _ in actual_viewport..new_rows {
            self.append_row(PageRow::new(new_cols));
        }
    }
}

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
            // Advance end until we'd exceed new_cols, respecting wide char boundaries.
            while end < content.len() {
                let w = content[end].width as usize;
                if w == 0 {
                    // Spacer: skip (belongs to previous wide char)
                    end += 1;
                    continue;
                }
                if col_count + w > new_cols {
                    break; // would overflow — wrap here
                }
                col_count += w;
                end += 1;
            }
            // If a wide char doesn't fit and terminal is >= 2 cols, skip the spacer.
            // If terminal < 2 cols, replace wide char with '?'.
            let mut cells: Vec<GraphemeCell> = Vec::with_capacity(new_cols);
            let mut placed_cols = 0usize;
            for cell in &content[pos..end] {
                if cell.width == 0 {
                    continue; // drop old spacers
                }
                if cell.width == 2 {
                    if placed_cols + 2 > new_cols {
                        // Wide char doesn't fit: replace with '?'
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
            pos = end;
            // Skip trailing spacers from previous iteration's wide char.
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
```

**Step 4: Run tests**

```bash
cargo test reflow -- --nocapture
cargo test page_list -- --nocapture
```
Expected: all pass.

**Step 5: Clippy**

```bash
cargo clippy 2>&1 | grep -E "^error|^warning"
```

**Step 6: Commit**

```bash
git add src/core/page_list.rs
git commit -m "feat(core): add grapheme-aware reflow to PageList"
```

---

## Task 6: Migrate `Terminal` to use `PageList` + `GraphemeCell`

This is the largest task. The current `Terminal` uses `Grid` + `VecDeque<Row>`.
We replace them with `PageList` and switch `cursor_row`/`cursor_col` to `TrackedPin`.

**Files:**
- Modify: `src/core/terminal.rs` (replace Grid + VecDeque with PageList, cursor with TrackedPin)
- Delete: `src/core/terminal/reflow.rs` (logic moves to PageList)
- Rewrite: `src/core/terminal/resize.rs` (delegate to PageList::reflow)
- Modify: `src/core/terminal/grid_ops.rs` (update to viewport API)
- Modify: all `src/core/terminal/handlers/*.rs` (grid.get → page_list.viewport_get, etc.)

**Step 1: Run existing terminal tests to establish baseline**

```bash
cargo test core_terminal -- --nocapture 2>&1 | tail -5
```
Note the exact number passing. All must pass after migration.

**Step 2: Update `Terminal` struct fields in `src/core/terminal.rs`**

Replace:
```rust
pub grid: Grid,
alt_grid: Option<Grid>,
pub cursor_row: usize,
pub cursor_col: usize,
saved_cursor: (usize, usize),
alt_saved_cursor: (usize, usize),
pub scrollback: VecDeque<Row>,
pub max_scrollback: usize,
scrollback_popped: usize,
```

With:
```rust
pub screen: PageList,
alt_screen: Option<PageList>,
pub cursor_pin: TrackedPin,
saved_cursor: PageCoord,
alt_saved_cursor: PageCoord,
pub max_scrollback: usize,
```

Add convenience accessors so handler code changes minimally:
```rust
impl Terminal {
    // These proxy to page_list so handlers need fewer changes.
    pub fn cols(&self) -> usize { self.screen.cols() }
    pub fn rows(&self) -> usize { self.screen.viewport_rows() }
    pub fn cursor_row(&self) -> usize {
        let abs = self.cursor_pin.get().abs_row;
        abs.saturating_sub(self.screen.scrollback_len())
    }
    pub fn cursor_col(&self) -> usize { self.cursor_pin.get().col }
    pub fn set_cursor(&mut self, row: usize, col: usize) {
        let abs = self.screen.scrollback_len() + row;
        self.cursor_pin.set(PageCoord { abs_row: abs, col });
    }
}
```

> **Note:** `cursor_row` and `cursor_col` were public fields. After migration they become
> methods. Update all call sites in handlers and tests — use `term.cursor_row()` and
> `term.cursor_col()` instead of direct field access.

**Step 3: Update `Terminal::new()` in `src/core/terminal.rs`**

```rust
pub fn new(rows: usize, cols: usize, max_scrollback: usize) -> Self {
    let mut screen = PageList::new(rows, cols, max_scrollback);
    let cursor_abs = screen.scrollback_len(); // row 0 of viewport
    let cursor_pin = screen.register_pin(PageCoord { abs_row: cursor_abs, col: 0 });
    Self {
        screen,
        alt_screen: None,
        cursor_pin,
        saved_cursor: PageCoord::default(),
        alt_saved_cursor: PageCoord::default(),
        max_scrollback,
        // ... rest of fields unchanged
    }
}
```

**Step 4: Update all handlers to use viewport API**

In every handler file, replace:
- `term.grid.get(term.cursor_row, col)` → `term.screen.viewport_get(term.cursor_row(), col)`
- `term.grid.set(term.cursor_row, col, cell)` → `term.screen.viewport_set(term.cursor_row(), col, cell)`
- `term.grid.cols` → `term.cols()`
- `term.grid.rows` → `term.rows()`
- `term.grid.is_wrapped(r)` → `term.screen.viewport_is_wrapped(r)`
- `term.grid.set_wrapped(r, w)` → `term.screen.viewport_set_wrapped(r, w)`
- `term.cursor_row` (field) → `term.cursor_row()` (method)
- `term.cursor_col` (field) → `term.cursor_col()` (method)
- `term.cursor_row = x` → `term.set_cursor(x, term.cursor_col())`
- `term.cursor_col = x` → `term.set_cursor(term.cursor_row(), x)`

Files to update: all `src/core/terminal/handlers/*.rs`, `src/core/terminal/grid_ops.rs`,
`src/core/terminal/alt_screen.rs`.

**Step 5: Rewrite `src/core/terminal/resize.rs`**

```rust
impl Terminal {
    pub fn resize(&mut self, rows: usize, cols: usize) {
        if self.rows() == rows && self.cols() == cols {
            return;
        }
        if let Some(ref mut alt) = self.alt_screen {
            // Alt screen: simple resize (no reflow).
            alt.simple_resize(rows, cols);
        }
        let old_cols = self.cols();
        if old_cols != cols && self.alt_screen.is_none() {
            self.screen.reflow(rows, cols, &self.cursor_pin);
        } else {
            self.screen.simple_resize(rows, cols);
            // Clamp cursor.
            let r = self.cursor_row().min(rows.saturating_sub(1));
            let c = self.cursor_col().min(cols.saturating_sub(1));
            self.set_cursor(r, c);
        }
        self.scroll_top = 0;
        self.scroll_bottom = rows - 1;
        self.resize_at = Some(std::time::Instant::now());
    }
}
```

Add `simple_resize()` to `PageList`:

```rust
impl PageList {
    /// Resize without reflow (used for alt screen and row-only changes).
    pub fn simple_resize(&mut self, new_rows: usize, new_cols: usize) {
        // Collect viewport rows, truncate or pad.
        let current_rows = self.viewport_rows;
        if new_rows < current_rows {
            // Shrink: discard bottom rows.
            self.viewport_rows = new_rows;
        } else {
            // Grow: add blank rows at bottom.
            for _ in current_rows..new_rows {
                self.append_row(PageRow::new(new_cols));
            }
            self.viewport_rows = new_rows;
        }
        self.cols = new_cols;
        // Truncate or extend all viewport rows to new_cols.
        for vrow in 0..self.viewport_rows {
            let abs = self.viewport_start_abs() + vrow;
            let (pi, ri) = self.abs_to_page(abs);
            let row = self.pages[pi].get_mut(ri);
            row.cells.resize(new_cols, GraphemeCell::default());
        }
    }
}
```

**Step 6: Delete old files**

```bash
rm src/core/terminal/reflow.rs
```

Remove `mod reflow;` from `src/core/terminal.rs`.

**Step 7: Run all tests**

```bash
cargo test -- --nocapture 2>&1 | tail -20
```
Expected: same number of tests passing as baseline (Step 1).

**Step 8: Clippy**

```bash
cargo clippy 2>&1 | grep -E "^error|^warning"
```

**Step 9: Commit**

```bash
git add src/core/terminal.rs src/core/terminal/ src/core/page_list.rs
git commit -m "feat(core): migrate Terminal to PageList and GraphemeCell"
```

---

## Task 7: Migrate renderer to read `GraphemeCell`

The renderer currently reads `cell.character: char` from `Grid`. After migration,
it reads `cell.grapheme(): &str` and must handle `width=2` (wide chars).

**Files:**
- Modify: `src/gui/events/render_shared.rs`
- Modify: `src/gui/renderer/gpu/frame.rs` (or wherever cells are drawn)
- Modify: `src/gui/renderer/cpu/mod.rs`

**Step 1: Run renderer tests / build to find compile errors**

```bash
cargo build 2>&1 | grep "^error" | head -20
```

Note every error location. Most will be `cell.character` → `cell.grapheme()` and
`cell.fg`/`cell.bg`/`cell.bold` etc. (these fields remain the same name on `GraphemeCell`).

**Step 2: Update each renderer access point**

Replace every occurrence of:
- `cell.character` → use `cell.grapheme()` which returns `&str`
- `term.grid.get(row, col)` → `term.screen.viewport_get(row, col)` (returns `&GraphemeCell`)
- `term.scrollback[i]` → `term.screen.scrollback_row(i)` (returns `&PageRow`)

For GPU renderer glyph lookup (currently uses a single `char` for atlas key):
- Change atlas key from `char` to `String` (or hash of grapheme bytes)
- For wide chars (`cell.width == 2`): render glyph spanning 2 columns; skip spacer cells
  (`cell.width == 0`).

**Step 3: Handle wide chars in layout**

In the cell-rendering loop, when `cell.width == 2`:
```rust
// Double the cell width for the glyph
let cell_width = if cell.width == 2 { metrics.cell_width * 2.0 } else { metrics.cell_width };
```

When `cell.width == 0`: skip rendering (spacer cell — covered by the preceding wide char).

**Step 4: Build and run**

```bash
cargo build 2>&1 | grep "^error"
cargo test -- --nocapture 2>&1 | tail -10
```

**Step 5: Clippy**

```bash
cargo clippy 2>&1 | grep -E "^error|^warning"
```

**Step 6: Commit**

```bash
git add src/gui/
git commit -m "feat(renderer): read GraphemeCell from PageList, handle wide chars"
```

---

## Task 8: Migrate `selection.rs` to use `PageCoord`

**Files:**
- Modify: `src/core/selection.rs`
- Modify: `src/gui/pane.rs` (selection usage)
- Modify: `src/gui/events/mouse/input.rs` (selection start/end)

**Step 1: Update `SelectionPoint` to use `PageCoord`**

`SelectionPoint` currently has `row: usize` (absolute) and `col: usize`.
`PageCoord` has `abs_row: usize` and `col: usize`. They are structurally identical.

Replace `SelectionPoint` with `PageCoord`:
- Remove `SelectionPoint` struct from `selection.rs`
- Replace `start: SelectionPoint` → `start: PageCoord`
- Update `Selection::contains(row, col)` to use `abs_row`

**Step 2: Update call sites**

```bash
cargo build 2>&1 | grep "SelectionPoint" | head -20
```
Fix each error by using `PageCoord { abs_row: ..., col: ... }`.

**Step 3: Register selection pins on Terminal**

In `Terminal`, add:
```rust
pub selection_start_pin: Option<TrackedPin>,
pub selection_end_pin: Option<TrackedPin>,
```

Selection start/end are set when user begins dragging:
```rust
pub fn set_selection_start(&mut self, vrow: usize, col: usize) {
    let abs = self.screen.scrollback_len() + vrow;
    let coord = PageCoord { abs_row: abs, col };
    let pin = self.screen.register_pin(coord);
    self.selection_start_pin = Some(pin);
}
```

**Step 4: Run tests**

```bash
cargo test -- --nocapture 2>&1 | tail -10
```

**Step 5: Clippy**

```bash
cargo clippy 2>&1 | grep -E "^error|^warning"
```

**Step 6: Commit**

```bash
git add src/core/selection.rs src/gui/
git commit -m "feat(core): replace SelectionPoint with PageCoord, register selection pins"
```

---

## Task 9: Settings — default 30 000 + UI stepper

**Files:**
- Modify: `src/config/model.rs`
- Modify: `src/gui/platform/macos/settings_window.rs`
- Modify: `src/gui/platform/linux/settings_window.rs`
- Modify: `src/gui/platform/windows/settings_window.rs`

**Step 1: Update `TerminalConfig` defaults and constants**

In `src/config/model.rs`, change:
```rust
pub struct TerminalConfig {
    pub max_scrollback: usize,
    pub cursor_blink_interval_ms: u64,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            max_scrollback: 30_000,   // was 1000
            cursor_blink_interval_ms: 500,
        }
    }
}

impl TerminalConfig {
    pub const SCROLLBACK_MIN: usize = 1_000;
    pub const SCROLLBACK_MAX: usize = 100_000;
    pub const SCROLLBACK_STEP: usize = 1_000;
    pub const BLINK_MS_MIN: u64 = 100;
    pub const BLINK_MS_MAX: u64 = 2000;
    pub const BLINK_MS_STEP: u64 = 50;
}
```

**Step 2: Update the config test**

In `src/config/model.rs` tests, change:
```rust
assert_eq!(deserialized.terminal.max_scrollback, 30_000);
// ... and other occurrences of 1000
```

Run:
```bash
cargo test config -- --nocapture
```
Expected: pass.

**Step 3: Add stepper to macOS settings window**

In `src/gui/platform/macos/settings_window.rs`, find the Terminal tab section (near
`cursor_blink_interval_ms` stepper). Add analogous stepper + text field for `max_scrollback`.

Follow the exact same pattern as `cursor_blink_interval_ms`:
- Add fields to `NativeSettingsState`: `scrollback_stepper`, `scrollback_field`
- Create `NSStepper` with min=1000, max=100000, increment=1000
- Create `NSTextField` displaying the value
- Wire the stepper action to update the field and send `AppConfig`

**Step 4: Add stepper to Linux and Windows settings windows**

Follow the same pattern in `src/gui/platform/linux/settings_window.rs` and
`src/gui/platform/windows/settings_window.rs`.

**Step 5: Update translation strings**

In `src/i18n/en.rs` and `src/i18n/uk.rs`, add:
```rust
scrollback_lines: "Scrollback lines",
// uk:
scrollback_lines: "Рядки прокрутки",
```

**Step 6: Run full test suite**

```bash
cargo test -- --nocapture 2>&1 | tail -10
cargo clippy 2>&1 | grep -E "^error|^warning"
```

**Step 7: Commit**

```bash
git add src/config/model.rs src/gui/platform/ src/i18n/
git commit -m "feat(config): set max_scrollback default to 30_000, add UI stepper"
```

---

## Task 10: Cleanup — remove old `Cell`, `Grid`, old files

**Files:**
- Delete: `src/core/cell.rs`
- Delete: `src/core/grid.rs`
- Modify: `src/core/mod.rs` (remove old exports)
- Update: all remaining references to old `Cell`/`Grid`/`Row` types

**Step 1: Find remaining references**

```bash
cargo build 2>&1 | grep -E "cannot find|unresolved" | head -30
```

Also:
```bash
grep -r "use crate::core::Cell\|use crate::core::Grid\|use crate::core::Row" src/
```

**Step 2: Fix each reference**

- `Cell` → `GraphemeCell`
- `Grid` → `PageList`
- `Row` (old) → `PageRow`
- `VecDeque<Row>` → removed (now inside `PageList`)

**Step 3: Remove old files**

```bash
rm src/core/cell.rs src/core/grid.rs
```

Update `src/core/mod.rs` — remove:
```rust
mod cell;
mod grid;
pub use cell::{Cell, UnderlineStyle};
pub use grid::{Grid, Row};
```

Keep: `pub use grapheme_cell::GraphemeCell;` and `pub use page::{Page, PageRow, PAGE_SIZE};`

**Step 4: Update `tests/unit/core_terminal.rs`**

The integration tests use `Cell`, `Grid`, scrollback indexing etc. Update to use
`GraphemeCell`, `PageList` APIs, and `terminal.screen.viewport_get()`.

Key pattern changes:
```rust
// Old
term.grid.get(row, col).unwrap().character
// New
term.screen.viewport_get(row, col).grapheme()

// Old
term.scrollback[i].cells[col].character
// New
term.screen.scrollback_row(i).cells[col].grapheme()

// Old
let cell = Cell { character: 'X', ..Cell::default() };
term.grid.set(row, col, cell);
// New
term.screen.viewport_set(row, col, GraphemeCell::from_char('X'));
```

**Step 5: Run full test suite**

```bash
cargo test -- --nocapture 2>&1 | tail -20
```
Expected: same number of tests as baseline (Task 6, Step 1).

**Step 6: Final clippy**

```bash
cargo clippy 2>&1 | grep -E "^error|^warning"
```
Expected: zero warnings.

**Step 7: Commit**

```bash
git add -A
git commit -m "chore(core): remove old Cell and Grid, complete PageList migration"
```

---

## Final Verification

Before opening a PR, run the full suite one more time:

```bash
cargo test -- --nocapture
cargo clippy
cargo build
cargo build --no-default-features
```

All must pass with zero warnings on both GPU and CPU builds.

```bash
git log --oneline feat/page-based-grapheme ^main
```

Review commit history for completeness.
