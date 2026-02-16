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
                self.refresh_tab_bar_visibility();
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

        let shell = pty::default_shell();
        let session = pty::Session::spawn(&shell, rows as u16, cols as u16)
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

        let mut terminal = Terminal::new(rows, cols);

        // Show "Last login" greeting with local time.
        {
            let msg = last_login_message();
            terminal.process(msg.as_bytes());
        }

        Ok(TabState {
            id,
            terminal,
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

#[cfg(unix)]
fn last_login_message() -> String {
    use std::fmt::Write as _;
    let dow_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let mon_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        let mut msg = String::new();
        let _ = write!(
            msg,
            "Last login: {} {} {:2} {:02}:{:02}:{:02} {}\r\n",
            dow_names[tm.tm_wday as usize],
            mon_names[tm.tm_mon as usize],
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec,
            tm.tm_year + 1900,
        );
        msg
    }
}

#[cfg(windows)]
fn last_login_message() -> String {
    use std::fmt::Write as _;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut msg = String::new();
    let _ = write!(msg, "Last login: {now}\r\n");
    msg
}
