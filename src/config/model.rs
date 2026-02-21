use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AppConfig {
    pub font: FontConfig,
    pub theme: ThemeChoice,
    pub terminal: TerminalConfig,
    pub layout: LayoutConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct FontConfig {
    pub size: f32,
    pub family: FontFamily,
    pub line_padding: u32,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            size: 14.0,
            family: FontFamily::default(),
            line_padding: 2,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct TerminalConfig {
    pub max_scrollback: usize,
    pub cursor_blink_interval_ms: u64,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            max_scrollback: 1000,
            cursor_blink_interval_ms: 500,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct LayoutConfig {
    pub window_padding: u32,
    pub tab_bar_height: u32,
    pub pane_inner_padding: u32,
    pub scrollbar_width: u32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            window_padding: 8,
            tab_bar_height: 36,
            pane_inner_padding: 4,
            scrollbar_width: 6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) enum FontFamily {
    #[default]
    JetBrainsMono,
    FiraCode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) enum ThemeChoice {
    #[default]
    FerrumDark,
    #[serde(alias = "CatppuccinLatte")]
    FerrumLight,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_round_trip() {
        let config = AppConfig::default();
        let serialized = ron::to_string(&config).expect("serialize");
        let deserialized: AppConfig = ron::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized.font.size, 14.0);
        assert_eq!(deserialized.theme, ThemeChoice::FerrumDark);
        assert_eq!(deserialized.terminal.max_scrollback, 1000);
        assert_eq!(deserialized.layout.window_padding, 8);
    }

    #[test]
    fn partial_config_uses_defaults() {
        let partial = "(theme: FerrumLight)";
        let config: AppConfig = ron::from_str(partial).expect("deserialize partial");
        assert_eq!(config.theme, ThemeChoice::FerrumLight);
        assert_eq!(config.font.size, 14.0);
        assert_eq!(config.terminal.max_scrollback, 1000);
    }

    #[test]
    fn default_values_are_correct() {
        let config = AppConfig::default();
        assert_eq!(config.font.size, 14.0);
        assert_eq!(config.font.family, FontFamily::JetBrainsMono);
        assert_eq!(config.font.line_padding, 2);
        assert_eq!(config.terminal.max_scrollback, 1000);
        assert_eq!(config.terminal.cursor_blink_interval_ms, 500);
        assert_eq!(config.layout.window_padding, 8);
        assert_eq!(config.layout.tab_bar_height, 36);
        assert_eq!(config.layout.pane_inner_padding, 4);
        assert_eq!(config.layout.scrollbar_width, 6);
    }
}
