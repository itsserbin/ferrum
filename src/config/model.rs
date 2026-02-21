use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AppConfig {
    pub font: FontConfig,
    pub theme: ThemeChoice,
    pub terminal: TerminalConfig,
    pub layout: LayoutConfig,
    pub security: SecuritySettings,
    pub language: crate::i18n::Locale,
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
            line_padding: 0,
        }
    }
}

impl FontConfig {
    pub const SIZE_MIN: f32 = 8.0;
    pub const SIZE_MAX: f32 = 32.0;
    pub const SIZE_STEP: f32 = 0.5;
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

impl TerminalConfig {
    pub const BLINK_MS_MIN: u64 = 100;
    pub const BLINK_MS_MAX: u64 = 2000;
    pub const BLINK_MS_STEP: u64 = 50;
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
    CascadiaCode,
    UbuntuMono,
    SourceCodePro,
}

impl FontFamily {
    /// Display names for UI dropdowns — order matches variant order.
    pub(crate) const DISPLAY_NAMES: &'static [&'static str] = &[
        "JetBrains Mono",
        "Fira Code",
        "Cascadia Code",
        "Ubuntu Mono",
        "Source Code Pro",
    ];

    /// All variants in declaration order.
    pub(crate) const ALL: &'static [FontFamily] = &[
        FontFamily::JetBrainsMono,
        FontFamily::FiraCode,
        FontFamily::CascadiaCode,
        FontFamily::UbuntuMono,
        FontFamily::SourceCodePro,
    ];

    /// Returns the index of this variant (matches `DISPLAY_NAMES` and `ALL`).
    pub(crate) fn index(self) -> usize {
        Self::ALL.iter().position(|&v| v == self).unwrap_or(0)
    }

    /// Returns the variant at the given index, or the default if out of range.
    pub(crate) fn from_index(i: usize) -> Self {
        Self::ALL.get(i).copied().unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) enum ThemeChoice {
    #[default]
    FerrumDark,
    FerrumLight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) enum SecurityMode {
    Disabled,
    #[default]
    Standard,
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SecuritySettings {
    pub mode: SecurityMode,
    pub paste_protection: bool,
    pub block_title_query: bool,
    pub limit_cursor_jumps: bool,
    pub clear_mouse_on_reset: bool,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            mode: SecurityMode::Standard,
            paste_protection: true,
            block_title_query: true,
            limit_cursor_jumps: true,
            clear_mouse_on_reset: true,
        }
    }
}

impl SecuritySettings {
    /// Converts settings into a runtime `SecurityConfig`.
    ///
    /// When mode is `Disabled`, all checks are turned off regardless of
    /// individual toggle values. `Standard` and `Custom` use individual toggles.
    pub(crate) fn to_runtime(&self) -> crate::core::SecurityConfig {
        match self.mode {
            SecurityMode::Disabled => crate::core::SecurityConfig {
                paste_protection: false,
                block_title_query: false,
                limit_cursor_jumps: false,
                clear_mouse_on_reset: false,
            },
            SecurityMode::Standard | SecurityMode::Custom => crate::core::SecurityConfig {
                paste_protection: self.paste_protection,
                block_title_query: self.block_title_query,
                limit_cursor_jumps: self.limit_cursor_jumps,
                clear_mouse_on_reset: self.clear_mouse_on_reset,
            },
        }
    }

    /// Infers the mode from the current toggle values.
    ///
    /// All ON → Standard, all OFF → Disabled, mixed → Custom.
    pub(crate) fn inferred_mode(&self) -> SecurityMode {
        let all = [
            self.paste_protection,
            self.block_title_query,
            self.limit_cursor_jumps,
            self.clear_mouse_on_reset,
        ];
        if all.iter().all(|&v| v) {
            SecurityMode::Standard
        } else if all.iter().all(|&v| !v) {
            SecurityMode::Disabled
        } else {
            SecurityMode::Custom
        }
    }
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
        assert_eq!(config.font.line_padding, 0);
        assert_eq!(config.terminal.max_scrollback, 1000);
        assert_eq!(config.terminal.cursor_blink_interval_ms, 500);
        assert_eq!(config.layout.window_padding, 8);
        assert_eq!(config.layout.tab_bar_height, 36);
        assert_eq!(config.layout.pane_inner_padding, 4);
        assert_eq!(config.layout.scrollbar_width, 6);
        assert_eq!(config.security.mode, SecurityMode::Standard);
        assert!(config.security.paste_protection);
        assert!(config.security.block_title_query);
        assert!(config.security.limit_cursor_jumps);
        assert!(config.security.clear_mouse_on_reset);
    }

    #[test]
    fn security_disabled_turns_off_all_checks() {
        let settings = SecuritySettings {
            mode: SecurityMode::Disabled,
            paste_protection: true,
            block_title_query: true,
            limit_cursor_jumps: true,
            clear_mouse_on_reset: true,
        };
        let runtime = settings.to_runtime();
        assert!(!runtime.paste_protection);
        assert!(!runtime.block_title_query);
        assert!(!runtime.limit_cursor_jumps);
        assert!(!runtime.clear_mouse_on_reset);
    }

    #[test]
    fn security_standard_uses_individual_toggles() {
        let settings = SecuritySettings {
            mode: SecurityMode::Standard,
            paste_protection: false,
            block_title_query: true,
            limit_cursor_jumps: false,
            clear_mouse_on_reset: true,
        };
        let runtime = settings.to_runtime();
        assert!(!runtime.paste_protection);
        assert!(runtime.block_title_query);
        assert!(!runtime.limit_cursor_jumps);
        assert!(runtime.clear_mouse_on_reset);
    }

    #[test]
    fn security_custom_uses_individual_toggles() {
        let settings = SecuritySettings {
            mode: SecurityMode::Custom,
            paste_protection: true,
            block_title_query: false,
            limit_cursor_jumps: true,
            clear_mouse_on_reset: false,
        };
        let runtime = settings.to_runtime();
        assert!(runtime.paste_protection);
        assert!(!runtime.block_title_query);
        assert!(runtime.limit_cursor_jumps);
        assert!(!runtime.clear_mouse_on_reset);
    }

    #[test]
    fn inferred_mode_all_on_is_standard() {
        let s = SecuritySettings::default();
        assert_eq!(s.inferred_mode(), SecurityMode::Standard);
    }

    #[test]
    fn inferred_mode_all_off_is_disabled() {
        let s = SecuritySettings {
            mode: SecurityMode::Custom,
            paste_protection: false,
            block_title_query: false,
            limit_cursor_jumps: false,
            clear_mouse_on_reset: false,
        };
        assert_eq!(s.inferred_mode(), SecurityMode::Disabled);
    }

    #[test]
    fn inferred_mode_mixed_is_custom() {
        let s = SecuritySettings {
            mode: SecurityMode::Standard,
            paste_protection: true,
            block_title_query: false,
            limit_cursor_jumps: true,
            clear_mouse_on_reset: true,
        };
        assert_eq!(s.inferred_mode(), SecurityMode::Custom);
    }
}
