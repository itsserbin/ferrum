use crate::core::Color;

use super::ThemeChoice;

/// Complete color palette resolved from a [`ThemeChoice`].
///
/// Contains all terminal and UI chrome colors needed by the renderers.
pub(crate) struct ThemePalette {
    // -- Terminal colors --
    pub default_fg: Color,
    pub default_bg: Color,
    pub ansi: [Color; 16],

    // -- Selection --
    pub selection_overlay_color: Color,
    pub selection_overlay_alpha: u8,

    // -- Scrollbar --
    pub scrollbar_color: Color,
    pub scrollbar_hover_color: Color,
    pub scrollbar_base_alpha: u8,

    // -- Accents --
    pub active_accent: Color,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub pin_active_color: Color,
    pub security_accent: Color,

    // -- Overlay / menu --
    pub menu_bg: Color,

    // -- Tab bar --
    pub bar_bg: Color,
    pub active_tab_bg: Color,
    pub inactive_tab_hover: Color,
    pub tab_text_active: Color,
    pub tab_text_inactive: Color,
    pub tab_border: Color,
    pub close_hover_bg: Color,
    pub rename_field_bg: Color,
    pub rename_field_border: Color,
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    pub rename_selection_bg: Color,
    pub insertion_color: Color,
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub win_btn_close_hover: Color,

    // -- Pane divider --
    pub split_divider_color: Color,
}

impl ThemeChoice {
    /// Resolves this theme choice into a full color palette.
    pub fn resolve(&self) -> ThemePalette {
        match self {
            ThemeChoice::FerrumDark => ThemePalette::ferrum_dark(),
            ThemeChoice::CatppuccinLatte => ThemePalette::catppuccin_latte(),
        }
    }
}

impl ThemePalette {
    /// Ferrum Dark — the default dark palette (current hardcoded values).
    fn ferrum_dark() -> Self {
        Self {
            default_fg: Color { r: 210, g: 219, b: 235 }, // #D2DBEB
            default_bg: Color { r: 40, g: 44, b: 52 },    // #282C34
            ansi: [
                Color { r: 69, g: 75, b: 89 },    //  0 black    #454B59
                Color { r: 224, g: 108, b: 117 },  //  1 red      #E06C75
                Color { r: 152, g: 195, b: 121 },  //  2 green    #98C379
                Color { r: 229, g: 192, b: 123 },  //  3 yellow   #E5C07B
                Color { r: 97, g: 175, b: 239 },   //  4 blue     #61AFEF
                Color { r: 198, g: 120, b: 221 },  //  5 magenta  #C678DD
                Color { r: 86, g: 182, b: 194 },   //  6 cyan     #56B6C2
                Color { r: 171, g: 178, b: 191 },  //  7 white    #ABB2BF
                Color { r: 106, g: 114, b: 130 },  //  8 br black #6A7282
                Color { r: 240, g: 139, b: 149 },  //  9 br red   #F08B95
                Color { r: 167, g: 213, b: 141 },  // 10 br green #A7D58D
                Color { r: 235, g: 203, b: 139 },  // 11 br yello #EBCB8B
                Color { r: 123, g: 195, b: 255 },  // 12 br blue  #7BC3FF
                Color { r: 215, g: 153, b: 240 },  // 13 br magen #D799F0
                Color { r: 111, g: 199, b: 211 },  // 14 br cyan  #6FC7D3
                Color { r: 221, g: 229, b: 245 },  // 15 br white #DDE5F5
            ],
            selection_overlay_color: Color::from_pixel(0x5F7FA3),
            selection_overlay_alpha: 96,
            scrollbar_color: Color { r: 108, g: 112, b: 134 },  // #6C7086
            scrollbar_hover_color: Color { r: 127, g: 132, b: 156 }, // #7F849C
            scrollbar_base_alpha: 180,
            active_accent: Color { r: 180, g: 190, b: 254 },    // #B4BEFE
            pin_active_color: Color::from_pixel(0xB4BEFE),
            security_accent: Color { r: 249, g: 226, b: 175 },  // #F9E2AF
            menu_bg: Color::from_pixel(0x1E2433),
            bar_bg: Color::from_pixel(0x1E2127),
            active_tab_bg: Color::from_pixel(0x282C34),
            inactive_tab_hover: Color::from_pixel(0x2E333C),
            tab_text_active: Color::from_pixel(0xD2DBEB),
            tab_text_inactive: Color::from_pixel(0x6C7480),
            tab_border: Color::from_pixel(0x2E333C),
            close_hover_bg: Color::from_pixel(0x454B59),
            rename_field_bg: Color::from_pixel(0x1E2127),
            rename_field_border: Color::from_pixel(0x6C7480),
            rename_selection_bg: Color::from_pixel(0xB4BEFE),
            insertion_color: Color::from_pixel(0xCBA6F7),
            win_btn_close_hover: Color::from_pixel(0xF38BA8),
            split_divider_color: Color::from_pixel(0x585B70),
        }
    }

