use crate::gui::renderer::{ContextAction, ContextMenuTarget};
use crate::gui::*;

impl FerrumWindow {
    pub(in crate::gui::events::mouse) fn handle_context_menu_left_click(
        &mut self,
        _event_loop: &ActiveEventLoop,
        state: ElementState,
        mx: f64,
        my: f64,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) -> bool {
        if state != ElementState::Pressed {
            return false;
        }
        let Some(menu) = self.context_menu.take() else {
            return false;
        };

        if let Some(item_idx) = self.backend.hit_test_context_menu(&menu, mx, my) {
            let action = menu.items[item_idx].0;
            match action {
                ContextAction::CloseTab => {
                    if let ContextMenuTarget::Tab { tab_index } = menu.target {
                        self.close_tab(tab_index);
                    }
                }
                ContextAction::RenameTab => {
                    if let ContextMenuTarget::Tab { tab_index } = menu.target {
                        self.start_rename(tab_index);
                    }
                }
                ContextAction::DuplicateTab => {
                    if let ContextMenuTarget::Tab { tab_index } = menu.target {
                        self.duplicate_tab(tab_index, next_tab_id, tx);
                    }
                }
                ContextAction::CopySelection => self.copy_selection(),
                ContextAction::Paste => self.paste_clipboard(),
                ContextAction::ClearSelection => {
                    if let Some(tab) = self.active_tab_mut() {
                        tab.selection = None;
                    }
                    self.selection_anchor = None;
                    self.keyboard_selection_anchor = None;
                }
            }
            true
        } else {
            // Click outside closes menu but should continue with normal click handling.
            false
        }
    }
}
