#[cfg(not(target_os = "macos"))]
use crate::gui::renderer::WindowButton;
use crate::gui::renderer::{TabBarHit, TabInfo};
use crate::gui::*;

const TOPBAR_DOUBLE_CLICK_MS: u128 = 400;

impl FerrumWindow {
    fn tab_infos_for_hit_test(&self) -> Vec<TabInfo<'_>> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(idx, tab)| TabInfo {
                title: &tab.title,
                is_active: idx == self.active_tab,
                security_count: if tab.security.has_events() {
                    tab.security.active_event_count()
                } else {
                    0
                },
                hover_progress: 0.0,
                close_hover_progress: 0.0,
                is_renaming: false,
                rename_text: None,
                rename_cursor: 0,
                rename_selection: None,
            })
            .collect()
    }

    pub(in crate::gui::events::mouse) fn tab_bar_hit(&self, mx: f64, my: f64) -> TabBarHit {
        let buf_width = self.window.inner_size().width;
        self.backend
            .hit_test_tab_bar(mx, my, self.tabs.len(), buf_width)
    }

    pub(in crate::gui::events::mouse) fn tab_bar_security_hit(
        &self,
        mx: f64,
        my: f64,
    ) -> Option<usize> {
        let tab_infos = self.tab_infos_for_hit_test();
        if tab_infos.is_empty() {
            return None;
        }
        let buf_width = self.window.inner_size().width;
        self.backend
            .hit_test_tab_security_badge(mx, my, &tab_infos, buf_width)
    }

    pub(in crate::gui::events::mouse) fn handle_tab_bar_left_click(
        &mut self,
        _event_loop: &ActiveEventLoop,
        state: ElementState,
        mx: f64,
        my: f64,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) {
        if state != ElementState::Pressed {
            // Mouse release in tab bar area.
            // End any active pointer selection state so terminal drag-selection doesn't stick.
            if self.is_selecting && self.renaming_tab.is_none() && self.is_mouse_reporting() {
                // If PTY owns mouse reporting, still emit release even when cursor is over tab bar.
                let (row, col) = self.pixel_to_grid(mx, my);
                self.send_mouse_event(0, col, row, false);
            }
            self.is_selecting = false;
            self.selection_anchor = None;
            self.finish_drag();
            return;
        }

        if let Some(tab_idx) = self.tab_bar_security_hit(mx, my) {
            self.cancel_drag();
            self.commit_rename();
            self.last_topbar_empty_click = None;
            self.open_security_popup_for_tab(tab_idx);
            return;
        }

        let hit = self.tab_bar_hit(mx, my);

        // Check if the click landed inside the rename text field area.
        // If so, handle cursor positioning instead of normal tab bar interaction.
        if self.renaming_tab.is_some() {
            if let TabBarHit::Tab(idx) = hit {
                if self
                    .renaming_tab
                    .as_ref()
                    .is_some_and(|r| r.tab_index == idx)
                {
                    self.handle_rename_field_click(mx);
                    return;
                }
            }
        }

        // Commit any active rename before processing the click (blur behavior).
        let had_rename = self.renaming_tab.is_some();
        self.commit_rename();

        match hit {
            TabBarHit::Tab(idx) => {
                self.handle_tab_click(idx, mx, my, had_rename);
            }
            TabBarHit::CloseTab(idx) => {
                self.last_topbar_empty_click = None;
                self.close_tab(idx);
            }
            TabBarHit::NewTab => {
                self.last_topbar_empty_click = None;
                let size = self.window.inner_size();
                let (rows, cols) = self.calc_grid_size(size.width, size.height);
                self.new_tab(rows, cols, next_tab_id, tx);
            }
            #[cfg(not(target_os = "macos"))]
            TabBarHit::WindowButton(btn) => {
                self.handle_window_button_click(btn);
            }
            TabBarHit::Empty => {
                self.handle_empty_bar_click();
            }
            #[cfg(not(target_os = "macos"))]
            TabBarHit::PinButton => {
                self.last_topbar_empty_click = None;
                self.last_tab_click = None;
                self.toggle_pin();
            }
        }
    }

    /// Handles a click on a tab: double-click rename, switch, drag.
    fn handle_tab_click(&mut self, idx: usize, mx: f64, my: f64, had_rename: bool) {
        self.last_topbar_empty_click = None;
        let now = std::time::Instant::now();
        if self.last_tab_click.is_some_and(|(last_idx, last_time)| {
            last_idx == idx && now.duration_since(last_time).as_millis() < 400
        }) {
            self.start_rename(idx);
            self.last_tab_click = None;
            return;
        }
        self.last_tab_click = Some((idx, now));
        self.switch_tab(idx);
        self.start_drag(idx, mx, my, had_rename);
    }

    /// Handles a click on a window button (minimize/maximize/close).
    #[cfg(not(target_os = "macos"))]
    fn handle_window_button_click(&mut self, btn: WindowButton) {
        self.last_topbar_empty_click = None;
        self.last_tab_click = None;
        match btn {
            WindowButton::Minimize => {
                self.window.set_minimized(true);
            }
            WindowButton::Maximize => {
                let maximized = self.window.is_maximized();
                self.window.set_maximized(!maximized);
            }
            WindowButton::Close => {
                self.pending_requests.push(WindowRequest::CloseWindow);
            }
        }
    }

    /// Handles a click on the empty bar area: double-click maximize, drag.
    fn handle_empty_bar_click(&mut self) {
        self.last_tab_click = None;
        let now = std::time::Instant::now();
        let is_double_click = self.last_topbar_empty_click.is_some_and(|last| {
            now.duration_since(last).as_millis() < TOPBAR_DOUBLE_CLICK_MS
        });
        if is_double_click {
            self.last_topbar_empty_click = None;
            let maximized = self.window.is_maximized();
            self.window.set_maximized(!maximized);
        } else {
            self.last_topbar_empty_click = Some(now);
            let _ = self.window.drag_window();
        }
    }

    /// Handles a mouse click inside the rename text field: positions cursor, clears selection.
    /// Also detects double-click (word select) and triple-click (select all).
    fn handle_rename_field_click(&mut self, mx: f64) {
        let Some(rename) = self.renaming_tab.as_mut() else {
            return;
        };

        let buf_width = self.window.inner_size().width;
        let tw = self.backend.tab_width(self.tabs.len(), buf_width);
        let tab_padding_h = self.backend.scaled_px(14);
        let text_x = self.backend.tab_origin_x(rename.tab_index, tw) + tab_padding_h;

        // Calculate cursor byte position from mouse x coordinate.
        let char_offset = if mx < text_x as f64 {
            0
        } else {
            ((mx as u32 - text_x + self.backend.cell_width() / 2) / self.backend.cell_width())
                as usize
        };
        let byte_pos = rename
            .text
            .char_indices()
            .nth(char_offset)
            .map(|(i, _)| i)
            .unwrap_or(rename.text.len());

        // Multi-click detection within the rename field.
        let now = std::time::Instant::now();
        let is_multi = self.last_tab_click.is_some_and(|(last_idx, last_time)| {
            last_idx == rename.tab_index && now.duration_since(last_time).as_millis() < 400
        });

        if is_multi {
            // Count rapid clicks: 2 = word select, 3+ = select all.
            let click_count = self.click_streak.saturating_add(1);
            self.click_streak = click_count;
            self.last_tab_click = Some((rename.tab_index, now));

            if click_count >= 3 {
                // Triple-click: select all.
                rename.selection_anchor = Some(0);
                rename.cursor = rename.text.len();
                self.click_streak = 0; // Reset streak.
                self.last_tab_click = None;
            } else {
                // Double-click: select word under cursor.
                let left = self.rename_word_left_boundary(byte_pos);
                let right = self.rename_word_right_boundary(byte_pos);
                let rename = self.renaming_tab.as_mut().unwrap();
                rename.selection_anchor = Some(left);
                rename.cursor = right;
            }
        } else {
            // Single click: position cursor, clear selection, arm drag.
            self.click_streak = 1;
            self.last_tab_click = Some((rename.tab_index, now));
            rename.selection_anchor = Some(byte_pos);
            rename.cursor = byte_pos;
            self.is_selecting = true;
        }
    }

    /// Updates rename cursor during mouse drag to create text selection.
    pub(in crate::gui::events::mouse) fn handle_rename_field_drag(&mut self, mx: f64) {
        let Some(rename) = self.renaming_tab.as_mut() else {
            return;
        };

        let buf_width = self.window.inner_size().width;
        let tw = self.backend.tab_width(self.tabs.len(), buf_width);
        let tab_padding_h = self.backend.scaled_px(14);
        let text_x = self.backend.tab_origin_x(rename.tab_index, tw) + tab_padding_h;

        let char_offset = if mx < text_x as f64 {
            0
        } else {
            ((mx as u32 - text_x + self.backend.cell_width() / 2) / self.backend.cell_width())
                as usize
        };
        let byte_pos = rename
            .text
            .char_indices()
            .nth(char_offset)
            .map(|(i, _)| i)
            .unwrap_or(rename.text.len());

        // selection_anchor was set on mouse press; only move cursor.
        rename.cursor = byte_pos;
    }

    /// Finds the left word boundary in the rename text at the given byte position.
    fn rename_word_left_boundary(&self, byte_pos: usize) -> usize {
        let Some(rename) = self.renaming_tab.as_ref() else {
            return 0;
        };
        let text = &rename.text;
        let mut idx = byte_pos.min(text.len());

        // Skip whitespace to the left.
        while idx > 0 {
            let prev = text[..idx]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            if !text[prev..idx]
                .chars()
                .next()
                .unwrap_or(' ')
                .is_whitespace()
            {
                break;
            }
            idx = prev;
        }
        // Skip word chars to the left.
        while idx > 0 {
            let prev = text[..idx]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            if text[prev..idx]
                .chars()
                .next()
                .unwrap_or(' ')
                .is_whitespace()
            {
                break;
            }
            idx = prev;
        }
        idx
    }

    /// Finds the right word boundary in the rename text at the given byte position.
    fn rename_word_right_boundary(&self, byte_pos: usize) -> usize {
        let Some(rename) = self.renaming_tab.as_ref() else {
            return 0;
        };
        let text = &rename.text;
        let mut idx = byte_pos.min(text.len());

        // Skip whitespace to the right.
        while idx < text.len() {
            let next = idx + text[idx..].chars().next().map_or(0, char::len_utf8);
            if !text[idx..next]
                .chars()
                .next()
                .unwrap_or(' ')
                .is_whitespace()
            {
                break;
            }
            idx = next;
        }
        // Skip word chars to the right.
        while idx < text.len() {
            let next = idx + text[idx..].chars().next().map_or(0, char::len_utf8);
            if text[idx..next]
                .chars()
                .next()
                .unwrap_or(' ')
                .is_whitespace()
            {
                break;
            }
            idx = next;
        }
        idx
    }
}
