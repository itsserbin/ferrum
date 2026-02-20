use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{Read, Write};

#[cfg(windows)]
use std::path::{Path, PathBuf};

// Embed script files at compile time
#[cfg(windows)]
const SCRIPT_LS: &str = include_str!("scripts/ls.cmd");
#[cfg(windows)]
const SCRIPT_LL: &str = include_str!("scripts/ll.cmd");
#[cfg(windows)]
const SCRIPT_CAT: &str = include_str!("scripts/cat.cmd");
#[cfg(windows)]
const SCRIPT_RM: &str = include_str!("scripts/rm.cmd");
#[cfg(windows)]
const SCRIPT_GREP: &str = include_str!("scripts/grep.cmd");
#[cfg(windows)]
const SCRIPT_HEAD: &str = include_str!("scripts/head.cmd");
#[cfg(windows)]
const SCRIPT_TAIL: &str = include_str!("scripts/tail.cmd");
#[cfg(windows)]
const SCRIPT_WC: &str = include_str!("scripts/wc.cmd");
#[cfg(windows)]
const SCRIPT_FIND: &str = include_str!("scripts/find.cmd");
#[cfg(windows)]
const SCRIPT_DU: &str = include_str!("scripts/du.cmd");
#[cfg(windows)]
const SCRIPT_PS: &str = include_str!("scripts/ps.cmd");
#[cfg(windows)]
const SCRIPT_INIT: &str = include_str!("scripts/init.cmd");

/// Create Unix-style command wrapper scripts in temp directory
#[cfg(windows)]
fn create_unix_aliases_script() -> Option<PathBuf> {
    use std::fs;

    let temp_dir = std::env::temp_dir();
    let scripts_dir = temp_dir.join("ferrum_scripts");

    // Create scripts directory
    if fs::create_dir_all(&scripts_dir).is_err() {
        return None;
    }

    // Write all command wrappers
    let scripts = [
        ("ls.cmd", SCRIPT_LS),
        ("ll.cmd", SCRIPT_LL),
        ("cat.cmd", SCRIPT_CAT),
        ("rm.cmd", SCRIPT_RM),
        ("grep.cmd", SCRIPT_GREP),
        ("head.cmd", SCRIPT_HEAD),
        ("tail.cmd", SCRIPT_TAIL),
        ("wc.cmd", SCRIPT_WC),
        ("find.cmd", SCRIPT_FIND),
        ("du.cmd", SCRIPT_DU),
        ("ps.cmd", SCRIPT_PS),
    ];

    for (filename, content) in &scripts {
        let script_path = scripts_dir.join(filename);
        if fs::write(&script_path, content).is_err() {
            return None;
        }
    }

    // Create init script with PATH modification
    let init_script = scripts_dir.join("init.cmd");
    let init_content = format!(
        "{}\nset PATH={};%PATH%\n",
        SCRIPT_INIT,
        scripts_dir.display()
    );

    fs::write(&init_script, init_content).ok()?;
    Some(init_script)
}

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

        #[cfg(windows)]
        {
            let shell_name = Path::new(shell)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(shell)
                .to_ascii_lowercase();

            // Force UTF-8 code page for cmd.exe so PTY output is UTF-8.
            // Also setup Unix-style command aliases using doskey.
            if shell_name == "cmd.exe" || shell_name == "cmd" {
                // Create a temporary startup script for Unix aliases
                let startup_script = create_unix_aliases_script();

                cmd.arg("/Q");
                if let Some(script_path) = startup_script {
                    // Use /K to run the init script
                    cmd.arg("/K");
                    cmd.arg(&script_path);
                } else {
                    cmd.arg("/K");
                    cmd.arg("chcp 65001 >nul");
                }
            }
        }

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

    /// Returns the PID of the shell process running inside this PTY session.
    pub fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// Performs a full graceful shutdown: kill the child process and wait
    /// for it to exit. Meant to be called from a background thread to
    /// avoid blocking the UI.
    pub fn shutdown(mut self) {
        if let Err(e) = self.child.kill() {
            if e.kind() != std::io::ErrorKind::InvalidInput {
                eprintln!("Failed to kill PTY child process: {}", e);
            }
        }
        if let Err(e) = self.child.wait() {
            if e.kind() != std::io::ErrorKind::InvalidInput {
                eprintln!("Failed to wait on PTY child process: {}", e);
            }
        }
    }
}

/// Returns `true` when the shell process has at least one child process.
/// This is used as a heuristic for "active command is running".
pub fn has_active_child_processes(shell_pid: u32) -> bool {
    #[cfg(unix)]
    {
        return has_active_child_processes_unix(shell_pid);
    }

    #[cfg(windows)]
    {
        return has_active_child_processes_windows(shell_pid);
    }

    #[allow(unreachable_code)]
    {
        let _ = shell_pid;
        false
    }
}

#[cfg(unix)]
fn has_active_child_processes_unix(shell_pid: u32) -> bool {
    use std::process::Command;

    let pid = shell_pid.to_string();

    // Fast path: pgrep exits with code 0 when a child exists, 1 when none.
    match Command::new("pgrep").arg("-P").arg(&pid).output() {
        Ok(output) => {
            if output.status.success() {
                return !String::from_utf8_lossy(&output.stdout).trim().is_empty();
            }
            if output.status.code() == Some(1) {
                return false;
            }
        }
        Err(_) => {}
    }

    // Fallback for environments without pgrep.
    let output = match Command::new("ps").arg("-e").arg("-o").arg("ppid=").output() {
        Ok(output) => output,
        Err(_) => return false,
    };

    if !output.status.success() {
        return false;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .any(|ppid| ppid == shell_pid)
}

#[cfg(windows)]
fn has_active_child_processes_windows(shell_pid: u32) -> bool {
    use std::process::Command;

    let script = format!(
        "$p = Get-CimInstance Win32_Process -Filter \"ParentProcessId = {shell_pid}\" -ErrorAction SilentlyContinue; if ($p) {{ '1' }} else {{ '0' }}"
    );

    let output = match Command::new("powershell")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(script)
        .output()
    {
        Ok(output) => output,
        Err(_) => return false,
    };

    if !output.status.success() {
        return false;
    }

    String::from_utf8_lossy(&output.stdout).trim() == "1"
}

impl Drop for Session {
    fn drop(&mut self) {
        // Kill only — no wait(). This is a safety net for sessions that
        // weren't extracted for background cleanup (e.g. during a panic).
        // Blocking wait() was removed to prevent UI thread hangs.
        if let Err(e) = self.child.kill() {
            if e.kind() != std::io::ErrorKind::InvalidInput {
                eprintln!("Failed to kill PTY child process: {}", e);
            }
        }
    }
}
