use crate::gui::renderer::{TabBarHit, TabInfo};
use crate::gui::*;

impl FerrumWindow {
    fn tab_infos_for_hit_test(&self) -> Vec<TabInfo<'_>> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(idx, tab)| TabInfo {
                title: &tab.title,
                is_active: idx == self.active_tab,
                security_count: if tab.security.has_events() {
                    tab.security.active_event_count()
                } else {
                    0
                },
                hover_progress: 0.0,
                close_hover_progress: 0.0,
                is_renaming: false,
                rename_text: None,
                rename_cursor: 0,
                rename_selection: None,
            })
            .collect()
    }

    pub(in crate::gui::events::mouse) fn tab_bar_hit(&self, mx: f64, my: f64) -> TabBarHit {
        let buf_width = self.window.inner_size().width;
        self.backend
            .hit_test_tab_bar(mx, my, self.tabs.len(), buf_width)
    }

    pub(in crate::gui::events::mouse) fn tab_bar_security_hit(
        &self,
        mx: f64,
        my: f64,
    ) -> Option<usize> {
        let tab_infos = self.tab_infos_for_hit_test();
        if tab_infos.is_empty() {
            return None;
        }
        let buf_width = self.window.inner_size().width;
        self.backend
            .hit_test_tab_security_badge(mx, my, &tab_infos, buf_width)
    }

    pub(in crate::gui::events::mouse) fn handle_tab_bar_left_click(
        &mut self,
        _event_loop: &ActiveEventLoop,
        state: ElementState,
        mx: f64,
        my: f64,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) {
        if state != ElementState::Pressed {
            // Mouse release in tab bar area.
            // End any active pointer selection state so terminal drag-selection doesn't stick.
            if self.is_selecting && self.renaming_tab.is_none() && self.is_mouse_reporting() {
                // If PTY owns mouse reporting, still emit release even when cursor is over tab bar.
                let (row, col) = self.pixel_to_grid(mx, my);
                self.send_mouse_event(0, col, row, false);
            }
            self.is_selecting = false;
            self.selection_anchor = None;
            self.finish_drag();
            return;
        }

        if let Some(tab_idx) = self.tab_bar_security_hit(mx, my) {
            self.cancel_drag();
            self.commit_rename();
            self.last_topbar_empty_click = None;
            self.open_security_popup_for_tab(tab_idx);
            return;
        }

        let hit = self.tab_bar_hit(mx, my);

        // Check if the click landed inside the rename text field area.
        // If so, handle cursor positioning instead of normal tab bar interaction.
        if self.renaming_tab.is_some()
            && let TabBarHit::Tab(idx) = hit
            && self
                .renaming_tab
                .as_ref()
                .is_some_and(|r| r.tab_index == idx)
        {
            self.handle_rename_field_click(mx);
            return;
        }

        // Commit any active rename before processing the click (blur behavior).
        let had_rename = self.renaming_tab.is_some();
        self.commit_rename();

        match hit {
            TabBarHit::Tab(idx) => {
                self.handle_tab_click(idx, mx, my, had_rename);
            }
            TabBarHit::CloseTab(idx) => {
                self.last_topbar_empty_click = None;
                self.close_tab(idx);
            }
            TabBarHit::NewTab => {
                self.last_topbar_empty_click = None;
                let size = self.window.inner_size();
                let (rows, cols) = self.calc_grid_size(size.width, size.height);
                self.new_tab(rows, cols, next_tab_id, tx);
            }
            #[cfg(not(target_os = "macos"))]
            TabBarHit::WindowButton(btn) => {
                self.handle_window_button_click(btn);
            }
            TabBarHit::Empty => {
                self.handle_empty_bar_click();
            }
            #[cfg(not(target_os = "macos"))]
            TabBarHit::PinButton => {
                self.last_topbar_empty_click = None;
                self.last_tab_click = None;
                self.toggle_pin();
            }
        }
    }
}
