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

        // Create initial tab in the first window.
        if let Some(win) = self.windows.get_mut(&win_id) {
            let size = win.window.inner_size();
            let (rows, cols) = win.calc_grid_size(size.width, size.height);
            win.new_tab(rows, cols, &mut self.next_tab_id, &self.tx);
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
                win.resize_direction = None;
                win.hovered_tab = None;
                win.security_popup = None;
                if !focused {
                    win.commit_rename();
                    win.context_menu = None;
                } else {
                    win.suppress_click_to_cursor_once = true;
                    win.window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                win.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                win.on_keyboard_input(event_loop, &event, &mut self.next_tab_id, &self.tx);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                win.on_mouse_wheel(delta);
            }
            WindowEvent::CursorLeft { .. } => {
                win.hovered_tab = None;
                win.resize_direction = None;
            }
            WindowEvent::CursorMoved { position, .. } => {
                win.on_cursor_moved(position);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                win.on_mouse_input(event_loop, state, button, &mut self.next_tab_id, &self.tx);
            }
            WindowEvent::Resized(size) => {
                win.on_resized(size);
            }
            WindowEvent::RedrawRequested => {
                win.on_redraw_requested();
            }
            _ => (),
        }

        // Process any pending window requests (detach, close).
        self.process_window_requests(event_loop, window_id);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(16),
        ));

        self.drain_pty_events(event_loop);

        for win in self.windows.values() {
            win.window.request_redraw();
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
                WindowRequest::DetachTab { tab } => {
                    self.create_window_with_tab(event_loop, tab);
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
            }
        }

        // If all windows are closed, exit the application.
        if self.windows.is_empty() {
            event_loop.exit();
        }
    }
}
