use crate::gui::renderer::ContextAction;
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

        if let Some(item_idx) = self.renderer.hit_test_context_menu(&menu, mx, my) {
            let action = menu.items[item_idx].0;
            let tab_idx = menu.tab_index;
            match action {
                ContextAction::Close => self.close_tab(tab_idx),
                ContextAction::Rename => self.start_rename(tab_idx),
                ContextAction::Duplicate => self.duplicate_tab(tab_idx, next_tab_id, tx),
            }
            true
        } else {
            // Click outside closes menu but should continue with normal click handling.
            false
        }
    }
}
