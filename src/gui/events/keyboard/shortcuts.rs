use crate::config::AppConfig;
use crate::gui::pane::{NavigateDirection, SplitDirection};
use crate::gui::*;

impl FerrumWindow {
    pub(super) fn physical_key_is(physical: &PhysicalKey, code: KeyCode) -> bool {
        matches!(physical, PhysicalKey::Code(current) if *current == code)
    }

    pub(super) fn physical_digit_index(physical: &PhysicalKey) -> Option<usize> {
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

    // ── Main dispatchers ────────────────────────────────────────────────

    pub(in crate::gui::events::keyboard) fn handle_ctrl_shortcuts(
        &mut self,
        key: &Key,
        physical: &PhysicalKey,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
        config: &AppConfig,
    ) -> bool {
        if !self.is_action_modifier() || self.modifiers.shift_key() {
            return false;
        }

        if let Some(result) = self.handle_clipboard_shortcuts(key, physical) {
            return result;
        }
        if let Some(result) = self.handle_tab_management_shortcuts(key, physical, next_tab_id, tx, config) {
            return result;
        }
        if self.handle_super_text_shortcuts(physical) {
            return true;
        }
        if let Some(result) = self.handle_super_navigation_shortcuts(key) {
            return result;
        }
        if Self::physical_key_is(physical, KeyCode::Comma) {
            self.toggle_settings_overlay(config);
            return true;
        }
        false
    }

    pub(in crate::gui::events::keyboard) fn handle_ctrl_shift_shortcuts(
        &mut self,
        key: &Key,
        physical: &PhysicalKey,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
        config: &AppConfig,
    ) -> bool {
        if !self.is_action_modifier() || !self.modifiers.shift_key() {
            return false;
        }

        // Pin/Unpin window (always-on-top toggle).
        if Self::physical_key_is(physical, KeyCode::KeyP) {
            self.toggle_pin();
            return true;
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
                self.new_tab_with_title(rows, cols, Some(closed.title), next_tab_id, tx, None, config);
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

        // ── Pane splitting ────────────────────────────────────────────────
        if Self::physical_key_is(physical, KeyCode::KeyR) {
            self.split_pane(SplitDirection::Horizontal, false, next_tab_id, tx, config);
            return true;
        }
        if Self::physical_key_is(physical, KeyCode::KeyD) {
            self.split_pane(SplitDirection::Vertical, false, next_tab_id, tx, config);
            return true;
        }
        if Self::physical_key_is(physical, KeyCode::KeyL) {
            self.split_pane(SplitDirection::Horizontal, true, next_tab_id, tx, config);
            return true;
        }
        if Self::physical_key_is(physical, KeyCode::KeyU) {
            self.split_pane(SplitDirection::Vertical, true, next_tab_id, tx, config);
            return true;
        }

        // ── Close terminal window ─────────────────────────────────────────
        if Self::physical_key_is(physical, KeyCode::KeyW) {
            self.request_close_window();
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
            // ── Pane navigation (arrow keys) ──────────────────────────────
            Key::Named(NamedKey::ArrowUp) if !self.modifiers.super_key() => {
                self.navigate_pane(NavigateDirection::Up);
                true
            }
            Key::Named(NamedKey::ArrowDown) if !self.modifiers.super_key() => {
                self.navigate_pane(NavigateDirection::Down);
                true
            }
            Key::Named(NamedKey::ArrowLeft) if !self.modifiers.super_key() => {
                self.navigate_pane(NavigateDirection::Left);
                true
            }
            Key::Named(NamedKey::ArrowRight) if !self.modifiers.super_key() => {
                self.navigate_pane(NavigateDirection::Right);
                true
            }
            // ── macOS line navigation (Cmd+Shift+Arrow) ───────────────────
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
