use crate::gui::input::encode_mouse_event;
use crate::gui::pane::{DIVIDER_WIDTH, PaneId};
use crate::gui::*;

impl FerrumWindow {
    fn pane_under_mouse(&self) -> Option<PaneId> {
        let terminal_rect = self.terminal_content_rect();
        self.active_tab_ref().and_then(|tab| {
            tab.pane_tree.pane_at_pixel(
                self.mouse_pos.0 as u32,
                self.mouse_pos.1 as u32,
                terminal_rect,
                DIVIDER_WIDTH,
            )
        })
    }

    fn wheel_target_pane_id(&self) -> Option<PaneId> {
        self.active_tab_ref()
            .map(|tab| self.pane_under_mouse().unwrap_or(tab.focused_pane))
    }

    fn wheel_grid_pos_for_pane(&self, pane_id: PaneId) -> Option<(usize, usize)> {
        let tab = self.active_tab_ref()?;
        let leaf = tab.pane_tree.find_leaf(pane_id)?;
        let terminal_rect = self.terminal_content_rect();
        let pane_rect = tab
            .pane_tree
            .layout(terminal_rect, DIVIDER_WIDTH)
            .into_iter()
            .find_map(|(id, rect)| (id == pane_id).then_some(rect))?;

        let local_x = (self.mouse_pos.0 as u32).saturating_sub(pane_rect.x);
        let local_y = (self.mouse_pos.1 as u32).saturating_sub(pane_rect.y);
        let col = ((local_x + self.backend.cell_width() / 2) as usize
            / self.backend.cell_width() as usize)
            .min(leaf.terminal.grid.cols.saturating_sub(1));
        let row = (local_y as usize / self.backend.cell_height() as usize)
            .min(leaf.terminal.grid.rows.saturating_sub(1));
        Some((row, col))
    }

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

        if raw_lines == 0 {
            return;
        }

        let Some(target_pane) = self.wheel_target_pane_id() else {
            return;
        };

        let mouse_reporting = !self.modifiers.shift_key()
            && self
                .active_tab_ref()
                .and_then(|tab| tab.pane_tree.find_leaf(target_pane))
                .is_some_and(|leaf| leaf.terminal.mouse_mode != MouseMode::Off);

        // Mouse reporting -- send scroll events to app for pane under cursor.
        if mouse_reporting {
            let sgr = self
                .active_tab_ref()
                .and_then(|tab| tab.pane_tree.find_leaf(target_pane))
                .is_some_and(|leaf| leaf.terminal.sgr_mouse);
            let Some((row, col)) = self.wheel_grid_pos_for_pane(target_pane) else {
                return;
            };
            let button = if raw_lines > 0 { 64u8 } else { 65u8 };
            let bytes = encode_mouse_event(button, col, row, true, sgr);
            if let Some(tab) = self.active_tab_mut()
                && let Some(leaf) = tab.pane_tree.find_leaf_mut(target_pane)
            {
                for _ in 0..raw_lines.unsigned_abs() {
                    let _ = leaf.pty_writer.write_all(&bytes);
                }
                let _ = leaf.pty_writer.flush();
            }
            return;
        }

        // Scrollback/alt-screen code for pane under cursor.
        if let Some(tab) = self.active_tab_mut()
            && let Some(leaf) = tab.pane_tree.find_leaf_mut(target_pane)
        {
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
