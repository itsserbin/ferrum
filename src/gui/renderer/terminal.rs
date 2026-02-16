use super::*;

impl CpuRenderer {
    /// Renders terminal cells with top/left offsets for tab bar and padding.
    pub fn render(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        grid: &Grid,
        selection: Option<&Selection>,
    ) {
        let y_offset = self.tab_bar_height_px() + self.window_padding_px();
        let x_offset = self.window_padding_px();

        for row in 0..grid.rows {
            for col in 0..grid.cols {
                let cell = grid.get(row, col);
                let x = col as u32 * self.cell_width + x_offset;
                let y = row as u32 * self.cell_height + y_offset;

                if x as usize >= buf_width || y as usize >= buf_height {
                    continue;
                }

                // Invert colors if the cell is selected
                let selected = selection.is_some_and(|s| s.contains(row, col));
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
                    for i in 0..8 {
                        if fg.r == Color::ANSI[i].r
                            && fg.g == Color::ANSI[i].g
                            && fg.b == Color::ANSI[i].b
                        {
                            fg = Color::ANSI[i + 8];
                            break;
                        }
                    }
                }

                self.draw_bg(buffer, buf_width, buf_height, x, y, bg);

                if cell.character != ' ' {
                    self.draw_char(buffer, buf_width, buf_height, x, y, cell.character, fg);
                }

                // Underline
                if cell.underline {
                    let underline_y = y + self.cell_height - 2;
                    if (underline_y as usize) < buf_height {
                        let pixel = fg.to_pixel();
                        for dx in 0..self.cell_width as usize {
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
}
