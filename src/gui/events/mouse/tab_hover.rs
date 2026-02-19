use crate::gui::*;

impl FerrumWindow {
    /// Updates the hovered tab index based on the current mouse position.
    /// Delegates to the renderer backend for hit testing.
    pub(in crate::gui::events::mouse) fn update_hover(&mut self, mx: f64, my: f64) {
        let size = self.window.inner_size();
        self.hovered_tab = self
            .backend
            .hit_test_tab_hover(mx, my, self.tabs.len(), size.width);
    }

    /// Clears the hover state when the cursor leaves the tab bar area.
    #[allow(dead_code)]
    pub(in crate::gui::events::mouse) fn clear_hover(&mut self) {
        self.hovered_tab = None;
    }
}
