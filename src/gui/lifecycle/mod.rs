use crate::gui::tabs::create::NewTabParams;
use crate::gui::*;

mod pty_events;
mod window_requests;

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Only create the initial window once.
        if !self.windows.is_empty() {
            return;
        }

        let context = match Context::new(event_loop.owned_display_handle()) {
            Ok(ctx) => ctx,
            Err(err) => {
                eprintln!("Failed to create rendering context: {err}");
                event_loop.exit();
                return;
            }
        };
        self.context = Some(context);

        let Some(win_id) = self.create_window(event_loop, None) else {
            event_loop.exit();
            return;
        };

        // Create initial tab in the first window.
        if let Some(win) = self.windows.get_mut(&win_id) {
            // Install newWindowForTab: on the window class (enables native "+" button).
            #[cfg(target_os = "macos")]
            platform::macos::install_new_tab_handler(&win.window);

            let size = win.window.inner_size();
            let (rows, cols) = win.calc_grid_size(size.width, size.height);
            win.new_tab(NewTabParams {
                rows,
                cols,
                title: None,
                next_tab_id: &mut self.next_tab_id,
                tx: &self.tx,
                cwd: None,
                config: &self.config,
            });
            #[cfg(target_os = "macos")]
            if let Some(tab) = win.tabs.first() {
                win.window.set_title(&tab.title);
            }
            win.window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(win) = self.windows.get_mut(&window_id) else {
            return;
        };
        let mut should_redraw = false;

        match event {
            WindowEvent::CloseRequested => {
                win.request_close_window();
                should_redraw = true;
            }
            WindowEvent::Focused(focused) => {
                win.modifiers = ModifiersState::empty();
                win.is_selecting = false;
                win.selection_anchor = None;
                win.keyboard_selection_anchor = None;
                win.selection_drag_mode = SelectionDragMode::Character;
                win.click_streak = 0;
                win.last_tab_click = None;
                win.last_topbar_empty_click = None;
                win.resize_direction = None;
                win.hovered_tab = None;
                win.security_popup = None;
                if !focused {
                    if win.dragging_tab.take().is_some() {
                        win.window.set_cursor(CursorIcon::Default);
                    }
                    win.divider_drag = None;
                    win.commit_rename();
                    #[cfg(not(target_os = "linux"))]
                    { win.pending_menu_context = None; }
                } else {
                    win.suppress_click_to_cursor_once = true;
                    #[cfg(target_os = "macos")]
                    {
                        win.pinned = platform::macos::is_window_pinned(&win.window);
                        platform::macos::set_pin_button_state(&win.window, win.pinned);
                    }
                }
                // DECSET 1004 focus reporting: send CSI I (focus) or CSI O (blur).
                if let Some(leaf) = win.active_leaf_mut()
                    && leaf.terminal.focus_reporting
                {
                    let seq = if focused { b"\x1b[I" } else { b"\x1b[O" };
                    leaf.write_pty(seq);
                }
                should_redraw = true;
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                win.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                win.on_keyboard_input(&event, &mut self.next_tab_id, &self.tx, &self.config);
                should_redraw = true;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                win.on_mouse_wheel(delta);
                should_redraw = true;
            }
            WindowEvent::CursorLeft { .. } => {
                win.hovered_tab = None;
                win.resize_direction = None;
                if win.dragging_tab.take().is_some() {
                    win.window.set_cursor(CursorIcon::Default);
                }
                should_redraw = true;
            }
            WindowEvent::CursorMoved { position, .. } => {
                win.on_cursor_moved(position);
                should_redraw = true;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                win.on_mouse_input(state, button, &mut self.next_tab_id, &self.tx, &self.config);
                should_redraw = true;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                win.on_scale_factor_changed(scale_factor);
                should_redraw = true;
            }
            WindowEvent::Resized(size) => {
                win.on_resized(size);
            }
            WindowEvent::RedrawRequested => {
                win.on_redraw_requested();
            }
            _ => (),
        }
        if should_redraw {
            win.window.request_redraw();
        }

        // Process any pending window requests (detach, close).
        self.process_window_requests(event_loop, window_id);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.drain_pty_events(event_loop);
        self.drain_menu_events();
        self.drain_update_events();

        // Handle native macOS "+" button clicks (newWindowForTab: action).
        #[cfg(target_os = "macos")]
        {
            let new_tab_requests = platform::macos::take_new_tab_requests();
            for _ in 0..new_tab_requests {
                let focused_id = self
                    .windows
                    .iter()
                    .find(|(_, w)| w.window.has_focus())
                    .map(|(id, _)| *id);
                if let Some(win_id) = focused_id {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        let cwd = win.active_leaf_ref().and_then(|l| l.cwd());
                        win.pending_requests.push(WindowRequest::NewTab { cwd });
                    }
                    self.process_window_requests(event_loop, win_id);
                }
            }
        }

        // Handle native macOS pin button clicks.
        #[cfg(target_os = "macos")]
        {
            let pin_requests = platform::macos::take_pin_button_requests();
            for _ in 0..pin_requests {
                let focused_id = self
                    .windows
                    .iter()
                    .find(|(_, w)| w.window.has_focus())
                    .map(|(id, _)| *id);
                if let Some(win_id) = focused_id
                    && let Some(win) = self.windows.get_mut(&win_id)
                {
                    win.toggle_pin();
                    win.window.request_redraw();
                }
            }

            let gear_requests = platform::macos::take_gear_button_requests();
            for _ in 0..gear_requests {
                let focused_id = self
                    .windows
                    .iter()
                    .find(|(_, w)| w.window.has_focus())
                    .map(|(id, _)| *id);
                if let Some(win_id) = focused_id
                    && let Some(win) = self.windows.get_mut(&win_id)
                {
                    win.toggle_settings_overlay(&self.config);
                    win.window.request_redraw();
                }
            }
        }

        // Poll atomic flags from native settings window ObjC callbacks.
        #[cfg(target_os = "macos")]
        {
            if platform::macos::settings_window::take_stepper_changed() {
                // Stepper/popup changed → update text fields to match, send config.
                platform::macos::settings_window::update_text_fields();
                platform::macos::settings_window::send_current_config();
            }
            if platform::macos::settings_window::take_text_field_changed() {
                // Text field edited → parse values into steppers, normalize, send config.
                platform::macos::settings_window::sync_text_fields_to_steppers();
                platform::macos::settings_window::update_text_fields();
                platform::macos::settings_window::send_current_config();
            }
            if platform::macos::settings_window::take_reset_requested() {
                platform::macos::settings_window::reset_controls_to_defaults();
                platform::macos::settings_window::send_current_config();
            }
            if platform::macos::settings_window::check_window_closed() {
                platform::macos::settings_window::send_current_config();
                platform::macos::settings_window::close_settings_window();
            }
        }

        // Windows/Linux: config changes are sent directly through the channel
        // from the settings window thread. We only need to detect window close
        // for bookkeeping (the window thread already saved config on close).
        #[cfg(target_os = "windows")]
        {
            platform::windows::settings_window::check_window_closed();
        }
        #[cfg(target_os = "linux")]
        {
            platform::linux::settings_window::check_window_closed();
        }

        // Apply config changes from native settings window.
        while let Ok(new_config) = self.settings_rx.try_recv() {
            crate::i18n::set_locale(new_config.language);
            for win in self.windows.values_mut() {
                win.apply_config_change(&new_config);
                win.window.request_redraw();
            }
            self.config = new_config;
        }

        let now = std::time::Instant::now();
        let mut next_wakeup: Option<std::time::Instant> = None;

        // Poll CWD via OS API for tabs without OSC 7 shell integration.
        {
            let cwd_poll_interval = std::time::Duration::from_secs(1);
            for win in self.windows.values_mut() {
                if now.duration_since(win.last_cwd_poll) >= cwd_poll_interval {
                    win.last_cwd_poll = now;
                    win.poll_cwd_for_tabs();
                }
            }
        }

        let update = self.available_release.as_ref();
        for win in self.windows.values_mut() {
            win.sync_window_title(update);
            if let Some((deadline, redraw_now)) = win.animation_schedule(now) {
                if redraw_now {
                    win.window.request_redraw();
                }
                next_wakeup = Some(next_wakeup.map_or(deadline, |current| current.min(deadline)));
            }
            // Ensure we wake up for next CWD poll
            let next_cwd = win.last_cwd_poll + std::time::Duration::from_secs(1);
            next_wakeup = Some(next_wakeup.map_or(next_cwd, |current| current.min(next_cwd)));
        }

        match next_wakeup {
            Some(deadline) => {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(deadline))
            }
            None => event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait),
        }
    }
}

