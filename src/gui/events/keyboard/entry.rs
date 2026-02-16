use crate::gui::*;

impl FerrumWindow {
    pub(crate) fn on_keyboard_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: &winit::event::KeyEvent,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) {
        if event.state != ElementState::Pressed {
            return;
        }

        // Reset blink phase so the cursor is immediately visible after keypress.
        self.cursor_blink_start = std::time::Instant::now();

        let key = &event.logical_key;

        // Escape cancels tab drag.
        if matches!(key, Key::Named(NamedKey::Escape)) {
            if self.dragging_tab.is_some() {
                self.dragging_tab = None;
                self.window.set_cursor(CursorIcon::Default);
                self.window.request_redraw();
                return;
            }
        }

        // Rename mode consumes all key input before PTY forwarding.
        if self.handle_rename_input(key) {
            return; // Do not forward rename keystrokes to PTY.
        }

        if self.handle_ctrl_shortcuts(event_loop, key, next_tab_id, tx) {
            return;
        }
        if self.handle_ctrl_shift_shortcuts(key, next_tab_id, tx) {
            return;
        }
        if self.handle_alt_shortcuts(key) {
            return;
        }

        if Self::is_modifier_only_key(key) {
            return;
        }

        // On macOS, Cmd+key = app shortcuts only; never forward to terminal.
        #[cfg(target_os = "macos")]
        if self.modifiers.super_key() {
            return;
        }

        self.forward_key_to_pty(key);
    }
}
