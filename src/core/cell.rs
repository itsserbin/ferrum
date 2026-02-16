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

impl Default for Cell {
    fn default() -> Self {
        Cell {
            character: ' ',
            fg: Color::DEFAULT_FG,
            bg: Color::DEFAULT_BG,
            bold: false,
            reverse: false,
            underline: false,
        }
    }
}
