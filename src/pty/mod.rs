use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{Read, Write};

#[cfg(unix)]
pub fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
}

#[cfg(windows)]
pub fn default_shell() -> String {
    std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
}

pub struct Session {
    master: Box<dyn portable_pty::MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl Session {
    pub fn spawn(shell: &str, rows: u16, cols: u16) -> anyhow::Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(shell);
        #[cfg(target_os = "macos")]
        cmd.arg("-l");
        cmd.env("TERM", "xterm-256color");
        let child = pair.slave.spawn_command(cmd)?;

        // Drop slave handle after spawn to avoid fd leaks and ensure EOF propagation.
        drop(pair.slave);

        Ok(Session {
            master: pair.master,
            child,
        })
    }

    /// Clones a PTY reader; can be called multiple times.
    pub fn reader(&self) -> anyhow::Result<Box<dyn Read + Send>> {
        self.master.try_clone_reader()
    }

    /// Takes PTY writer ownership; callable only once.
    pub fn writer(&self) -> anyhow::Result<Box<dyn Write + Send>> {
        self.master.take_writer()
    }

    pub fn resize(&self, rows: u16, cols: u16) -> anyhow::Result<()> {
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
