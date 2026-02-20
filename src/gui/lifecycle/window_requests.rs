use crate::gui::*;

impl App {
    /// Opens a new tab in the native macOS tab group of the source window.
    #[cfg(target_os = "macos")]
    fn open_tab_in_native_group(
        &mut self,
        event_loop: &ActiveEventLoop,
        source_window_id: WindowId,
        title: String,
        cwd: Option<String>,
    ) {
        let existing_win = self
            .windows
            .get(&source_window_id)
            .map(|w| w.window.clone());
        let group_pinned = existing_win
            .as_ref()
            .is_some_and(|w| platform::macos::is_window_pinned(w));
        if let Some(new_id) = self.create_window(event_loop, None)
            && let Some(new_win) = self.windows.get_mut(&new_id)
        {
            let size = new_win.window.inner_size();
            let (rows, cols) = new_win.calc_grid_size(size.width, size.height);
            new_win.new_tab_with_title(rows, cols, Some(title), &mut self.next_tab_id, &self.tx, cwd);
            if let Some(tab) = new_win.tabs.first() {
                new_win.window.set_title(&tab.title);
            }
            if let Some(existing) = existing_win {
                platform::macos::add_as_tab(&existing, &new_win.window);
                platform::macos::set_native_tab_group_pin_state(&existing, group_pinned);
                new_win.pinned = group_pinned;
            }
            new_win.window.request_redraw();
        }
    }

    /// Processes pending requests from a window (detach tab, close window).
    pub(super) fn process_window_requests(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
    ) {
        let Some(win) = self.windows.get_mut(&window_id) else {
            return;
        };
        let requests: Vec<WindowRequest> = win.pending_requests.drain(..).collect();

        for request in requests {
            match request {
                #[cfg(not(target_os = "macos"))]
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
                    // Extract all PTY sessions before dropping the window
                    // so that Session::drop() doesn't block the UI thread.
                    let sessions: Vec<crate::pty::Session> = if let Some(win) = self.windows.get_mut(&window_id) {
                        win.tabs
                            .iter_mut()
                            .flat_map(|tab| tab.pane_tree.drain_sessions())
                            .flatten()
                            .collect()
                    } else {
                        Vec::new()
                    };

                    // Spawn background thread for cleanup (kill + wait).
                    if !sessions.is_empty()
                        && let Err(e) = std::thread::Builder::new()
                            .name("pty-cleanup".into())
                            .spawn(move || {
                                for session in sessions {
                                    session.shutdown();
                                }
                            })
                    {
                        eprintln!("Failed to spawn PTY cleanup thread: {e}");
                    }

                    // Now drop the window — sessions are already extracted,
                    // so Drop won't block.
                    #[cfg(target_os = "macos")]
                    {
                        if let Some(win) = self.windows.remove(&window_id) {
                            platform::macos::remove_toolbar_item(&win.window);
                        }
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        self.windows.remove(&window_id);
                    }
                }
                WindowRequest::NewWindow { cwd } => {
                    let tab_title = format!("bash #{}", self.windows.len() + 1);
                    if let Some(new_id) = self.create_window(event_loop, None)
                        && let Some(new_win) = self.windows.get_mut(&new_id)
                    {
                        let size = new_win.window.inner_size();
                        let (rows, cols) = new_win.calc_grid_size(size.width, size.height);
                        new_win.new_tab_with_title(
                            rows,
                            cols,
                            Some(tab_title),
                            &mut self.next_tab_id,
                            &self.tx,
                            cwd,
                        );
                        #[cfg(target_os = "macos")]
                        if let Some(tab) = new_win.tabs.first() {
                            new_win.window.set_title(&tab.title);
                        }
                        new_win.window.request_redraw();
                    }
                }
                #[cfg(target_os = "macos")]
                WindowRequest::NewTab { cwd } => {
                    let tab_title = format!("bash #{}", self.windows.len() + 1);
                    self.open_tab_in_native_group(event_loop, window_id, tab_title, cwd);
                }
                #[cfg(target_os = "macos")]
                WindowRequest::ReopenTab { title } => {
                    self.open_tab_in_native_group(event_loop, window_id, title, None);
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            for win in self.windows.values_mut() {
                platform::macos::sync_native_tab_bar_visibility(&win.window);
                win.schedule_native_tab_bar_resync();
            }
        }

        // If all windows are closed, exit the application.
        if self.windows.is_empty() {
            event_loop.exit();
        }
    }
}
