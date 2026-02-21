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

/// Returns the embedded Symbols Nerd Font Mono bytes (fallback for missing glyphs).
pub(crate) fn fallback_font_data() -> &'static [u8] {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/fonts/SymbolsNerdFontMono-Regular.ttf"
    ))
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
    fn fallback_font_loads_as_valid() {
        let data = super::fallback_font_data();
        let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default());
        assert!(font.is_ok(), "fallback font should be a valid font");
    }

    #[test]
    fn fallback_covers_nerd_font_symbols() {
        let primary_data = font_data(FontFamily::JetBrainsMono);
        let primary =
            fontdue::Font::from_bytes(primary_data, fontdue::FontSettings::default()).unwrap();

        let fallback_data = fallback_font_data();
        let fallback =
            fontdue::Font::from_bytes(fallback_data, fontdue::FontSettings::default()).unwrap();

        // U+E700 is a Nerd Font dev icon — not in JetBrains Mono but present in Symbols Nerd Font Mono.
        assert!(
            !primary.has_glyph('\u{E700}'),
            "primary should lack U+E700"
        );
        assert!(
            fallback.has_glyph('\u{E700}'),
            "fallback should have U+E700"
        );
    }
}
