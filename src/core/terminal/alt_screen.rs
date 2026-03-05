//! Alternate screen buffer management (used by vim, htop, less, etc.).

use crate::core::{Grid, PageCoord, PageList};

use super::CursorStyle;

impl super::Terminal {
    /// Enters the alternate screen buffer.
    ///
    /// Saves the current grid, cursor position, and scroll region,
    /// then replaces the grid with a blank one.
    pub(super) fn enter_alt_screen(&mut self) {
        if self.alt_screen.is_none() {
            let rows = self.grid.rows;
            let cols = self.grid.cols;

            // ── screen (PageList) ────────────────────────────────────────────
            let alt_screen = PageList::new(rows, cols, 0); // alt screen has no scrollback
            let abs_start = alt_screen.viewport_start_abs();
            // Save cursor coord before swapping.
            let cur_abs = self.screen.viewport_start_abs() + self.cursor_row;
            self.alt_saved_cursor = PageCoord { abs_row: cur_abs, col: self.cursor_col };
            let main_screen = std::mem::replace(&mut self.screen, alt_screen);
            self.alt_screen = Some(main_screen);
            // Register a new cursor pin on the alt screen at (0, 0).
            let new_cursor_coord = PageCoord { abs_row: abs_start, col: 0 };
            self.cursor_pin = self.screen.register_pin(new_cursor_coord);

            // ── grid (display cache) ─────────────────────────────────────────
            let alt_grid = Grid::new(rows, cols);
            self.alt_grid = Some(std::mem::replace(&mut self.grid, alt_grid));

            self.saved_scroll_top = self.scroll_top;
            self.saved_scroll_bottom = self.scroll_bottom;
            self.cursor_row = 0;
            self.cursor_col = 0;
            self.cursor_style = CursorStyle::BlinkingBlock;
            self.scroll_top = 0;
            self.scroll_bottom = rows - 1;
        }
    }

    /// Leaves the alternate screen and restores the main buffer.
    pub(super) fn leave_alt_screen(&mut self) {
        if let Some(main_screen) = self.alt_screen.take() {
            // ── screen (PageList) ────────────────────────────────────────────
            self.screen = main_screen;
            // Restore cursor position from saved coord.
            let saved = self.alt_saved_cursor;
            let vstart = self.screen.viewport_start_abs();
            let rows = self.screen.viewport_rows();
            let cols = self.screen.cols();
            self.cursor_row =
                saved.abs_row.saturating_sub(vstart).min(rows.saturating_sub(1));
            self.cursor_col = saved.col.min(cols.saturating_sub(1));
            let restored_abs = vstart + self.cursor_row;
            self.cursor_pin = self.screen.register_pin(PageCoord {
                abs_row: restored_abs,
                col: self.cursor_col,
            });

            // ── grid (display cache) ─────────────────────────────────────────
            if let Some(main_grid) = self.alt_grid.take() {
                self.grid = main_grid;
            }

            self.scroll_top = self.saved_scroll_top;
            self.scroll_bottom = self.saved_scroll_bottom;
            self.cursor_style = CursorStyle::default();
        }
    }
}
