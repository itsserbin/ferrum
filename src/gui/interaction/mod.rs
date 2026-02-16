mod clipboard;
mod cursor_move;
mod geometry;
mod mouse_reporting;
mod selection;

use crate::gui::FerrumWindow;

impl FerrumWindow {
    /// Returns `true` when the platform's "action" modifier is held.
    ///
    /// We treat both Ctrl and Super/Cmd as action modifiers on every platform.
    /// This keeps terminal/app shortcuts consistent for external keyboards with Cmd keys.
    pub(in crate::gui) fn is_action_modifier(&self) -> bool {
        self.modifiers.control_key() || self.modifiers.super_key()
    }
}
