mod cell;
mod color;
mod grapheme_cell;
mod grid;
mod page;
mod page_list;
mod position;
mod security;
mod selection;

pub mod terminal;

pub use cell::{Cell, UnderlineStyle};
pub use color::Color;
pub use grapheme_cell::GraphemeCell;
pub use grid::{Grid, Row};
pub use page::{Page, PageRow, PAGE_SIZE};
pub use page_list::PageList;
pub use position::Position;
pub use security::{SecurityConfig, SecurityEventKind, SecurityGuard};
pub use selection::{Selection, SelectionPoint};
pub use terminal::{CursorStyle, MouseMode};

#[cfg(test)]
mod tests {
    use super::{Page, PageRow, PAGE_SIZE};

    #[test]
    fn page_types_are_accessible_via_core() {
        let mut page = Page::new(4);
        page.push(PageRow::new(4));
        assert_eq!(page.len, 1);
        assert_eq!(PAGE_SIZE, 256);
    }
}
