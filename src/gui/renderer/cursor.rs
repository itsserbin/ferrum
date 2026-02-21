use super::*;
use super::RenderTarget;
use crate::core::Cell;
use crate::gui::pane::PaneRect;

impl CpuRenderer {
    pub fn draw_cursor(
        &mut self,
        target: &mut RenderTarget<'_>,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
    ) {
        let buf_width = target.width;
        let buf_height = target.height;
        let x = col as u32 * self.metrics.cell_width + self.window_padding_px();
        let y = row as u32 * self.metrics.cell_height
            + self.tab_bar_height_px()
            + self.window_padding_px();
        let cursor_pixel = self.palette.default_fg.to_pixel();

        match style {
            CursorStyle::BlinkingBlock | CursorStyle::SteadyBlock => {
                // Filled block with inverted foreground/background.
                let cell = grid.get(row, col).unwrap_or(&Cell::DEFAULT);
                self.draw_bg(target, x, y, self.palette.default_fg);
                if cell.character != ' ' {
                    self.draw_char(target, x, y, cell.character, self.palette.default_bg);
                }
            }
            CursorStyle::BlinkingUnderline | CursorStyle::SteadyUnderline => {
                // 2px underline at the bottom of the cell.
                let underline_h = 2usize;
                let base_y = y as usize + self.metrics.cell_height as usize - underline_h;
                for dy in 0..underline_h {
                    let py = base_y + dy;
                    if py >= buf_height {
                        break;
                    }
                    for dx in 0..self.metrics.cell_width as usize {
                        let px = x as usize + dx;
                        if px < buf_width {
                            target.buffer[py * buf_width + px] = cursor_pixel;
                        }
                    }
                }
            }
            CursorStyle::BlinkingBar | CursorStyle::SteadyBar => {
                // 2px vertical bar at the left edge.
                let bar_width = 2usize;
                for dy in 0..self.metrics.cell_height as usize {
                    let py = y as usize + dy;
                    if py >= buf_height {
                        break;
                    }
                    for dx in 0..bar_width {
                        let px = x as usize + dx;
                        if px < buf_width {
                            target.buffer[py * buf_width + px] = cursor_pixel;
                        }
                    }
                }
            }
        }
    }

    /// Draws the cursor at a position offset by a pane rectangle.
    pub fn draw_cursor_in_rect(
        &mut self,
        target: &mut RenderTarget<'_>,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
        rect: PaneRect,
    ) {
        let buf_width = target.width;
        let buf_height = target.height;
        let x = col as u32 * self.metrics.cell_width + rect.x;
        let y = row as u32 * self.metrics.cell_height + rect.y;
        let cursor_pixel = self.palette.default_fg.to_pixel();

        let rect_right = (rect.x + rect.width) as usize;
        let rect_bottom = (rect.y + rect.height) as usize;

        match style {
            CursorStyle::BlinkingBlock | CursorStyle::SteadyBlock => {
                let cell = grid.get(row, col).unwrap_or(&Cell::DEFAULT);
                self.draw_bg(target, x, y, self.palette.default_fg);
                if cell.character != ' ' {
                    self.draw_char(target, x, y, cell.character, self.palette.default_bg);
                }
            }
            CursorStyle::BlinkingUnderline | CursorStyle::SteadyUnderline => {
                let underline_h = 2usize;
                let base_y = y as usize + self.metrics.cell_height as usize - underline_h;
                for dy in 0..underline_h {
                    let py = base_y + dy;
                    if py >= buf_height || py >= rect_bottom {
                        break;
                    }
                    for dx in 0..self.metrics.cell_width as usize {
                        let px = x as usize + dx;
                        if px < buf_width && px < rect_right {
                            target.buffer[py * buf_width + px] = cursor_pixel;
                        }
                    }
                }
            }
            CursorStyle::BlinkingBar | CursorStyle::SteadyBar => {
                let bar_width = 2usize;
                for dy in 0..self.metrics.cell_height as usize {
                    let py = y as usize + dy;
                    if py >= buf_height || py >= rect_bottom {
                        break;
                    }
                    for dx in 0..bar_width {
                        let px = x as usize + dx;
                        if px < buf_width && px < rect_right {
                            target.buffer[py * buf_width + px] = cursor_pixel;
                        }
                    }
                }
            }
        }
    }
}
