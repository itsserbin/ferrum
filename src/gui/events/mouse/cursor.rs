use crate::gui::renderer::TabBarHit;
use crate::gui::*;

/// Resize edge thickness in logical pixels.
#[cfg(not(target_os = "macos"))]
const RESIZE_EDGE: u32 = 4;

/// Detects whether the cursor is near a window edge for resize purposes.
/// Returns the resize direction if within the edge zone, None otherwise.
#[cfg(not(target_os = "macos"))]
fn resize_direction(x: f64, y: f64, width: u32, height: u32, edge: u32) -> Option<ResizeDirection> {
    let e = edge as f64;
    let w = width as f64;
    let h = height as f64;

    let left = x < e;
    let right = x >= w - e;
    let top = y < e;
    let bottom = y >= h - e;

    match (left, right, top, bottom) {
        (true, _, true, _) => Some(ResizeDirection::NorthWest),
        (_, true, true, _) => Some(ResizeDirection::NorthEast),
        (true, _, _, true) => Some(ResizeDirection::SouthWest),
        (_, true, _, true) => Some(ResizeDirection::SouthEast),
        (true, _, _, _) => Some(ResizeDirection::West),
        (_, true, _, _) => Some(ResizeDirection::East),
        (_, _, true, _) => Some(ResizeDirection::North),
        (_, _, _, true) => Some(ResizeDirection::South),
        _ => None,
    }
}

/// Returns the appropriate cursor icon for a given resize direction.
#[cfg(not(target_os = "macos"))]
fn resize_cursor_icon(dir: ResizeDirection) -> CursorIcon {
    match dir {
        ResizeDirection::North | ResizeDirection::South => CursorIcon::NsResize,
        ResizeDirection::East | ResizeDirection::West => CursorIcon::EwResize,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::NwseResize,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::NeswResize,
    }
}

impl FerrumWindow {
    pub(crate) fn on_cursor_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let (mx, my) = self.normalized_window_pos(position.x, position.y);
        self.mouse_pos = (mx, my);
        let tab_bar_height = self.backend.tab_bar_height_px() as f64;

        // On non-macOS, check resize edges BEFORE any other hit testing.
        #[cfg(not(target_os = "macos"))]
        {
            let size = self.window.inner_size();
            let edge = self.backend.scaled_px(RESIZE_EDGE);
            if let Some(dir) = resize_direction(mx, my, size.width, size.height, edge) {
                self.window.set_cursor(resize_cursor_icon(dir));
                self.resize_direction = Some(dir);
                return;
            }
            self.resize_direction = None;
        }

        // Update drag state tracking (custom tab bar only -- not on macOS).
        #[cfg(not(target_os = "macos"))]
        if self.update_drag(mx, my) {
            return;
        }

        self.update_hover(mx, my);

        // Track hovered context-menu item.
        if let Some(ref mut menu) = self.context_menu {
            menu.hover_index = self.backend.hit_test_context_menu(menu, mx, my);
            self.hovered_tab = None;
            let cursor = if menu.hover_index.is_some() {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            };
            self.window.set_cursor(cursor);
            return;
        }

        let cursor = if my < tab_bar_height {
            match self.tab_bar_hit(mx, my) {
                TabBarHit::Tab(_) | TabBarHit::CloseTab(_) | TabBarHit::NewTab => {
                    CursorIcon::Pointer
                }
                #[cfg(not(target_os = "macos"))]
                TabBarHit::WindowButton(_) => CursorIcon::Pointer,
                #[cfg(not(target_os = "macos"))]
                TabBarHit::PinButton => CursorIcon::Pointer,
                TabBarHit::Empty => CursorIcon::Default,
            }
        } else {
            CursorIcon::Text
        };
        self.window.set_cursor(cursor);

        // Handle rename field drag selection in tab bar.
        if my < tab_bar_height {
            if self.is_selecting && self.renaming_tab.is_some() {
                self.handle_rename_field_drag(mx);
            }
            return;
        }

        // Scrollbar drag / hover / terminal area.
        if self.handle_scrollbar_drag(my, tab_bar_height) {
            return;
        }
        self.update_scrollbar_hover(mx, my, tab_bar_height);

        let (row, col) = self.pixel_to_grid(mx, my);

        if self.handle_mouse_motion_reporting(row, col) {
            return;
        }

        // Drag-based text selection (shell mode or Shift override).
        if self.is_selecting {
            self.update_drag_selection(row, col);
        }
    }

    /// Handles scrollbar thumb dragging. Returns `true` when drag is active
    /// and the event should not propagate further.
    fn handle_scrollbar_drag(&mut self, my: f64, tab_bar_height: f64) -> bool {
        if !self.active_tab_ref().is_some_and(|t| t.scrollbar.dragging) {
            return false;
        }

        let window_padding = self.backend.window_padding_px() as f64;
        let size = self.window.inner_size();
        let buf_height = size.height as usize;
        let tab = self.active_tab_ref().unwrap();
        let scrollback_len = tab.terminal.scrollback.len();
        let grid_rows = tab.terminal.grid.rows;
        let drag_start_y = tab.scrollbar.drag_start_y;
        let drag_start_offset = tab.scrollbar.drag_start_offset;

        if let Some((_thumb_y, thumb_height)) = self.backend.scrollbar_thumb_bounds(
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
        true
    }

    /// Updates the scrollbar hover state based on mouse position.
    fn update_scrollbar_hover(&mut self, mx: f64, my: f64, tab_bar_height: f64) {
        let window_padding = self.backend.window_padding_px() as f64;
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
            if new_hover != was_hover {
                tab.scrollbar.last_activity = std::time::Instant::now();
            }
        }

        if new_hover {
            self.window.set_cursor(CursorIcon::Default);
        }
    }

    /// Handles mouse motion/drag reporting to the PTY application.
    /// Returns `true` when the event was consumed by mouse reporting.
    fn handle_mouse_motion_reporting(&mut self, row: usize, col: usize) -> bool {
        let mouse_mode = self
            .active_tab_ref()
            .map_or(MouseMode::Off, |t| t.terminal.mouse_mode);

        if self.modifiers.shift_key() {
            return false;
        }

        if mouse_mode == MouseMode::AnyEvent && !self.is_selecting {
            self.send_mouse_event(35, col, row, true);
            return true;
        }
        if (mouse_mode == MouseMode::ButtonEvent || mouse_mode == MouseMode::AnyEvent)
            && self.is_selecting
        {
            self.send_mouse_event(32, col, row, true);
            return true;
        }

        // In mouse-reporting mode, PTY app owns mouse interactions.
        if mouse_mode != MouseMode::Off && self.is_selecting {
            return true;
        }

        false
    }
}
