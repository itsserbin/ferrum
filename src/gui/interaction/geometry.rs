use crate::gui::pane::DIVIDER_WIDTH;
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

        let terminal_rect = self.terminal_content_rect();
        let pane_pad = if tab.has_multiple_panes() {
            self.backend.pane_inner_padding_px()
        } else {
            0
        };
        let pane_rect = tab
            .pane_tree
            .layout(terminal_rect, DIVIDER_WIDTH)
            .into_iter()
            .find_map(|(id, rect)| (id == focused_id).then_some(rect.inset(pane_pad)));
        let pane_rect = match pane_rect {
            Some(r) => r,
            None => return (0, 0),
        };

        let local_x = (x as u32).saturating_sub(pane_rect.x);
        let local_y = (y as u32).saturating_sub(pane_rect.y);
        let col = ((local_x + self.backend.cell_width() / 2) as usize
            / self.backend.cell_width() as usize)
            .min(leaf.terminal.grid.cols.saturating_sub(1));
        let row = (local_y as usize / self.backend.cell_height() as usize)
            .min(leaf.terminal.grid.rows.saturating_sub(1));
        (row, col)
    }

    /// Returns `true` if the given x coordinate is within the scrollbar hit zone.
    pub(in crate::gui) fn is_in_scrollbar_zone(&self, x: f64, window_width: u32) -> bool {
        let hit_zone = self.backend.scrollbar_hit_zone_px();
        x >= window_width.saturating_sub(hit_zone) as f64
    }
}
