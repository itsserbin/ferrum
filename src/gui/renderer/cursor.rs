use super::*;

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    pub fn draw_cursor(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
    ) {
        let x = col as u32 * self.cell_width + WINDOW_PADDING;
        let y = row as u32 * self.cell_height + TAB_BAR_HEIGHT + WINDOW_PADDING;
        let cursor_pixel = Color::DEFAULT_FG.to_pixel();

        match style {
            CursorStyle::BlinkingBlock | CursorStyle::SteadyBlock => {
                // Filled block with inverted foreground/background.
                let cell = grid.get(row, col);
                self.draw_bg(buffer, buf_width, buf_height, x, y, Color::DEFAULT_FG);
                if cell.character != ' ' {
                    self.draw_char(
                        buffer,
                        buf_width,
                        buf_height,
                        x,
                        y,
                        cell.character,
                        Color::DEFAULT_BG,
                    );
                }
            }
            CursorStyle::BlinkingUnderline | CursorStyle::SteadyUnderline => {
                // 2px underline at the bottom of the cell.
                let underline_h = 2usize;
                let base_y = y as usize + self.cell_height as usize - underline_h;
                for dy in 0..underline_h {
                    let py = base_y + dy;
                    if py >= buf_height {
                        break;
                    }
                    for dx in 0..self.cell_width as usize {
                        let px = x as usize + dx;
                        if px < buf_width {
                            buffer[py * buf_width + px] = cursor_pixel;
                        }
                    }
                }
            }
            CursorStyle::BlinkingBar | CursorStyle::SteadyBar => {
                // 2px vertical bar at the left edge.
                let bar_width = 2usize;
                for dy in 0..self.cell_height as usize {
                    let py = y as usize + dy;
                    if py >= buf_height {
                        break;
                    }
                    for dx in 0..bar_width {
                        let px = x as usize + dx;
                        if px < buf_width {
                            buffer[py * buf_width + px] = cursor_pixel;
                        }
                    }
                }
            }
        }
    }
}
