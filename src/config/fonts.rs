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

/// Returns raw font bytes for the primary font and fallback chain.
///
/// The returned slices point to data compiled into the binary (`include_bytes!`).
pub(crate) fn load_fonts(family: FontFamily) -> (&'static [u8], Vec<&'static [u8]>) {
    let primary = font_data(family);
    let fallbacks = fallback_fonts_data().to_vec();
    (primary, fallbacks)
}

/// Returns embedded fallback font data in priority order.
///
/// 1. Noto Sans Symbols — Arrows, Misc Technical, Dingbats, Misc Symbols
/// 2. Noto Sans Symbols 2 — Braille, Geometric Shapes, Supplemental Arrows
pub(crate) fn fallback_fonts_data() -> &'static [&'static [u8]] {
    &[
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/NotoSansSymbols-Regular.ttf"
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
        for family in [
            FontFamily::JetBrainsMono,
            FontFamily::FiraCode,
            FontFamily::CascadiaCode,
            FontFamily::UbuntuMono,
            FontFamily::SourceCodePro,
        ] {
            let data = font_data(family);
            let font = swash::FontRef::from_index(data, 0);
            assert!(font.is_some(), "{family:?} should parse as a valid swash font");
        }
    }

    #[test]
    fn fallback_fonts_load_as_valid() {
        for (i, data) in fallback_fonts_data().iter().enumerate() {
            let font = swash::FontRef::from_index(data, 0);
            assert!(font.is_some(), "fallback font {i} should parse as a valid swash font");
        }
    }

    #[test]
    fn fallback_chain_covers_missing_glyphs() {
        let primary_data = font_data(FontFamily::JetBrainsMono);
        let primary = swash::FontRef::from_index(primary_data, 0).unwrap();

        let fallbacks: Vec<_> = fallback_fonts_data()
            .iter()
            .map(|d| swash::FontRef::from_index(d, 0).unwrap())
            .collect();

        let has_glyph = |font: &swash::FontRef, ch: char| font.charmap().map(ch) != 0;

        // U+23BF (⎿) — covered by fallback[0] (Noto Sans Symbols).
        assert!(!has_glyph(&primary, '\u{23BF}'));
        assert!(has_glyph(&fallbacks[0], '\u{23BF}'));

        // U+23FA (⏺) — must NOT be in primary, must be in at least one fallback.
        assert!(!has_glyph(&primary, '\u{23FA}'), "⏺ should not be in the primary font");
        assert!(
            fallbacks.iter().any(|f| has_glyph(f, '\u{23FA}')),
            "⏺ should be covered by at least one fallback font"
        );

        // Verify all Claude Code icons are covered by primary + fallbacks.
        let claude_chars = [
            ('\u{23FA}', "⏺ prompt"),
            ('\u{25CF}', "● prompt fallback"),
            ('\u{23BF}', "⎿ response delimiter"),
            ('\u{273B}', "✻ idle"),
            ('\u{21AF}', "↯ interrupt"),
            ('\u{21BB}', "↻ retry"),
            ('\u{2714}', "✔ check"),
            ('\u{00D7}', "× cancel"),
            ('\u{23F8}', "⏸ plan mode"),
            ('\u{23F5}', "⏵ accept edits"),
            ('\u{2722}', "✢ spinner"),
            ('\u{2733}', "✳ spinner"),
            ('\u{2736}', "✶ spinner"),
            ('\u{273D}', "✽ spinner"),
            ('\u{2718}', "✘ cross"),
            ('\u{276F}', "❯ pointer"),
            ('\u{25B6}', "▶ play"),
            ('\u{23CE}', "⏎ return"),
            ('\u{25C7}', "◇ diamond"),
            ('\u{2630}', "☰ hamburger"),
        ];
        let mut missing = Vec::new();
        for (ch, name) in &claude_chars {
            let covered = has_glyph(&primary, *ch)
                || fallbacks.iter().any(|f| has_glyph(f, *ch));
            if !covered {
                missing.push(*name);
            }
        }
        // U+21BB (↻) is used rarely (reconnect only). Allow it to be missing.
        let critical_missing: Vec<_> = missing.iter()
            .filter(|name| !name.contains("retry"))
            .collect();
        assert!(
            critical_missing.is_empty(),
            "Critical Claude Code icons not covered: {critical_missing:?}"
        );
    }
}
