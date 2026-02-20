use anyhow::Context;

use crate::config::AppConfig;
use crate::gui::pane::{
    DIVIDER_WIDTH, NavigateDirection, PaneLeaf, PaneNode, PaneRect, SplitDirection,
};
use crate::gui::tabs::normalized_active_index_after_remove;
use crate::gui::*;

impl FerrumWindow {
    fn has_running_processes_in_window(&self) -> bool {
        self.tabs.iter().any(|tab| {
            tab.pane_tree.leaf_ids().into_iter().any(|leaf_id| {
                tab.pane_tree
                    .find_leaf(leaf_id)
                    .and_then(|leaf| leaf.session.as_ref())
                    .and_then(|session| session.process_id())
                    .is_some_and(pty::has_active_child_processes)
            })
        })
    }

    pub(in crate::gui) fn request_close_window(&mut self) {
        let should_confirm = self.has_running_processes_in_window();
        if !should_confirm || platform::confirm_window_close(&self.window) {
            self.pending_requests.push(WindowRequest::CloseWindow);
        }
    }

    pub(in crate::gui) fn refresh_tab_bar_visibility(&mut self) {
        #[cfg(not(target_os = "macos"))]
        {
            // Linux/Windows policy: keep custom tab bar always visible.
            let prev_height = self.backend.tab_bar_height_px();
            self.backend.set_tab_bar_visible(true);
            let next_height = self.backend.tab_bar_height_px();

            if prev_height != next_height && !self.tabs.is_empty() {
                let size = self.window.inner_size();
                let (rows, cols) = self.calc_grid_size(size.width, size.height);
                self.resize_all_tabs(rows, cols);
            }
        }
    }

    /// Closes a tab by index.
    pub(in crate::gui) fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        if self.tabs.len() == 1 {
            self.request_close_window();
            return;
        }

        // Keep title for reopen (Ctrl+Shift+T).
        let title = self.tabs[index].title.clone();
        self.closed_tabs.push(ClosedTabInfo { title });

        self.adjust_rename_after_tab_remove(index);
        self.adjust_security_popup_after_tab_remove(index);
        self.tabs.remove(index);
        self.refresh_tab_bar_visibility();

