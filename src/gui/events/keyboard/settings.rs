use crate::config::AppConfig;
use crate::gui::settings::{SettingsCategory, SettingsOverlay};
use crate::gui::*;

impl FerrumWindow {
    /// Toggles the settings overlay open/closed.
    pub(in crate::gui) fn toggle_settings_overlay(&mut self, config: &AppConfig) {
        if self.settings_overlay.is_some() {
            self.close_settings_overlay();
        } else {
            self.settings_overlay = Some(SettingsOverlay::new(config));
            self.window.request_redraw();
        }
    }

    /// Handles keyboard input when the settings overlay is open.
    /// Returns `true` if the event was consumed.
    pub(super) fn handle_settings_keyboard(&mut self, key: &Key) -> bool {
        let Some(overlay) = self.settings_overlay.as_mut() else {
            return false;
        };

        match key {
            Key::Named(NamedKey::Escape) => {
                self.close_settings_overlay();
                true
            }
            Key::Named(NamedKey::ArrowUp) => {
                let cats = SettingsCategory::CATEGORIES;
                let current_idx = cats
                    .iter()
                    .position(|c| *c == overlay.active_category)
                    .unwrap_or(0);
                if current_idx > 0 {
                    overlay.active_category = cats[current_idx - 1];
                    overlay.hovered_item = None;
                    overlay.scroll_offset = 0;
                }
                self.window.request_redraw();
                true
            }
            Key::Named(NamedKey::ArrowDown) => {
                let cats = SettingsCategory::CATEGORIES;
                let current_idx = cats
                    .iter()
                    .position(|c| *c == overlay.active_category)
                    .unwrap_or(0);
                if current_idx + 1 < cats.len() {
                    overlay.active_category = cats[current_idx + 1];
                    overlay.hovered_item = None;
                    overlay.scroll_offset = 0;
                }
                self.window.request_redraw();
                true
            }
            _ => true, // Consume all other keys when overlay is open
        }
    }
}
