use std::fmt;

use crate::config::{AppConfig, FontFamily, ThemeChoice};

/// Categories in the settings sidebar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui) enum SettingsCategory {
    Font,
    Theme,
    Terminal,
    Layout,
}

impl SettingsCategory {
    /// All categories in display order.
    pub(in crate::gui) const CATEGORIES: &'static [SettingsCategory] = &[
        SettingsCategory::Font,
        SettingsCategory::Theme,
        SettingsCategory::Terminal,
        SettingsCategory::Layout,
    ];
}

impl fmt::Display for SettingsCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsCategory::Font => write!(f, "Font"),
            SettingsCategory::Theme => write!(f, "Theme"),
            SettingsCategory::Terminal => write!(f, "Terminal"),
            SettingsCategory::Layout => write!(f, "Layout"),
        }
    }
}

/// A single setting row rendered in the content area.
#[derive(Debug, Clone)]
pub(in crate::gui) enum SettingItem {
    FloatSlider {
        label: &'static str,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    },
    IntSlider {
        label: &'static str,
        value: u32,
        min: u32,
        max: u32,
    },
    LargeIntSlider {
        label: &'static str,
        value: usize,
        min: usize,
        max: usize,
        step: usize,
    },
    EnumChoice {
        label: &'static str,
        options: &'static [&'static str],
        selected: usize,
    },
}

/// Which half of a stepper control the mouse is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui) enum StepperHalf {
    Minus,
    Plus,
}

/// State for the settings overlay panel.
pub(in crate::gui) struct SettingsOverlay {
    /// Currently selected sidebar category.
    pub active_category: SettingsCategory,
    /// Index into `CATEGORIES` that the mouse is hovering over (sidebar).
    pub hovered_category: Option<usize>,
    /// Index into the items list that the mouse is hovering over (content area).
    pub hovered_item: Option<usize>,
    /// Snapshot taken when the overlay was opened, used for revert.
    #[allow(dead_code)] // Reserved for future cancel/revert feature.
    pub original_config: AppConfig,
    /// Live-edited config; changes apply immediately for preview.
    pub editing_config: AppConfig,
    /// Vertical scroll offset for long item lists.
    pub scroll_offset: usize,
    /// Index of the currently open dropdown (if any).
    pub open_dropdown: Option<usize>,
    /// Hovered option within an open dropdown.
    pub hovered_dropdown_option: Option<usize>,
    /// Hovered stepper button: (item_index, which half).
    pub hovered_stepper: Option<(usize, StepperHalf)>,
    /// Index of the item whose dropdown button is hovered.
    pub hovered_dropdown: Option<usize>,
    /// Whether the close (X) button is hovered.
    pub hovered_close: bool,
}

impl SettingsOverlay {
    /// Opens the settings overlay with a snapshot of the current config.
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub(in crate::gui) fn new(config: &AppConfig) -> Self {
        Self {
            active_category: SettingsCategory::Font,
            hovered_category: None,
            hovered_item: None,
            original_config: config.clone(),
            editing_config: config.clone(),
            scroll_offset: 0,
            open_dropdown: None,
            hovered_dropdown_option: None,
            hovered_stepper: None,
            hovered_dropdown: None,
            hovered_close: false,
        }
    }

    /// Returns the setting items for the currently active category.
    pub(in crate::gui) fn items(&self) -> Vec<SettingItem> {
        match self.active_category {
            SettingsCategory::Font => self.font_items(),
            SettingsCategory::Theme => self.theme_items(),
            SettingsCategory::Terminal => self.terminal_items(),
            SettingsCategory::Layout => self.layout_items(),
        }
    }

    fn font_items(&self) -> Vec<SettingItem> {
        let font = &self.editing_config.font;
        vec![
            SettingItem::FloatSlider {
                label: "Font Size",
                value: font.size,
                min: 8.0,
                max: 32.0,
                step: 0.5,
            },
            SettingItem::EnumChoice {
                label: "Font Family",
                options: &["JetBrains Mono", "Fira Code"],
                selected: match font.family {
                    FontFamily::JetBrainsMono => 0,
                    FontFamily::FiraCode => 1,
                },
            },
            SettingItem::IntSlider {
                label: "Line Padding",
                value: font.line_padding,
                min: 0,
                max: 10,
            },
        ]
    }

