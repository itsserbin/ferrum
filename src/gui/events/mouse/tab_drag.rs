use crate::gui::*;

/// Vertical distance from tab bar center at which a drag becomes a detach.
#[cfg(not(target_os = "macos"))]
const DETACH_THRESHOLD_Y: u32 = 30;

/// Minimum mouse movement to activate a tab drag (logical pixels).
#[cfg(not(target_os = "macos"))]
pub(in crate::gui::events::mouse) const DRAG_ACTIVATION_THRESHOLD: u32 = 5;

impl FerrumWindow {
    /// Arms a potential tab drag if there are at least 2 tabs and no rename was just committed.
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::events::mouse) fn start_drag(
        &mut self,
        tab_index: usize,
        mx: f64,
        my: f64,
        had_rename: bool,
    ) {
        if self.tabs.len() > 1 && !had_rename {
            self.dragging_tab = Some(DragState {
                source_index: tab_index,
                start_x: mx,
                start_y: my,
                current_x: mx,
                current_y: my,
                is_active: false,
                indicator_x: -1.0,
            });
        }
    }

    /// Updates the drag position and checks for activation threshold and detach conditions.
    /// Returns true if the event was consumed by an active drag (caller should return early).
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::events::mouse) fn update_drag(&mut self, mx: f64, my: f64) -> bool {
        let Some(ref mut drag) = self.dragging_tab else {
            return false;
        };

        drag.current_x = mx;
        drag.current_y = my;

        if !drag.is_active {
            let dx = mx - drag.start_x;
            let dy = my - drag.start_y;
            let threshold = self.backend.scaled_px(DRAG_ACTIVATION_THRESHOLD) as f64;
            if (dx * dx + dy * dy).sqrt() > threshold {
                drag.is_active = true;
            }
        }

        if drag.is_active {
            let tab_bar_height = self.backend.tab_bar_height_px() as f64;
            // Detach: cursor moved far enough vertically from the tab bar.
            let detach_threshold_y = self.backend.scaled_px(DETACH_THRESHOLD_Y) as f64;
            let beyond_below = my > tab_bar_height + detach_threshold_y;
            let beyond_above = my < self.backend.scaled_px(DRAG_ACTIVATION_THRESHOLD) as f64;
            if (beyond_below || beyond_above) && self.tabs.len() > 1 {
                self.detach_dragged_tab();
                return true;
            }

            self.window.set_cursor(CursorIcon::Grabbing);
            self.window.request_redraw();
            return true;
        }

        false
    }

    /// Handles mouse release: drops the tab at the new position or treats as normal click.
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::events::mouse) fn finish_drag(&mut self) {
        let Some(drag) = self.dragging_tab.take() else {
            return;
        };
        if !drag.is_active {
            return; // Was never activated (< threshold movement), normal click already happened.
        }

        let source = drag.source_index;
        let buf_width = self.window.inner_size().width;
        let tab_count = self.tabs.len();

        // Calculate insertion index.
        let insert_at = self
            .backend
            .tab_insert_index_from_x(drag.current_x, tab_count, buf_width);

        // Convert insertion index to the actual destination after removal.
        let dest = if insert_at > source {
            (insert_at - 1).min(tab_count - 1)
        } else {
            insert_at
        };

        if dest != source && dest < tab_count {
            // Compute per-tab pixel offsets BEFORE the move (for slide animation).
            let tw = self.backend.tab_width(tab_count, buf_width) as f32;
            let mut offsets = vec![0.0f32; tab_count];

            // Tabs between source and dest shift by one tab width.
            if source < dest {
                for offset in offsets.iter_mut().take(dest + 1).skip(source + 1) {
                    *offset = tw;
                }
                offsets[source] = -((dest - source) as f32 * tw);
            } else {
                for offset in offsets.iter_mut().take(source).skip(dest) {
                    *offset = -tw;
                }
                offsets[source] = (source - dest) as f32 * tw;
            }

            // Perform the actual reorder.
            let tab = self.tabs.remove(source);
            self.tabs.insert(dest, tab);

            // Fix up offsets to match new indices.
            let moved_offset = offsets.remove(source);
            offsets.insert(dest, moved_offset);

            if self.active_tab == source {
                self.active_tab = dest;
            } else if source < self.active_tab && dest >= self.active_tab {
                self.active_tab -= 1;
            } else if source > self.active_tab && dest <= self.active_tab {
                self.active_tab += 1;
            }

            // Start slide animation.
            self.tab_reorder_animation = Some(TabReorderAnimation {
                started: std::time::Instant::now(),
                duration_ms: 150,
                offsets,
            });
        }

        // Always restore cursor and redraw after drag ends.
        self.window.set_cursor(CursorIcon::Default);
        self.window.request_redraw();
    }

    #[cfg(not(target_os = "macos"))]
    /// Cancels any in-progress drag without performing a reorder.
    pub(in crate::gui::events::mouse) fn cancel_drag(&mut self) {
        self.dragging_tab = None;
    }

    /// Detaches the currently dragged tab into a new window (called during drag, button still held).
    #[cfg(not(target_os = "macos"))]
    pub(in crate::gui::events::mouse) fn detach_dragged_tab(&mut self) {
        let Some(drag) = self.dragging_tab.take() else {
            return;
        };
        if drag.source_index >= self.tabs.len() {
            return;
        }

        // Compute new window position so it appears under the cursor.
        let cursor_pos = self.window.outer_position().ok().map(|outer| {
            winit::dpi::PhysicalPosition::new(
                outer.x + drag.current_x as i32 - 100,
                outer.y + drag.current_y as i32 - 10,
            )
        });

        self.adjust_rename_after_tab_remove(drag.source_index);
        self.adjust_security_popup_after_tab_remove(drag.source_index);
        let tab = self.tabs.remove(drag.source_index);
        self.refresh_tab_bar_visibility();

        if !self.tabs.is_empty() {
            let len_before = self.tabs.len() + 1;
            self.active_tab = crate::gui::tabs::normalized_active_index_after_remove(
                self.active_tab,
                len_before,
                drag.source_index,
            )
            .unwrap_or(0);
        }

        self.pending_requests.push(WindowRequest::DetachTab {
            tab: Box::new(tab),
            cursor_pos,
        });
        self.window.set_cursor(CursorIcon::Default);
        self.window.request_redraw();
    }
}
