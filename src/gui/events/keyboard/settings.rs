use crate::config::AppConfig;
#[cfg(not(target_os = "macos"))]
use crate::gui::settings::SettingsOverlay;
use crate::gui::settings::SettingsCategory;
use crate::gui::*;

impl FerrumWindow {
    /// Toggles the settings overlay open/closed.
    ///
    /// On macOS, opens the native settings window instead of the in-app overlay.
    pub(in crate::gui) fn toggle_settings_overlay(&mut self, config: &AppConfig) {
        #[cfg(target_os = "macos")]
        {
            use crate::gui::platform::macos::settings_window;
            if settings_window::is_settings_window_open() {
                settings_window::close_settings_window();
            } else {
                settings_window::open_settings_window(config, self.settings_tx.clone());
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            if self.settings_overlay.is_some() {
                self.close_settings_overlay();
            } else {
                self.settings_overlay = Some(SettingsOverlay::new(config));
                self.window.request_redraw();
            }
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
