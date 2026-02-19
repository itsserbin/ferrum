use crate::gui::renderer::{ContextMenu, TabBarHit};
use crate::gui::*;

impl FerrumWindow {
    pub(crate) fn on_mouse_input(
        &mut self,
        state: ElementState,
        button: winit::event::MouseButton,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) {
        self.apply_pending_resize();

        match button {
            winit::event::MouseButton::Left => self.on_left_mouse_input(state, next_tab_id, tx),
            winit::event::MouseButton::Middle => self.on_middle_mouse_input(state),
            winit::event::MouseButton::Right => self.on_right_mouse_input(state),
            _ => {}
        }
    }

    fn on_middle_mouse_input(&mut self, state: ElementState) {
        if state != ElementState::Pressed {
            return;
        }
        self.commit_rename();
        let (mx, my) = self.mouse_pos;
        if my >= self.backend.tab_bar_height_px() as f64 {
            return;
        }
        if let TabBarHit::Tab(idx) | TabBarHit::CloseTab(idx) = self.tab_bar_hit(mx, my) {
            self.close_tab(idx);
        }
    }

    fn on_right_mouse_input(&mut self, state: ElementState) {
        match state {
            ElementState::Pressed => {
                self.commit_rename();
                self.security_popup = None;
                let (mx, my) = self.mouse_pos;
                let tab_bar_height = self.backend.tab_bar_height_px();
                if my < tab_bar_height as f64 {
                    // Right click on a tab opens its context menu.
                    if let TabBarHit::Tab(idx) | TabBarHit::CloseTab(idx) = self.tab_bar_hit(mx, my)
                    {
                        self.context_menu =
                            Some(ContextMenu::for_tab(mx as u32, tab_bar_height, idx));
                    }
                    return;
                }

                if self.is_mouse_reporting() {
                    let (row, col) = self.pixel_to_grid(self.mouse_pos.0, self.mouse_pos.1);
                    self.send_mouse_event(2, col, row, true);
                    return;
                }

                self.context_menu = None;

                let (row, col) = self.pixel_to_grid(mx, my);
                let abs_row = self.screen_to_abs(row);
                let clicked_selection = self
                    .active_tab_ref()
                    .and_then(|tab| tab.selection)
                    .is_some_and(|selection| selection.contains(abs_row, col));

                if clicked_selection {
                    self.context_menu = Some(ContextMenu::for_terminal_selection(
                        mx.max(0.0) as u32,
                        my.max(0.0) as u32,
                    ));
                } else {
                    self.paste_clipboard();
                }
            }
            ElementState::Released => {
                if self.is_mouse_reporting() {
                    let (row, col) = self.pixel_to_grid(self.mouse_pos.0, self.mouse_pos.1);
                    self.send_mouse_event(2, col, row, false);
                }
            }
        }
    }

    fn handle_security_popup_left_click(&mut self, state: ElementState, mx: f64, my: f64) -> bool {
        if state != ElementState::Pressed {
            return false;
        }
        let Some(popup) = self.security_popup.take() else {
            return false;
        };

        let size = self.window.inner_size();
        let (buf_width, buf_height) = (size.width as usize, size.height as usize);

        self.backend
            .hit_test_security_popup(&popup, mx, my, buf_width, buf_height)
    }

