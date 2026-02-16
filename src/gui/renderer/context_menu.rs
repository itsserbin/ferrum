use super::*;

impl ContextMenu {
    pub fn new(x: u32, y: u32, tab_index: usize) -> Self {
        ContextMenu {
            x,
            y,
            tab_index,
            items: vec![
                (ContextAction::Rename, "Перейменувати"),
                (ContextAction::Duplicate, "Дублювати"),
                (ContextAction::Close, "Закрити"),
            ],
            hover_index: None,
        }
    }

    /// Menu width in pixels.
    pub(crate) fn width(&self, cell_width: u32) -> u32 {
        cell_width * 16
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
        let mw = menu.width(self.cell_width);
        let ih = menu.item_height(self.cell_height);
        let mh = menu.height(self.cell_height);

        if x < menu.x as f64
            || x >= (menu.x + mw) as f64
            || y < menu.y as f64
            || y >= (menu.y + mh) as f64
        {
            return None;
        }

        let rel_y = (y - menu.y as f64 - 2.0) as u32;
        let idx = rel_y / ih;
        if (idx as usize) < menu.items.len() {
            Some(idx as usize)
        } else {
            None
        }
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

        let hover_pixel = 0x31394D;
        let radius = self.scaled_px(6);

        self.draw_rounded_rect(
            buffer, buf_width, buf_height, mx as i32, my as i32, mw, mh, radius, 0x1E2433, 248,
        );
        self.draw_rounded_rect(
            buffer, buf_width, buf_height, mx as i32, my as i32, mw, mh, radius, 0xFFFFFF, 20,
        );

        // Draw menu items.
        for (i, (action, label)) in menu.items.iter().enumerate() {
            let item_y = my + self.scaled_px(2) + i as u32 * ih;

            // Hover highlight for the active row.
            if menu.hover_index == Some(i) {
                let hover_x = mx + self.scaled_px(4);
                let hover_w = mw.saturating_sub(self.scaled_px(8));
                let hover_h = ih.saturating_sub(self.scaled_px(1));
                self.draw_rounded_rect(
                    buffer,
                    buf_width,
                    buf_height,
                    hover_x as i32,
                    item_y as i32,
                    hover_w,
                    hover_h,
                    self.scaled_px(6),
                    hover_pixel,
                    220,
                );
            }

            let fg = if *action == ContextAction::Close {
                Color {
                    r: 243,
                    g: 139,
                    b: 168,
                } // Red for destructive action.
            } else {
                Color::DEFAULT_FG
            };

            let text_x = mx + self.cell_width;
            let text_y = item_y + self.scaled_px(2);
            for (ci, ch) in label.chars().enumerate() {
                let cx = text_x + ci as u32 * self.cell_width;
                if cx + self.cell_width <= mx + mw {
                    self.draw_char_at(buffer, buf_width, buf_height, cx, text_y, ch, fg);
                }
            }
        }
    }
}
