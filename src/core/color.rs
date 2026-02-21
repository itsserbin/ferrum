#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// Sentinel value for "use the theme's default foreground."
    ///
    /// Compile-time constant used by `Cell::DEFAULT`. Renderers compare
    /// against this to remap cells to the active theme palette.
    pub const SENTINEL_FG: Color = Color {
        r: 210,
        g: 219,
        b: 235,
    }; // #D2DBEB

    /// Sentinel value for "use the theme's default background."
    ///
    /// See [`SENTINEL_FG`](Self::SENTINEL_FG) for details.
    pub const SENTINEL_BG: Color = Color {
        r: 40,
        g: 44,
        b: 52,
    }; // #282C34

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

    /// If this color matches one of the base 8 ANSI colors (indices 0-7)
    /// in the given `palette`, returns the corresponding bright variant (8-15).
    /// Otherwise returns `self` unchanged.  Used to implement the
    /// bold-implies-bright convention for terminal emulators.
    pub fn bold_bright_with_palette(self, palette: &[Color; 16]) -> Color {
        for i in 0..8 {
            if self == palette[i] {
                return palette[i + 8];
            }
        }
        self
    }

    /// Returns a dimmed version of this color by reducing brightness.
    ///
    /// `factor` is the dimming amount (0.0 = no change, 1.0 = fully black).
    pub fn dimmed(self, factor: f32) -> Color {
        let brightness = 1.0 - factor.clamp(0.0, 1.0);
        Color {
            r: (self.r as f32 * brightness) as u8,
            g: (self.g as f32 * brightness) as u8,
            b: (self.b as f32 * brightness) as u8,
        }
    }

    /// 256-color palette lookup for indices 16-255 (color cube + grayscale).
    ///
    /// Indices 0-15 require a theme palette — use [`Terminal::color_from_256`]
    /// instead.  Out-of-range values return [`SENTINEL_FG`](Self::SENTINEL_FG).
    pub fn from_256(n: u16) -> Color {
        match n {
            0..=15 | 256.. => Color::SENTINEL_FG,
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
    fn from_256_ansi_range_returns_sentinel() {
        for i in 0..16u16 {
            assert_eq!(Color::from_256(i), Color::SENTINEL_FG);
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
        assert_eq!(Color::from_256(256), Color::SENTINEL_FG);
    }

    fn test_ansi_palette() -> [Color; 16] {
        crate::config::ThemeChoice::FerrumDark.resolve().ansi
    }

    #[test]
    fn bold_bright_maps_base_to_bright() {
        let ansi = test_ansi_palette();
        for i in 0..8 {
            assert_eq!(ansi[i].bold_bright_with_palette(&ansi), ansi[i + 8]);
        }
    }

    #[test]
    fn bold_bright_returns_self_for_non_ansi() {
        let ansi = test_ansi_palette();
        let custom = Color { r: 1, g: 2, b: 3 };
        assert_eq!(custom.bold_bright_with_palette(&ansi), custom);
    }

    #[test]
    fn bold_bright_returns_self_for_bright_colors() {
        let ansi = test_ansi_palette();
        // Bright colors that don't match any base 0-7 should return self
        let bright_black = ansi[8];
        let matches_base = (0..8).any(|i| ansi[i] == bright_black);
        if !matches_base {
            assert_eq!(bright_black.bold_bright_with_palette(&ansi), bright_black);
        }
    }
}
