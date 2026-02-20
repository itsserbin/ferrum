use crate::gui::*;

impl FerrumWindow {
    /// Normalizes pointer coordinates into the current window's physical pixel bounds.
    pub(in crate::gui) fn normalized_window_pos(&self, x: f64, y: f64) -> (f64, f64) {
        let size = self.window.inner_size();

        let mut nx = if x.is_finite() { x } else { 0.0 };
        let mut ny = if y.is_finite() { y } else { 0.0 };

        if size.width == 0 {
            nx = 0.0;
        } else {
            nx = nx.clamp(0.0, size.width.saturating_sub(1) as f64);
        }

        if size.height == 0 {
            ny = 0.0;
        } else {
            ny = ny.clamp(0.0, size.height.saturating_sub(1) as f64);
        }

        (nx, ny)
    }

    /// Converts window pixels to terminal grid coordinates.
    pub(in crate::gui) fn pixel_to_grid(&self, x: f64, y: f64) -> (usize, usize) {
        let window_padding = self.backend.window_padding_px();
        let tab_bar_height = self.backend.tab_bar_height_px();
        let col = ((x as u32).saturating_sub(window_padding) + self.backend.cell_width() / 2)
            as usize
            / self.backend.cell_width() as usize;
        let row = (y as u32).saturating_sub(tab_bar_height + window_padding) as usize
            / self.backend.cell_height() as usize;
        if let Some(leaf) = self.active_leaf_ref() {
            let row = row.min(leaf.terminal.grid.rows.saturating_sub(1));
            let col = col.min(leaf.terminal.grid.cols.saturating_sub(1));
            (row, col)
        } else {
            (0, 0)
        }
    }

    /// Returns `true` if the given x coordinate is within the scrollbar hit zone.
    pub(in crate::gui) fn is_in_scrollbar_zone(&self, x: f64, window_width: u32) -> bool {
        let hit_zone = self.backend.scrollbar_hit_zone_px();
        x >= window_width.saturating_sub(hit_zone) as f64
    }
}
