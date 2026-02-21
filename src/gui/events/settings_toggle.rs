use crate::config::AppConfig;
use crate::gui::*;

impl FerrumWindow {
    /// Toggles the native settings window open/closed.
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

        #[cfg(target_os = "windows")]
        {
            use crate::gui::platform::windows::settings_window;
            if settings_window::is_settings_window_open() {
                settings_window::close_settings_window();
            } else {
                settings_window::open_settings_window(config, self.settings_tx.clone());
            }
        }

        #[cfg(target_os = "linux")]
        {
            use crate::gui::platform::linux::settings_window;
            if settings_window::is_settings_window_open() {
                settings_window::close_settings_window();
            } else {
                settings_window::open_settings_window(config, self.settings_tx.clone());
            }
        }
    }
}
