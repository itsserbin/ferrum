use crate::gui::input::encode_mouse_event;
use crate::gui::*;

impl FerrumWindow {
    /// Sends a mouse event to the active tab PTY.
    pub(in crate::gui) fn send_mouse_event(
        &mut self,
        button: u8,
        col: usize,
        row: usize,
        pressed: bool,
    ) {
        let sgr = self.active_tab_ref().is_some_and(|t| t.terminal.sgr_mouse);
        let bytes = encode_mouse_event(button, col, row, pressed, sgr);
        if let Some(tab) = self.active_tab_mut() {
            let _ = tab.pty_writer.write_all(&bytes);
            let _ = tab.pty_writer.flush();
        }
    }

    /// Returns whether terminal mouse tracking is active (Shift forces local selection mode).
    pub(in crate::gui) fn is_mouse_reporting(&self) -> bool {
        let mode = self
            .active_tab_ref()
            .map_or(MouseMode::Off, |t| t.terminal.mouse_mode);
        mode != MouseMode::Off && !self.modifiers.shift_key()
    }
}
