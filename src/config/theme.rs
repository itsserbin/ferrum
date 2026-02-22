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
    #[cfg(not(target_os = "macos"))]
    pub active_accent: Color,
    #[cfg(not(target_os = "macos"))]
    pub pin_active_color: Color,

    // -- Tab bar --
    #[cfg(not(target_os = "macos"))]
    pub bar_bg: Color,
    pub active_tab_bg: Color,
    #[cfg(not(target_os = "macos"))]
    pub inactive_tab_hover: Color,
    pub tab_text_active: Color,
    #[cfg(not(target_os = "macos"))]
    pub tab_text_inactive: Color,
    pub tab_border: Color,
    #[cfg(not(target_os = "macos"))]
    pub close_hover_bg: Color,
    #[cfg(not(target_os = "macos"))]
    pub rename_field_bg: Color,
    #[cfg(not(target_os = "macos"))]
    pub rename_field_border: Color,
    #[cfg(all(feature = "gpu", not(target_os = "macos")))]
    pub rename_selection_bg: Color,
    #[cfg(not(target_os = "macos"))]
    pub insertion_color: Color,
    #[cfg(not(target_os = "macos"))]
    pub win_btn_close_hover: Color,

    // -- Pane divider --
    pub split_divider_color: Color,
}

impl ThemeChoice {
    /// Resolves this theme choice into a full color palette.
    pub fn resolve(&self) -> ThemePalette {
        match self {
            ThemeChoice::FerrumDark => ThemePalette::ferrum_dark(),
            ThemeChoice::FerrumLight => ThemePalette::ferrum_light(),
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
            #[cfg(not(target_os = "macos"))]
            active_accent: Color { r: 180, g: 190, b: 254 },    // #B4BEFE
            #[cfg(not(target_os = "macos"))]
            pin_active_color: Color::from_pixel(0xB4BEFE),
            #[cfg(not(target_os = "macos"))]
            bar_bg: Color::from_pixel(0x1E2127),
            active_tab_bg: Color::from_pixel(0x282C34),
            #[cfg(not(target_os = "macos"))]
            inactive_tab_hover: Color::from_pixel(0x2E333C),
            tab_text_active: Color::from_pixel(0xD2DBEB),
            #[cfg(not(target_os = "macos"))]
            tab_text_inactive: Color::from_pixel(0x6C7480),
            tab_border: Color::from_pixel(0x2E333C),
            #[cfg(not(target_os = "macos"))]
            close_hover_bg: Color::from_pixel(0x454B59),
            #[cfg(not(target_os = "macos"))]
            rename_field_bg: Color::from_pixel(0x1E2127),
            #[cfg(not(target_os = "macos"))]
            rename_field_border: Color::from_pixel(0x6C7480),
            #[cfg(all(feature = "gpu", not(target_os = "macos")))]
            rename_selection_bg: Color::from_pixel(0xB4BEFE),
            #[cfg(not(target_os = "macos"))]
            insertion_color: Color::from_pixel(0xCBA6F7),
            #[cfg(not(target_os = "macos"))]
            win_btn_close_hover: Color::from_pixel(0xF38BA8),
            split_divider_color: Color::from_pixel(0x585B70),
        }
    }