    fn on_left_mouse_input(
        &mut self,
        state: ElementState,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) {
        let (mx, my) = self.mouse_pos;
        let tab_bar_height = self.backend.tab_bar_height_px() as f64;

        // On non-macOS, initiate OS-level resize drag when pressing on window edges.
        #[cfg(not(target_os = "macos"))]
        if state == ElementState::Pressed
            && let Some(dir) = self.resize_direction
        {
            let _ = self.window.drag_resize_window(dir);
            return;
        }

        // If releasing mouse during an active tab drag, handle drop regardless of position.
        // (Custom tab bar drag — not used on macOS.)
        #[cfg(not(target_os = "macos"))]
        if state == ElementState::Released {
            if self.dragging_tab.as_ref().is_some_and(|d| d.is_active) {
                self.handle_tab_bar_left_click(state, mx, my, next_tab_id, tx);
                return;
            }
            // Cancel non-active drag on release outside tab bar.
            if self.dragging_tab.is_some() {
                self.dragging_tab = None;
            }
        }

        if self.handle_security_popup_left_click(state, mx, my) {
            return;
        }

        if self.handle_context_menu_left_click(state, mx, my, next_tab_id, tx) {
            return;
        }

        if my < tab_bar_height {
            self.handle_tab_bar_left_click(state, mx, my, next_tab_id, tx);
            return;
        }

        // Clicking on terminal area commits any active rename (blur behavior).
        if state == ElementState::Pressed {
            self.dragging_tab = None; // Cancel potential drag if clicking terminal area.
            self.commit_rename();
        }

        if self.handle_scrollbar_left_click(state, mx, my) {
            return;
        }

        self.handle_terminal_left_click(state, mx, my);
    }

    /// Handles left mouse down/up on the scrollbar zone.
    /// Returns `true` if the event was consumed (click was in scrollbar zone).
    fn handle_scrollbar_left_click(&mut self, state: ElementState, mx: f64, my: f64) -> bool {
        // On release: end scrollbar drag if active.
        if state == ElementState::Released {
            if self.active_tab_ref().is_some_and(|t| t.scrollbar.dragging) {
                if let Some(tab) = self.active_tab_mut() {
                    tab.scrollbar.dragging = false;
                    tab.scrollbar.last_activity = std::time::Instant::now();
                }
                return true;
            }
            return false;
        }

        // Pressed: check if click is in scrollbar zone.
        let size = self.window.inner_size();
        if !self.is_in_scrollbar_zone(mx, size.width) {
            return false;
        }

        let tab = match self.active_tab_ref() {
            Some(t) => t,
            None => return false,
        };
        let scrollback_len = tab.terminal.scrollback.len();
        if scrollback_len == 0 {
            return false;
        }

        let buf_height = size.height as usize;
        let grid_rows = tab.terminal.grid.rows;
        let scroll_offset = tab.scroll_offset;
        let tab_bar_height = self.backend.tab_bar_height_px() as f64;
        let window_padding = self.backend.window_padding_px() as f64;

        let track_top = tab_bar_height + window_padding;
        let track_bottom = buf_height as f64 - window_padding;

        // Ignore clicks outside the track area.
        if my < track_top || my > track_bottom {
            return false;
        }

        let track_height = track_bottom - track_top;

        // Guard against division by zero with extremely small windows.
        if track_height <= 0.0 {
            return false;
        }

        // Check if click is on the thumb or on the track.
        if let Some((thumb_y, thumb_height)) = self.backend.scrollbar_thumb_bounds(
            buf_height,
            scroll_offset,
            scrollback_len,
            grid_rows,
        ) {
            let on_thumb = my >= thumb_y as f64 && my <= (thumb_y + thumb_height) as f64;

            if on_thumb {
                // Start thumb drag.
                if let Some(tab) = self.active_tab_mut() {
                    tab.scrollbar.dragging = true;
                    tab.scrollbar.drag_start_y = my;
                    tab.scrollbar.drag_start_offset = tab.scroll_offset;
                    tab.scrollbar.last_activity = std::time::Instant::now();
                }
            } else {
                // Click on track: jump to proportional position.
                let click_ratio = (my - track_top) / track_height;
                let max_offset = scrollback_len;
                let new_offset =
                    (max_offset as f64 - click_ratio * max_offset as f64).round() as isize;
                let clamped = new_offset.max(0) as usize;
                if let Some(tab) = self.active_tab_mut() {
                    tab.scroll_offset = clamped.min(tab.terminal.scrollback.len());
                    tab.scrollbar.last_activity = std::time::Instant::now();
                }
            }
        }

        self.window.request_redraw();
        true
    }
}