    fn theme_items(&self) -> Vec<SettingItem> {
        vec![SettingItem::EnumChoice {
            label: "Theme",
            options: &["Ferrum Dark", "Ferrum Light"],
            selected: match self.editing_config.theme {
                ThemeChoice::FerrumDark => 0,
                ThemeChoice::FerrumLight => 1,
            },
        }]
    }

    fn terminal_items(&self) -> Vec<SettingItem> {
        let terminal = &self.editing_config.terminal;
        vec![
            SettingItem::LargeIntSlider {
                label: "Max Scrollback",
                value: terminal.max_scrollback,
                min: 0,
                max: 50_000,
                step: 100,
            },
            SettingItem::LargeIntSlider {
                label: "Cursor Blink (ms)",
                value: terminal.cursor_blink_interval_ms as usize,
                min: 100,
                max: 2000,
                step: 50,
            },
        ]
    }

    fn layout_items(&self) -> Vec<SettingItem> {
        let layout = &self.editing_config.layout;
        vec![
            SettingItem::IntSlider {
                label: "Window Padding",
                value: layout.window_padding,
                min: 0,
                max: 32,
            },
            SettingItem::IntSlider {
                label: "Tab Bar Height",
                value: layout.tab_bar_height,
                min: 24,
                max: 60,
            },
            SettingItem::IntSlider {
                label: "Pane Padding",
                value: layout.pane_inner_padding,
                min: 0,
                max: 16,
            },
            SettingItem::IntSlider {
                label: "Scrollbar Width",
                value: layout.scrollbar_width,
                min: 2,
                max: 16,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_clones_config_into_both_fields() {
        let config = AppConfig::default();
        let overlay = SettingsOverlay::new(&config);
        assert_eq!(overlay.original_config.font.size, config.font.size);
        assert_eq!(overlay.editing_config.font.size, config.font.size);
        assert_eq!(overlay.active_category, SettingsCategory::Font);
        assert_eq!(overlay.scroll_offset, 0);
    }

    #[test]
    fn font_items_returns_three_settings() {
        let overlay = SettingsOverlay::new(&AppConfig::default());
        let items = overlay.items();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn theme_items_returns_one_setting() {
        let mut overlay = SettingsOverlay::new(&AppConfig::default());
        overlay.active_category = SettingsCategory::Theme;
        let items = overlay.items();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn terminal_items_returns_two_settings() {
        let mut overlay = SettingsOverlay::new(&AppConfig::default());
        overlay.active_category = SettingsCategory::Terminal;
        let items = overlay.items();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn layout_items_returns_four_settings() {
        let mut overlay = SettingsOverlay::new(&AppConfig::default());
        overlay.active_category = SettingsCategory::Layout;
        let items = overlay.items();
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn categories_array_has_four_entries() {
        assert_eq!(SettingsCategory::CATEGORIES.len(), 4);
    }

    #[test]
    fn display_labels_are_correct() {
        assert_eq!(SettingsCategory::Font.to_string(), "Font");
        assert_eq!(SettingsCategory::Theme.to_string(), "Theme");
        assert_eq!(SettingsCategory::Terminal.to_string(), "Terminal");
        assert_eq!(SettingsCategory::Layout.to_string(), "Layout");
    }

    #[test]
    fn font_family_enum_choice_maps_correctly() {
        let mut config = AppConfig::default();
        config.font.family = FontFamily::FiraCode;
        let overlay = SettingsOverlay::new(&config);
        let items = overlay.items();
        match &items[1] {
            SettingItem::EnumChoice { selected, .. } => assert_eq!(*selected, 1),
            _ => panic!("expected EnumChoice for Font Family"),
        }
    }

    #[test]
    fn theme_enum_choice_maps_correctly() {
        let mut config = AppConfig::default();
        config.theme = ThemeChoice::FerrumLight;
        let mut overlay = SettingsOverlay::new(&config);
        overlay.active_category = SettingsCategory::Theme;
        let items = overlay.items();
        match &items[0] {
            SettingItem::EnumChoice { selected, .. } => assert_eq!(*selected, 1),
            _ => panic!("expected EnumChoice for Theme"),
        }
    }
}
