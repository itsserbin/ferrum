/// Cross-platform OS API fallback for querying the current working directory
/// of a process given its PID.
///
/// Used when shell integration (OSC 7) is unavailable — falls back to
/// platform-specific APIs to determine CWD from the shell process directly.

pub fn get_process_cwd(pid: u32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        return get_cwd_linux(pid);
    }
    #[cfg(target_os = "macos")]
    {
        return get_cwd_macos(pid);
    }
    #[cfg(target_os = "windows")]
    {
        return get_cwd_windows(pid);
    }
    #[allow(unreachable_code)]
    {
        let _ = pid;
        None
    }
}

#[cfg(target_os = "linux")]
fn get_cwd_linux(pid: u32) -> Option<String> {
    let link = format!("/proc/{}/cwd", pid);
    std::fs::read_link(link)
        .ok()
        .and_then(|p| p.to_str().map(String::from))
}

#[cfg(target_os = "macos")]
fn get_cwd_macos(pid: u32) -> Option<String> {
    use std::mem;

    const PROC_PIDVNODEPATHINFO: i32 = 9;
    const MAXPATHLEN: usize = 1024;

    #[repr(C)]
    struct VInfoPathInfo {
        cdir: VnodePathInfo,
        rdir: VnodePathInfo,
    }

    #[repr(C)]
    struct VnodePathInfo {
        _vip_vi: [u8; 152], // vnode_info_path padding
        vip_path: [u8; MAXPATHLEN],
    }

    unsafe extern "C" {
        fn proc_pidinfo(
            pid: i32,
            flavor: i32,
            arg: u64,
            buffer: *mut libc::c_void,
            buffersize: i32,
        ) -> i32;
    }

    let mut info: VInfoPathInfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<VInfoPathInfo>() as i32;

    let ret = unsafe {
        proc_pidinfo(
            pid as i32,
            PROC_PIDVNODEPATHINFO,
            0,
            &mut info as *mut _ as *mut libc::c_void,
            size,
        )
    };

    if ret <= 0 {
        return None;
    }

    let path_bytes = &info.cdir.vip_path;
    let nul = path_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(MAXPATHLEN);
    std::str::from_utf8(&path_bytes[..nul])
        .ok()
        .filter(|s| !s.is_empty())
        .map(String::from)
}

#[cfg(target_os = "windows")]
fn get_cwd_windows(_pid: u32) -> Option<String> {
    // TODO: Implement via NtQueryInformationProcess + ReadProcessMemory
    // For now, cmd.exe and PowerShell emit OSC 7 via shell integration.
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_cwd_matches_env() {
        let pid = std::process::id();
        let cwd = get_process_cwd(pid);
        // On macOS/Linux this should work; Windows stub returns None
        #[cfg(unix)]
        {
            assert!(cwd.is_some(), "should be able to query own CWD on unix");
            let expected = std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string();
            assert_eq!(cwd.unwrap(), expected);
        }
    }

    #[test]
    fn invalid_pid_returns_none() {
        // Use a PID that's almost certainly not running
        assert!(get_process_cwd(u32::MAX - 1).is_none());
    }
}
