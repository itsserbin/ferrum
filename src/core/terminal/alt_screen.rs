//! Alternate screen buffer management (used by vim, htop, less, etc.).

use crate::core::{PageCoord, PageList};

use super::CursorStyle;

impl super::Terminal {
    /// Enters the alternate screen buffer.
    ///
    /// Saves the current screen, cursor position, and scroll region,
    /// then replaces the screen with a blank one.
    pub(super) fn enter_alt_screen(&mut self) {
        if self.alt_screen.is_none() {
            let rows = self.screen.viewport_rows();
            let cols = self.screen.cols();

            let alt_screen = PageList::new(rows, cols, 0); // alt screen has no scrollback
            let abs_start = alt_screen.viewport_start_abs();
            // Save cursor coord before swapping.
            self.alt_saved_cursor = PageCoord {
                abs_row: self.cursor_pin.coord().abs_row,
                col: self.cursor_col(),
            };
            let main_screen = std::mem::replace(&mut self.screen, alt_screen);
            self.alt_screen = Some(main_screen);
            // Register a new cursor pin on the alt screen at (0, 0).
            let new_cursor_coord = PageCoord { abs_row: abs_start, col: 0 };
            self.cursor_pin = self.screen.pin_at(new_cursor_coord);

            self.saved_scroll_top = self.scroll_top;
            self.saved_scroll_bottom = self.scroll_bottom;
            // cursor_row() == 0, cursor_col() == 0 automatically (pin at abs_start, col 0).
            self.cursor_style = CursorStyle::BlinkingBlock;
            self.scroll_top = 0;
            self.scroll_bottom = rows - 1;
        }
    }

    /// Leaves the alternate screen and restores the main buffer.
    pub(super) fn leave_alt_screen(&mut self) {
        if let Some(main_screen) = self.alt_screen.take() {
            self.screen = main_screen;
            // Restore cursor position from saved coord.
            let saved = self.alt_saved_cursor;
            let vstart = self.screen.viewport_start_abs();
            let rows = self.screen.viewport_rows();
            let cols = self.screen.cols();
            let row = saved.abs_row.saturating_sub(vstart).min(rows.saturating_sub(1));
            let col = saved.col.min(cols.saturating_sub(1));
            self.cursor_pin = self.screen.pin_at(PageCoord {
                abs_row: vstart + row,
                col,
            });

            self.scroll_top = self.saved_scroll_top;
            self.scroll_bottom = self.saved_scroll_bottom;
            self.cursor_style = CursorStyle::default();
        }
    }
}
