use crate::core::SelectionPoint;
use crate::gui::*;

const MULTI_CLICK_TIMEOUT_MS: u128 = 400;

impl FerrumWindow {
    fn update_terminal_click_streak(&mut self, pos: Position) -> u8 {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_click_time);
        if self.click_streak > 0
            && elapsed.as_millis() < MULTI_CLICK_TIMEOUT_MS
            && self.last_click_pos == pos
        {
            self.click_streak = (self.click_streak % 3) + 1;
        } else {
            self.click_streak = 1;
        }
        self.last_click_time = now;
        self.last_click_pos = pos;
        self.click_streak
    }

    pub(in crate::gui::events::mouse) fn handle_terminal_left_click(
        &mut self,
        state: ElementState,
        mx: f64,
        my: f64,
    ) {
        // Terminal area click
        let (row, col) = self.pixel_to_grid(mx, my);

        // Mouse reporting mode (Shift overrides to selection)
        if self.is_mouse_reporting() {
            match state {
                ElementState::Pressed => {
                    self.is_selecting = true; // track for drag reporting
                    self.selection_anchor = None;
                    self.send_mouse_event(0, col, row, true);
                }
                ElementState::Released => {
                    self.is_selecting = false;
                    self.selection_anchor = None;
                    self.send_mouse_event(0, col, row, false);
                }
            }
            return;
        }

        // Shell mode — click-to-cursor + full editor-style selection.
        let pos = Position { row, col };
        let idx = self.active_tab;
        if idx >= self.tabs.len() {
            return;
        }

        let abs_pos = SelectionPoint {
            row: self.screen_to_abs(row),
            col,
        };

        match state {
            ElementState::Pressed => {
                let is_focus_click = self.suppress_click_to_cursor_once;
                self.suppress_click_to_cursor_once = false;
                if is_focus_click {
                    return;
                }

                self.is_selecting = true;

                if self.modifiers.shift_key() {
                    self.click_streak = 0;
                    self.last_click_time = std::time::Instant::now();
                    self.last_click_pos = pos;

                    let anchor = self.tabs[idx]
                        .selection
                        .map(|sel| sel.start)
                        .unwrap_or(abs_pos);
                    self.selection_anchor = Some(anchor);
                    self.selection_drag_mode = SelectionDragMode::Character;
                    self.tabs[idx].selection = Some(Selection {
                        start: anchor,
                        end: abs_pos,
                    });
                    return;
                }

                self.selection_anchor = Some(abs_pos);
                match self.update_terminal_click_streak(pos) {
                    1 => {
                        // Single-click arms char-wise drag; selection starts on movement.
                        self.selection_drag_mode = SelectionDragMode::Character;
                        self.tabs[idx].selection = None;
                    }
                    2 => {
                        // Double-click selects word and keeps word-wise drag active.
                        self.selection_drag_mode = SelectionDragMode::Word;
                        self.select_word_at(row, col);
                    }
                    _ => {
                        // Triple-click selects line and keeps line-wise drag active.
                        self.selection_drag_mode = SelectionDragMode::Line;
                        self.select_line_at(row);
                    }
                }
            }
            ElementState::Released => {
                let should_move_cursor = self.selection_drag_mode == SelectionDragMode::Character
                    && self.tabs[idx].selection.is_none();
                self.is_selecting = false;
                self.selection_anchor = None;
                if should_move_cursor {
                    // No drag happened: treat as click-to-cursor.
                    self.move_cursor_to(row, col);
                }
            }
        }
    }
}
