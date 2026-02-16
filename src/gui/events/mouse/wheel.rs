use crate::gui::*;

impl FerrumWindow {
    pub(crate) fn on_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let raw_lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => y as isize,
            MouseScrollDelta::PixelDelta(pos) => {
                (pos.y / self.renderer.cell_height as f64) as isize
            }
        };

        // Mouse reporting — send scroll events to app
        if self.is_mouse_reporting() {
            let (row, col) = self.pixel_to_grid(self.mouse_pos.0, self.mouse_pos.1);
            let button = if raw_lines > 0 { 64u8 } else { 65u8 };
            for _ in 0..raw_lines.unsigned_abs() {
                self.send_mouse_event(button, col, row, true);
            }
            return;
        }

        // Existing scrollback/alt-screen code
        if let Some(tab) = self.active_tab_mut() {
            if tab.terminal.is_alt_screen() {
                let lines = raw_lines;
                let seq = if lines > 0 { b"\x1b[A" } else { b"\x1b[B" };
                for _ in 0..lines.unsigned_abs() {
                    let _ = tab.pty_writer.write_all(seq);
                }
                let _ = tab.pty_writer.flush();
                return;
            }

            let lines = raw_lines * 3;
            if lines > 0 {
                tab.scroll_offset =
                    (tab.scroll_offset + lines as usize).min(tab.terminal.scrollback.len());
            } else if lines < 0 {
                tab.scroll_offset = tab.scroll_offset.saturating_sub((-lines) as usize);
            }
            tab.scrollbar.last_activity = std::time::Instant::now();
        }
    }
}
