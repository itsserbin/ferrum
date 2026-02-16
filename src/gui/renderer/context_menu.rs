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
    fn width(&self, cell_width: u32) -> u32 {
        cell_width * 16
    }

    /// Single menu item height in pixels.
    fn item_height(&self, cell_height: u32) -> u32 {
        cell_height + 4
    }

    /// Total menu height in pixels.
    fn height(&self, cell_height: u32) -> u32 {
        self.item_height(cell_height) * self.items.len() as u32 + 4
    }
}

impl Renderer {
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
        let mw = menu.width(self.cell_width) as usize;
        let ih = menu.item_height(self.cell_height);
        let mh = menu.height(self.cell_height) as usize;
        let mx = menu.x as usize;
        let my = menu.y as usize;

        let bg_pixel = MENU_BG.to_pixel();
        let border_pixel = SEPARATOR_COLOR.to_pixel();
        let hover_pixel = MENU_HOVER_BG.to_pixel();

        // Draw menu background and border.
        for py in my..((my + mh).min(buf_height)) {
            for px in mx..((mx + mw).min(buf_width)) {
                let idx = py * buf_width + px;
                if idx < buffer.len() {
                    let is_border = py == my || py == my + mh - 1 || px == mx || px == mx + mw - 1;
                    buffer[idx] = if is_border { border_pixel } else { bg_pixel };
                }
            }
        }

        // Draw menu items.
        for (i, (action, label)) in menu.items.iter().enumerate() {
            let item_y = my as u32 + 2 + i as u32 * ih;

            // Hover highlight for the active row.
            if menu.hover_index == Some(i) {
                for py in item_y as usize..(item_y + ih) as usize {
                    for px in (mx + 1)..(mx + mw - 1) {
                        if py < buf_height && px < buf_width {
                            let idx = py * buf_width + px;
                            if idx < buffer.len() {
                                buffer[idx] = hover_pixel;
                            }
                        }
                    }
                }
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

            let text_x = mx as u32 + self.cell_width;
            let text_y = item_y + 2;
            for (ci, ch) in label.chars().enumerate() {
                let cx = text_x + ci as u32 * self.cell_width;
                if cx as usize + self.cell_width as usize <= mx + mw {
                    self.draw_char_at(buffer, buf_width, buf_height, cx, text_y, ch, fg);
                }
            }
        }
    }
}
