use crate::core::Color;

/// Style of underline decoration on a terminal cell.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum UnderlineStyle {
    #[default]
    None,
    Single,
    Double,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cell {
    pub character: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub reverse: bool,
    pub underline_style: UnderlineStyle,
    pub strikethrough: bool,
}

impl Cell {
    /// A default cell constant for use with `unwrap_or` when `Grid::get()` returns `None`.
    pub const DEFAULT: Cell = Cell {
        character: ' ',
        fg: Color::SENTINEL_FG,
        bg: Color::SENTINEL_BG,
        bold: false,
        dim: false,
        italic: false,
        reverse: false,
        underline_style: UnderlineStyle::None,
        strikethrough: false,
    };
}

impl Default for Cell {
    fn default() -> Self {
        Cell::DEFAULT
    }
}
