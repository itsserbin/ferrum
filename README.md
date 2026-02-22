# Ferrum

GPU-accelerated terminal emulator written in Rust. Inspired by [Ghostty](https://ghostty.org).

## Install

### macOS

**Homebrew (recommended):**

```bash
brew install --cask itsserbin/tap/ferrum && xattr -cr /Applications/Ferrum.app
```

**DMG installer:**

| Download | Architecture |
|----------|--------------|
| [Ferrum for Apple Silicon](https://github.com/itsserbin/ferrum/releases/latest/download/ferrum-aarch64-apple-darwin.dmg) | M1 / M2 / M3 / M4 |
| [Ferrum for Intel Mac](https://github.com/itsserbin/ferrum/releases/latest/download/ferrum-x86_64-apple-darwin.dmg) | x86_64 |

Open the DMG, drag Ferrum to Applications. Ferrum is ad-hoc signed but not notarized — run once to remove macOS Gatekeeper's quarantine flag:

```bash
xattr -cr /Applications/Ferrum.app
```

To use `ferrum` from Terminal:

```bash
sudo ln -sf /Applications/Ferrum.app/Contents/MacOS/Ferrum /usr/local/bin/ferrum
```

### Windows

[Download Ferrum for Windows (x64)](https://github.com/itsserbin/ferrum/releases/latest/download/Ferrum-Setup-x64.exe)

### Linux

Download `.deb` or `.rpm` from [GitHub Releases](https://github.com/itsserbin/ferrum/releases/latest).

**Debian / Ubuntu:**

```bash
sudo dpkg -i ferrum_*_amd64.deb
```

**Fedora / RHEL:**

```bash
sudo rpm -i ferrum-*.x86_64.rpm
```

## Features

- **GPU rendering** — wgpu-based with automatic CPU fallback; no GPU, no problem
- **Native macOS tab bar** — each tab is a real NSWindow in a native tab group, not a custom-drawn strip
- **Pane splitting** — binary tree layout, horizontal/vertical, arbitrary nesting, drag-to-resize dividers
- **Detachable windows** — drag a tab out of the bar to open it as a standalone window (Windows/Linux)

## Keyboard Shortcuts

### Tabs

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl+T` | New tab |
| `Cmd/Ctrl+W` | Close pane / tab / window |
| `Cmd/Ctrl+N` | New window |
| `Cmd/Ctrl+Tab` | Next tab |
| `Cmd/Ctrl+Shift+Tab` | Previous tab |
| `Cmd/Ctrl+1`…`9` | Jump to tab 1–9 |
| `Cmd/Ctrl+Shift+T` | Restore last closed tab |

### Panes

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl+Shift+R` | Split right |
| `Cmd/Ctrl+Shift+D` | Split down |
| `Cmd/Ctrl+Shift+L` | Split left |
| `Cmd/Ctrl+Shift+U` | Split up |
| `Cmd/Ctrl+Shift+↑↓←→` | Navigate between panes |

### Clipboard & Selection

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl+C` | Copy selection |
| `Cmd/Ctrl+V` | Paste |
| `Cmd/Ctrl+X` | Cut |
| `Shift+←/→` | Extend selection by character |

### UI

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl+,` | Settings |
| `Cmd/Ctrl+Shift+P` | Toggle always-on-top |
| `Cmd/Ctrl+↑` / `Cmd/Ctrl+↓` | Scroll to top / bottom |

## Build from source

```bash
cargo build --release                        # GPU renderer (default)
cargo build --release --no-default-features  # CPU-only
```
