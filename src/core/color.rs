#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const DEFAULT_FG: Color = Color {
        r: 210,
        g: 219,
        b: 235,
    }; // #D2DBEB
    pub const DEFAULT_BG: Color = Color {
        r: 40,
        g: 44,
        b: 52,
    }; // #282C34

    // Custom dark palette (ghostty-like vibe, not a 1:1 copy).
    pub const ANSI: [Color; 16] = [
        Color {
            r: 69,
            g: 75,
            b: 89,
        }, //  0 black    #454B59
        Color {
            r: 224,
            g: 108,
            b: 117,
        }, //  1 red      #E06C75
        Color {
            r: 152,
            g: 195,
            b: 121,
        }, //  2 green    #98C379
        Color {
            r: 229,
            g: 192,
            b: 123,
        }, //  3 yellow   #E5C07B
        Color {
            r: 97,
            g: 175,
            b: 239,
        }, //  4 blue     #61AFEF
        Color {
            r: 198,
            g: 120,
            b: 221,
        }, //  5 magenta  #C678DD
        Color {
            r: 86,
            g: 182,
            b: 194,
        }, //  6 cyan     #56B6C2
        Color {
            r: 171,
            g: 178,
            b: 191,
        }, //  7 white    #ABB2BF
        Color {
            r: 106,
            g: 114,
            b: 130,
        }, //  8 br black #6A7282
        Color {
            r: 240,
            g: 139,
            b: 149,
        }, //  9 br red   #F08B95
        Color {
            r: 167,
            g: 213,
            b: 141,
        }, // 10 br green #A7D58D
        Color {
            r: 235,
            g: 203,
            b: 139,
        }, // 11 br yello #EBCB8B
        Color {
            r: 123,
            g: 195,
            b: 255,
        }, // 12 br blue  #7BC3FF
        Color {
            r: 215,
            g: 153,
            b: 240,
        }, // 13 br magen #D799F0
        Color {
            r: 111,
            g: 199,
            b: 211,
        }, // 14 br cyan  #6FC7D3
        Color {
            r: 221,
            g: 229,
            b: 245,
        }, // 15 br white #DDE5F5
    ];

    pub const fn to_pixel(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Constructs a Color from a 0xRRGGBB u32 pixel value.
    pub const fn from_pixel(pixel: u32) -> Color {
        Color {
            r: ((pixel >> 16) & 0xFF) as u8,
            g: ((pixel >> 8) & 0xFF) as u8,
            b: (pixel & 0xFF) as u8,
        }
    }

    /// If this color matches one of the base 8 ANSI colors (indices 0-7),
    /// returns the corresponding bright variant (indices 8-15).
    /// Otherwise returns `self` unchanged.  Used to implement the
    /// bold-implies-bright convention for terminal emulators.
    pub fn bold_bright(self) -> Color {
        for i in 0..8 {
            if self.r == Self::ANSI[i].r && self.g == Self::ANSI[i].g && self.b == Self::ANSI[i].b {
                return Self::ANSI[i + 8];
            }
        }
        self
    }

    /// 256-color palette: 0-15 = ANSI, 16-231 = 6x6x6 color cube, 232-255 = grayscale
    pub fn from_256(n: u16) -> Color {
        match n {
            0..=15 => Color::ANSI[n as usize],
            16..=231 => {
                let n = n - 16;
                let r = (n / 36) as u8;
                let g = ((n % 36) / 6) as u8;
                let b = (n % 6) as u8;
                Color {
                    r: if r > 0 { 55 + r * 40 } else { 0 },
                    g: if g > 0 { 55 + g * 40 } else { 0 },
                    b: if b > 0 { 55 + b * 40 } else { 0 },
                }
            }
            232..=255 => {
                let v = (8 + (n - 232) * 10) as u8;
                Color { r: v, g: v, b: v }
            }
            _ => Color::DEFAULT_FG,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_pixel_roundtrip() {
        let color = Color {
            r: 0xAB,
            g: 0xCD,
            b: 0xEF,
        };
        assert_eq!(color.to_pixel(), 0xABCDEF);
        assert_eq!(Color::from_pixel(0xABCDEF), color);
    }

    #[test]
    fn from_256_ansi_range() {
        for i in 0..16u16 {
            assert_eq!(Color::from_256(i), Color::ANSI[i as usize]);
        }
    }

    #[test]
    fn from_256_color_cube() {
        // Index 16 = first color cube entry = rgb(0,0,0)
        assert_eq!(Color::from_256(16), Color { r: 0, g: 0, b: 0 });
        // Index 196: n-16=180, r=180/36=5, g=(180%36)/6=0, b=180%6=0
        // r=5 -> 55+5*40=255, g=0, b=0
        assert_eq!(Color::from_256(196), Color { r: 255, g: 0, b: 0 });
    }

    #[test]
    fn from_256_grayscale() {
        // Index 232: v = 8 + (232-232)*10 = 8
        assert_eq!(Color::from_256(232), Color { r: 8, g: 8, b: 8 });
        // Index 255: v = 8 + (255-232)*10 = 238
        assert_eq!(
            Color::from_256(255),
            Color {
                r: 238,
                g: 238,
                b: 238
            }
        );
    }

    #[test]
    fn from_256_out_of_range() {
        assert_eq!(Color::from_256(256), Color::DEFAULT_FG);
    }

    #[test]
    fn bold_bright_maps_base_to_bright() {
        for i in 0..8 {
            assert_eq!(Color::ANSI[i].bold_bright(), Color::ANSI[i + 8]);
        }
    }

    #[test]
    fn bold_bright_returns_self_for_non_ansi() {
        let custom = Color { r: 1, g: 2, b: 3 };
        assert_eq!(custom.bold_bright(), custom);
    }

    #[test]
    fn bold_bright_returns_self_for_bright_colors() {
        // Bright colors that don't match any base 0-7 should return self
        let bright_black = Color::ANSI[8];
        let matches_base = (0..8).any(|i| {
            bright_black.r == Color::ANSI[i].r
                && bright_black.g == Color::ANSI[i].g
                && bright_black.b == Color::ANSI[i].b
        });
        if !matches_base {
            assert_eq!(bright_black.bold_bright(), bright_black);
        }
    }
}