impl App {
    #[cfg(not(target_os = "linux"))]
    fn drain_menu_events(&mut self) {
        while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
            for win in self.windows.values_mut() {
                if let Some(ctx) = win.pending_menu_context.take() {
                    let action_map = match &ctx {
                        MenuContext::Tab { action_map, .. } => action_map,
                        MenuContext::Terminal { action_map, .. } => action_map,
                    };
                    let tab_index = match &ctx {
                        MenuContext::Tab { tab_index, .. } => Some(*tab_index),
                        MenuContext::Terminal { .. } => None,
                    };
                    let pane_id = match &ctx {
                        MenuContext::Tab { .. } => None,
                        MenuContext::Terminal { pane_id, .. } => *pane_id,
                    };
                    if let Some((_, action)) = action_map.iter().find(|(id, _)| *id == event.id) {
                        win.handle_menu_action(
                            *action,
                            tab_index,
                            pane_id,
                            &mut self.next_tab_id,
                            &self.tx,
                            &self.config,
                        );
                    }
                    win.window.request_redraw();
                    break;
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn drain_menu_events(&mut self) {}

    fn drain_update_events(&mut self) {
        while let Ok(release) = self.update_rx.try_recv() {
            eprintln!(
                "Update available: {} ({})",
                release.tag_name, release.html_url
            );
            self.available_release = Some(release);
            for win in self.windows.values() {
                win.window.request_redraw();
            }
        }
    }
}
