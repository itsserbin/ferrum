use crate::gui::menus::MenuAction;
use crate::gui::*;

impl FerrumWindow {
    pub(in crate::gui) fn handle_menu_action(
        &mut self,
        action: MenuAction,
        tab_index: Option<usize>,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) {
        match action {
            MenuAction::Copy => self.copy_selection(),
            MenuAction::Paste => self.paste_clipboard(),
            MenuAction::SelectAll => {
                // TODO: implement select all
            }
            MenuAction::ClearSelection => {
                if let Some(leaf) = self.active_leaf_mut() {
                    leaf.selection = None;
                }
                self.selection_anchor = None;
                self.keyboard_selection_anchor = None;
            }
            MenuAction::SplitRight
            | MenuAction::SplitDown
            | MenuAction::SplitLeft
            | MenuAction::SplitUp => {
                // TODO: split pane (Task 9)
            }
            MenuAction::ClosePane => {
                // TODO: close pane (Task 9)
            }
            MenuAction::ClearTerminal => {
                // Send "clear screen + move cursor home" escape sequence to the PTY.
                // ESC[2J = erase entire display, ESC[H = move cursor to home.
                self.write_pty_bytes(b"\x1b[2J\x1b[H");
            }
            MenuAction::ResetTerminal => {
                // Send soft terminal reset (CSI ! p = DECSTR).
                self.write_pty_bytes(b"\x1b[!p");
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
