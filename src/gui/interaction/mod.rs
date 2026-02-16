mod clipboard;
mod cursor_move;
mod geometry;
mod mouse_reporting;
mod selection;

use crate::gui::FerrumWindow;

impl FerrumWindow {
    /// Returns `true` when the platform's "action" modifier is held.
    ///
    /// On macOS, both Ctrl and Cmd trigger app-level shortcuts (Cmd+C, Ctrl+C copy).
    /// On other platforms, only Ctrl is used.
    pub(in crate::gui) fn is_action_modifier(&self) -> bool {
        if cfg!(target_os = "macos") {
            self.modifiers.control_key() || self.modifiers.super_key()
        } else {
            self.modifiers.control_key()
        }
    }
}
