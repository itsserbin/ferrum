use crate::gui::*;

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

        // Install native macOS "+" button handler before creating first tab.
        #[cfg(target_os = "macos")]
        platform::macos::install_new_tab_responder();

        // Create initial tab in the first window.
        if let Some(win) = self.windows.get_mut(&win_id) {
            let size = win.window.inner_size();
            let (rows, cols) = win.calc_grid_size(size.width, size.height);
            win.new_tab(rows, cols, &mut self.next_tab_id, &self.tx);
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
                win.pending_requests.push(WindowRequest::CloseWindow);
            }
            WindowEvent::Focused(focused) => {
                win.modifiers = ModifiersState::empty();
                win.is_selecting = false;
                win.selection_anchor = None;
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
                    win.commit_rename();
                    win.context_menu = None;
                } else {
                    win.suppress_click_to_cursor_once = true;
                }
                should_redraw = true;
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                win.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                win.on_keyboard_input(event_loop, &event, &mut self.next_tab_id, &self.tx);
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
                win.on_mouse_input(event_loop, state, button, &mut self.next_tab_id, &self.tx);
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

        // Handle native macOS "+" button clicks (newWindowForTab: action).
        #[cfg(target_os = "macos")]
        if platform::macos::take_new_tab_request() {
            let focused_id = self
                .windows
                .iter()
                .find(|(_, w)| w.window.has_focus())
                .map(|(id, _)| *id);
            if let Some(win_id) = focused_id {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.pending_requests.push(WindowRequest::NewTab);
                }
                self.process_window_requests(event_loop, win_id);
            }
        }

        let now = std::time::Instant::now();
        let mut next_wakeup: Option<std::time::Instant> = None;

        for win in self.windows.values() {
            if let Some((deadline, redraw_now)) = win.animation_schedule(now) {
                if redraw_now {
                    win.window.request_redraw();
                }
                next_wakeup = Some(next_wakeup.map_or(deadline, |current| current.min(deadline)));
            }
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
    /// Drains all pending PTY events from the channel, routing each to the correct window.
    fn drain_pty_events(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(event) = self.rx.try_recv() {
            let tab_id = match &event {
                PtyEvent::Data { tab_id, .. } => *tab_id,
                PtyEvent::Exited { tab_id } => *tab_id,
            };

            // Find which window owns this tab.
            let win_id = self
                .windows
                .iter()
                .find(|(_, win)| win.tabs.iter().any(|t| t.id == tab_id))
                .map(|(id, _)| *id);

            if let Some(win_id) = win_id {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.on_pty_event(&event);
                    win.window.request_redraw();
                }
                self.process_window_requests(event_loop, win_id);
            }
        }
    }

    /// Processes pending requests from a window (detach tab, close window).
    fn process_window_requests(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId) {
        let Some(win) = self.windows.get_mut(&window_id) else {
            return;
        };
        let requests: Vec<WindowRequest> = win.pending_requests.drain(..).collect();

        for request in requests {
            match request {
                WindowRequest::DetachTab { tab, cursor_pos } => {
                    self.create_window_with_tab(event_loop, tab, cursor_pos);
                    // If source window is now empty, close it.
                    if self
                        .windows
                        .get(&window_id)
                        .is_some_and(|w| w.tabs.is_empty())
                    {
                        self.windows.remove(&window_id);
                    }
                }
                WindowRequest::CloseWindow => {
                    self.windows.remove(&window_id);
                }
                #[cfg(target_os = "macos")]
                WindowRequest::NewWindow => {
                    if let Some(new_id) = self.create_window(event_loop, None) {
                        if let Some(new_win) = self.windows.get_mut(&new_id) {
                            let size = new_win.window.inner_size();
                            let (rows, cols) = new_win.calc_grid_size(size.width, size.height);
                            new_win.new_tab(rows, cols, &mut self.next_tab_id, &self.tx);
                            if let Some(tab) = new_win.tabs.first() {
                                new_win.window.set_title(&tab.title);
                            }
                            new_win.window.request_redraw();
                        }
                    }
                }
                #[cfg(target_os = "macos")]
                WindowRequest::NewTab => {
                    let existing_win = self.windows.get(&window_id).map(|w| w.window.clone());
                    if let Some(new_id) = self.create_window(event_loop, None) {
                        if let Some(new_win) = self.windows.get_mut(&new_id) {
                            let size = new_win.window.inner_size();
                            let (rows, cols) = new_win.calc_grid_size(size.width, size.height);
                            new_win.new_tab(rows, cols, &mut self.next_tab_id, &self.tx);
                            if let Some(tab) = new_win.tabs.first() {
                                new_win.window.set_title(&tab.title);
                            }
                            if let Some(existing) = existing_win {
                                platform::macos::add_as_tab(&existing, &new_win.window);
                            }
                            new_win.window.request_redraw();
                        }
                    }
                }
                #[cfg(target_os = "macos")]
                WindowRequest::ReopenTab { title } => {
                    let existing_win = self.windows.get(&window_id).map(|w| w.window.clone());
                    if let Some(new_id) = self.create_window(event_loop, None) {
                        if let Some(new_win) = self.windows.get_mut(&new_id) {
                            let size = new_win.window.inner_size();
                            let (rows, cols) = new_win.calc_grid_size(size.width, size.height);
                            new_win.new_tab_with_title(
                                rows,
                                cols,
                                Some(title),
                                &mut self.next_tab_id,
                                &self.tx,
                            );
                            if let Some(tab) = new_win.tabs.first() {
                                new_win.window.set_title(&tab.title);
                            }
                            if let Some(existing) = existing_win {
                                platform::macos::add_as_tab(&existing, &new_win.window);
                            }
                            new_win.window.request_redraw();
                        }
                    }
                }
            }
        }

        // If all windows are closed, exit the application.
        if self.windows.is_empty() {
            event_loop.exit();
        }
    }
}
