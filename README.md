# Ferrum

Ferrum is a GPU-accelerated terminal emulator written in Rust.

## Install

### Homebrew (macOS/Linux)

```bash
brew tap itsserbin/homebrew-tap
brew install ferrum
```

Or install directly without pre-tapping:

```bash
brew install itsserbin/homebrew-tap/ferrum
```

### GitHub Releases (all platforms)

Download the latest artifacts from:

- https://github.com/itsserbin/ferrum/releases/latest

Expected release files:

- `Ferrum-x86_64-unknown-linux-gnu.zip`
- `ferrum_<version>_amd64.deb`
- `ferrum-<version>.x86_64.rpm`
- `Ferrum-x86_64-pc-windows-msvc.zip`
- `Ferrum-x86_64-pc-windows-msvc.msi`
- `ferrum-aarch64-apple-darwin.dmg` (macOS Apple Silicon)
- `ferrum-x86_64-apple-darwin.dmg` (macOS Intel)

### macOS DMG

Download the `.dmg` file for your architecture from [GitHub Releases](https://github.com/itsserbin/ferrum/releases/latest), open it, and drag Ferrum to Applications.

> **Note:** Since Ferrum is not signed with an Apple Developer certificate, macOS will show a warning on first launch. To open it: right-click the app, select Open, then click Open again. You only need to do this once.

To use `ferrum` from the terminal:

```bash
sudo ln -sf /Applications/Ferrum.app/Contents/MacOS/Ferrum /usr/local/bin/ferrum
```

### Linux packages

Debian/Ubuntu:

```bash
sudo dpkg -i ferrum_0.1.0_amd64.deb
```

Fedora/RHEL/openSUSE:

```bash
sudo rpm -i ferrum-0.1.0.x86_64.rpm
```

## Update checks

Ferrum performs a non-blocking background check against the GitHub Releases API and notifies when a newer release is available.

- URL: `https://api.github.com/repos/itsserbin/ferrum/releases/latest`
- Cache: `~/.config/ferrum/update-check.json`
- Cache TTL: 24 hours

## Build from source

```bash
cargo build --release
```
