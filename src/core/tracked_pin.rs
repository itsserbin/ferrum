use std::cell::Cell;
use std::rc::Rc;

/// A coordinate in the `PageList` address space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PageCoord {
    pub abs_row: usize,
    pub col: usize,
}

/// A tracked handle to a `PageCoord` inside a `PageList`.
///
/// Cloning a `TrackedPin` shares the same underlying coordinate — when
/// `PageList` updates the pin, all clones see the change.
///
/// `Terminal` is single-threaded; `Rc<Cell<>>` avoids the locking overhead of
/// `Arc<Mutex<>>` on every cursor movement.
#[derive(Clone, Debug)]
pub struct TrackedPin {
    inner: Rc<Cell<PageCoord>>,
}

impl TrackedPin {
    pub(crate) fn new(coord: PageCoord) -> Self {
        Self {
            inner: Rc::new(Cell::new(coord)),
        }
    }

    pub fn coord(&self) -> PageCoord {
        self.inner.get()
    }

    pub fn set_coord(&self, coord: PageCoord) {
        self.inner.set(coord);
    }

    pub fn set_col(&self, col: usize) {
        self.inner.set(PageCoord { col, ..self.inner.get() });
    }

    pub fn set_abs_row(&self, abs_row: usize) {
        self.inner.set(PageCoord { abs_row, ..self.inner.get() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PageList;

    #[test]
    fn pin_tracks_viewport_position() {
        let list = PageList::new(24, 80, 1000);
        let abs = list.viewport_start_abs();
        let pin = PageList::pin_at(PageCoord {
            abs_row: abs + 23,
            col: 40,
        });
        assert_eq!(pin.coord().abs_row, abs + 23);
        assert_eq!(pin.coord().col, 40);
    }

    #[test]
    fn pin_clone_shares_coordinate() {
        let list = PageList::new(24, 80, 1000);
        let pin = PageList::pin_at(PageCoord { abs_row: 0, col: 5 });
        let pin_clone = pin.clone();
        pin.set_col(0);
        // The clone sees the same update.
        assert_eq!(pin_clone.coord().col, 0);
    }

    #[test]
    fn pin_col_can_be_reset() {
        let list = PageList::new(24, 80, 1000);
        let pin = PageList::pin_at(PageCoord {
            abs_row: 10,
            col: 5,
        });
        pin.set_col(0);
        assert_eq!(pin.coord().col, 0);
    }

    #[test]
    fn multiple_pins_are_independent() {
        let list = PageList::new(24, 80, 1000);
        let pin_a = PageList::pin_at(PageCoord {
            abs_row: 1,
            col: 10,
        });
        let pin_b = PageList::pin_at(PageCoord {
            abs_row: 2,
            col: 20,
        });
        pin_a.set_col(0);
        // pin_b is unaffected.
        assert_eq!(pin_b.coord().col, 20);
    }
}
