# Ferrum

Ferrum is a GPU-accelerated terminal emulator written in Rust.

## Install

### macOS

**Homebrew (recommended):**

```bash
brew install itsserbin/tap/ferrum
```

**DMG installer:**

Download the `.dmg` for your architecture from [GitHub Releases](https://github.com/itsserbin/ferrum/releases/latest):

| File | Architecture |
|------|--------------|
| `ferrum-aarch64-apple-darwin.dmg` | Apple Silicon (M1/M2/M3/M4) |
| `ferrum-x86_64-apple-darwin.dmg` | Intel |

Open the DMG and drag Ferrum to Applications.

> **macOS Sequoia+:** Since Ferrum is not notarized with an Apple Developer certificate, macOS will block the app on first launch. To fix this, run once in Terminal:
> ```bash
> xattr -cr /Applications/Ferrum.app
> ```
> Then open Ferrum normally. This is only needed once.

To use `ferrum` from the terminal:

```bash
sudo ln -sf /Applications/Ferrum.app/Contents/MacOS/Ferrum /usr/local/bin/ferrum
```

### Windows

Download the installer from [GitHub Releases](https://github.com/itsserbin/ferrum/releases/latest):

| File | Type |
|------|------|
| `Ferrum-Setup-x64.exe` | Installer (x64) |

### Linux

**Debian / Ubuntu:**

```bash
# Download .deb from GitHub Releases, then:
sudo dpkg -i ferrum_*_amd64.deb
```

**Fedora / RHEL:**

```bash
# Download .rpm from GitHub Releases, then:
sudo rpm -i ferrum-*.x86_64.rpm
```

**Homebrew:**

```bash
brew install itsserbin/tap/ferrum
```

## Update checks

Ferrum performs a non-blocking background check against the GitHub Releases API and notifies when a newer release is available.

- Cache: `~/.config/ferrum/update-check.json`
- Cache TTL: 24 hours

## Build from source

```bash
cargo build --release
```