        let len_before = self.tabs.len() + 1;
        self.active_tab =
            normalized_active_index_after_remove(self.active_tab, len_before, index).unwrap_or(0);
    }

    /// Duplicates a tab by creating a new session with copied title.
    pub(in crate::gui) fn duplicate_tab(
        &mut self,
        index: usize,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
        config: &AppConfig,
    ) {
        if index >= self.tabs.len() {
            return;
        }
        let title = format!("{} (copy)", self.tabs[index].title);
        let cwd = self.tabs[index]
            .focused_leaf()
            .and_then(|l| l.cwd());
        let size = self.window.inner_size();
        let (rows, cols) = self.calc_grid_size(size.width, size.height);
        self.new_tab_with_title(rows, cols, Some(title), next_tab_id, tx, cwd, config);
    }

    /// Switches active tab.
    pub(in crate::gui) fn switch_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
            self.security_popup = None;
            // Recalculate pane layout for the newly active tab so each pane
            // gets correct dimensions (they may have been stale since the last
            // window resize happened while a different tab was active).
            self.resize_all_panes();
        }
    }

    /// Resizes all tab terminals and their PTY sessions.
    pub(in crate::gui) fn resize_all_tabs(&mut self, rows: usize, cols: usize) {
        for tab in &mut self.tabs {
            // Iterate over all leaves in the pane tree.
            let leaf_ids = tab.pane_tree.leaf_ids();
            for leaf_id in leaf_ids {
                if let Some(leaf) = tab.pane_tree.find_leaf_mut(leaf_id) {
                    if leaf.terminal.grid.rows == rows && leaf.terminal.grid.cols == cols {
                        continue;
                    }

                    leaf.terminal.resize(rows, cols);

                    // Clamp scroll_offset to valid range after resize (scrollback may have changed)
                    leaf.scroll_offset = leaf.scroll_offset.min(leaf.terminal.scrollback.len());

                    if let Some(ref session) = leaf.session
                        && let Err(err) = session.resize(rows as u16, cols as u16)
                    {
                        eprintln!(
                            "Failed to resize PTY for tab {}, pane {}: {err}",
                            tab.id, leaf_id
                        );
                    }
                }
            }
        }
    }

    /// Starts inline rename for the selected tab.
    pub(in crate::gui) fn start_rename(&mut self, tab_index: usize) {
        if tab_index < self.tabs.len() {
            let text = self.tabs[tab_index].title.clone();
            let cursor = text.len();
            self.renaming_tab = Some(RenameState {
                tab_index,
                text: text.clone(),
                original_title: text,
                cursor,
                selection_anchor: Some(0),
            });
        }
    }

    /// Commits the current rename: trims text, applies if non-empty, drops rename state.
    pub(in crate::gui) fn commit_rename(&mut self) {
        if let Some(rename) = self.renaming_tab.take() {
            let trimmed = rename.text.trim().to_string();
            if !trimmed.is_empty()
                && let Some(tab) = self.tabs.get_mut(rename.tab_index)
            {
                tab.title = trimmed;
                tab.is_renamed = true;
                #[cfg(target_os = "macos")]
                self.window.set_title(&tab.title);
            }
            // If trimmed is empty, the old title stays (we just dropped the rename state).
        }
    }

    /// Cancels the current rename, reverting to the original title.
    pub(in crate::gui) fn cancel_rename(&mut self) {
        if let Some(rename) = self.renaming_tab.take()
            && let Some(tab) = self.tabs.get_mut(rename.tab_index)
        {
            tab.title = rename.original_title;
        }
    }

    pub(in crate::gui) fn adjust_rename_after_tab_remove(&mut self, removed_index: usize) {
        let Some(rename) = self.renaming_tab.as_mut() else {
            return;
        };

        if rename.tab_index == removed_index {
            self.renaming_tab = None;
        } else if rename.tab_index > removed_index {
            rename.tab_index -= 1;
        }
    }

    pub(in crate::gui) fn adjust_security_popup_after_tab_remove(&mut self, removed_index: usize) {
        let Some(popup) = self.security_popup.as_mut() else {
            return;
        };

        if popup.tab_index == removed_index {
            self.security_popup = None;
        } else if popup.tab_index > removed_index {
            popup.tab_index -= 1;
        }
    }

    /// Splits the focused pane in the active tab, creating a new terminal pane.
    ///
    /// `reverse`: when true the new pane is placed *before* the original
    /// (used for SplitLeft / SplitUp).
    pub(in crate::gui) fn split_pane(
        &mut self,
        direction: SplitDirection,
        reverse: bool,
        _next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
        config: &AppConfig,
    ) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };

        let focused_pane = tab.focused_pane;
        let pane_id = tab.next_pane_id;
        tab.next_pane_id += 1;
        let tab_id = tab.id;

        // Calculate rows/cols for the new pane (roughly half the focused pane's terminal).
        let (rows, cols) = {
            let Some(leaf) = tab.pane_tree.find_leaf(focused_pane) else {
                return;
            };
            let term_rows = leaf.terminal.grid.rows;
            let term_cols = leaf.terminal.grid.cols;
            match direction {
                SplitDirection::Horizontal => (term_rows, (term_cols / 2).max(1)),
                SplitDirection::Vertical => ((term_rows / 2).max(1), term_cols),
            }
        };

        // Inherit CWD from the focused pane (if available via OSC 7).
        let cwd = tab
            .pane_tree
            .find_leaf(focused_pane)
            .and_then(|leaf| leaf.cwd());

        // Spawn a new PTY session.
        let shell = pty::default_shell();
        let session = match pty::Session::spawn(&shell, rows as u16, cols as u16, cwd.as_deref())
            .context("failed to spawn PTY session for new pane")
        {
            Ok(s) => s,
            Err(err) => {
                eprintln!("Failed to split pane: {err}");
                return;
            }
        };
        let pty_writer = match session
            .writer()
            .context("failed to acquire PTY writer for new pane")
        {
            Ok(w) => w,
            Err(err) => {
                eprintln!("Failed to split pane: {err}");
                return;
            }
        };

        // Spawn PTY reader thread (same pattern as tabs/create.rs).
        {
            let tx = tx.clone();
            let mut reader = match session
                .reader()
                .context("failed to clone PTY reader for new pane")
            {
                Ok(r) => r,
                Err(err) => {
                    eprintln!("Failed to split pane: {err}");
                    return;
                }
            };
            let reader_pane_id = pane_id;
            std::thread::Builder::new()
                .name(format!("pty-reader-{}-{}", tab_id, reader_pane_id))
                .spawn(move || {
                    use std::io::Read;
                    let mut buf = [0u8; 4096];
                    loop {
                        match reader.read(&mut buf) {
                            Ok(0) => {
                                let _ = tx.send(PtyEvent::Exited {
                                    tab_id,
                                    pane_id: reader_pane_id,
                                });
                                break;
                            }
                            Err(err) => {
                                eprintln!(
                                    "PTY read error for tab {tab_id} pane {reader_pane_id}: {err}"
                                );
                                let _ = tx.send(PtyEvent::Exited {
                                    tab_id,
                                    pane_id: reader_pane_id,
                                });
                                break;
                            }
                            Ok(n) => {
                                if tx
                                    .send(PtyEvent::Data {
                                        tab_id,
                                        pane_id: reader_pane_id,
                                        bytes: buf[..n].to_vec(),
                                    })
                                    .is_err()
                                {
                                    eprintln!(
                                        "PTY reader {}-{}: channel disconnected",
                                        tab_id, reader_pane_id
                                    );
                                    break;
                                }
                            }
                        }
                    }
                })
                .ok();
        }

        let palette = config.theme.resolve();
        let terminal = Terminal::with_config(
            rows,
            cols,
            config.terminal.max_scrollback,
            palette.default_fg,
            palette.default_bg,
        );

        let new_leaf = PaneNode::Leaf(Box::new(PaneLeaf {
            id: pane_id,
            terminal,
            session: Some(session),
            pty_writer,
            selection: None,
            scroll_offset: 0,
            security: SecurityGuard::new(),
            scrollbar: ScrollbarState::new(),
        }));

        // Re-borrow tab after the reader thread was spawned.
        let tab = &mut self.tabs[self.active_tab];
        tab.pane_tree
            .split_with_node(focused_pane, direction, new_leaf, reverse);
        tab.focused_pane = pane_id;

        self.resize_all_panes();
    }

    /// Closes the focused pane in the active tab.
    /// If the tab has only one pane, this is a no-op (can't close the last pane).
    pub(in crate::gui) fn close_focused_pane(&mut self) {
        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        if tab.pane_tree.is_leaf() {
            return; // Can't close the only pane
        }
        let closing_id = tab.focused_pane;
        tab.pane_tree.close(closing_id);
        // Keep focus in reverse pane-creation order.
        if let Some(next_focus) = tab.focus_after_closing_pane(closing_id) {
            tab.focused_pane = next_focus;
        }
        self.resize_all_panes();
    }

    /// Navigates focus to the nearest pane in the given direction from the currently
    /// focused pane, using spatial proximity of pane centers.
    pub(in crate::gui) fn navigate_pane(&mut self, direction: NavigateDirection) {
        let size = self.window.inner_size();
        let tab_bar_h = self.backend.tab_bar_height_px();
        let padding = self.backend.window_padding_px();
        let terminal_rect = PaneRect {
            x: padding,
            y: tab_bar_h + padding,
            width: size.width.saturating_sub(padding * 2),
            height: size.height.saturating_sub(tab_bar_h + padding * 2),
        };
        let divider_px = DIVIDER_WIDTH;
        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        let layout = tab.pane_tree.layout(terminal_rect, divider_px);
        if let Some(target) = PaneNode::navigate_spatial(&layout, tab.focused_pane, direction) {
            tab.focused_pane = target;
        }
    }

    /// Recalculates pane dimensions for the active tab based on current window size,
    /// resizing each pane's terminal and PTY session accordingly.
    pub(in crate::gui) fn resize_all_panes(&mut self) {
        let size = self.window.inner_size();
        let tab_bar_h = self.backend.tab_bar_height_px();
        let padding = self.backend.window_padding_px();
        let cw = self.backend.cell_width();
        let ch = self.backend.cell_height();

        let terminal_rect = PaneRect {
            x: padding,
            y: tab_bar_h + padding,
            width: size.width.saturating_sub(padding * 2),
            height: size.height.saturating_sub(tab_bar_h + padding * 2),
        };

        let divider_px = DIVIDER_WIDTH;
        let scaled_pane_pad = self.backend.pane_inner_padding_px();
        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        let pane_pad = if tab.has_multiple_panes() {
            scaled_pane_pad
        } else {
            0
        };
        let layout = tab.pane_tree.layout(terminal_rect, divider_px);

        for (pane_id, rect) in layout {
            if let Some(leaf) = tab.pane_tree.find_leaf_mut(pane_id) {
                let content = rect.inset(pane_pad);
                let cols = (content.width / cw).max(1) as usize;
                let rows = (content.height / ch).max(1) as usize;
                leaf.terminal.resize(rows, cols);
                leaf.scroll_offset = leaf.scroll_offset.min(leaf.terminal.scrollback.len());
                if let Some(ref session) = leaf.session
                    && let Err(err) = session.resize(rows as u16, cols as u16)
                {
                    eprintln!("Failed to resize PTY for pane {}: {err}", pane_id);
                }
            }
        }
    }
}
