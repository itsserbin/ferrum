#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const DEFAULT_FG: Color = Color {
        r: 205,
        g: 214,
        b: 244,
    }; // #CDD6F4
    pub const DEFAULT_BG: Color = Color {
        r: 30,
        g: 30,
        b: 46,
    }; // #1E1E2E

    // Catppuccin Mocha — 16 ANSI colors (0-7 normal, 8-15 bright)
    pub const ANSI: [Color; 16] = [
        Color {
            r: 69,
            g: 71,
            b: 90,
        }, //  0 black    #45475A
        Color {
            r: 243,
            g: 139,
            b: 168,
        }, //  1 red      #F38BA8
        Color {
            r: 166,
            g: 227,
            b: 161,
        }, //  2 green    #A6E3A1
        Color {
            r: 249,
            g: 226,
            b: 175,
        }, //  3 yellow   #F9E2AF
        Color {
            r: 137,
            g: 180,
            b: 250,
        }, //  4 blue     #89B4FA
        Color {
            r: 245,
            g: 194,
            b: 231,
        }, //  5 magenta  #F5C2E7
        Color {
            r: 148,
            g: 226,
            b: 213,
        }, //  6 cyan     #94E2D5
        Color {
            r: 186,
            g: 194,
            b: 222,
        }, //  7 white    #BAC2DE
        Color {
            r: 88,
            g: 91,
            b: 112,
        }, //  8 br black #585B70
        Color {
            r: 243,
            g: 139,
            b: 168,
        }, //  9 br red   #F38BA8
        Color {
            r: 166,
            g: 227,
            b: 161,
        }, // 10 br green #A6E3A1
        Color {
            r: 249,
            g: 226,
            b: 175,
        }, // 11 br yello #F9E2AF
        Color {
            r: 137,
            g: 180,
            b: 250,
        }, // 12 br blue  #89B4FA
        Color {
            r: 245,
            g: 194,
            b: 231,
        }, // 13 br magen #F5C2E7
        Color {
            r: 148,
            g: 226,
            b: 213,
        }, // 14 br cyan  #94E2D5
        Color {
            r: 166,
            g: 173,
            b: 200,
        }, // 15 br white #A6ADC8
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
        assert_eq!(
            Color::from_256(16),
            Color { r: 0, g: 0, b: 0 }
        );
        // Index 196: n-16=180, r=180/36=5, g=(180%36)/6=0, b=180%6=0
        // r=5 -> 55+5*40=255, g=0, b=0
        assert_eq!(
            Color::from_256(196),
            Color { r: 255, g: 0, b: 0 }
        );
    }

    #[test]
    fn from_256_grayscale() {
        // Index 232: v = 8 + (232-232)*10 = 8
        assert_eq!(
            Color::from_256(232),
            Color { r: 8, g: 8, b: 8 }
        );
        // Index 255: v = 8 + (255-232)*10 = 238
        assert_eq!(
            Color::from_256(255),
            Color { r: 238, g: 238, b: 238 }
        );
    }

    #[test]
    fn from_256_out_of_range() {
        assert_eq!(Color::from_256(256), Color::DEFAULT_FG);
    }
}
