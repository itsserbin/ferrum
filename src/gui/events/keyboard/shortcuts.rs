use crate::gui::*;

impl FerrumWindow {
    pub(in crate::gui::events::keyboard) fn handle_ctrl_shortcuts(
        &mut self,
        _event_loop: &ActiveEventLoop,
        key: &Key,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) -> bool {
        if !self.modifiers.control_key() || self.modifiers.shift_key() {
            return false;
        }

        match key {
            Key::Character(c) if c.as_str() == "t" => {
                let size = self.window.inner_size();
                let (rows, cols) = self.calc_grid_size(size.width, size.height);
                self.new_tab(rows, cols, next_tab_id, tx);
                true
            }
            Key::Character(c) if c.as_str() == "w" => {
                self.close_tab(self.active_tab);
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
                    if digit == 9 {
                        if !self.tabs.is_empty() {
                            self.active_tab = self.tabs.len() - 1;
                        }
                    } else {
                        self.switch_tab((digit - 1) as usize);
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
                } else {
                    false
                }
            }
            Key::Named(NamedKey::Tab) => {
                if !self.tabs.is_empty() {
                    self.active_tab = (self.active_tab + 1) % self.tabs.len();
                }
                true
            }
            _ => false,
        }
    }

    pub(in crate::gui::events::keyboard) fn handle_ctrl_shift_shortcuts(
        &mut self,
        key: &Key,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) -> bool {
        if !self.modifiers.control_key() || !self.modifiers.shift_key() {
            return false;
        }

        match key {
            Key::Character(c) if c.as_str() == "T" || c.as_str() == "t" => {
                let Some(closed) = self.closed_tabs.pop() else {
                    return true;
                };
                let size = self.window.inner_size();
                let (rows, cols) = self.calc_grid_size(size.width, size.height);
                self.new_tab_with_title(rows, cols, Some(closed.title), next_tab_id, tx);
                true
            }
            Key::Named(NamedKey::Tab) => {
                if !self.tabs.is_empty() {
                    self.active_tab = if self.active_tab == 0 {
                        self.tabs.len() - 1
                    } else {
                        self.active_tab - 1
                    };
                }
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
