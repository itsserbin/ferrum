use crate::gui::renderer::{TabBarHit, TabInfo};
use crate::gui::*;

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
                is_renaming: false,
                rename_text: None,
                rename_cursor: 0,
                rename_selection: None,
            })
            .collect()
    }

    fn tab_bar_hit_candidates(&self, mx: f64, my: f64) -> [(f64, f64); 2] {
        let scale = self.window.scale_factor();
        if (scale - 1.0).abs() < f64::EPSILON {
            [(mx, my), (mx, my)]
        } else {
            [(mx, my), (mx * scale, my * scale)]
        }
    }

    pub(in crate::gui::events::mouse) fn is_window_close_button_with_fallback(
        &self,
        mx: f64,
        my: f64,
    ) -> bool {
        let buf_width = self.window.inner_size().width as usize;
        for (i, (x, y)) in self.tab_bar_hit_candidates(mx, my).into_iter().enumerate() {
            if i == 1 && (x - mx).abs() < 0.5 && (y - my).abs() < 0.5 {
                continue;
            }
            if self.renderer.is_window_close_button(x, y, buf_width) {
                return true;
            }
        }
        false
    }

    pub(in crate::gui::events::mouse) fn tab_bar_hit_with_fallback(
        &self,
        mx: f64,
        my: f64,
    ) -> TabBarHit {
        let buf_width = self.window.inner_size().width;
        for (i, (x, y)) in self.tab_bar_hit_candidates(mx, my).into_iter().enumerate() {
            if i == 1 && (x - mx).abs() < 0.5 && (y - my).abs() < 0.5 {
                continue;
            }
            let hit = self
                .renderer
                .hit_test_tab_bar(x, y, self.tabs.len(), buf_width);
            if !matches!(hit, TabBarHit::Empty) {
                return hit;
            }
        }
        TabBarHit::Empty
    }

    pub(in crate::gui::events::mouse) fn tab_bar_security_hit_with_fallback(
        &self,
        mx: f64,
        my: f64,
    ) -> Option<usize> {
        let tab_infos = self.tab_infos_for_hit_test();
        if tab_infos.is_empty() {
            return None;
        }
        let buf_width = self.window.inner_size().width;
        for (i, (x, y)) in self.tab_bar_hit_candidates(mx, my).into_iter().enumerate() {
            if i == 1 && (x - mx).abs() < 0.5 && (y - my).abs() < 0.5 {
                continue;
            }
            if let Some(hit) = self
                .renderer
                .hit_test_tab_security_badge(x, y, &tab_infos, buf_width)
            {
                return Some(hit);
            }
        }
        None
    }

    fn open_security_popup_for_tab(&mut self, tab_index: usize) {
        let Some(tab) = self.tabs.get_mut(tab_index) else {
            self.security_popup = None;
            return;
        };
        let events = tab.security.take_active_events();
        if events.is_empty() {
            self.security_popup = None;
            return;
        }

        let event_count = events.len();
        let mut lines = Vec::with_capacity(events.len());
        for event in events.iter().rev() {
            let age = event.timestamp.elapsed().as_secs();
            lines.push(format!("{} ({}s ago)", event.kind.label(), age));
        }

        let buf_width = self.window.inner_size().width;
        let (popup_x, popup_y) = self
            .renderer
            .security_badge_rect(tab_index, self.tabs.len(), buf_width, event_count)
            .map(|(x, y, w, h)| (x.saturating_sub(w), y + h + 6))
            .unwrap_or((16, TAB_BAR_HEIGHT + 6));

        self.security_popup = Some(SecurityPopup {
            tab_index,
            x: popup_x,
            y: popup_y,
            title: "Security events",
            lines,
        });
        self.context_menu = None;
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
            if self.is_selecting && self.renaming_tab.is_some() {
                self.is_selecting = false;
            }
            self.handle_tab_drag_release();
            return;
        }

        // Window close button has priority over tab hit-test.
        if self.is_window_close_button_with_fallback(mx, my) {
            self.dragging_tab = None;
            self.commit_rename();
            self.pending_requests.push(WindowRequest::CloseWindow);
            return;
        }

        if let Some(tab_idx) = self.tab_bar_security_hit_with_fallback(mx, my) {
            self.dragging_tab = None;
            self.commit_rename();
            self.open_security_popup_for_tab(tab_idx);
            return;
        }

        let hit = self.tab_bar_hit_with_fallback(mx, my);

        // Check if the click landed inside the rename text field area.
        // If so, handle cursor positioning instead of normal tab bar interaction.
        if self.renaming_tab.is_some() {
            if let TabBarHit::Tab(idx) = hit {
                if self.renaming_tab.as_ref().is_some_and(|r| r.tab_index == idx) {
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
                // Double-click starts inline rename.
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

                // Arm potential drag if there are at least 2 tabs and no rename was just committed.
                if self.tabs.len() > 1 && !had_rename {
                    self.dragging_tab = Some(DragState {
                        source_index: idx,
                        start_x: mx,
                        start_y: my,
                        current_x: mx,
                        current_y: my,
                        is_active: false,
                    });
                }
            }
            TabBarHit::CloseTab(idx) => {
                self.close_tab(idx);
            }
            TabBarHit::NewTab => {
                let size = self.window.inner_size();
                let (rows, cols) = self.calc_grid_size(size.width, size.height);
                self.new_tab(rows, cols, next_tab_id, tx);
            }
            TabBarHit::Empty => {
                let _ = self.window.drag_window();
            }
        }
    }

    /// Handles mouse release: drops the tab at the new position or treats as normal click.
    fn handle_tab_drag_release(&mut self) {
        let Some(drag) = self.dragging_tab.take() else {
            return;
        };
        if !drag.is_active {
            return; // Was never activated (< 5px movement), normal click already happened.
        }

        let source = drag.source_index;
        let buf_width = self.window.inner_size().width;
        let tw = self.renderer.tab_width(self.tabs.len(), buf_width);
        let tab_count = self.tabs.len();

        // Calculate insertion index.
        let mut insert_at = tab_count;
        for i in 0..tab_count {
            let tab_center = i as f64 * tw as f64 + tw as f64 / 2.0;
            if drag.current_x < tab_center {
                insert_at = i;
                break;
            }
        }

        // Convert insertion index to the actual destination after removal.
        let dest = if insert_at > source {
            (insert_at - 1).min(tab_count - 1)
        } else {
            insert_at
        };

        if dest != source && dest < tab_count {
            let tab = self.tabs.remove(source);
            self.tabs.insert(dest, tab);

            if self.active_tab == source {
                self.active_tab = dest;
            } else if source < self.active_tab && dest >= self.active_tab {
                self.active_tab -= 1;
            } else if source > self.active_tab && dest <= self.active_tab {
                self.active_tab += 1;
            }
        }

        // Always restore cursor after drag ends.
        self.window.set_cursor(CursorIcon::Default);
    }

    /// Handles a mouse click inside the rename text field: positions cursor, clears selection.
    /// Also detects double-click (word select) and triple-click (select all).
    fn handle_rename_field_click(&mut self, mx: f64) {
        let Some(rename) = self.renaming_tab.as_mut() else {
            return;
        };

        let buf_width = self.window.inner_size().width;
        let tw = self.renderer.tab_width(self.tabs.len(), buf_width);
        let tab_padding_h = 14u32;
        let text_x = rename.tab_index as u32 * tw + tab_padding_h;

        // Calculate cursor byte position from mouse x coordinate.
        let char_offset = if mx < text_x as f64 {
            0
        } else {
            ((mx as u32 - text_x + self.renderer.cell_width / 2) / self.renderer.cell_width)
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
        let tw = self.renderer.tab_width(self.tabs.len(), buf_width);
        let tab_padding_h = 14u32;
        let text_x = rename.tab_index as u32 * tw + tab_padding_h;

        let char_offset = if mx < text_x as f64 {
            0
        } else {
            ((mx as u32 - text_x + self.renderer.cell_width / 2) / self.renderer.cell_width)
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
            if !text[prev..idx].chars().next().unwrap_or(' ').is_whitespace() {
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
            if text[prev..idx].chars().next().unwrap_or(' ').is_whitespace() {
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
            if !text[idx..next].chars().next().unwrap_or(' ').is_whitespace() {
                break;
            }
            idx = next;
        }
        // Skip word chars to the right.
        while idx < text.len() {
            let next = idx + text[idx..].chars().next().map_or(0, char::len_utf8);
            if text[idx..next].chars().next().unwrap_or(' ').is_whitespace() {
                break;
            }
            idx = next;
        }
        idx
    }
}
