use crate::gui::renderer::{SCROLLBAR_HIT_ZONE, TAB_BAR_HEIGHT, WINDOW_PADDING};
use crate::gui::*;

impl FerrumWindow {
    /// Converts window pixels to terminal grid coordinates.
    pub(in crate::gui) fn pixel_to_grid(&self, x: f64, y: f64) -> (usize, usize) {
        let col = ((x as u32).saturating_sub(WINDOW_PADDING) + self.renderer.cell_width / 2)
            as usize
            / self.renderer.cell_width as usize;
        let row = (y as u32).saturating_sub(TAB_BAR_HEIGHT + WINDOW_PADDING) as usize
            / self.renderer.cell_height as usize;
        if let Some(tab) = self.active_tab_ref() {
            let row = row.min(tab.terminal.grid.rows.saturating_sub(1));
            let col = col.min(tab.terminal.grid.cols.saturating_sub(1));
            (row, col)
        } else {
            (0, 0)
        }
    }

    /// Returns `true` if the given x coordinate is within the scrollbar hit zone.
    pub(in crate::gui) fn is_in_scrollbar_zone(&self, x: f64, window_width: u32) -> bool {
        x >= (window_width - SCROLLBAR_HIT_ZONE) as f64
    }
}
