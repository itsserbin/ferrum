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

        let key = Self::normalize_non_text_key(&event.logical_key, &event.physical_key);

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
        if self.handle_rename_input(&key) {
            return; // Do not forward rename keystrokes to PTY.
        }

        if self.handle_selection_delete_key(&key) {
            return;
        }
        if self.handle_word_delete_key(&key) {
            return;
        }

        if self.handle_ctrl_shortcuts(event_loop, &key, &event.physical_key, next_tab_id, tx) {
            return;
        }
        if self.handle_ctrl_shift_shortcuts(&key, &event.physical_key, next_tab_id, tx) {
            return;
        }
        if self.handle_alt_shortcuts(&key, &event.physical_key) {
            return;
        }

        if self.handle_shift_arrow_selection(&key) {
            return;
        }

        if Self::is_modifier_only_key(&key) {
            return;
        }

        if !self.modifiers.shift_key() {
            self.keyboard_selection_anchor = None;
        }

        // Super/Cmd+key combinations are app-level shortcuts only; never forward to terminal.
        if self.modifiers.super_key() {
            return;
        }

        self.forward_key_to_pty(&key);
    }
}
