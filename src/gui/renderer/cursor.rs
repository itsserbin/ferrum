use super::*;
use super::RenderTarget;
use crate::core::PageList;
use super::super::pane::PaneRect;

/// A pixel-space rectangle used as a fill target or clip boundary inside the cursor renderer.
struct PixelRect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

/// Returns the character at `(row, col)` in the viewport, or `None` when out of bounds.
fn block_char_at(screen: &PageList, row: usize, col: usize) -> Option<char> {
    if row < screen.viewport_rows() && col < screen.cols() {
        Some(screen.viewport_get(row, col).first_char())
    } else {
        None
    }
}

impl CpuRenderer {
    /// Fills `region` with `pixel`, clipped to `clip` and the buffer bounds.
    fn fill_rect_pixels(
        target: &mut RenderTarget<'_>,
        region: PixelRect,
        pixel: u32,
        clip: PixelRect,
    ) {
        let buf_width = target.width;
        for dy in 0..region.h {
            let py = region.y + dy;
            if py >= target.height || py >= clip.y + clip.h {
                break;
            }
            for dx in 0..region.w {
                let px = region.x + dx;
                if px < buf_width && px < clip.x + clip.w {
                    target.buffer[py * buf_width + px] = pixel;
                }
            }
        }
    }

    /// Draws the cursor shape at pixel position `pos`, clipped to `clip`.
    ///
    /// `block_char` is read from the screen grid before calling this and passed
    /// directly, reducing the argument count to satisfy the clippy limit.
    fn draw_cursor_shape(
        &mut self,
        target: &mut RenderTarget<'_>,
        pos: (u32, u32),
        cursor_pixel: u32,
        clip: PixelRect,
        block_char: Option<char>,
        style: CursorStyle,
    ) {
        let (x, y) = pos;
        match style {
            CursorStyle::BlinkingBlock | CursorStyle::SteadyBlock => {
                // Filled block with inverted foreground/background.
                self.draw_bg(target, x, y, self.palette.default_fg);
                let ch = block_char.unwrap_or(' ');
                if ch != ' ' {
                    self.draw_char(target, x, y, ch, self.palette.default_bg);
                }
            }
            CursorStyle::BlinkingUnderline | CursorStyle::SteadyUnderline => {
                // 2px underline at the bottom of the cell.
                let underline_h = 2usize;
                let base_y = y as usize + self.metrics.cell_height as usize - underline_h;
                Self::fill_rect_pixels(
                    target,
                    PixelRect { x: x as usize, y: base_y, w: self.metrics.cell_width as usize, h: underline_h },
                    cursor_pixel,
                    clip,
                );
            }
            CursorStyle::BlinkingBar | CursorStyle::SteadyBar => {
                // 2px vertical bar at the left edge.
                Self::fill_rect_pixels(
                    target,
                    PixelRect { x: x as usize, y: y as usize, w: 2, h: self.metrics.cell_height as usize },
                    cursor_pixel,
                    clip,
                );
            }
        }
    }

    pub fn draw_cursor(
        &mut self,
        target: &mut RenderTarget<'_>,
        row: usize,
        col: usize,
        screen: &PageList,
        style: CursorStyle,
    ) {
        let x = col as u32 * self.metrics.cell_width + self.window_padding_px();
        let y = row as u32 * self.metrics.cell_height
            + self.tab_bar_height_px()
            + self.window_padding_px();
        let cursor_pixel = self.palette.default_fg.to_pixel();
        let block_char = block_char_at(screen, row, col);
        // No clip rectangle: use the full buffer.
        let clip = PixelRect { x: 0, y: 0, w: target.width, h: target.height };
        self.draw_cursor_shape(target, (x, y), cursor_pixel, clip, block_char, style);
    }

    /// Draws the cursor at a position offset by a pane rectangle.
    pub fn draw_cursor_in_rect(
        &mut self,
        target: &mut RenderTarget<'_>,
        row: usize,
        col: usize,
        screen: &PageList,
        style: CursorStyle,
        rect: PaneRect,
    ) {
        let x = col as u32 * self.metrics.cell_width + rect.x;
        let y = row as u32 * self.metrics.cell_height + rect.y;
        let cursor_pixel = self.palette.default_fg.to_pixel();
        let block_char = block_char_at(screen, row, col);
        let clip = PixelRect {
            x: rect.x as usize,
            y: rect.y as usize,
            w: rect.width as usize,
            h: rect.height as usize,
        };
        self.draw_cursor_shape(target, (x, y), cursor_pixel, clip, block_char, style);
    }
}
