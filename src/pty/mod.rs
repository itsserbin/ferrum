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

// Shell integration scripts (all platforms)
const SHELL_INTEGRATION_ZSH: &str = include_str!("shell-integration/zsh/ferrum-integration");
const SHELL_INTEGRATION_BASH: &str = include_str!("shell-integration/bash/ferrum.bash");
const SHELL_INTEGRATION_FISH: &str =
    include_str!("shell-integration/fish/vendor_conf.d/ferrum-shell-integration.fish");

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

/// Write shell integration scripts to a temp directory so they can be
/// sourced by the spawned shell.  Returns the root temp dir on success.
fn setup_shell_integration() -> Option<std::path::PathBuf> {
    let temp_dir = std::env::temp_dir().join("ferrum_shell_integration");
    let zsh_dir = temp_dir.join("zsh");
    let bash_dir = temp_dir.join("bash");
    let fish_dir = temp_dir.join("fish").join("vendor_conf.d");
    std::fs::create_dir_all(&zsh_dir).ok()?;
    std::fs::create_dir_all(&bash_dir).ok()?;
    std::fs::create_dir_all(&fish_dir).ok()?;
    std::fs::write(zsh_dir.join("ferrum-integration"), SHELL_INTEGRATION_ZSH).ok()?;
    std::fs::write(bash_dir.join("ferrum.bash"), SHELL_INTEGRATION_BASH).ok()?;
    std::fs::write(
        fish_dir.join("ferrum-shell-integration.fish"),
        SHELL_INTEGRATION_FISH,
    )
    .ok()?;
    Some(temp_dir)
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
    pub fn spawn(shell: &str, rows: u16, cols: u16, cwd: Option<&str>) -> anyhow::Result<Self> {
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

        if let Some(dir) = cwd {
            let path = std::path::Path::new(dir);
            if path.is_dir() {
                cmd.cwd(dir);
                cmd.env("PWD", dir);
            }
        }

        #[cfg(target_os = "macos")]
        cmd.arg("-l");
        cmd.env("TERM", "xterm-256color");

        // Shell integration: set marker env and configure per-shell sourcing.
        cmd.env("FERRUM_SHELL_INTEGRATION", "1");

        if let Some(integration_dir) = setup_shell_integration() {
            let shell_name = std::path::Path::new(shell)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(shell);

            match shell_name {
                "zsh" => {
                    let zdotdir = integration_dir.join("zsh");
                    let user_zdotdir = std::env::var("ZDOTDIR").unwrap_or_else(|_| {
                        std::env::var("HOME").unwrap_or_else(|_| String::from("/"))
                    });
                    let zshenv_content = format!(
                        "ZDOTDIR=\"{user_zdotdir}\"\n\
                         [[ -f \"$ZDOTDIR/.zshenv\" ]] && source \"$ZDOTDIR/.zshenv\"\n\
                         source \"{}/ferrum-integration\"\n",
                        zdotdir.display()
                    );
                    let _ = std::fs::write(zdotdir.join(".zshenv"), zshenv_content);
                    cmd.env("ZDOTDIR", zdotdir.to_string_lossy().as_ref());
                }
                "bash" => {
                    let bash_integration = integration_dir.join("bash").join("ferrum.bash");
                    cmd.env("BASH_ENV", bash_integration.to_string_lossy().as_ref());
                }
                "fish" => {
                    let fish_dir = integration_dir.join("fish");
                    let existing = std::env::var("XDG_DATA_DIRS")
                        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
                    let new_xdg = format!("{}:{}", fish_dir.display(), existing);
                    cmd.env("XDG_DATA_DIRS", &new_xdg);
                }
                _ => {}
            }
        }

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
    /// for it to exit. Meant to be called from a background thread.
    pub fn shutdown(mut self) {
        #[cfg(windows)]
        self.shutdown_windows();

        #[cfg(not(windows))]
        self.shutdown_unix();
    }

    #[cfg(not(windows))]
    fn shutdown_unix(&mut self) {
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

    #[cfg(windows)]
    fn shutdown_windows(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Console::{GenerateConsoleCtrlEvent, CTRL_BREAK_EVENT};
        use windows_sys::Win32::System::Threading::{
            OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
        };

        // Try graceful shutdown first: send CTRL_BREAK to the process group.
        // This lets cmd.exe exit cleanly without flashing its console window.
        // Note: portable-pty spawns conpty processes with CREATE_NEW_PROCESS_GROUP,
        // so the child PID equals its process group ID.
        if let Some(pid) = self.child.process_id() {
            unsafe {
                // Send CTRL_BREAK to the process group (PID == PGID for conpty).
                let _ = GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid);

                // Wait up to 150ms for the process to exit gracefully.
                let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
                if handle != 0 {
                    WaitForSingleObject(handle, 150);
                    CloseHandle(handle);
                }
            }
        }

        // Hard kill if still alive (kill() handles the "already dead" case).
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
    use std::mem;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
        TH32CS_SNAPPROCESS,
    };

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == -1_isize as _ {
            return false;
        }

        let mut entry: PROCESSENTRY32 = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32>() as u32;

        if Process32First(snapshot, &mut entry) == 0 {
            CloseHandle(snapshot);
            return false;
        }

        loop {
            if entry.th32ParentProcessID == shell_pid {
                CloseHandle(snapshot);
                return true;
            }
            if Process32Next(snapshot, &mut entry) == 0 {
                break;
            }
        }

        CloseHandle(snapshot);
        false
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Kill only — no wait(). This is a safety net for sessions that
        // weren't extracted for background cleanup (e.g. during a panic).
        // Blocking wait() was removed to prevent UI thread hangs.
        // Errors are silenced: the process is usually already dead
        // (killed by shutdown() on the background thread).
        let _ = self.child.kill();
    }
}
