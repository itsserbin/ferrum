use crate::gui::*;

/// Clamps a coordinate to the valid range for a window dimension.
/// Returns 0.0 for non-finite inputs or when `size` is 0.
fn clamp_axis(v: f64, size: u32) -> f64 {
    let v = if v.is_finite() { v } else { 0.0 };
    v.clamp(0.0, size.saturating_sub(1) as f64)
}

impl FerrumWindow {
    /// Normalizes pointer coordinates into the current window's physical pixel bounds.
    pub(in crate::gui) fn normalized_window_pos(&self, x: f64, y: f64) -> (f64, f64) {
        let size = self.window.inner_size();
        (clamp_axis(x, size.width), clamp_axis(y, size.height))
    }

    /// Converts window pixels to terminal grid coordinates for the active pane.
    ///
    /// In split screen mode the active pane does not always start at
    /// `(window_padding, tab_bar_height + window_padding)`, so we look up
    /// the pane's actual rect in the layout and compute coordinates relative
    /// to that rect's origin instead of the content area's origin.
    pub(in crate::gui) fn pixel_to_grid(&self, x: f64, y: f64) -> (usize, usize) {
        let tab = match self.active_tab_ref() {
            Some(t) => t,
            None => return (0, 0),
        };
        let focused_id = tab.focused_pane;
        let leaf = match tab.pane_tree.find_leaf(focused_id) {
            Some(l) => l,
            None => return (0, 0),
        };

        let pane_rect = match self.pane_content_rect(focused_id) {
            Some(r) => r,
            None => return (0, 0),
        };

        let local_x = (x as u32).saturating_sub(pane_rect.x);
        let local_y = (y as u32).saturating_sub(pane_rect.y);
        self.local_pixel_to_grid(
            local_x,
            local_y,
            leaf.terminal.screen.cols(),
            leaf.terminal.screen.viewport_rows(),
        )
    }

    /// Converts local (pane-relative) pixel coordinates to grid (row, col) coordinates.
    pub(in crate::gui) fn local_pixel_to_grid(
        &self,
        local_x: u32,
        local_y: u32,
        max_cols: usize,
        max_rows: usize,
    ) -> (usize, usize) {
        let col = ((local_x + self.backend.cell_width() / 2) as usize
            / self.backend.cell_width() as usize)
            .min(max_cols.saturating_sub(1));
        let row = (local_y as usize / self.backend.cell_height() as usize)
            .min(max_rows.saturating_sub(1));
        (row, col)
    }

    /// Returns `true` if the given x coordinate is within the scrollbar hit zone.
    pub(in crate::gui) fn is_in_scrollbar_zone(&self, x: f64, window_width: u32) -> bool {
        let hit_zone = self.backend.scrollbar_hit_zone_px();
        x >= window_width.saturating_sub(hit_zone) as f64
    }
}
