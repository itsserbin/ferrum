# Ferrum

GPU-accelerated terminal emulator written in Rust. Cross-platform: macOS, Windows, Linux.

## Features

- **GPU rendering** — wgpu-based renderer with automatic CPU fallback (softbuffer)
- **Tabs** — multi-tab interface; native tab bar on macOS, custom on Windows/Linux
- **Pane splitting** — horizontal/vertical splits, arbitrary nesting, drag-to-resize dividers
- **Shell integration** — OSC 7 CWD tracking; bash, zsh, fish, PowerShell, cmd.exe
- **Tab titles** — auto-populated from CWD; renameable via double-click or right-click
- **Text selection** — character, word, and line modes via click/drag/double/triple-click
- **Scrollback** — 1000 lines; draggable scrollbar
- **Tab reordering** — drag-and-drop with animation
- **Detachable windows** — drag tab out of bar to open in new window (Windows/Linux)
- **Always-on-top** — pin window toggle
- **Context menus** — terminal area and tab bar (copy, paste, split, rename, duplicate, close)
- **Color scheme** — Catppuccin Mocha; xterm-256color, true color (24-bit)
- **Update checker** — non-blocking background check, 24h cache

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

## Install

### macOS

**Homebrew (recommended):**

```bash
brew install --cask itsserbin/tap/ferrum && xattr -cr /Applications/Ferrum.app
```

**DMG installer:**

Download from [GitHub Releases](https://github.com/itsserbin/ferrum/releases/latest):

| File | Architecture |
|------|--------------|
| `ferrum-aarch64-apple-darwin.dmg` | Apple Silicon (M1/M2/M3/M4) |
| `ferrum-x86_64-apple-darwin.dmg` | Intel |

Open the DMG, drag Ferrum to Applications. Ferrum is ad-hoc signed but not notarized — run once to remove macOS Gatekeeper's quarantine flag:

```bash
xattr -cr /Applications/Ferrum.app
```

To use `ferrum` from Terminal:

```bash
sudo ln -sf /Applications/Ferrum.app/Contents/MacOS/Ferrum /usr/local/bin/ferrum
```

### Windows

Download `Ferrum-Setup-x64.exe` from [GitHub Releases](https://github.com/itsserbin/ferrum/releases/latest).

### Linux

Download packages from [GitHub Releases](https://github.com/itsserbin/ferrum/releases/latest).

**Debian / Ubuntu:**

```bash
sudo dpkg -i ferrum_*_amd64.deb
```

**Fedora / RHEL:**

```bash
sudo rpm -i ferrum-*.x86_64.rpm
```

## Build from source

```bash
cargo build --release                        # GPU renderer (default)
cargo build --release --no-default-features  # CPU-only
```
