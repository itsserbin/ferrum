use super::FontFamily;

/// Returns the embedded font bytes for the given font family.
///
/// All fonts are compiled into the binary via `include_bytes!`.
pub(crate) fn font_data(family: FontFamily) -> &'static [u8] {
    match family {
        FontFamily::JetBrainsMono => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/fonts/JetBrainsMono-Regular.ttf"
            ))
        }
        FontFamily::FiraCode => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/fonts/FiraCode-Regular.ttf"
            ))
        }
        FontFamily::CascadiaCode => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/fonts/CascadiaCode-Regular.ttf"
            ))
        }
        FontFamily::UbuntuMono => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/fonts/UbuntuMono-Regular.ttf"
            ))
        }
        FontFamily::SourceCodePro => {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/fonts/SourceCodePro-Regular.ttf"
            ))
        }
    }
}

/// Returns embedded fallback font data in priority order.
///
/// 1. Symbols Nerd Font Mono — Nerd Font icons (Powerline, devicons, etc.)
/// 2. Noto Sans Symbols 2 — standard Unicode symbols (Misc Technical, Braille, etc.)
pub(crate) fn fallback_fonts_data() -> &'static [&'static [u8]] {
    &[
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/SymbolsNerdFontMono-Regular.ttf"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/NotoSansSymbols2-Regular.ttf"
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Validates that every `FontFamily` variant loads as a valid font.
    #[test]
    fn all_fonts_load_as_valid() {
        let families = [
            FontFamily::JetBrainsMono,
            FontFamily::FiraCode,
            FontFamily::CascadiaCode,
            FontFamily::UbuntuMono,
            FontFamily::SourceCodePro,
        ];
        for family in families {
            let data = font_data(family);
            let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default());
            assert!(font.is_ok(), "{family:?} should be a valid font");
        }
    }

    #[test]
    fn fallback_fonts_load_as_valid() {
        for (i, data) in fallback_fonts_data().iter().enumerate() {
            let font = fontdue::Font::from_bytes(*data, fontdue::FontSettings::default());
            assert!(font.is_ok(), "fallback font {i} should be valid");
        }
    }

    #[test]
    fn fallback_chain_covers_missing_glyphs() {
        let primary_data = font_data(FontFamily::JetBrainsMono);
        let primary =
            fontdue::Font::from_bytes(primary_data, fontdue::FontSettings::default()).unwrap();

        let fallbacks: Vec<_> = fallback_fonts_data()
            .iter()
            .map(|d| fontdue::Font::from_bytes(*d, fontdue::FontSettings::default()).unwrap())
            .collect();

        // U+E700 (Nerd Font devicon) — covered by fallback[0] (Symbols Nerd Font Mono).
        assert!(!primary.has_glyph('\u{E700}'));
        assert!(fallbacks[0].has_glyph('\u{E700}'));

        // U+23FA (⏺) — covered by fallback[1] (Noto Sans Symbols 2).
        assert!(!primary.has_glyph('\u{23FA}'));
        assert!(fallbacks[1].has_glyph('\u{23FA}'));
    }
}
