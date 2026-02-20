use crate::gui::pane::{PaneLeaf, PaneNode};
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

        let pane_id: u64 = 0;

        let shell = pty::default_shell();
        let session = pty::Session::spawn(&shell, rows as u16, cols as u16)
            .context("failed to spawn PTY session")?;
        let pty_writer = session.writer().context("failed to acquire PTY writer")?;

        // Spawn a dedicated PTY reader thread for this tab/pane.
        let tx = tx.clone();
        let mut reader = session.reader().context("failed to clone PTY reader")?;
        let tab_id = id;
        let reader_pane_id = pane_id;
        std::thread::Builder::new()
            .name(format!("pty-reader-{}-{}", tab_id, reader_pane_id))
            .spawn(move || {
                use std::io::Read;
                let mut buf = [0u8; 4096];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => {
                            let _ = tx.send(PtyEvent::Exited { tab_id, pane_id: reader_pane_id });
                            break;
                        }
                        Err(err) => {
                            eprintln!("PTY read error for tab {tab_id}: {err}");
                            let _ = tx.send(PtyEvent::Exited { tab_id, pane_id: reader_pane_id });
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
                                eprintln!("PTY reader {}: channel disconnected", tab_id);
                                break;
                            }
                        }
                    }
                }
            })
            .context("failed to spawn PTY reader thread")?;

        let mut terminal = Terminal::new(rows, cols);

        // Show "Last login" greeting with local time.
        {
            let msg = last_login_message();
            terminal.process(msg.as_bytes());
        }

        let leaf = PaneLeaf {
            id: pane_id,
            terminal,
            session: Some(session),
            pty_writer,
            selection: None,
            scroll_offset: 0,
            security: SecurityGuard::new(),
            scrollbar: ScrollbarState::new(),
        };

        Ok(TabState {
            id,
            title: title.unwrap_or_else(|| format!("bash #{}", id + 1)),
            pane_tree: PaneNode::Leaf(leaf),
            focused_pane: pane_id,
            next_pane_id: 1,
        })
    }
}

#[cfg(unix)]
fn last_login_message() -> String {
    use std::fmt::Write as _;
    let dow_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let mon_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    // SAFETY: These libc calls are safe because:
    // 1. libc::time(null) is explicitly allowed by POSIX - passing null means
    //    the time value is only returned (not stored through a pointer).
    // 2. std::mem::zeroed() for libc::tm is safe because tm is a plain C struct
    //    with no invariants - all zero bytes represent valid (if meaningless) values.
    // 3. libc::localtime_r is the thread-safe variant that writes to our stack-local
    //    tm struct. The pointer to tm is valid for the duration of the call.
    // 4. The tm fields (tm_wday, tm_mon, etc.) are guaranteed to be in valid ranges
    //    by the libc implementation after a successful localtime_r call.
    // 5. We use .get().unwrap_or() for array access to handle any unexpected values.
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        let mut msg = String::new();
        let dow = dow_names.get(tm.tm_wday as usize).unwrap_or(&"???");
        let mon = mon_names.get(tm.tm_mon as usize).unwrap_or(&"???");
        let _ = write!(
            msg,
            "Last login: {} {} {:2} {:02}:{:02}:{:02} {}\r\n",
            dow,
            mon,
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
    let dow_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let mon_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut msg = String::new();
    // SAFETY: These libc calls are safe because:
    // 1. libc::time(null) is explicitly allowed - passing null means the time value
    //    is only returned (not stored through a pointer).
    // 2. std::mem::zeroed() for libc::tm is safe because tm is a plain C struct
    //    with no invariants - all zero bytes represent valid (if meaningless) values.
    // 3. libc::localtime_s (Windows secure variant) writes to our stack-local tm struct.
    //    The pointers to tm and now are valid for the duration of the call.
    // 4. We check the return value of localtime_s and fall back to epoch time on failure.
    // 5. On success, tm fields are guaranteed to be in valid ranges by the CRT.
    // 6. We use .get().unwrap_or() for array access to handle any unexpected values.
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_s(&mut tm, &now) == 0 {
            let dow = dow_names.get(tm.tm_wday as usize).unwrap_or(&"???");
            let mon = mon_names.get(tm.tm_mon as usize).unwrap_or(&"???");
            let _ = write!(
                msg,
                "Last login: {} {} {:2} {:02}:{:02}:{:02} {}\r\n",
                dow,
                mon,
                tm.tm_mday,
                tm.tm_hour,
                tm.tm_min,
                tm.tm_sec,
                tm.tm_year + 1900,
            );
        } else {
            let epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let _ = write!(msg, "Last login: {epoch}\r\n");
        }
    }
    msg
}
