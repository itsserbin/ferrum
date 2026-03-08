use crate::core::PageCoord;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Selection {
    pub start: PageCoord,
    pub end: PageCoord,
}

impl Selection {
    /// Normalize — start is always before end
    pub fn normalized(&self) -> (PageCoord, PageCoord) {
        if self.start.abs_row < self.end.abs_row
            || (self.start.abs_row == self.end.abs_row && self.start.col <= self.end.col)
        {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    /// Check if a cell falls within the selection (absolute coords)
    pub fn contains(&self, row: usize, col: usize) -> bool {
        let (start, end) = self.normalized();
        if row < start.abs_row || row > end.abs_row {
            return false;
        }
        if row == start.abs_row && row == end.abs_row {
            return col >= start.col && col <= end.col;
        }
        if row == start.abs_row {
            return col >= start.col;
        }
        if row == end.abs_row {
            return col <= end.col;
        }
        true
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_single_row() {
        let sel = Selection {
            start: PageCoord { abs_row: 5, col: 2 },
            end: PageCoord { abs_row: 5, col: 7 },
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
            start: PageCoord { abs_row: 10, col: 3 },
            end: PageCoord { abs_row: 12, col: 5 },
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
            start: PageCoord { abs_row: 2, col: 3 },
            end: PageCoord { abs_row: 5, col: 7 },
        };
        let (s, e) = sel.normalized();
        assert_eq!(s, sel.start);
        assert_eq!(e, sel.end);
    }

    #[test]
    fn normalized_reversed() {
        let sel = Selection {
            start: PageCoord { abs_row: 5, col: 7 },
            end: PageCoord { abs_row: 2, col: 3 },
        };
        let (s, e) = sel.normalized();
        assert_eq!(s.abs_row, 2);
        assert_eq!(s.col, 3);
        assert_eq!(e.abs_row, 5);
        assert_eq!(e.col, 7);
    }

}
