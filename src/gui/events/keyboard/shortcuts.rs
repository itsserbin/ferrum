use crate::gui::*;

impl FerrumWindow {
    pub(in crate::gui::events::keyboard) fn handle_ctrl_shortcuts(
        &mut self,
        _event_loop: &ActiveEventLoop,
        key: &Key,
        _next_tab_id: &mut u64,
        _tx: &mpsc::Sender<PtyEvent>,
    ) -> bool {
        if !self.is_action_modifier() || self.modifiers.shift_key() {
            return false;
        }

        match key {
            Key::Character(c) if c.as_str() == "t" => {
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
                true
            }
            Key::Character(c) if c.as_str() == "w" => {
                self.close_tab(self.active_tab);
                true
            }
            #[cfg(target_os = "macos")]
            Key::Character(c) if c.as_str() == "n" => {
                self.pending_requests.push(WindowRequest::NewWindow);
                true
            }
            Key::Character(c) => {
                let digit = c
                    .as_str()
                    .chars()
                    .next()
                    .and_then(|ch| ch.to_digit(10))
                    .filter(|digit| (1..=9).contains(digit));
                if let Some(digit) = digit {
                    #[cfg(target_os = "macos")]
                    {
                        if digit == 9 {
                            crate::gui::platform::macos::select_tab(&self.window, usize::MAX);
                        } else {
                            crate::gui::platform::macos::select_tab(&self.window, (digit - 1) as usize);
                        }
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        if digit == 9 {
                            if !self.tabs.is_empty() {
                                self.active_tab = self.tabs.len() - 1;
                            }
                        } else {
                            self.switch_tab((digit - 1) as usize);
                        }
                    }
                    true
                } else if c.as_str() == "c" {
                    if self.active_tab_ref().is_some_and(|t| t.selection.is_some()) {
                        self.copy_selection_and_clear();
                        true
                    } else {
                        false
                    }
                } else if c.as_str() == "v" {
                    self.paste_clipboard();
                    true
                } else if self.modifiers.super_key() && c.as_str().eq_ignore_ascii_case("a") {
                    self.write_pty_bytes(b"\x01"); // Ctrl+A — beginning of line
                    true
                } else if self.modifiers.super_key() && c.as_str().eq_ignore_ascii_case("e") {
                    self.write_pty_bytes(b"\x05"); // Ctrl+E — end of line
                    true
                } else if self.modifiers.super_key() && c.as_str().eq_ignore_ascii_case("b") {
                    self.write_pty_bytes(b"\x1bb"); // Alt+B — previous word
                    true
                } else if self.modifiers.super_key() && c.as_str().eq_ignore_ascii_case("f") {
                    self.write_pty_bytes(b"\x1bf"); // Alt+F — next word
                    true
                } else if self.modifiers.super_key() && c.as_str().eq_ignore_ascii_case("d") {
                    self.write_pty_bytes(b"\x1bd"); // Alt+D — delete next word
                    true
                } else if self.modifiers.super_key() && c.as_str().eq_ignore_ascii_case("k") {
                    self.write_pty_bytes(b"\x0b"); // Ctrl+K — delete to end of line
                    true
                } else if self.modifiers.super_key() && c.as_str().eq_ignore_ascii_case("u") {
                    self.write_pty_bytes(b"\x15"); // Ctrl+U — delete to beginning of line
                    true
                } else {
                    false
                }
            }
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
                self.write_pty_bytes(b"\x01"); // Ctrl+A — beginning of line
                true
            }
            Key::Named(NamedKey::ArrowRight) if self.modifiers.super_key() => {
                self.write_pty_bytes(b"\x05"); // Ctrl+E — end of line
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
                self.write_pty_bytes(b"\x15"); // Ctrl+U — delete to beginning of line
                true
            }
            Key::Named(NamedKey::Delete) if self.modifiers.super_key() => {
                self.write_pty_bytes(b"\x0b"); // Ctrl+K — delete to end of line
                true
            }
            _ => false,
        }
    }

    pub(in crate::gui::events::keyboard) fn handle_ctrl_shift_shortcuts(
        &mut self,
        key: &Key,
        _next_tab_id: &mut u64,
        _tx: &mpsc::Sender<PtyEvent>,
    ) -> bool {
        if !self.is_action_modifier() || !self.modifiers.shift_key() {
            return false;
        }

        match key {
            Key::Character(c) if c.as_str() == "T" || c.as_str() == "t" => {
                let Some(closed) = self.closed_tabs.pop() else {
                    return true;
                };
                #[cfg(target_os = "macos")]
                {
                    self.pending_requests
                        .push(WindowRequest::ReopenTab { title: closed.title });
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let size = self.window.inner_size();
                    let (rows, cols) = self.calc_grid_size(size.width, size.height);
                    self.new_tab_with_title(rows, cols, Some(closed.title), _next_tab_id, _tx);
                }
                true
            }
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
                self.write_pty_bytes(b"\x01"); // Ctrl+A — beginning of line
                true
            }
            Key::Named(NamedKey::ArrowRight) if self.modifiers.super_key() => {
                self.write_pty_bytes(b"\x05"); // Ctrl+E — end of line
                true
            }
            Key::Character(c) if c.as_str() == "C" || c.as_str() == "c" => {
                self.copy_selection_and_clear();
                true
            }
            Key::Character(c) if c.as_str() == "V" || c.as_str() == "v" => {
                self.paste_clipboard();
                true
            }
            _ => false,
        }
    }

    pub(in crate::gui::events::keyboard) fn handle_alt_shortcuts(&mut self, key: &Key) -> bool {
        if !self.modifiers.alt_key() {
            return false;
        }

        match key {
            Key::Named(NamedKey::Tab) => true, // Let Alt+Tab pass through to window manager.
            Key::Character(c) => match c
                .as_str()
                .chars()
                .next()
                .and_then(|ch| ch.to_digit(10))
                .filter(|digit| (1..=9).contains(digit))
            {
                Some(digit) => {
                    self.switch_tab((digit - 1) as usize);
                    true
                }
                None => false,
            },
            _ => false,
        }
    }
}
