use crate::config::AppConfig;
use crate::core::{PageCoord, Selection};
use crate::gui::menus::MenuAction;
use crate::gui::pane::SplitDirection;
use crate::gui::*;

impl FerrumWindow {
    /// Byte sequence sent to the PTY after a programmatic clear/reset.
    ///
    /// On Unix, `\x0c` (form feed) tells bash/zsh to redraw the prompt.
    /// On Windows, `cls\r\n` clears the conpty virtual screen and resets
    /// its internal cursor to (0,0), so the fresh prompt appears at top.
    #[cfg(unix)]
    const CLEAR_PTY_SEQUENCE: &[u8] = b"\x0c";
    #[cfg(windows)]
    const CLEAR_PTY_SEQUENCE: &[u8] = b"cls\r\n";

    fn focus_menu_target_pane(&mut self, pane_id: Option<pane::PaneId>) {
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
        pane_id: Option<pane::PaneId>,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
        config: &AppConfig,
    ) {
        self.focus_menu_target_pane(pane_id);

        match action {
            MenuAction::Copy => self.copy_selection(),
            MenuAction::Paste => self.paste_clipboard(),
            MenuAction::SelectAll => {
                if let Some(leaf) = self.active_leaf_mut() {
                    let last_row = leaf.terminal.screen.scrollback_len()
                        + leaf.terminal.screen.viewport_rows().saturating_sub(1);
                    let last_col = leaf.terminal.screen.cols().saturating_sub(1);
                    leaf.set_selection(Selection {
                        start: PageCoord { abs_row: 0, col: 0 },
                        end: PageCoord {
                            abs_row: last_row,
                            col: last_col,
                        },
                    });
                }
            }
            MenuAction::ClearSelection => {
                if let Some(leaf) = self.active_leaf_mut() {
                    leaf.clear_selection();
                }
                self.selection_anchor = None;
                self.keyboard_selection_anchor = None;
            }
            MenuAction::SplitRight => {
                self.split_pane(SplitDirection::Horizontal, false, next_tab_id, tx, config);
            }
            MenuAction::SplitDown => {
                self.split_pane(SplitDirection::Vertical, false, next_tab_id, tx, config);
            }
            MenuAction::SplitLeft => {
                self.split_pane(SplitDirection::Horizontal, true, next_tab_id, tx, config);
            }
            MenuAction::SplitUp => {
                self.split_pane(SplitDirection::Vertical, true, next_tab_id, tx, config);
            }
            MenuAction::ClosePane => {
                self.close_focused_pane();
            }
            MenuAction::ClearTerminal => {
                if let Some(leaf) = self.active_leaf_mut() {
                    leaf.terminal.clear_screen();
                    leaf.scroll_offset = 0;
                    leaf.clear_selection();
                    leaf.write_pty(Self::CLEAR_PTY_SEQUENCE);
                }
                self.selection_anchor = None;
                self.keyboard_selection_anchor = None;
            }
            MenuAction::ResetTerminal => {
                if let Some(leaf) = self.active_leaf_mut() {
                    leaf.terminal.full_reset();
                    leaf.scroll_offset = 0;
                    leaf.clear_selection();
                    leaf.write_pty(Self::CLEAR_PTY_SEQUENCE);
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
                    self.duplicate_tab(idx, next_tab_id, tx, config);
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
