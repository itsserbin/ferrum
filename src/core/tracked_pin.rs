use std::sync::{Arc, Mutex};

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
#[derive(Clone, Debug)]
pub struct TrackedPin {
    inner: Arc<Mutex<PageCoord>>,
}

impl TrackedPin {
    pub(crate) fn new(coord: PageCoord) -> Self {
        Self {
            inner: Arc::new(Mutex::new(coord)),
        }
    }

    pub fn coord(&self) -> PageCoord {
        *self.inner.lock().expect("TrackedPin lock not poisoned")
    }

    pub fn set_coord(&self, coord: PageCoord) {
        *self.inner.lock().expect("TrackedPin lock not poisoned") = coord;
    }

    pub fn set_col(&self, col: usize) {
        self.inner
            .lock()
            .expect("TrackedPin lock not poisoned")
            .col = col;
    }

    pub fn set_abs_row(&self, abs_row: usize) {
        self.inner
            .lock()
            .expect("TrackedPin lock not poisoned")
            .abs_row = abs_row;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PageList;

    #[test]
    fn pin_tracks_viewport_position() {
        let mut list = PageList::new(24, 80, 1000);
        let abs = list.viewport_start_abs();
        let pin = list.register_pin(PageCoord {
            abs_row: abs + 23,
            col: 40,
        });
        assert_eq!(pin.coord().abs_row, abs + 23);
        assert_eq!(pin.coord().col, 40);
    }

    #[test]
    fn pin_clone_shares_coordinate() {
        let mut list = PageList::new(24, 80, 1000);
        let pin = list.register_pin(PageCoord { abs_row: 0, col: 5 });
        let pin_clone = pin.clone();
        list.set_pin_col(&pin, 0);
        // The clone sees the same update.
        assert_eq!(pin_clone.coord().col, 0);
    }

    #[test]
    fn pin_col_can_be_reset() {
        let mut list = PageList::new(24, 80, 1000);
        let pin = list.register_pin(PageCoord {
            abs_row: 10,
            col: 5,
        });
        list.set_pin_col(&pin, 0);
        assert_eq!(pin.coord().col, 0);
    }

    #[test]
    fn multiple_pins_are_independent() {
        let mut list = PageList::new(24, 80, 1000);
        let pin_a = list.register_pin(PageCoord {
            abs_row: 1,
            col: 10,
        });
        let pin_b = list.register_pin(PageCoord {
            abs_row: 2,
            col: 20,
        });
        list.set_pin_col(&pin_a, 0);
        // pin_b is unaffected.
        assert_eq!(pin_b.coord().col, 20);
    }
}
