//! Cursor rendering for the GPU renderer (block, underline, bar styles).

use crate::core::{Cell, CursorStyle, Grid};
use crate::gui::pane::PaneRect;

impl super::GpuRenderer {
    fn draw_cursor_with_origin(
        &mut self,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
        origin_x: f32,
        origin_y: f32,
    ) {
        let x = col as f32 * self.metrics.cell_width as f32 + origin_x;
        let y = row as f32 * self.metrics.cell_height as f32 + origin_y;
        let cw = self.metrics.cell_width as f32;
        let ch = self.metrics.cell_height as f32;
        let cursor_color = self.palette.default_fg.to_pixel();

        match style {
            CursorStyle::BlinkingBlock | CursorStyle::SteadyBlock => {
                self.push_rect(x, y, cw, ch, cursor_color, 1.0);
                let cell = grid.get(row, col).unwrap_or(&Cell::DEFAULT);
                if cell.character != ' ' {
                    let cp = cell.character as u32;
                    let info = self.atlas.get_or_insert(
                        cp,
                        &self.font,
                        &self.fallback_font,
                        self.metrics.font_size,
                        &self.queue,
                    );
                    if info.w > 0.0 && info.h > 0.0 {
                        let gx = x + info.offset_x;
                        let gy = y + info.offset_y;
                        self.push_glyph(
                            gx,
                            gy,
                            (info.x, info.y, info.w, info.h),
                            self.palette.default_bg.to_pixel(),
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

    pub(super) fn draw_cursor_impl(
        &mut self,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
    ) {
        let origin_x = self.metrics.window_padding_px() as f32;
        let origin_y = (self.metrics.tab_bar_height_px() + self.metrics.window_padding_px()) as f32;
        self.draw_cursor_with_origin(row, col, grid, style, origin_x, origin_y);
    }

    pub(super) fn draw_cursor_in_rect_impl(
        &mut self,
        row: usize,
        col: usize,
        grid: &Grid,
        style: CursorStyle,
        rect: PaneRect,
    ) {
        self.draw_cursor_with_origin(row, col, grid, style, rect.x as f32, rect.y as f32);
    }
}
