//! Alternate screen buffer management (used by vim, htop, less, etc.).

use crate::core::Grid;

use super::CursorStyle;

impl super::Terminal {
    /// Enters the alternate screen buffer.
    ///
    /// Saves the current grid, cursor position, and scroll region,
    /// then replaces the grid with a blank one.
    pub(super) fn enter_alt_screen(&mut self) {
        if self.alt_grid.is_none() {
            let alt = Grid::new(self.grid.rows, self.grid.cols);
            self.alt_grid = Some(std::mem::replace(&mut self.grid, alt));
            self.alt_saved_cursor = (self.cursor_row, self.cursor_col);
            self.saved_scroll_top = self.scroll_top;
            self.saved_scroll_bottom = self.scroll_bottom;
            self.cursor_row = 0;
            self.cursor_col = 0;
            self.cursor_style = CursorStyle::BlinkingBlock;
            self.scroll_top = 0;
            self.scroll_bottom = self.grid.rows - 1;
        }
    }

    /// Leaves the alternate screen and restores the main buffer.
    pub(super) fn leave_alt_screen(&mut self) {
        if let Some(main_grid) = self.alt_grid.take() {
            self.grid = main_grid;
            self.cursor_row = self.alt_saved_cursor.0;
            self.cursor_col = self.alt_saved_cursor.1;
            self.scroll_top = self.saved_scroll_top;
            self.scroll_bottom = self.saved_scroll_bottom;
            self.cursor_style = CursorStyle::default();
        }
    }
}
