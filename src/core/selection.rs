#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SelectionPoint {
    pub row: usize, // absolute row in buffer (scrollback + grid)
    pub col: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Selection {
    pub start: SelectionPoint,
    pub end: SelectionPoint,
}

impl Selection {
    /// Normalize — start is always before end
    pub fn normalized(&self) -> (SelectionPoint, SelectionPoint) {
        if self.start.row < self.end.row
            || (self.start.row == self.end.row && self.start.col <= self.end.col)
        {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    /// Check if a cell falls within the selection (absolute coords)
    pub fn contains(&self, row: usize, col: usize) -> bool {
        let (start, end) = self.normalized();
        if row < start.row || row > end.row {
            return false;
        }
        if row == start.row && row == end.row {
            return col >= start.col && col <= end.col;
        }
        if row == start.row {
            return col >= start.col;
        }
        if row == end.row {
            return col <= end.col;
        }
        true
    }

    /// Adjust selection after scrollback overflow: decrement rows by popped count.
    /// Returns None if selection is entirely invalidated.
    pub fn adjust_for_scrollback_pop(&self, popped: usize) -> Option<Selection> {
        if popped == 0 {
            return Some(*self);
        }
        if self.start.row < popped && self.end.row < popped {
            return None;
        }
        let (_norm_start, norm_end) = self.normalized();
        if norm_end.row < popped {
            return None;
        }
        Some(Selection {
            start: SelectionPoint {
                row: self.start.row.saturating_sub(popped),
                col: self.start.col,
            },
            end: SelectionPoint {
                row: self.end.row.saturating_sub(popped),
                col: self.end.col,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_single_row() {
        let sel = Selection {
            start: SelectionPoint { row: 5, col: 2 },
            end: SelectionPoint { row: 5, col: 7 },
        };
        assert!(sel.contains(5, 2));
        assert!(sel.contains(5, 4));
        assert!(sel.contains(5, 7));
        assert!(!sel.contains(5, 1));
        assert!(!sel.contains(5, 8));
        assert!(!sel.contains(4, 4));
        assert!(!sel.contains(6, 4));
    }

    #[test]
    fn contains_multi_row() {
        let sel = Selection {
            start: SelectionPoint { row: 10, col: 3 },
            end: SelectionPoint { row: 12, col: 5 },
        };
        assert!(sel.contains(10, 3));
        assert!(sel.contains(10, 79));
        assert!(!sel.contains(10, 2));
        assert!(sel.contains(11, 0));
        assert!(sel.contains(11, 79));
        assert!(sel.contains(12, 0));
        assert!(sel.contains(12, 5));
        assert!(!sel.contains(12, 6));
    }

    #[test]
    fn normalized_already_ordered() {
        let sel = Selection {
            start: SelectionPoint { row: 2, col: 3 },
            end: SelectionPoint { row: 5, col: 7 },
        };
        let (s, e) = sel.normalized();
        assert_eq!(s, sel.start);
        assert_eq!(e, sel.end);
    }

    #[test]
    fn normalized_reversed() {
        let sel = Selection {
            start: SelectionPoint { row: 5, col: 7 },
            end: SelectionPoint { row: 2, col: 3 },
        };
        let (s, e) = sel.normalized();
        assert_eq!(s.row, 2);
        assert_eq!(s.col, 3);
        assert_eq!(e.row, 5);
        assert_eq!(e.col, 7);
    }

    #[test]
    fn adjust_for_scrollback_pop_zero() {
        let sel = Selection {
            start: SelectionPoint { row: 10, col: 0 },
            end: SelectionPoint { row: 15, col: 5 },
        };
        let adjusted = sel.adjust_for_scrollback_pop(0).unwrap();
        assert_eq!(adjusted, sel);
    }

    #[test]
    fn adjust_for_scrollback_pop_partial() {
        let sel = Selection {
            start: SelectionPoint { row: 10, col: 0 },
            end: SelectionPoint { row: 15, col: 5 },
        };
        let adjusted = sel.adjust_for_scrollback_pop(1).unwrap();
        assert_eq!(adjusted.start.row, 9);
        assert_eq!(adjusted.end.row, 14);
    }

    #[test]
    fn adjust_for_scrollback_pop_invalidates() {
        let sel = Selection {
            start: SelectionPoint { row: 0, col: 0 },
            end: SelectionPoint { row: 0, col: 5 },
        };
        assert!(sel.adjust_for_scrollback_pop(1).is_none());
    }

    #[test]
    fn adjust_for_scrollback_pop_start_underflows() {
        let sel = Selection {
            start: SelectionPoint { row: 0, col: 0 },
            end: SelectionPoint { row: 5, col: 5 },
        };
        let adjusted = sel.adjust_for_scrollback_pop(2).unwrap();
        assert_eq!(adjusted.start.row, 0);
        assert_eq!(adjusted.start.col, 0);
        assert_eq!(adjusted.end.row, 3);
    }
}
