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
    /// Draws context menu overlay using a shared layout.
    pub fn draw_context_menu(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        menu: &ContextMenu,
    ) {
        let layout = menu.layout(self.metrics.cell_width, self.metrics.cell_height, self.ui_scale());
        let clip_right = (layout.bg.x + layout.bg.w) as u32;

        self.draw_rounded_rect_cmd(buffer, buf_width, buf_height, &layout.bg);
        self.draw_rounded_rect_cmd(buffer, buf_width, buf_height, &layout.border);

        for item in &layout.items {
            if let Some(ref hover) = item.hover_rect {
                self.draw_rounded_rect_cmd(buffer, buf_width, buf_height, hover);
            }

            let fg = Color::from_pixel(item.text.color);
            let text_x = item.text.x as u32;
            let text_y = item.text.y as u32;
            for (ci, ch) in item.text.text.chars().enumerate() {
                let cx = text_x + ci as u32 * self.metrics.cell_width;
                if cx + self.metrics.cell_width <= clip_right {
                    self.draw_char(buffer, buf_width, buf_height, cx, text_y, ch, fg);
                }
            }
        }
    }
}
