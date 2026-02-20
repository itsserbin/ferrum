use super::FontFamily;

/// Returns the embedded font bytes for the given font family.
///
/// Both fonts are compiled into the binary via `include_bytes!`.
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jetbrains_mono_loads_as_valid_font() {
        let data = font_data(FontFamily::JetBrainsMono);
        let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default());
        assert!(font.is_ok(), "JetBrainsMono should be a valid font");
    }

    #[test]
    fn fira_code_loads_as_valid_font() {
        let data = font_data(FontFamily::FiraCode);
        let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default());
        assert!(font.is_ok(), "FiraCode should be a valid font");
    }
}
