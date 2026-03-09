#[cfg(not(target_os = "macos"))]
use crate::config::AppConfig;
use crate::gui::pane::{DIVIDER_HIT_ZONE, DIVIDER_WIDTH};
use crate::gui::renderer::TabBarHit;
#[cfg(not(target_os = "linux"))]
use crate::gui::state::MenuContext;
use crate::gui::*;
use crate::update::AvailableRelease;

impl FerrumWindow {
    #[cfg(target_os = "macos")]
    pub(crate) fn on_mouse_input(
        &mut self,
        state: ElementState,
        button: winit::event::MouseButton,
        available_release: Option<&AvailableRelease>,
    ) {
        self.apply_pending_resize();

        match button {
            winit::event::MouseButton::Left => {
                self.on_left_mouse_input(state, available_release)
            }
            _ => self.on_non_left_mouse_input(state, button),
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) fn on_mouse_input(
        &mut self,
        state: ElementState,
        button: winit::event::MouseButton,
        available_release: Option<&AvailableRelease>,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
        config: &AppConfig,
    ) {
        self.apply_pending_resize();

        match button {
            winit::event::MouseButton::Left => {
                self.on_left_mouse_input(state, available_release, next_tab_id, tx, config)
            }
            _ => self.on_non_left_mouse_input(state, button),
        }
    }

    /// Handles Middle and Right mouse button events, which behave identically
    /// on all platforms.
    fn on_non_left_mouse_input(&mut self, state: ElementState, button: winit::event::MouseButton) {
        match button {
            winit::event::MouseButton::Middle => self.on_middle_mouse_input(state),
            winit::event::MouseButton::Right => self.on_right_mouse_input(state),
            _ => {}
        }
    }

    /// Closes the tab identified by `hit` if the hit is a tab.
    /// On non-macOS, also matches `TabBarHit::CloseTab`.
    fn close_tab_if_tab_hit(&mut self, hit: TabBarHit) {
        match hit {
            TabBarHit::Tab(idx) => self.close_tab(idx),
            #[cfg(not(target_os = "macos"))]
            TabBarHit::CloseTab(idx) => self.close_tab(idx),
            _ => {}
        }
    }

    fn on_middle_mouse_input(&mut self, state: ElementState) {
        if state != ElementState::Pressed {
            return;
        }
        self.commit_rename();
        let (mx, my) = self.mouse_pos;
        if my >= self.backend.tab_bar_height_px() as f64 {
            return;
        }
        let hit = self.tab_bar_hit(mx, my);
        self.close_tab_if_tab_hit(hit);
    }

    /// Shows the tab context menu for the given hit if it targets a tab.
    /// On non-macOS, also matches `TabBarHit::CloseTab`.
    #[cfg(not(target_os = "linux"))]
    fn show_context_menu_for_hit(&mut self, hit: TabBarHit) {
        let idx = match hit {
            TabBarHit::Tab(idx) => idx,
            #[cfg(not(target_os = "macos"))]
            TabBarHit::CloseTab(idx) => idx,
            _ => return,
        };
        let (menu, action_map) = menus::build_tab_context_menu();
        self.pending_menu_context = Some(MenuContext::Tab {
            tab_index: idx,
            action_map,
        });
        menus::show_context_menu(&self.window, &menu, None);
    }

    /// Sends a right-button mouse event to the terminal if mouse reporting is active.
    fn send_right_mouse_event(&mut self, pressed: bool) {
        if self.is_mouse_reporting() {
            let (row, col) = self.pixel_to_grid(self.mouse_pos.0, self.mouse_pos.1);
            self.send_mouse_event(2, col, row, pressed);
        }
    }

    fn on_right_mouse_input(&mut self, state: ElementState) {
        match state {
            ElementState::Pressed => {
                self.commit_rename();

                #[cfg(not(target_os = "linux"))]
                {
                    let (mx, my) = self.mouse_pos;
                    let tab_bar_height = self.backend.tab_bar_height_px();

                    if my < tab_bar_height as f64 {
                        // Right-click on a tab: show native tab context menu.
                        let hit = self.tab_bar_hit(mx, my);
                        self.show_context_menu_for_hit(hit);
                        return;
                    }

                    // In mouse-reporting mode, forward right-click to the terminal.
                    if self.is_mouse_reporting() {
                        self.send_right_mouse_event(true);
                        return;
                    }

                    // Right-click on terminal area: show native terminal context menu.
                    let terminal_rect = self.terminal_content_rect();
                    let clicked_pane = self.active_tab_ref().and_then(|tab| {
                        tab.pane_tree
                            .pane_at_pixel(mx as u32, my as u32, terminal_rect, DIVIDER_WIDTH)
                    });
                    self.focus_pane_at_pixel(mx, my);

                    let has_selection = self
                        .active_leaf_ref()
                        .and_then(|leaf| leaf.selection)
                        .is_some();
                    let has_multiple_panes = self
                        .active_tab_ref()
                        .is_some_and(|t| t.has_multiple_panes());
                    let (menu, action_map) =
                        menus::build_terminal_context_menu(has_selection, has_multiple_panes);
                    self.pending_menu_context = Some(MenuContext::Terminal {
                        pane_id: clicked_pane,
                        action_map,
                    });
                    menus::show_context_menu(&self.window, &menu, None);
                }

                #[cfg(target_os = "linux")]
                self.send_right_mouse_event(true);
            }
            ElementState::Released => self.send_right_mouse_event(false),
        }
    }

    /// Focuses the pane at the given pixel position if it differs from the currently focused pane.
    fn focus_pane_at_pixel(&mut self, mx: f64, my: f64) {
        let terminal_rect = self.terminal_content_rect();
        let clicked_pane = self.active_tab_ref().and_then(|tab| {
            tab.pane_tree
                .pane_at_pixel(mx as u32, my as u32, terminal_rect, DIVIDER_WIDTH)
        });
        if let Some(pane_id) = clicked_pane
            && let Some(tab) = self.active_tab_mut()
            && pane_id != tab.focused_pane
        {
            tab.focused_pane = pane_id;
        }
    }

    /// Handles a left-press on the terminal area: starts divider drag if a divider was hit,
    /// otherwise updates pane focus to the clicked pane.
    ///
    /// Returns `true` if a divider was hit and the click should not be forwarded to the terminal.
    fn handle_left_press_common(&mut self, mx: f64, my: f64) -> bool {
        let terminal_rect = self.terminal_content_rect();

        if let Some(tab) = self.active_tab_ref()
            && let Some(hit) = tab.pane_tree.hit_test_divider(
                mx as u32,
                my as u32,
                terminal_rect,
                DIVIDER_WIDTH,
                DIVIDER_HIT_ZONE,
            )
        {
            self.divider_drag = Some(DividerDragState {
                initial_mouse_pos: (mx as u32, my as u32),
                direction: hit.direction,
            });
            return true; // Don't forward click to terminal
        }

        self.focus_pane_at_pixel(mx, my);
        false
    }

    /// Handles the common tail of a left-click that has already bypassed the tab bar:
    /// commits rename on press, scrollbar interaction, divider drag start, and terminal click.
    fn handle_left_click_below_tab_bar(&mut self, state: ElementState, mx: f64, my: f64) {
        // Clicking on terminal area commits any active rename (blur behavior).
        if state == ElementState::Pressed {
            #[cfg(not(target_os = "macos"))]
            {
                self.dragging_tab = None; // Cancel potential drag if clicking terminal area.
            }
            self.commit_rename();
        }

        if self.handle_scrollbar_left_click(state, mx, my) {
            return;
        }

        // Check if clicking on a pane divider (start drag resize).
        if state == ElementState::Pressed && self.handle_left_press_common(mx, my) {
            return;
        }

        self.handle_terminal_left_click(state, mx, my);
    }

    /// Ends divider drag on mouse release: resizes panes, sends SIGWINCH, requests redraw.
    /// Returns `true` if the event was a divider-drag release and was fully handled.
    fn handle_divider_drag_release(&mut self, state: ElementState) -> bool {
        if state == ElementState::Released && self.divider_drag.take().is_some() {
            self.resize_all_panes(false);
            self.send_sigwinch_to_all_panes();
            self.window.request_redraw();
            return true;
        }
        false
    }

    #[cfg(target_os = "macos")]
    fn on_left_mouse_input(
        &mut self,
        state: ElementState,
        available_release: Option<&AvailableRelease>,
    ) {
        let (mx, my) = self.mouse_pos;
        let tab_bar_height = self.backend.tab_bar_height_px() as f64;

        // Handle update banner clicks before anything else.
        if self.handle_update_banner_click(state, mx, my, available_release) {
            return;
        }

        if self.handle_divider_drag_release(state) {
            return;
        }

        if my < tab_bar_height {
            self.handle_tab_bar_left_click(state, mx, my);
            return;
        }

        self.handle_left_click_below_tab_bar(state, mx, my);
    }

    #[cfg(not(target_os = "macos"))]
    fn on_left_mouse_input(
        &mut self,
        state: ElementState,
        available_release: Option<&AvailableRelease>,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
        config: &AppConfig,
    ) {
        let (mx, my) = self.mouse_pos;
        let tab_bar_height = self.backend.tab_bar_height_px() as f64;

        // Handle update banner clicks before anything else.
        if self.handle_update_banner_click(state, mx, my, available_release) {
            return;
        }

        // On non-macOS, initiate OS-level resize drag when pressing on window edges.
        if state == ElementState::Pressed
            && let Some(dir) = self.resize_direction
        {
            if let Err(e) = self.window.drag_resize_window(dir) {
                eprintln!("[ferrum] drag_resize_window failed: {e}");
            }
            return;
        }

        // If releasing mouse during an active tab drag, handle drop regardless of position.
        // (Custom tab bar drag -- not used on macOS.)
        if state == ElementState::Released {
            if self.dragging_tab.as_ref().is_some_and(|d| d.is_active) {
                self.handle_tab_bar_left_click(state, mx, my, next_tab_id, tx, config);
                return;
            }
            // Cancel non-active drag on release outside tab bar.
            if self.dragging_tab.is_some() {
                self.dragging_tab = None;
            }
        }

        if self.handle_divider_drag_release(state) {
            return;
        }

        if my < tab_bar_height {
            self.handle_tab_bar_left_click(state, mx, my, next_tab_id, tx, config);
            return;
        }

        self.handle_left_click_below_tab_bar(state, mx, my);
    }

    /// Handles left mouse down/up on the scrollbar zone.
    /// Returns `true` if the event was consumed (click was in scrollbar zone).
    fn handle_scrollbar_left_click(&mut self, state: ElementState, mx: f64, my: f64) -> bool {
        // On release: end scrollbar drag if active.
        if state == ElementState::Released {
            if self.active_leaf_ref().is_some_and(|l| l.scrollbar.dragging) {
                if let Some(leaf) = self.active_leaf_mut() {
                    leaf.scrollbar.dragging = false;
                    leaf.scrollbar.last_activity = std::time::Instant::now();
                }
                return true;
            }
            return false;
        }

        // Pressed: check if click is in scrollbar zone.
        let size = self.window.inner_size();
        if !self.is_in_scrollbar_zone(mx, size.width) {
            return false;
        }

        let leaf = match self.active_leaf_ref() {
            Some(l) => l,
            None => return false,
        };
        let scrollback_len = leaf.terminal.screen.scrollback_len();
        if scrollback_len == 0 {
            return false;
        }

        let buf_height = size.height as usize;
        let grid_rows = leaf.terminal.screen.viewport_rows();
        let scroll_offset = leaf.scroll_offset;
        let tab_bar_height = self.backend.tab_bar_height_px() as f64;
        let window_padding = self.backend.window_padding_px() as f64;

        let track_top = tab_bar_height + window_padding;
        let track_bottom = buf_height as f64 - window_padding;

        // Ignore clicks outside the track area.
        if my < track_top || my > track_bottom {
            return false;
        }

        let track_height = track_bottom - track_top;

        // Guard against division by zero with extremely small windows.
        if track_height <= 0.0 {
            return false;
        }

        // Check if click is on the thumb or on the track.
        if let Some((thumb_y, thumb_height)) = self.backend.scrollbar_thumb_bounds(
            buf_height,
            scroll_offset,
            scrollback_len,
            grid_rows,
        ) {
            let on_thumb = my >= thumb_y as f64 && my <= (thumb_y + thumb_height) as f64;

            if on_thumb {
                // Start thumb drag.
                if let Some(leaf) = self.active_leaf_mut() {
                    leaf.scrollbar.dragging = true;
                    leaf.scrollbar.drag_start_y = my;
                    leaf.scrollbar.drag_start_offset = leaf.scroll_offset;
                    leaf.scrollbar.last_activity = std::time::Instant::now();
                }
            } else {
                // Click on track: jump to proportional position.
                let click_ratio = (my - track_top) / track_height;
                let max_offset = scrollback_len;
                let new_offset =
                    (max_offset as f64 - click_ratio * max_offset as f64).round().max(0.0) as usize;
                self.apply_scroll_offset(new_offset);
            }
        }

        self.window.request_redraw();
        true
    }
}
