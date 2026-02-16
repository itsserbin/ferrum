use crate::gui::*;
use anyhow::Context;

impl FerrumWindow {
    /// Creates a new tab with default title.
    pub(in crate::gui) fn new_tab(
        &mut self,
        rows: usize,
        cols: usize,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) {
        self.new_tab_with_title(rows, cols, None, next_tab_id, tx);
    }

    /// Creates a new tab with an optional custom title.
    pub(in crate::gui) fn new_tab_with_title(
        &mut self,
        rows: usize,
        cols: usize,
        title: Option<String>,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) {
        match Self::build_tab_state(rows, cols, title, next_tab_id, tx) {
            Ok(tab) => {
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
            }
            Err(err) => {
                eprintln!("Failed to create tab: {err}");
            }
        }
    }

    fn build_tab_state(
        rows: usize,
        cols: usize,
        title: Option<String>,
        next_tab_id: &mut u64,
        tx: &mpsc::Sender<PtyEvent>,
    ) -> anyhow::Result<TabState> {
        let id = *next_tab_id;
        *next_tab_id += 1;

        let session = pty::Session::spawn(pty::DEFAULT_SHELL, rows as u16, cols as u16)
            .context("failed to spawn PTY session")?;
        let pty_writer = session.writer().context("failed to acquire PTY writer")?;

        // Spawn a dedicated PTY reader thread for this tab.
        let tx = tx.clone();
        let mut reader = session.reader().context("failed to clone PTY reader")?;
        let tab_id = id;
        std::thread::spawn(move || {
            use std::io::Read;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = tx.send(PtyEvent::Exited { tab_id });
                        break;
                    }
                    Err(err) => {
                        eprintln!("PTY read error for tab {tab_id}: {err}");
                        let _ = tx.send(PtyEvent::Exited { tab_id });
                        break;
                    }
                    Ok(n) => {
                        if tx
                            .send(PtyEvent::Data {
                                tab_id,
                                bytes: buf[..n].to_vec(),
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        });

        Ok(TabState {
            id,
            terminal: Terminal::new(rows, cols),
            session,
            pty_writer,
            title: title.unwrap_or_else(|| format!("bash #{}", id + 1)),
            scroll_offset: 0,
            selection: None,
            security: SecurityGuard::new(),
            scrollbar: ScrollbarState::new(),
        })
    }
}
