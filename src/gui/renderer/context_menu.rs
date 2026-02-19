use super::*;

impl ContextMenu {
    pub fn for_tab(x: u32, y: u32, tab_index: usize) -> Self {
        let items = vec![
            (ContextAction::RenameTab, "Rename"),
            (ContextAction::DuplicateTab, "Duplicate"),
            (ContextAction::CloseTab, "Close"),
        ];
        ContextMenu {
            x,
            y,
            target: ContextMenuTarget::Tab { tab_index },
            hover_progress: vec![0.0; items.len()],
            items,
            hover_index: None,
            opened_at: std::time::Instant::now(),
        }
    }

    pub fn for_terminal_selection(x: u32, y: u32) -> Self {
        let items = vec![
            (ContextAction::CopySelection, "Copy"),
            (ContextAction::Paste, "Paste"),
            (ContextAction::ClearSelection, "Clear Selection"),
        ];
        ContextMenu {
            x,
            y,
            target: ContextMenuTarget::TerminalSelection,
            hover_progress: vec![0.0; items.len()],
            items,
            hover_index: None,
            opened_at: std::time::Instant::now(),
        }
    }

    /// Menu width in pixels.
    pub(crate) fn width(&self, cell_width: u32) -> u32 {
        let label_chars = self
            .items
            .iter()
            .map(|(_, label)| label.chars().count() as u32)
            .max()
            .unwrap_or(12);
        cell_width * label_chars.saturating_add(4).max(16)
    }

    /// Single menu item height in pixels.
    pub(crate) fn item_height(&self, cell_height: u32) -> u32 {
        cell_height + 4
    }

    /// Total menu height in pixels.
    pub(crate) fn height(&self, cell_height: u32) -> u32 {
        self.item_height(cell_height) * self.items.len() as u32 + 4
    }
}

impl CpuRenderer {
    /// Hit-tests context menu and returns hovered item index.
    pub fn hit_test_context_menu(&self, menu: &ContextMenu, x: f64, y: f64) -> Option<usize> {
        menu.hit_test(x, y, self.cell_width, self.cell_height)
    }

    /// Draws context menu overlay.
    pub fn draw_context_menu(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        menu: &ContextMenu,
    ) {
        let mw = menu.width(self.cell_width);
        let ih = menu.item_height(self.cell_height);
        let mh = menu.height(self.cell_height);
        let mx = menu.x;
        let my = menu.y;

        let radius = self.scaled_px(6);
        let open_t = (menu.opened_at.elapsed().as_secs_f32() / 0.14).clamp(0.0, 1.0);
        let open_ease = 1.0 - (1.0 - open_t) * (1.0 - open_t);
        let panel_alpha = (228.0 + open_ease * 20.0).round().clamp(0.0, 255.0) as u8;

        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            mx as i32,
            my as i32,
            mw,
            mh,
            radius,
            MENU_BG,
            panel_alpha,
        );
        self.draw_rounded_rect(
            buffer, buf_width, buf_height, mx as i32, my as i32, mw, mh, radius, 0xFFFFFF, 30,
        );

        // Draw menu items.
        for (i, (action, label)) in menu.items.iter().enumerate() {
            let item_y = my + self.scaled_px(2) + i as u32 * ih;
            let hover_t = menu
                .hover_progress
                .get(i)
                .copied()
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);

            // Hover highlight for the active row.
            if hover_t > 0.01 {
                let hover_x = mx + self.scaled_px(4);
                let hover_w = mw.saturating_sub(self.scaled_px(8));
                let hover_h = ih.saturating_sub(self.scaled_px(1));
                let alpha = (120.0 + hover_t * 100.0).round().clamp(0.0, 255.0) as u8;
                self.draw_rounded_rect(
                    buffer,
                    buf_width,
                    buf_height,
                    hover_x as i32,
                    item_y as i32,
                    hover_w,
                    hover_h,
                    self.scaled_px(6),
                    MENU_HOVER_BG,
                    alpha,
                );
            }

            let fg = if *action == ContextAction::CloseTab {
                DESTRUCTIVE_COLOR // Red for destructive action.
            } else {
                Color::DEFAULT_FG
            };

            let text_x = mx + self.cell_width;
            let text_y = item_y + self.scaled_px(2);
            for (ci, ch) in label.chars().enumerate() {
                let cx = text_x + ci as u32 * self.cell_width;
                if cx + self.cell_width <= mx + mw {
                    self.draw_char(buffer, buf_width, buf_height, cx, text_y, ch, fg);
                }
            }
        }
    }
}
