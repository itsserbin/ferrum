//! Utilities for formatting filesystem paths for display in tab titles.
//!
//! Replaces the home directory prefix with `~`, then truncates middle
//! segments with `...` when the path exceeds the available character count.

#[cfg(not(target_os = "macos"))]
/// Formats a CWD path for display in a tab title.
///
/// - Replaces home directory prefix with `~`
/// - If the result exceeds `max_chars`, collapses middle segments to `...`
/// - If still too long, collapses beginning segments
/// - If even the last segment doesn't fit, returns `fallback`
pub fn format_tab_path(path: &str, max_chars: usize, fallback: &str) -> String {
    if max_chars == 0 {
        return fallback.to_string();
    }

    let display = replace_home_prefix(path);

    // Fast path: fits as-is
    if display.chars().count() <= max_chars {
        return display;
    }

    let sep = std::path::MAIN_SEPARATOR;
    let segments: Vec<&str> = display.split(sep).filter(|s| !s.is_empty()).collect();
    let last = match segments.last() {
        Some(s) => *s,
        None => return fallback.to_string(),
    };

    // If even the last segment doesn't fit, return fallback
    if last.chars().count() > max_chars {
        return fallback.to_string();
    }

    // Determine prefix: ~ or / or empty
    let prefix = if display.starts_with('~') {
        "~"
    } else if display.starts_with(sep) {
        &display[..sep.len_utf8()]
    } else {
        ""
    };

    // Try keeping last N segments with ... prefix
    // Start from all segments and remove from middle
    let skip_prefix = if display.starts_with('~') { 1 } else { 0 };
    let content_segments = &segments[skip_prefix..];

    for keep_end in (1..=content_segments.len()).rev() {
        let end_segs = &content_segments[content_segments.len() - keep_end..];
        let sep_str = &sep.to_string();
        let joined = end_segs.join(sep_str);
        let candidate = if prefix.is_empty() {
            format!("...{sep}{joined}")
        } else {
            format!("{prefix}{sep}...{sep}{joined}")
        };
        if candidate.chars().count() <= max_chars {
            return candidate;
        }
    }

    // Just the last segment
    last.to_string()
}

/// Replaces the user's home directory prefix with `~`.
pub fn replace_home_prefix(path: &str) -> String {
    let home = home_dir();
    if home.is_empty() {
        return path.to_string();
    }
    if let Some(rest) = path.strip_prefix(&home) {
        if rest.is_empty() {
            "~".to_string()
        } else {
            format!("~{rest}")
        }
    } else {
        path.to_string()
    }
}

fn home_dir() -> String {
    #[cfg(unix)]
    {
        std::env::var("HOME").unwrap_or_default()
    }
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").unwrap_or_default()
    }
    #[cfg(not(any(unix, windows)))]
    {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn short_path_no_truncation() {
        let result = format_tab_path("/tmp/foo", 30, "#1");
        // /tmp/foo is <=30 chars, so no truncation. Home prefix replacement may apply.
        assert!(!result.contains("..."));
    }

    #[test]
    fn path_outside_home_unchanged() {
        let result = replace_home_prefix("/etc/nginx");
        assert_eq!(result, "/etc/nginx");
    }

    #[test]
    fn home_prefix_replaced() {
        let home = home_dir();
        if home.is_empty() {
            return; // Skip on platforms without HOME
        }
        let path = format!("{}/projects/ferrum", home);
        let result = replace_home_prefix(&path);
        assert!(result.starts_with('~'), "Expected ~ prefix, got: {result}");
        assert!(result.contains("projects"));
    }

    #[test]
    fn home_dir_itself() {
        let home = home_dir();
        if home.is_empty() {
            return;
        }
        let result = replace_home_prefix(&home);
        assert_eq!(result, "~");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn long_path_gets_truncated() {
        // Create a path that's definitely longer than 20 chars
        let home = home_dir();
        if home.is_empty() {
            return;
        }
        let path = format!("{}/aaa/bbb/ccc/ddd/eee/target", home);
        let result = format_tab_path(&path, 20, "#1");
        assert!(result.contains("..."), "Expected ... in: {result}");
        assert!(result.ends_with("target"), "Expected 'target' at end: {result}");
        assert!(result.chars().count() <= 20, "Too long: {result}");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn very_narrow_returns_last_segment() {
        let result = format_tab_path("/very/long/path/to/mydir", 5, "#1");
        assert_eq!(result, "mydir");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn extremely_narrow_returns_fallback() {
        let result = format_tab_path("/very/long/path/to/extremely_long_dirname", 3, "#1");
        assert_eq!(result, "#1");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn zero_max_chars_returns_fallback() {
        let result = format_tab_path("/some/path", 0, "#2");
        assert_eq!(result, "#2");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn root_path() {
        let result = format_tab_path("/", 10, "#1");
        // "/" has no segments to truncate
        assert!(!result.is_empty());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn single_segment_path() {
        let result = format_tab_path("/usr", 30, "#1");
        assert_eq!(result, "/usr");
    }
}
