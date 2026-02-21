use crate::core::Color;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cell {
    pub character: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub reverse: bool,
    pub underline: bool,
}

impl Cell {
    /// A default cell constant for use with `unwrap_or` when `Grid::get()` returns `None`.
    pub const DEFAULT: Cell = Cell {
        character: ' ',
        fg: Color::SENTINEL_FG,
        bg: Color::SENTINEL_BG,
        bold: false,
        reverse: false,
        underline: false,
    };
}

impl Default for Cell {
    fn default() -> Self {
        Cell::DEFAULT
    }
}
