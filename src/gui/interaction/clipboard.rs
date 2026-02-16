use crate::gui::*;

const BRACKETED_PASTE_START: &[u8] = b"\x1b[200~";
const BRACKETED_PASTE_END: &[u8] = b"\x1b[201~";

impl FerrumWindow {
    fn wrap_bracketed_paste(text: &str) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            BRACKETED_PASTE_START.len() + text.len() + BRACKETED_PASTE_END.len(),
        );
        bytes.extend_from_slice(BRACKETED_PASTE_START);
        bytes.extend_from_slice(text.as_bytes());
        bytes.extend_from_slice(BRACKETED_PASTE_END);
        bytes
    }

    pub(in crate::gui) fn copy_selection(&mut self) {
        let tab = match self.active_tab_ref() {
            Some(t) => t,
            None => return,
        };
        let Some(ref sel) = tab.selection else { return };
        let (start, end) = sel.normalized();

        let mut text = String::new();
        for row in start.row..=end.row {
            let col_start = if row == start.row { start.col } else { 0 };
            let col_end = if row == end.row {
                end.col
            } else {
                tab.terminal.grid.cols - 1
            };

            for col in col_start..=col_end {
                text.push(tab.terminal.grid.get(row, col).character);
            }
            if row < end.row {
                text.push('\n');
            }
        }

        let text: String = text
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");

        if let Some(ref mut clipboard) = self.clipboard {
            let _ = clipboard.set_text(&text);
        }
    }

    pub(in crate::gui) fn paste_clipboard(&mut self) {
        let text = self
            .clipboard
            .as_mut()
            .and_then(|cb| cb.get_text().ok())
            .unwrap_or_default();

        if text.is_empty() {
            return;
        }
        if let Some(tab) = self.active_tab_mut() {
            tab.security.check_paste_payload(&text);
            let bytes = if tab.security.should_wrap_paste() {
                Self::wrap_bracketed_paste(&text)
            } else {
                text.as_bytes().to_vec()
            };
            let _ = tab.pty_writer.write_all(&bytes);
            let _ = tab.pty_writer.flush();
        }
    }
}
