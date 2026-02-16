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

        // Show "Last login" greeting.
        {
            use std::fmt::Write as _;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let secs_per_day = 86400;
            let secs_per_hour = 3600;
            let secs_per_min = 60;
            let days_since_epoch = now / secs_per_day;
            let time_of_day = now % secs_per_day;
            let hour = time_of_day / secs_per_hour;
            let min = (time_of_day % secs_per_hour) / secs_per_min;
            let sec = time_of_day % secs_per_min;
            let dow = ((days_since_epoch + 4) % 7) as usize;
            let dow_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
            let (year, month, day) = days_to_ymd(days_since_epoch);
            let mon_names = [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun",
                "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ];
            let mut msg = String::new();
            let _ = write!(
                msg,
                "Last login: {} {} {:2} {:02}:{:02}:{:02} {}\r\n",
                dow_names[dow], mon_names[month as usize - 1], day, hour, min, sec, year,
            );
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

/// Converts days since Unix epoch to (year, month, day).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Civil calendar algorithm from Howard Hinnant.
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
