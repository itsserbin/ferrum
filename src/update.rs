//! Background release update checker.
//!
//! Fetches the latest GitHub release metadata, compares it to the current
//! `CARGO_PKG_VERSION`, and caches the API result for 24 hours.
//! Failures are non-fatal and silently ignored.

use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/itsserbin/ferrum/releases/latest";
const CACHE_TTL: Duration = Duration::from_secs(60 * 60 * 24);

/// Latest release info that should be shown to the user.
#[derive(Clone, Debug)]
pub(crate) struct AvailableRelease {
    /// Tag name reported by GitHub (for example: `v0.1.0`).
    pub(crate) tag_name: String,
    /// Browser URL of the release page.
    pub(crate) html_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateCache {
    checked_at_unix: u64,
    tag_name: String,
    html_url: String,
}

/// Spawns a detached background thread that checks for a newer release.
///
/// If a newer version is found, it sends one `AvailableRelease` to `tx`.
pub(crate) fn spawn_update_checker(tx: mpsc::Sender<AvailableRelease>) {
    let _ = std::thread::Builder::new()
        .name("release-update-checker".to_string())
        .spawn(move || {
            if let Some(release) = check_for_update() {
                let _ = tx.send(release);
            }
        });
}

/// Checks for a newer release using cache-first strategy.
///
/// Uses cached data when fresh, falls back to network otherwise, and if
/// network fails, falls back to stale cache.
fn check_for_update() -> Option<AvailableRelease> {
    let current_version = parse_version(env!("CARGO_PKG_VERSION"))?;
    let now_unix = unix_now_secs()?;

    let cached = read_cache();
    if let Some(entry) = cached.as_ref()
        && cache_is_fresh(entry, now_unix)
    {
        return available_release_if_newer(entry, &current_version);
    }

    if let Some(latest) = fetch_latest_release(now_unix) {
        write_cache(&latest);
        return available_release_if_newer(&latest, &current_version);
    }

    cached
        .as_ref()
        .and_then(|entry| available_release_if_newer(entry, &current_version))
}

fn fetch_latest_release(now_unix: u64) -> Option<UpdateCache> {
    let user_agent = format!("ferrum/{}", env!("CARGO_PKG_VERSION"));
    let mut response = ureq::get(GITHUB_LATEST_RELEASE_URL)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", &user_agent)
        .call()
        .ok()?;
    let release: GithubRelease = response.body_mut().read_json().ok()?;
    Some(UpdateCache {
        checked_at_unix: now_unix,
        tag_name: release.tag_name,
        html_url: release.html_url,
    })
}

fn available_release_if_newer(cache: &UpdateCache, current: &Version) -> Option<AvailableRelease> {
    let latest = parse_version(&cache.tag_name)?;
    (latest > *current).then(|| AvailableRelease {
        tag_name: cache.tag_name.clone(),
        html_url: cache.html_url.clone(),
    })
}

fn parse_version(raw: &str) -> Option<Version> {
    let normalized = raw.trim().trim_start_matches('v');
    Version::parse(normalized).ok()
}

fn cache_is_fresh(cache: &UpdateCache, now_unix: u64) -> bool {
    now_unix.saturating_sub(cache.checked_at_unix) < CACHE_TTL.as_secs()
}

fn read_cache() -> Option<UpdateCache> {
    let path = cache_path()?;
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_cache(cache: &UpdateCache) {
    let Some(path) = cache_path() else {
        return;
    };
    let Some(dir) = path.parent() else {
        return;
    };
    if fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(json) = serde_json::to_vec(cache) else {
        return;
    };
    let _ = fs::write(path, json);
}

fn cache_path() -> Option<PathBuf> {
    let base = crate::config::config_base_dir()?;
    Some(base.join("ferrum").join("update-check.json"))
}

fn unix_now_secs() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}
