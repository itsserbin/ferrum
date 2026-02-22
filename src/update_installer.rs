//! Platform-specific binary replacement and relaunch logic.

use std::env;
use std::fs;

/// Spawns a background thread to download and install the new version.
/// On success, relaunches the app. Failures are logged and non-fatal.
pub(crate) fn spawn_installer(tag_name: &str) {
    let tag = tag_name.to_owned();
    let _ = std::thread::Builder::new()
        .name("ferrum-installer".into())
        .spawn(move || {
            if let Err(e) = run_install(&tag) {
                eprintln!("[update] install failed: {e}");
            }
        });
}

fn run_install(tag: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    return install_macos(tag);
    #[cfg(target_os = "windows")]
    return install_windows(tag);
    #[cfg(target_os = "linux")]
    return install_linux(tag);
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    anyhow::bail!("unsupported platform");
}

#[cfg(target_os = "macos")]
fn install_macos(tag: &str) -> anyhow::Result<()> {
    // Try Homebrew first.
    let brew_check = std::process::Command::new("brew")
        .args(["list", "--cask", "ferrum"])
        .output();
    if brew_check.is_ok_and(|o| o.status.success()) {
        let status = std::process::Command::new("brew")
            .args(["upgrade", "--cask", "ferrum"])
            .status()?;
        if status.success() {
            relaunch();
        }
        return Ok(());
    }
    // Fallback: download zip, replace binary, relaunch.
    // CI produces: ferrum-{aarch64-apple-darwin,x86_64-apple-darwin}.zip
    // The binary inside the zip is named "ferrum" (lowercase).
    let target = if cfg!(target_arch = "aarch64") {
        "aarch64-apple-darwin"
    } else {
        "x86_64-apple-darwin"
    };
    let url = format!(
        "https://github.com/itsserbin/ferrum/releases/download/{tag}/ferrum-{target}.zip"
    );
    let zip_bytes = download(&url)?;
    let new_bin = extract_binary_from_zip(&zip_bytes, "ferrum")?;
    replace_current_binary(&new_bin)?;
    relaunch();
}

#[cfg(target_os = "windows")]
fn install_windows(tag: &str) -> anyhow::Result<()> {
    // CI produces: Ferrum-Setup-x64.exe
    let url = format!(
        "https://github.com/itsserbin/ferrum/releases/download/{tag}/Ferrum-Setup-x64.exe"
    );
    let installer_bytes = download(&url)?;
    let tmp = env::temp_dir().join("ferrum-setup.exe");
    fs::write(&tmp, &installer_bytes)?;
    std::process::Command::new(&tmp)
        .args(["/VERYSILENT", "/CLOSEAPPLICATIONS", "/RESTARTAPPLICATIONS"])
        .spawn()?;
    std::process::exit(0);
}

#[cfg(target_os = "linux")]
fn install_linux(tag: &str) -> anyhow::Result<()> {
    // CI produces: ferrum-{x86_64,aarch64}-unknown-linux-gnu.zip
    // The binary inside the zip is named "ferrum".
    let target = if cfg!(target_arch = "aarch64") {
        "aarch64-unknown-linux-gnu"
    } else {
        "x86_64-unknown-linux-gnu"
    };
    let url = format!(
        "https://github.com/itsserbin/ferrum/releases/download/{tag}/ferrum-{target}.zip"
    );
    let archive_bytes = download(&url)?;
    let new_bin = extract_binary_from_zip(&archive_bytes, "ferrum")?;
    let current = env::current_exe()?;
    let needs_sudo = current.starts_with("/usr") || current.starts_with("/opt");
    if needs_sudo {
        let tmp = env::temp_dir().join("ferrum-new");
        fs::write(&tmp, &new_bin)?;
        let tmp_str = tmp
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid tmp path"))?;
        let current_str = current
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid current path"))?;
        let status = std::process::Command::new("pkexec")
            .args(["cp", tmp_str, current_str])
            .status()?;
        anyhow::ensure!(status.success(), "pkexec cp failed");
    } else {
        replace_current_binary(&new_bin)?;
    }
    relaunch();
}

fn download(url: &str) -> anyhow::Result<Vec<u8>> {
    let user_agent = format!("ferrum/{}", env!("CARGO_PKG_VERSION"));
    let bytes = ureq::get(url)
        .header("User-Agent", &user_agent)
        .call()?
        .body_mut()
        .read_to_vec()?;
    Ok(bytes)
}

fn replace_current_binary(new_bin: &[u8]) -> anyhow::Result<()> {
    let current = env::current_exe()?;
    let tmp = current.with_extension("new");
    fs::write(&tmp, new_bin)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
    }
    fs::rename(&tmp, &current)?;
    Ok(())
}

fn relaunch() -> ! {
    let exe = env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("ferrum"));
    let _ = std::process::Command::new(exe).spawn();
    std::process::exit(0);
}

fn extract_binary_from_zip(bytes: &[u8], name: &str) -> anyhow::Result<Vec<u8>> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name().ends_with(name) {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }
    anyhow::bail!("binary '{}' not found in zip", name)
}
