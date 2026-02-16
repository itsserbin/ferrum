use crate::gui::renderer::TabBarHit;
use crate::gui::*;

/// Vertical distance from tab bar center at which a drag becomes a detach.
const DETACH_THRESHOLD_Y: u32 = 30;

impl FerrumWindow {
    pub(crate) fn on_cursor_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let (mx, my) = self.normalized_window_pos(position.x, position.y);
        self.mouse_pos = (mx, my);
        let tab_bar_height = self.renderer.tab_bar_height_px() as f64;
        let window_padding = self.renderer.window_padding_px() as f64;
        let detach_threshold_y = self.renderer.scaled_px(DETACH_THRESHOLD_Y) as f64;

        // Update drag state tracking.
        if let Some(ref mut drag) = self.dragging_tab {
            drag.current_x = mx;
            drag.current_y = my;
            if !drag.is_active {
                let dx = mx - drag.start_x;
                let dy = my - drag.start_y;
                if (dx * dx + dy * dy).sqrt() > 5.0 {
                    drag.is_active = true;
                }
            }
            if drag.is_active {
                // Detach: cursor moved far enough vertically from the tab bar.
                let beyond_below = my > tab_bar_height + detach_threshold_y;
                let beyond_above = my < self.renderer.scaled_px(5) as f64;
                if (beyond_below || beyond_above) && self.tabs.len() > 1 {
                    self.detach_dragged_tab();
                    return;
                }

                self.window.set_cursor(CursorIcon::Grabbing);
                self.window.request_redraw();
                return;
            }
        }

        let dir = self.edge_direction(mx, my);
        let cursor = if let Some(d) = dir {
            match d {
                ResizeDirection::North | ResizeDirection::South => CursorIcon::NsResize,
                ResizeDirection::East | ResizeDirection::West => CursorIcon::EwResize,
                ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::NwseResize,
                ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::NeswResize,
            }
        } else if my < tab_bar_height {
            if self.is_window_close_button_hit(mx, my)
                || self.is_window_maximize_button_hit(mx, my)
                || self.is_window_minimize_button_hit(mx, my)
            {
                CursorIcon::Pointer
            } else {
                match self.tab_bar_hit(mx, my) {
                    TabBarHit::Tab(_) | TabBarHit::CloseTab(_) | TabBarHit::NewTab => {
                        CursorIcon::Pointer
                    }
                    TabBarHit::Empty => CursorIcon::Default,
                }
            }
        } else {
            CursorIcon::Text
        };
        self.window.set_cursor(cursor);
        self.resize_direction = dir;

        // Track hovered tab for visual feedback.
        let size = self.window.inner_size();
        self.hovered_tab = self
            .renderer
            .hit_test_tab_hover(mx, my, self.tabs.len(), size.width);

        // Track hovered context-menu item.
        if let Some(ref mut menu) = self.context_menu {
            menu.hover_index = self.renderer.hit_test_context_menu(menu, mx, my);
        }

        // Handle rename field drag selection in tab bar.
        if my < tab_bar_height {
            if self.is_selecting && self.renaming_tab.is_some() {
                self.handle_rename_field_drag(mx);
            }
            return;
        }

        // Scrollbar drag: update scroll_offset based on mouse delta.
        if self.active_tab_ref().is_some_and(|t| t.scrollbar.dragging) {
            let size = self.window.inner_size();
            let buf_height = size.height as usize;
            let tab = self.active_tab_ref().unwrap();
            let scrollback_len = tab.terminal.scrollback.len();
            let grid_rows = tab.terminal.grid.rows;
            let drag_start_y = tab.scrollbar.drag_start_y;
            let drag_start_offset = tab.scrollbar.drag_start_offset;

            if let Some((_thumb_y, thumb_height)) = self.renderer.scrollbar_thumb_bounds(
                buf_height,
                tab.scroll_offset,
                scrollback_len,
                grid_rows,
            ) {
                let track_top = (tab_bar_height + window_padding) as f32;
                let track_bottom = buf_height as f32 - window_padding as f32;
                let track_height = track_bottom - track_top;
                let scrollable_track = track_height - thumb_height;

                if scrollable_track > 0.0 {
                    let delta_y = my - drag_start_y;
                    let lines_per_pixel = scrollback_len as f64 / scrollable_track as f64;
                    let new_offset = drag_start_offset as f64 - delta_y * lines_per_pixel;
                    let new_offset = new_offset.round() as isize;
                    let clamped = new_offset.max(0) as usize;
                    if let Some(tab) = self.active_tab_mut() {
                        tab.scroll_offset = clamped.min(tab.terminal.scrollback.len());
                        tab.scrollbar.last_activity = std::time::Instant::now();
                    }
                }
            }
            self.window.request_redraw();
            return;
        }

        // Scrollbar hover detection.
        {
            let size = self.window.inner_size();
            let in_zone = self.is_in_scrollbar_zone(mx, size.width);
            let has_scrollback = self
                .active_tab_ref()
                .is_some_and(|t| !t.terminal.scrollback.is_empty());
            let track_top = tab_bar_height + window_padding;
            let track_bottom = size.height as f64 - window_padding;
            let in_track = my >= track_top && my <= track_bottom;
            let new_hover = in_zone && has_scrollback && in_track;

            if let Some(tab) = self.active_tab_mut() {
                let was_hover = tab.scrollbar.hover;
                tab.scrollbar.hover = new_hover;
                if new_hover && !was_hover {
                    tab.scrollbar.last_activity = std::time::Instant::now();
                } else if !new_hover && was_hover {
                    tab.scrollbar.last_activity = std::time::Instant::now();
                }
            }

            if new_hover && self.resize_direction.is_none() {
                self.window.set_cursor(CursorIcon::Default);
            }
        }

        let (row, col) = self.pixel_to_grid(mx, my);

        // Mouse drag/motion reporting
        let mouse_mode = self
            .active_tab_ref()
            .map_or(MouseMode::Off, |t| t.terminal.mouse_mode);
        if !self.modifiers.shift_key() {
            if mouse_mode == MouseMode::AnyEvent && !self.is_selecting {
                self.send_mouse_event(35, col, row, true);
                return;
            }
            if (mouse_mode == MouseMode::ButtonEvent || mouse_mode == MouseMode::AnyEvent)
                && self.is_selecting
            {
                self.send_mouse_event(32, col, row, true);
                return;
            }

            // In mouse-reporting mode, PTY app owns mouse interactions.
            if mouse_mode != MouseMode::Off && self.is_selecting {
                return;
            }
        }

        // Drag-based text selection (shell mode or Shift override).
        if self.is_selecting {
            self.update_drag_selection(row, col);
        }
    }

    /// Detaches the currently dragged tab into a new window (called during drag, button still held).
    fn detach_dragged_tab(&mut self) {
        let Some(drag) = self.dragging_tab.take() else {
            return;
        };
        if drag.source_index >= self.tabs.len() {
            return;
        }

        self.adjust_rename_after_tab_remove(drag.source_index);
        self.adjust_security_popup_after_tab_remove(drag.source_index);
        let tab = self.tabs.remove(drag.source_index);

        if !self.tabs.is_empty() {
            let len_before = self.tabs.len() + 1;
            self.active_tab = crate::gui::tabs::normalized_active_index_after_remove(
                self.active_tab,
                len_before,
                drag.source_index,
            )
            .unwrap_or(0);
        }

        self.pending_requests.push(WindowRequest::DetachTab { tab });
        self.window.set_cursor(CursorIcon::Default);
        self.window.request_redraw();
    }
}
