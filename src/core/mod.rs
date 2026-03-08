mod color;
mod grapheme_cell;
mod page;
mod page_list;
mod position;
mod security;
mod selection;
mod tracked_pin;

pub mod terminal;

pub use color::Color;
pub use grapheme_cell::{GraphemeCell, UnderlineStyle};
pub use page::{Page, PageRow, PAGE_SIZE};
pub use page_list::PageList;
pub use position::Position;
pub use security::{SecurityConfig, SecurityEventKind, SecurityGuard};
pub use selection::Selection;
pub use terminal::{CursorStyle, MouseMode};
pub use tracked_pin::{PageCoord, TrackedPin};

#[cfg(test)]
mod tests {
    use super::{Page, PageCoord, PageRow, TrackedPin, PAGE_SIZE};

    #[test]
    fn page_types_are_accessible_via_core() {
        let mut page = Page::new();
        page.push(PageRow::new(4));
        assert!(!page.is_full());
        assert_eq!(PAGE_SIZE, 256);
    }

    #[test]
    fn tracked_pin_types_are_accessible_via_core() {
        let coord = PageCoord {
            abs_row: 1,
            col: 2,
        };
        let pin = TrackedPin::new(coord);
        assert_eq!(pin.coord(), coord);
    }
}
