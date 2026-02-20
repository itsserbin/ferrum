use crate::gui::*;

impl FerrumWindow {
    pub(crate) fn on_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let raw_lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => {
                // Line-based scroll (mouse wheel) -- reset accumulator
                self.scroll_accumulator = 0.0;
                y as isize
            }
            MouseScrollDelta::PixelDelta(pos) => {
                // Pixel-based scroll (trackpad) -- accumulate small deltas
                self.scroll_accumulator += pos.y;
                let cell_h = self.backend.cell_height() as f64;
                let lines = (self.scroll_accumulator / cell_h) as isize;
                if lines != 0 {
                    self.scroll_accumulator -= lines as f64 * cell_h;
                }
                lines
            }
        };

        // Mouse reporting -- send scroll events to app
        if self.is_mouse_reporting() {
            let (row, col) = self.pixel_to_grid(self.mouse_pos.0, self.mouse_pos.1);
            let button = if raw_lines > 0 { 64u8 } else { 65u8 };
            for _ in 0..raw_lines.unsigned_abs() {
                self.send_mouse_event(button, col, row, true);
            }
            return;
        }

        // Existing scrollback/alt-screen code
        if let Some(leaf) = self.active_leaf_mut() {
            if leaf.terminal.is_alt_screen() {
                let lines = raw_lines;
                let seq = if lines > 0 { b"\x1b[A" } else { b"\x1b[B" };
                for _ in 0..lines.unsigned_abs() {
                    let _ = leaf.pty_writer.write_all(seq);
                }
                let _ = leaf.pty_writer.flush();
                return;
            }

            let lines = raw_lines;
            if lines > 0 {
                leaf.scroll_offset =
                    (leaf.scroll_offset + lines as usize).min(leaf.terminal.scrollback.len());
            } else if lines < 0 {
                leaf.scroll_offset = leaf.scroll_offset.saturating_sub((-lines) as usize);
            }
            leaf.scrollbar.last_activity = std::time::Instant::now();
        }
    }
}
