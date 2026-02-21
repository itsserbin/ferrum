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
}
