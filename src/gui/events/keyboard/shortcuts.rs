use crate::gui::*;

impl FerrumWindow {
    fn physical_key_is(physical: &PhysicalKey, code: KeyCode) -> bool {
        matches!(physical, PhysicalKey::Code(current) if *current == code)
    }

    fn physical_digit_index(physical: &PhysicalKey) -> Option<usize> {
        let PhysicalKey::Code(code) = physical else {
            return None;
        };

        match code {
            KeyCode::Digit1 | KeyCode::Numpad1 => Some(0),
            KeyCode::Digit2 | KeyCode::Numpad2 => Some(1),
            KeyCode::Digit3 | KeyCode::Numpad3 => Some(2),
            KeyCode::Digit4 | KeyCode::Numpad4 => Some(3),
            KeyCode::Digit5 | KeyCode::Numpad5 => Some(4),
            KeyCode::Digit6 | KeyCode::Numpad6 => Some(5),
            KeyCode::Digit7 | KeyCode::Numpad7 => Some(6),
            KeyCode::Digit8 | KeyCode::Numpad8 => Some(7),
            KeyCode::Digit9 | KeyCode::Numpad9 => Some(8),
            _ => None,
        }
    }

    pub(in crate::gui::events::keyboard) fn handle_ctrl_shortcuts(
        &mut self,
        _event_loop: &ActiveEventLoop,
        key: &Key,
        physical: &PhysicalKey,
        _next_tab_id: &mut u64,
        _tx: &mpsc::Sender<PtyEvent>,
    ) -> bool {
        if !self.is_action_modifier() || self.modifiers.shift_key() {
            return false;
        }

        let is_copy_key = matches!(key, Key::Named(NamedKey::Copy))
            || Self::physical_key_is(physical, KeyCode::KeyC);
        if is_copy_key {
            if self.active_tab_ref().is_some_and(|t| t.selection.is_some()) {
                self.copy_selection();
                return true;
            }
            return false;
        }

        let is_paste_key = matches!(key, Key::Named(NamedKey::Paste))
            || Self::physical_key_is(physical, KeyCode::KeyV);
        if is_paste_key {
            self.paste_clipboard();
            return true;
        }

        if Self::physical_key_is(physical, KeyCode::KeyX) {
            return self.cut_selection();
        }

        if Self::physical_key_is(physical, KeyCode::KeyT) {
            #[cfg(target_os = "macos")]
            {
                self.pending_requests.push(WindowRequest::NewTab);
            }
            #[cfg(not(target_os = "macos"))]
            {
                let size = self.window.inner_size();
                let (rows, cols) = self.calc_grid_size(size.width, size.height);
                self.new_tab(rows, cols, _next_tab_id, _tx);
            }
            return true;
        }

        if Self::physical_key_is(physical, KeyCode::KeyW) {
            self.close_tab(self.active_tab);
            return true;
        }

        if Self::physical_key_is(physical, KeyCode::KeyN) {
            self.pending_requests.push(WindowRequest::NewWindow);
            return true;
        }

        if let Some(digit_index) = Self::physical_digit_index(physical) {
            #[cfg(target_os = "macos")]
            {
                if digit_index == 8 {
                    crate::gui::platform::macos::select_tab(&self.window, usize::MAX);
                } else {
                    crate::gui::platform::macos::select_tab(&self.window, digit_index);
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                if digit_index == 8 {
                    if !self.tabs.is_empty() {
                        self.active_tab = self.tabs.len() - 1;
                    }
                } else {
                    self.switch_tab(digit_index);
                }
            }
            return true;
        }

        if self.modifiers.super_key() {
            if Self::physical_key_is(physical, KeyCode::KeyA) {
                self.write_pty_bytes(b"\x01"); // Ctrl+A - beginning of line
                return true;
            }
            if Self::physical_key_is(physical, KeyCode::KeyE) {
                self.write_pty_bytes(b"\x05"); // Ctrl+E - end of line
                return true;
            }
            if Self::physical_key_is(physical, KeyCode::KeyB) {
                self.write_pty_bytes(b"\x1bb"); // Alt+B - previous word
                return true;
            }
            if Self::physical_key_is(physical, KeyCode::KeyF) {
                self.write_pty_bytes(b"\x1bf"); // Alt+F - next word
                return true;
            }
            if Self::physical_key_is(physical, KeyCode::KeyD) {
                self.write_pty_bytes(b"\x1bd"); // Alt+D - delete next word
                return true;
            }
            if Self::physical_key_is(physical, KeyCode::KeyK) {
                self.write_pty_bytes(b"\x0b"); // Ctrl+K - delete to end of line
                return true;
            }
            if Self::physical_key_is(physical, KeyCode::KeyU) {
                self.write_pty_bytes(b"\x15"); // Ctrl+U - delete to beginning of line
                return true;
            }
        }

        match key {
            Key::Named(NamedKey::Tab) => {
                #[cfg(target_os = "macos")]
                crate::gui::platform::macos::select_next_tab(&self.window);
                #[cfg(not(target_os = "macos"))]
                {
                    if !self.tabs.is_empty() {
                        self.active_tab = (self.active_tab + 1) % self.tabs.len();
                    }
                }
                true
            }
            // Cmd/Super text navigation on all platforms.
            Key::Named(NamedKey::ArrowLeft) if self.modifiers.super_key() => {
                self.write_pty_bytes(b"\x01"); // Ctrl+A - beginning of line
                true
            }
            Key::Named(NamedKey::ArrowRight) if self.modifiers.super_key() => {
                self.write_pty_bytes(b"\x05"); // Ctrl+E - end of line
                true
            }
            Key::Named(NamedKey::ArrowUp) if self.modifiers.super_key() => {
                // Scroll to top of scrollback.
                if let Some(tab) = self.active_tab_mut() {
                    tab.scroll_offset = tab.terminal.scrollback.len();
                }
                true
            }
            Key::Named(NamedKey::ArrowDown) if self.modifiers.super_key() => {
                // Scroll to bottom.
                if let Some(tab) = self.active_tab_mut() {
                    tab.scroll_offset = 0;
                }
                true
            }
            Key::Named(NamedKey::Backspace) if self.modifiers.super_key() => {
                self.write_pty_bytes(b"\x15"); // Ctrl+U - delete to beginning of line
                true
            }
            Key::Named(NamedKey::Delete) if self.modifiers.super_key() => {
                self.write_pty_bytes(b"\x0b"); // Ctrl+K - delete to end of line
                true
            }
            _ => false,
        }
    }

    pub(in crate::gui::events::keyboard) fn handle_ctrl_shift_shortcuts(
        &mut self,
        key: &Key,
        physical: &PhysicalKey,
        _next_tab_id: &mut u64,
        _tx: &mpsc::Sender<PtyEvent>,
    ) -> bool {
        if !self.is_action_modifier() || !self.modifiers.shift_key() {
            return false;
        }

        if Self::physical_key_is(physical, KeyCode::KeyT) {
            let Some(closed) = self.closed_tabs.pop() else {
                return true;
            };
            #[cfg(target_os = "macos")]
            {
                self.pending_requests.push(WindowRequest::ReopenTab {
                    title: closed.title,
                });
            }
            #[cfg(not(target_os = "macos"))]
            {
                let size = self.window.inner_size();
                let (rows, cols) = self.calc_grid_size(size.width, size.height);
                self.new_tab_with_title(rows, cols, Some(closed.title), _next_tab_id, _tx);
            }
            return true;
        }

        let is_copy_key = matches!(key, Key::Named(NamedKey::Copy))
            || Self::physical_key_is(physical, KeyCode::KeyC);
        if is_copy_key {
            self.copy_selection();
            return true;
        }

        let is_paste_key = matches!(key, Key::Named(NamedKey::Paste))
            || Self::physical_key_is(physical, KeyCode::KeyV);
        if is_paste_key {
            self.paste_clipboard();
            return true;
        }

        match key {
            Key::Named(NamedKey::Tab) => {
                #[cfg(target_os = "macos")]
                crate::gui::platform::macos::select_previous_tab(&self.window);
                #[cfg(not(target_os = "macos"))]
                {
                    if !self.tabs.is_empty() {
                        self.active_tab = if self.active_tab == 0 {
                            self.tabs.len() - 1
                        } else {
                            self.active_tab - 1
                        };
                    }
                }
                true
            }
            Key::Named(NamedKey::ArrowLeft) if self.modifiers.super_key() => {
                self.write_pty_bytes(b"\x01"); // Ctrl+A - beginning of line
                true
            }
            Key::Named(NamedKey::ArrowRight) if self.modifiers.super_key() => {
                self.write_pty_bytes(b"\x05"); // Ctrl+E - end of line
                true
            }
            _ => false,
        }
    }

    pub(in crate::gui::events::keyboard) fn handle_alt_shortcuts(
        &mut self,
        key: &Key,
        physical: &PhysicalKey,
    ) -> bool {
        if !self.modifiers.alt_key() {
            return false;
        }

        match key {
            Key::Named(NamedKey::Tab) => true, // Let Alt+Tab pass through to window manager.
            _ => {
                if let Some(digit_index) = Self::physical_digit_index(physical) {
                    self.switch_tab(digit_index);
                    true
                } else {
                    false
                }
            }
        }
    }
}
