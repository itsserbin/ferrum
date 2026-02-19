//! Cursor rendering for the GPU renderer (block, underline, bar styles).

use crate::core::{Cell, Color, CursorStyle, Grid};

impl super::GpuRenderer {
    pub(super) fn draw_cursor_impl(
        &mut self,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
    ) {
        let x =
            col as f32 * self.metrics.cell_width as f32 + self.metrics.window_padding_px() as f32;
        let y = row as f32 * self.metrics.cell_height as f32
            + self.metrics.tab_bar_height_px() as f32
            + self.metrics.window_padding_px() as f32;
        let cw = self.metrics.cell_width as f32;
        let ch = self.metrics.cell_height as f32;
        let cursor_color = Color::DEFAULT_FG.to_pixel();

        match style {
            CursorStyle::BlinkingBlock | CursorStyle::SteadyBlock => {
                self.push_rect(x, y, cw, ch, cursor_color, 1.0);
                let cell = grid.get(row, col).unwrap_or(&Cell::DEFAULT);
                if cell.character != ' ' {
                    let cp = cell.character as u32;
                    let info = self.atlas.get_or_insert(
                        cp,
                        &self.font,
                        self.metrics.font_size,
                        &self.queue,
                    );
                    if info.w > 0.0 && info.h > 0.0 {
                        let gx = x + info.offset_x;
                        let gy = y + info.offset_y;
                        self.push_glyph(
                            gx,
                            gy,
                            info.x,
                            info.y,
                            info.w,
                            info.h,
                            Color::DEFAULT_BG.to_pixel(),
                            1.0,
                        );
                    }
                }
            }
            CursorStyle::BlinkingUnderline | CursorStyle::SteadyUnderline => {
                self.push_rect(x, y + ch - 2.0, cw, 2.0, cursor_color, 1.0);
            }
            CursorStyle::BlinkingBar | CursorStyle::SteadyBar => {
                self.push_rect(x, y, 2.0, ch, cursor_color, 1.0);
            }
        }
    }
}
