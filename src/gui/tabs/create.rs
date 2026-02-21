use crate::config::AppConfig;
use crate::gui::pane::{PaneLeaf, PaneNode};
use crate::gui::tabs::pty_reader::spawn_pty_reader;
use crate::gui::*;
use anyhow::Context;

/// Bundles the parameters needed to create a new tab, avoiding long argument lists.
pub(in crate::gui) struct NewTabParams<'a> {
    pub rows: usize,
    pub cols: usize,
    pub title: Option<String>,
    pub next_tab_id: &'a mut u64,
    pub tx: &'a mpsc::Sender<PtyEvent>,
    pub cwd: Option<String>,
    pub config: &'a AppConfig,
}

impl FerrumWindow {
    /// Creates a new tab with default title.
    pub(in crate::gui) fn new_tab(&mut self, mut params: NewTabParams<'_>) {
        match self.build_tab_state(&mut params) {
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

    fn build_tab_state(&self, params: &mut NewTabParams<'_>) -> anyhow::Result<TabState> {
        let id = *params.next_tab_id;
        *params.next_tab_id += 1;

        let pane_id: u64 = 0;

        let shell = pty::default_shell();
        let session = pty::Session::spawn(
            &shell,
            params.rows as u16,
            params.cols as u16,
            params.cwd.as_deref(),
        )
        .context("failed to spawn PTY session")?;
        let pty_writer = session.writer().context("failed to acquire PTY writer")?;

        // Spawn a dedicated PTY reader thread for this tab/pane.
        let reader = session.reader().context("failed to clone PTY reader")?;
        spawn_pty_reader(
            reader,
            params.tx.clone(),
            self.event_proxy.clone(),
            id,
            pane_id,
        )
        .context("failed to spawn PTY reader thread")?;

        let palette = params.config.theme.resolve();
        let mut terminal = Terminal::with_config(
            params.rows,
            params.cols,
            params.config.terminal.max_scrollback,
            palette.default_fg,
            palette.default_bg,
            palette.ansi,
        );

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

        let shell_name = std::path::Path::new(&shell)
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("shell")
            .to_string();

        let is_renamed = params.title.is_some();
        let tab_title = params
            .title
            .as_ref()
            .cloned()
            .unwrap_or(shell_name);

        Ok(TabState {
            id,
            title: tab_title,
            pane_tree: PaneNode::Leaf(Box::new(leaf)),
            focused_pane: pane_id,
            next_pane_id: 1,
            is_renamed,
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
