# Release Notes Formatting Design

## Problem
All release assets are dumped as a flat list in GitHub Releases. Users must scan 18+ files to find the right download for their OS.

## Solution
A separate GitHub Actions workflow (`release-notes.yml`) that automatically updates the release body with OS-grouped download sections after all assets are published.

## Trigger
- `workflow_run` on completion of: `Release`, `Publish macOS DMG`, `Windows Installer`
- Linux Packages workflow triggers on tag push, so its assets (.deb, .rpm) are waited for too.
- Each trigger checks if all expected assets are present. If not — exits silently (next trigger will complete it).

## Logic
1. Determine release tag from `workflow_run` event
2. Fetch asset list: `gh release view $TAG --json assets`
3. Verify all expected assets are present (dmg x2, exe, deb, rpm, zip x3)
4. If incomplete — exit 0 (next workflow_run will retry)
5. Generate markdown body with OS sections
6. Update: `gh release edit $TAG --notes "..."`

## Body Format
Sections: macOS, Windows, Linux, Checksums & Source.
Each section has a table with download links, architecture, and description.

## Files
- **Create:** `.github/workflows/release-notes.yml`
- **Do NOT modify:** `release.yml` (cargo-dist auto-generated), `macos-dmg.yml`, `linux-packages.yml`, `windows-installer.yml`, `Cargo.toml`
