use crate::core::Position;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Selection {
    pub start: Position,
    pub end: Position,
}

impl Selection {
    /// Normalize — start is always before end
    pub fn normalized(&self) -> (Position, Position) {
        if self.start.row < self.end.row
            || (self.start.row == self.end.row && self.start.col <= self.end.col)
        {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    /// Check if a cell falls within the selection
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
        true // rows between start and end are fully selected
    }
}