    /// Catppuccin Latte — official light theme palette.
    fn catppuccin_latte() -> Self {
        Self {
            default_fg: Color::from_pixel(0x4C4F69),  // Text
            default_bg: Color::from_pixel(0xEFF1F5),  // Base
            ansi: [
                Color::from_pixel(0x5C5F77),  //  0 black    Subtext1
                Color::from_pixel(0xD20F39),  //  1 red      Red
                Color::from_pixel(0x40A02B),  //  2 green    Green
                Color::from_pixel(0xDF8E1D),  //  3 yellow   Yellow
                Color::from_pixel(0x1E66F5),  //  4 blue     Blue
                Color::from_pixel(0xEA76CB),  //  5 magenta  Pink
                Color::from_pixel(0x179299),  //  6 cyan     Teal
                Color::from_pixel(0xACB0BE),  //  7 white    Surface2
                Color::from_pixel(0x6C6F85),  //  8 br black Subtext0
                Color::from_pixel(0xD20F39),  //  9 br red   Red
                Color::from_pixel(0x40A02B),  // 10 br green Green
                Color::from_pixel(0xDF8E1D),  // 11 br yello Yellow
                Color::from_pixel(0x1E66F5),  // 12 br blue  Blue
                Color::from_pixel(0xEA76CB),  // 13 br magen Pink
                Color::from_pixel(0x179299),  // 14 br cyan  Teal
                Color::from_pixel(0xBCC0CC),  // 15 br white Surface1
            ],
            selection_overlay_color: Color::from_pixel(0x7287FD), // Lavender
            selection_overlay_alpha: 96,
            scrollbar_color: Color::from_pixel(0x9CA0B0),        // Overlay0
            scrollbar_hover_color: Color::from_pixel(0x8C8FA1),  // Overlay1
            scrollbar_base_alpha: 180,
            active_accent: Color::from_pixel(0x7287FD),          // Lavender
            pin_active_color: Color::from_pixel(0x7287FD),       // Lavender
            security_accent: Color::from_pixel(0xDF8E1D),        // Yellow
            menu_bg: Color::from_pixel(0xE6E9EF),               // Mantle
            bar_bg: Color::from_pixel(0xDCE0E8),                // Crust
            active_tab_bg: Color::from_pixel(0xEFF1F5),         // Base
            inactive_tab_hover: Color::from_pixel(0xE6E9EF),    // Mantle
            tab_text_active: Color::from_pixel(0x4C4F69),       // Text
            tab_text_inactive: Color::from_pixel(0x9CA0B0),     // Overlay0
            tab_border: Color::from_pixel(0xCCD0DA),            // Surface0
            close_hover_bg: Color::from_pixel(0xBCC0CC),        // Surface1
            rename_field_bg: Color::from_pixel(0xDCE0E8),       // Crust
            rename_field_border: Color::from_pixel(0x9CA0B0),   // Overlay0
            rename_selection_bg: Color::from_pixel(0x7287FD),   // Lavender
            insertion_color: Color::from_pixel(0x8839EF),       // Mauve
            win_btn_close_hover: Color::from_pixel(0xD20F39),   // Red
            split_divider_color: Color::from_pixel(0xACB0BE),   // Surface2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ferrum_dark_matches_hardcoded_defaults() {
        let palette = ThemeChoice::FerrumDark.resolve();
        assert_eq!(palette.default_fg, Color { r: 210, g: 219, b: 235 });
        assert_eq!(palette.default_bg, Color { r: 40, g: 44, b: 52 });
        assert_eq!(palette.ansi.len(), 16);
        assert_eq!(palette.bar_bg.to_pixel(), 0x1E2127);
        assert_eq!(palette.active_tab_bg.to_pixel(), 0x282C34);
    }

    #[test]
    fn catppuccin_latte_is_light_theme() {
        let palette = ThemeChoice::CatppuccinLatte.resolve();
        // Light theme: background should be brighter than foreground
        let bg_brightness =
            palette.default_bg.r as u32 + palette.default_bg.g as u32 + palette.default_bg.b as u32;
        let fg_brightness =
            palette.default_fg.r as u32 + palette.default_fg.g as u32 + palette.default_fg.b as u32;
        assert!(
            bg_brightness > fg_brightness,
            "Latte bg ({bg_brightness}) should be brighter than fg ({fg_brightness})"
        );
    }

    #[test]
    fn each_theme_has_16_ansi_colors() {
        for theme in [ThemeChoice::FerrumDark, ThemeChoice::CatppuccinLatte] {
            let palette = theme.resolve();
            assert_eq!(palette.ansi.len(), 16);
        }
    }
}