    /// Ferrum Light — original warm light palette.
    fn ferrum_light() -> Self {
        Self {
            default_fg: Color { r: 46, g: 52, b: 64 },    // #2E3440
            default_bg: Color { r: 245, g: 240, b: 235 },  // #F5F0EB
            ansi: [
                Color { r: 59, g: 66, b: 82 },     //  0 black    #3B4252
                Color { r: 191, g: 59, b: 59 },     //  1 red      #BF3B3B
                Color { r: 58, g: 140, b: 46 },     //  2 green    #3A8C2E
                Color { r: 166, g: 113, b: 10 },    //  3 yellow   #A6710A
                Color { r: 43, g: 108, b: 179 },    //  4 blue     #2B6CB3
                Color { r: 155, g: 59, b: 181 },    //  5 magenta  #9B3BB5
                Color { r: 26, g: 138, b: 138 },    //  6 cyan     #1A8A8A
                Color { r: 184, g: 178, b: 172 },   //  7 white    #B8B2AC
                Color { r: 107, g: 115, b: 133 },   //  8 br black #6B7385
                Color { r: 217, g: 96, b: 96 },     //  9 br red   #D96060
                Color { r: 90, g: 175, b: 69 },     // 10 br green #5AAF45
                Color { r: 196, g: 139, b: 30 },    // 11 br yello #C48B1E
                Color { r: 74, g: 144, b: 217 },    // 12 br blue  #4A90D9
                Color { r: 184, g: 95, b: 209 },    // 13 br magen #B85FD1
                Color { r: 55, g: 168, b: 168 },    // 14 br cyan  #37A8A8
                Color { r: 213, g: 207, b: 201 },   // 15 br white #D5CFC9
            ],
            selection_overlay_color: Color::from_pixel(0x6B8EB5), // Steel blue
            selection_overlay_alpha: 80,
            scrollbar_color: Color::from_pixel(0xA8A2A0),
            scrollbar_hover_color: Color::from_pixel(0x8F8985),
            scrollbar_base_alpha: 160,
            #[cfg(not(target_os = "macos"))]
            active_accent: Color::from_pixel(0x4A6FA5),          // Deep steel blue
            #[cfg(not(target_os = "macos"))]
            pin_active_color: Color::from_pixel(0x4A6FA5),
            #[cfg(not(target_os = "macos"))]
            bar_bg: Color::from_pixel(0xE5DFD9),
            active_tab_bg: Color::from_pixel(0xF5F0EB),         // = default_bg
            #[cfg(not(target_os = "macos"))]
            inactive_tab_hover: Color::from_pixel(0xECE7E2),
            tab_text_active: Color::from_pixel(0x2E3440),       // = default_fg
            #[cfg(not(target_os = "macos"))]
            tab_text_inactive: Color::from_pixel(0x8A8480),
            tab_border: Color::from_pixel(0xDDD7D1),
            #[cfg(not(target_os = "macos"))]
            close_hover_bg: Color::from_pixel(0xDDD7D1),
            #[cfg(not(target_os = "macos"))]
            rename_field_bg: Color::from_pixel(0xFFFFFF),
            #[cfg(not(target_os = "macos"))]
            rename_field_border: Color::from_pixel(0xA8A2A0),
            #[cfg(all(feature = "gpu", not(target_os = "macos")))]
            rename_selection_bg: Color::from_pixel(0x4A6FA5),
            #[cfg(not(target_os = "macos"))]
            insertion_color: Color::from_pixel(0x9B3BB5),        // Magenta hue
            #[cfg(not(target_os = "macos"))]
            win_btn_close_hover: Color::from_pixel(0xD93B3B),
            split_divider_color: Color::from_pixel(0xC8C2BC),
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
        #[cfg(not(target_os = "macos"))]
        assert_eq!(palette.bar_bg.to_pixel(), 0x1E2127);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(palette.active_tab_bg.to_pixel(), 0x282C34);
    }

    #[test]
    fn ferrum_light_is_light_theme() {
        let palette = ThemeChoice::FerrumLight.resolve();
        // Light theme: background should be brighter than foreground
        let bg_brightness =
            palette.default_bg.r as u32 + palette.default_bg.g as u32 + palette.default_bg.b as u32;
        let fg_brightness =
            palette.default_fg.r as u32 + palette.default_fg.g as u32 + palette.default_fg.b as u32;
        assert!(
            bg_brightness > fg_brightness,
            "Ferrum Light bg ({bg_brightness}) should be brighter than fg ({fg_brightness})"
        );
    }

    #[test]
    fn each_theme_has_16_ansi_colors() {
        for theme in [ThemeChoice::FerrumDark, ThemeChoice::FerrumLight] {
            let palette = theme.resolve();
            assert_eq!(palette.ansi.len(), 16);
        }
    }
}
