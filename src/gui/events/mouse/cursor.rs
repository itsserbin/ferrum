use crate::gui::pane::{DIVIDER_HIT_ZONE, DIVIDER_WIDTH, SplitDirection};
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

        // Settings overlay consumes all mouse movement when open.
        if self.handle_settings_mouse_move(mx, my) {
            return;
        }

        // Handle active divider drag (pane resize).
        if self.divider_drag.is_some() {
            self.handle_divider_drag(mx, my);
            return;
        }

        // Update drag state tracking (custom tab bar only -- not on macOS).
        #[cfg(not(target_os = "macos"))]
        if self.update_drag(mx, my) {
            return;
        }

        // Update hovered tab based on current mouse position.
        let size = self.window.inner_size();
        self.hovered_tab = self
            .backend
            .hit_test_tab_hover(mx, my, self.tabs.len(), size.width);

        let cursor = if my < tab_bar_height {
            match self.tab_bar_hit(mx, my) {
                TabBarHit::Tab(_) | TabBarHit::CloseTab(_) | TabBarHit::NewTab => {
                    CursorIcon::Pointer
                }
                #[cfg(not(target_os = "macos"))]
                TabBarHit::WindowButton(_) => CursorIcon::Pointer,
                #[cfg(not(target_os = "macos"))]
                TabBarHit::PinButton => CursorIcon::Pointer,
                #[cfg(not(target_os = "macos"))]
                TabBarHit::SettingsButton => CursorIcon::Pointer,
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

        // Check if hovering over a pane divider — show resize cursor.
        if self.handle_divider_hover(mx, my) {
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

    /// Handles divider drag during mouse movement — updates the split ratio.
    fn handle_divider_drag(&mut self, mx: f64, my: f64) {
        let terminal_rect = self.terminal_content_rect();
        let divider_px = DIVIDER_WIDTH;

        let (hit_pos, direction) = {
            let drag = self.divider_drag.as_ref().unwrap();
            (drag.initial_mouse_pos, drag.direction)
        };

        // Set appropriate resize cursor during drag.
        let cursor = match direction {
            SplitDirection::Horizontal => CursorIcon::ColResize,
            SplitDirection::Vertical => CursorIcon::RowResize,
        };
        self.window.set_cursor(cursor);

        // Compute the new pixel position based on drag direction.
        let new_pixel_pos = match direction {
            SplitDirection::Horizontal => mx as u32,
            SplitDirection::Vertical => my as u32,
        };

        let mut resized = false;
        if let Some(tab) = self.active_tab_mut() {
            resized = tab.pane_tree.resize_divider_at(
                hit_pos.0,
                hit_pos.1,
                terminal_rect,
                divider_px,
                DIVIDER_HIT_ZONE,
                new_pixel_pos,
            );
        }
        // Keep the hit anchor in sync with drag progression so subsequent
        // events continue targeting the same divider after it moves.
        if resized && let Some(drag) = self.divider_drag.as_mut() {
            drag.initial_mouse_pos = (mx as u32, my as u32);
        }
        if resized {
            // Apply terminal/PTTY resize immediately so width reflow is visible while dragging.
            self.resize_all_panes();
        }

        self.window.request_redraw();
    }

    /// Checks if hovering over a pane divider. Returns `true` if cursor was
    /// set to a resize cursor (event consumed).
    fn handle_divider_hover(&mut self, mx: f64, my: f64) -> bool {
        let terminal_rect = self.terminal_content_rect();
        let has_multiple_panes = self
            .active_tab_ref()
            .is_some_and(|t| t.has_multiple_panes());

        if !has_multiple_panes {
            return false;
        }

        if let Some(tab) = self.active_tab_ref()
            && let Some(hit) = tab.pane_tree.hit_test_divider(
                mx as u32,
                my as u32,
                terminal_rect,
                DIVIDER_WIDTH,
                DIVIDER_HIT_ZONE,
            )
        {
            let cursor = match hit.direction {
                SplitDirection::Horizontal => CursorIcon::ColResize,
                SplitDirection::Vertical => CursorIcon::RowResize,
            };
            self.window.set_cursor(cursor);
            return true;
        }

        false
    }

    /// Handles scrollbar thumb dragging. Returns `true` when drag is active
    /// and the event should not propagate further.
    fn handle_scrollbar_drag(&mut self, my: f64, tab_bar_height: f64) -> bool {
        if !self.active_leaf_ref().is_some_and(|l| l.scrollbar.dragging) {
            return false;
        }

        let window_padding = self.backend.window_padding_px() as f64;
        let size = self.window.inner_size();
        let buf_height = size.height as usize;
        let Some(leaf) = self.active_leaf_ref() else {
            return false;
        };
        let scrollback_len = leaf.terminal.scrollback.len();
        let grid_rows = leaf.terminal.grid.rows;
        let drag_start_y = leaf.scrollbar.drag_start_y;
        let drag_start_offset = leaf.scrollbar.drag_start_offset;

        if let Some((_, thumb_height)) = self.backend.scrollbar_thumb_bounds(
            buf_height,
            leaf.scroll_offset,
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
                if let Some(leaf) = self.active_leaf_mut() {
                    leaf.scroll_offset = clamped.min(leaf.terminal.scrollback.len());
                    leaf.scrollbar.last_activity = std::time::Instant::now();
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
            .active_leaf_ref()
            .is_some_and(|l| !l.terminal.scrollback.is_empty());
        let track_top = tab_bar_height + window_padding;
        let track_bottom = size.height as f64 - window_padding;
        let in_track = my >= track_top && my <= track_bottom;
        let new_hover = in_zone && has_scrollback && in_track;

        if let Some(leaf) = self.active_leaf_mut() {
            let was_hover = leaf.scrollbar.hover;
            leaf.scrollbar.hover = new_hover;
            if new_hover != was_hover {
                leaf.scrollbar.last_activity = std::time::Instant::now();
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
            .active_leaf_ref()
            .map_or(MouseMode::Off, |l| l.terminal.mouse_mode);

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
