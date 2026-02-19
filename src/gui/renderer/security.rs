use super::shared::ui_layout;
use super::*;

impl SecurityPopup {
    pub(crate) fn line_height(&self, cell_height: u32) -> u32 {
        cell_height + 4
    }

    pub(crate) fn width(&self, cell_width: u32) -> u32 {
        let max_line_chars = self
            .lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        let title_chars = self.title.chars().count();
        let content_chars = max_line_chars.max(title_chars) as u32;
        ((content_chars + 3).max(24)) * cell_width
    }

    pub(crate) fn height(&self, cell_height: u32) -> u32 {
        let title_h = self.line_height(cell_height);
        let lines_h = self.line_height(cell_height) * self.lines.len() as u32;
        title_h + lines_h + 8
    }

    /// Computes the popup rectangle clamped to fit within the given buffer bounds.
    ///
    /// Returns `(x, y, width, height)`.
    pub(crate) fn clamped_rect(
        &self,
        cell_width: u32,
        cell_height: u32,
        buf_width: u32,
        buf_height: u32,
    ) -> (u32, u32, u32, u32) {
        let width = self.width(cell_width).min(buf_width);
        let height = self.height(cell_height).min(buf_height);
        let x = self.x.min(buf_width.saturating_sub(width));
        let y = self.y.min(buf_height.saturating_sub(height));
        (x, y, width, height)
    }

    /// Returns `true` when the pointer at `(x, y)` falls inside the popup.
    pub(crate) fn hit_test(
        &self,
        x: f64,
        y: f64,
        cell_width: u32,
        cell_height: u32,
        buf_width: u32,
        buf_height: u32,
    ) -> bool {
        let (px, py, pw, ph) = self.clamped_rect(cell_width, cell_height, buf_width, buf_height);
        x >= px as f64 && x < (px + pw) as f64 && y >= py as f64 && y < (py + ph) as f64
    }
}

impl CpuRenderer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_security_shield_icon(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x: u32,
        y: u32,
        size: u32,
        color: Color,
    ) {
        let pixel = color.to_pixel();
        let spans = ui_layout::shield_icon_spans(size);

        for (dy, &(left, right)) in spans.iter().enumerate() {
            let py = y as usize + dy;
            if py >= buf_height {
                break;
            }

            for dx in left..=right {
                let px = x as usize + dx as usize;
                if px >= buf_width {
                    continue;
                }
                let idx = py * buf_width + px;
                if idx < buffer.len() {
                    buffer[idx] = pixel;
                }
            }
        }
    }

    /// Draws security popup overlay using a shared layout.
    pub fn draw_security_popup(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        popup: &SecurityPopup,
    ) {
        let layout = popup.layout(
            self.metrics.cell_width,
            self.metrics.cell_height,
            self.ui_scale(),
            buf_width as u32,
            buf_height as u32,
        );

        self.draw_rounded_rect_cmd(buffer, buf_width, buf_height, &layout.bg);
        self.draw_rounded_rect_cmd(buffer, buf_width, buf_height, &layout.border);

        // Title text.
        let title_fg = Color::from_pixel(layout.title.color);
        let title_x = layout.title.x as u32;
        let title_y = layout.title.y as u32;
        for (i, ch) in layout.title.text.chars().enumerate() {
            let x = title_x + i as u32 * self.metrics.cell_width;
            self.draw_char(buffer, buf_width, buf_height, x, title_y, ch, title_fg);
        }

        // Separator line.
        self.draw_flat_rect_cmd(buffer, buf_width, buf_height, &layout.separator);

        // Content lines.
        for text_cmd in &layout.lines {
            let fg = Color::from_pixel(text_cmd.color);
            let tx = text_cmd.x as u32;
            let ty = text_cmd.y as u32;
            for (i, ch) in text_cmd.text.chars().enumerate() {
                let x = tx + i as u32 * self.metrics.cell_width;
                self.draw_char(buffer, buf_width, buf_height, x, ty, ch, fg);
            }
        }
    }
}
