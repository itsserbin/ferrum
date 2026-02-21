use crate::gui::*;

impl FerrumWindow {
    fn rename_byte_position_from_mouse(mx: f64, text_x: u32, cell_width: u32, text: &str) -> usize {
        if cell_width == 0 {
            return text.len();
        }

        let relative_x = (mx - text_x as f64).max(0.0) as u32;
        let char_offset = ((relative_x + cell_width / 2) / cell_width) as usize;
        text.char_indices()
            .nth(char_offset)
            .map(|(i, _)| i)
            .unwrap_or(text.len())
    }

    /// Handles a mouse click inside the rename text field: positions cursor, clears selection.
    /// Also detects double-click (word select) and triple-click (select all).
    pub(in crate::gui::events::mouse) fn handle_rename_field_click(&mut self, mx: f64) {
        let Some(rename) = self.renaming_tab.as_mut() else {
            return;
        };

        let buf_width = self.window.inner_size().width;
        let tw = self.backend.tab_width(self.tabs.len(), buf_width);
        let tab_padding_h = self.backend.scaled_px(14);
        let text_x = self.backend.tab_origin_x(rename.tab_index, tw) + tab_padding_h;
        let cell_width = self.backend.cell_width();

        // Calculate cursor byte position from mouse x coordinate.
        let byte_pos =
            Self::rename_byte_position_from_mouse(mx, text_x, cell_width, rename.text.as_str());

        // Multi-click detection within the rename field.
        let now = std::time::Instant::now();
        let is_multi = self.last_tab_click.is_some_and(|(last_idx, last_time)| {
            last_idx == rename.tab_index
                && now.duration_since(last_time).as_millis() < super::MULTI_CLICK_TIMEOUT_MS
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
                let left = Self::rename_word_left_boundary(rename.text.as_str(), byte_pos);
                let right = Self::rename_word_right_boundary(rename.text.as_str(), byte_pos);
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
        let cell_width = self.backend.cell_width();

        let byte_pos =
            Self::rename_byte_position_from_mouse(mx, text_x, cell_width, rename.text.as_str());

        // selection_anchor was set on mouse press; only move cursor.
        rename.cursor = byte_pos;
    }

    /// Finds the left word boundary in the rename text at the given byte position.
    fn rename_word_left_boundary(text: &str, byte_pos: usize) -> usize {
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
    fn rename_word_right_boundary(text: &str, byte_pos: usize) -> usize {
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
