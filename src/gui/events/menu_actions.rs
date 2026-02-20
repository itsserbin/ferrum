use std::io::Write;

use crate::core::{Grid, Selection, SelectionPoint};
use crate::gui::menus::MenuAction;
use crate::gui::pane::SplitDirection;
use crate::gui::*;

impl FerrumWindow {
    /// Byte sequence sent to the PTY after a programmatic clear/reset.
    ///
    /// On Unix, `\x0c` (form feed) tells bash/zsh to redraw the prompt.
    /// On Windows, cmd.exe does not understand form feed — sending `\r\n`
    /// triggers a fresh prompt instead.
    #[cfg(unix)]
    const CLEAR_PTY_SEQUENCE: &[u8] = b"\x0c";
    #[cfg(windows)]
    const CLEAR_PTY_SEQUENCE: &[u8] = b"\r\n";

    fn focus_menu_target_pane(&mut self, pane_id: Option<crate::gui::pane::PaneId>) {
        let Some(pane_id) = pane_id else {
            return;
        };
        if let Some(tab) = self.active_tab_mut()
            && tab.pane_tree.find_leaf(pane_id).is_some()
        {
            tab.focused_pane = pane_id;
        }
    }

    pub(in crate::gui) fn handle_menu_action(
        &mut self,
        action: MenuAction,
        tab_index: Option<usize>,
        pane_id: Option<crate::gui::pane::PaneId>,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) {
        self.focus_menu_target_pane(pane_id);

        match action {
            MenuAction::Copy => self.copy_selection(),
            MenuAction::Paste => self.paste_clipboard(),
            MenuAction::SelectAll => {
                if let Some(leaf) = self.active_leaf_mut() {
                    let last_row = leaf.terminal.scrollback.len()
                        + leaf.terminal.grid.rows.saturating_sub(1);
                    let last_col = leaf.terminal.grid.cols.saturating_sub(1);
                    leaf.selection = Some(Selection {
                        start: SelectionPoint { row: 0, col: 0 },
                        end: SelectionPoint {
                            row: last_row,
                            col: last_col,
                        },
                    });
                }
            }
            MenuAction::ClearSelection => {
                if let Some(leaf) = self.active_leaf_mut() {
                    leaf.selection = None;
                }
                self.selection_anchor = None;
                self.keyboard_selection_anchor = None;
            }
            MenuAction::SplitRight => {
                self.split_pane(SplitDirection::Horizontal, false, next_tab_id, tx);
            }
            MenuAction::SplitDown => {
                self.split_pane(SplitDirection::Vertical, false, next_tab_id, tx);
            }
            MenuAction::SplitLeft => {
                self.split_pane(SplitDirection::Horizontal, true, next_tab_id, tx);
            }
            MenuAction::SplitUp => {
                self.split_pane(SplitDirection::Vertical, true, next_tab_id, tx);
            }
            MenuAction::ClosePane => {
                self.close_focused_pane();
            }
            MenuAction::ClearTerminal => {
                if let Some(leaf) = self.active_leaf_mut() {
                    let rows = leaf.terminal.grid.rows;
                    let cols = leaf.terminal.grid.cols;
                    leaf.terminal.grid = Grid::new(rows, cols);
                    leaf.terminal.scrollback.clear();
                    leaf.terminal.cursor_row = 0;
                    leaf.terminal.cursor_col = 0;
                    leaf.terminal.reset_scroll_region();
                    leaf.scroll_offset = 0;
                    leaf.selection = None;
                    let _ = leaf.pty_writer.write_all(Self::CLEAR_PTY_SEQUENCE);
                    let _ = leaf.pty_writer.flush();
                }
                self.selection_anchor = None;
                self.keyboard_selection_anchor = None;
            }
            MenuAction::ResetTerminal => {
                if let Some(leaf) = self.active_leaf_mut() {
                    leaf.terminal.full_reset();
                    leaf.scroll_offset = 0;
                    leaf.selection = None;
                    let _ = leaf.pty_writer.write_all(Self::CLEAR_PTY_SEQUENCE);
                    let _ = leaf.pty_writer.flush();
                }
                self.selection_anchor = None;
                self.keyboard_selection_anchor = None;
            }
            MenuAction::RenameTab => {
                if let Some(idx) = tab_index {
                    self.start_rename(idx);
                }
            }
            MenuAction::DuplicateTab => {
                if let Some(idx) = tab_index {
                    self.duplicate_tab(idx, next_tab_id, tx);
                }
            }
            MenuAction::CloseTab => {
                if let Some(idx) = tab_index {
                    self.close_tab(idx);
                }
            }
        }
    }
}
