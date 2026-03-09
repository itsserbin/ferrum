use crate::core::Color;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Style of underline decoration on a terminal cell.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum UnderlineStyle {
    #[default]
    None,
    Single,
    Double,
}

/// Storage for a grapheme cluster — inline (≤8 UTF-8 bytes) or heap (rare ZWJ sequences).
#[derive(Clone, PartialEq, Eq, Debug)]
enum GraphemeStr {
    Inline { bytes: [u8; 8], len: u8 },
    Heap(Box<str>),
}

impl GraphemeStr {
    fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        if bytes.len() <= 8 {
            let mut buf = [0u8; 8];
            buf[..bytes.len()].copy_from_slice(bytes);
            GraphemeStr::Inline {
                bytes: buf,
                len: bytes.len() as u8,
            }
        } else {
            GraphemeStr::Heap(s.into())
        }
    }

    fn as_str(&self) -> &str {
        match self {
            GraphemeStr::Inline { bytes, len } => {
                // SAFETY: `GraphemeStr` is only ever constructed via `from_str()`,
                // which takes a `&str`. The bytes copied into the inline buffer are
                // therefore guaranteed to be valid UTF-8.
                unsafe { std::str::from_utf8_unchecked(&bytes[..*len as usize]) }
            }
            GraphemeStr::Heap(s) => s,
        }
    }
}

impl Default for GraphemeStr {
    fn default() -> Self {
        GraphemeStr::Inline {
            bytes: [b' ', 0, 0, 0, 0, 0, 0, 0],
            len: 1,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GraphemeCell {
    grapheme: GraphemeStr,
    pub width: u8,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub reverse: bool,
    pub underline_style: UnderlineStyle,
    pub strikethrough: bool,
    /// Index into the terminal's hyperlink URL table (0 = no link; n ≥ 1 = URL at index n-1).
    pub hyperlink_id: u16,
}

impl GraphemeCell {
    /// Returns the grapheme cluster stored in this cell as a `&str`.
    pub fn grapheme(&self) -> &str {
        self.grapheme.as_str()
    }

    /// A space cell with width 1 and sentinel colors — the terminal "empty" state.
    pub fn from_char(c: char) -> Self {
        let width = UnicodeWidthChar::width(c).unwrap_or(1).min(2) as u8;
        let mut buf = [0u8; 4];
        let encoded = c.encode_utf8(&mut buf);
        GraphemeCell {
            grapheme: GraphemeStr::from_str(encoded),
            width,
            ..Self::default()
        }
    }

    /// Creates a `GraphemeCell` from a grapheme cluster string (e.g. a ZWJ emoji sequence).
    pub fn from_str(s: &str) -> Self {
        let width = UnicodeWidthStr::width(s).min(2) as u8;
        GraphemeCell {
            grapheme: GraphemeStr::from_str(s),
            width,
            ..Self::default()
        }
    }

    /// Creates a spacer cell (width 0) used as the right half of a wide character.
    pub fn spacer() -> Self {
        GraphemeCell {
            width: 0,
            ..Self::default()
        }
    }

    /// Returns the first char of this cell's grapheme cluster, or `' '` if empty.
    pub fn first_char(&self) -> char {
        self.grapheme().chars().next().unwrap_or(' ')
    }

    /// Returns `true` if this cell is identical to `GraphemeCell::default()`.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl Default for GraphemeCell {
    fn default() -> Self {
        GraphemeCell {
            grapheme: GraphemeStr::default(),
            width: 1,
            fg: Color::SENTINEL_FG,
            bg: Color::SENTINEL_BG,
            bold: false,
            dim: false,
            italic: false,
            reverse: false,
            underline_style: UnderlineStyle::None,
            strikethrough: false,
            hyperlink_id: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_char_has_width_1() {
        let cell = GraphemeCell::from_char('A');
        assert_eq!(cell.width, 1);
        assert_eq!(cell.grapheme(), "A");
    }

    #[test]
    fn wide_char_has_width_2() {
        let cell = GraphemeCell::from_char('日');
        assert_eq!(cell.width, 2);
        assert_eq!(cell.grapheme(), "日");
    }

    #[test]
    fn spacer_cell_has_width_0() {
        let cell = GraphemeCell::spacer();
        assert_eq!(cell.width, 0);
    }

    #[test]
    fn default_cell_is_space_width_1() {
        let cell = GraphemeCell::default();
        assert_eq!(cell.grapheme(), " ");
        assert_eq!(cell.width, 1);
    }

    #[test]
    fn is_default_detects_empty_cell() {
        let cell = GraphemeCell::default();
        assert!(cell.is_default());
        let styled = GraphemeCell { bold: true, ..GraphemeCell::default() };
        assert!(!styled.is_default());
    }

    #[test]
    fn from_char_emoji_is_wide() {
        let cell = GraphemeCell::from_char('🚀');
        assert_eq!(cell.width, 2);
    }

    #[test]
    fn grapheme_cluster_stores_correctly() {
        let family = "👨‍👩‍👧";
        let cell = GraphemeCell::from_str(family);
        assert_eq!(cell.grapheme(), family);
        assert_eq!(cell.width, 2);
    }
}
