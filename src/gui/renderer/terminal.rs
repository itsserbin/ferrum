use super::*;
use crate::gui::pane::PaneRect;

impl CpuRenderer {
    /// Renders terminal cells with top/left offsets for tab bar and padding.
    pub fn render(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        grid: &Grid,
        selection: Option<&Selection>,
        viewport_start: usize,
    ) {
        let y_offset = self.tab_bar_height_px() + self.window_padding_px();
        let x_offset = self.window_padding_px();

        for row in 0..grid.rows {
            let abs_row = viewport_start + row;
            for col in 0..grid.cols {
                // Safe: iterating within grid bounds
                let cell = grid.get_unchecked(row, col);
                let x = col as u32 * self.metrics.cell_width + x_offset;
                let y = row as u32 * self.metrics.cell_height + y_offset;

                if x as usize >= buf_width || y as usize >= buf_height {
                    continue;
                }

                // Invert colors if the cell is selected
                let selected = selection.is_some_and(|s| s.contains(abs_row, col));
                let (mut fg, mut bg) = if selected {
                    (cell.bg, cell.fg)
                } else {
                    (cell.fg, cell.bg)
                };

                // Reverse video
                if cell.reverse && !selected {
                    std::mem::swap(&mut fg, &mut bg);
                }

                // Bold: bright variant
                if cell.bold {
                    fg = fg.bold_bright();
                }

                self.draw_bg(buffer, buf_width, buf_height, x, y, bg);

                if cell.character != ' ' {
                    self.draw_char(buffer, buf_width, buf_height, x, y, cell.character, fg);
                }

                // Underline
                if cell.underline {
                    let underline_y = y + self.metrics.cell_height - 2;
                    if (underline_y as usize) < buf_height {
                        let pixel = fg.to_pixel();
                        for dx in 0..self.metrics.cell_width as usize {
                            let px = x as usize + dx;
                            if px < buf_width {
                                let idx = underline_y as usize * buf_width + px;
                                if idx < buffer.len() {
                                    buffer[idx] = pixel;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Renders terminal cells into a sub-rectangle of the buffer.
    ///
    /// Like `render()` but uses `rect` as the origin and clips to its bounds.
    #[allow(clippy::too_many_arguments)]
    pub fn render_in_rect(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        grid: &Grid,
        selection: Option<&Selection>,
        viewport_start: usize,
        rect: PaneRect,
    ) {
        let rect_right = (rect.x + rect.width) as usize;
        let rect_bottom = (rect.y + rect.height) as usize;

        for row in 0..grid.rows {
            let abs_row = viewport_start + row;
            for col in 0..grid.cols {
                let cell = grid.get_unchecked(row, col);
                let x = col as u32 * self.metrics.cell_width + rect.x;
                let y = row as u32 * self.metrics.cell_height + rect.y;

                // Clip to pane rect and buffer bounds.
                if x as usize >= rect_right
                    || y as usize >= rect_bottom
                    || x as usize >= buf_width
                    || y as usize >= buf_height
                {
                    continue;
                }

                let selected = selection.is_some_and(|s| s.contains(abs_row, col));
                let (mut fg, mut bg) = if selected {
                    (cell.bg, cell.fg)
                } else {
                    (cell.fg, cell.bg)
                };

                if cell.reverse && !selected {
                    std::mem::swap(&mut fg, &mut bg);
                }

                if cell.bold {
                    fg = fg.bold_bright();
                }

                self.draw_bg(buffer, buf_width, buf_height, x, y, bg);

                if cell.character != ' ' {
                    self.draw_char(buffer, buf_width, buf_height, x, y, cell.character, fg);
                }

                if cell.underline {
                    let underline_y = y + self.metrics.cell_height - 2;
                    if (underline_y as usize) < buf_height && (underline_y as usize) < rect_bottom {
                        let pixel = fg.to_pixel();
                        for dx in 0..self.metrics.cell_width as usize {
                            let px = x as usize + dx;
                            if px < buf_width && px < rect_right {
                                let idx = underline_y as usize * buf_width + px;
                                if idx < buffer.len() {
                                    buffer[idx] = pixel;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
