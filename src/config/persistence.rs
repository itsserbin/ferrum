use std::fs;
use std::path::PathBuf;

use super::AppConfig;

/// Returns the platform-specific base config directory.
///
/// Resolution order:
/// 1. `XDG_CONFIG_HOME`
/// 2. `$HOME/.config`
/// 3. `%USERPROFILE%/.config`
pub(crate) fn config_base_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg));
    }
    if let Some(home) = std::env::var_os("HOME") {
        return Some(PathBuf::from(home).join(".config"));
    }
    std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".config"))
}

/// Returns the path to `~/.config/ferrum/config.ron`.
fn config_path() -> Option<PathBuf> {
    config_base_dir().map(|base| base.join("ferrum").join("config.ron"))
}

/// Loads the config from disk, falling back to defaults on any error.
pub(crate) fn load_config() -> AppConfig {
    let Some(path) = config_path() else {
        return AppConfig::default();
    };
    let Ok(contents) = fs::read_to_string(&path) else {
        return AppConfig::default();
    };
    ron::from_str(&contents).unwrap_or_else(|e| {
        eprintln!("[ferrum] Failed to parse config: {e}");
        AppConfig::default()
    })
}

/// Persists the config to disk. Errors are silently ignored.
pub(crate) fn save_config(config: &AppConfig) {
    let Some(path) = config_path() else {
        return;
    };
    let Some(dir) = path.parent() else {
        return;
    };
    if fs::create_dir_all(dir).is_err() {
        return;
    }
    let pretty = ron::ser::PrettyConfig::default();
    let Ok(serialized) = ron::ser::to_string_pretty(config, pretty) else {
        return;
    };
    fs::write(path, serialized).ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_config_returns_default_when_no_file() {
        // Point XDG_CONFIG_HOME at a guaranteed non-existent path so that
        // load_config() falls back to AppConfig::default().
        // SAFETY: test-only mutation; this test is not run concurrently with
        // other env-reading tests (single-threaded by default in Rust tests).
        unsafe { std::env::set_var("XDG_CONFIG_HOME", "/nonexistent/ferrum-test-path") };
        let config = load_config();
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        assert_eq!(config.font.size, 14.0);
        assert_eq!(config.terminal.max_scrollback, 30_000);
    }

    #[test]
    fn config_base_dir_returns_some() {
        // On most systems HOME or USERPROFILE is set.
        let dir = config_base_dir();
        assert!(dir.is_some(), "config_base_dir should return Some on dev machines");
    }
}
