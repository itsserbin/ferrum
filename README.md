# Ferrum

GPU-accelerated terminal emulator written in Rust.
Cross-platform: macOS (Apple Silicon + Intel), Linux, Windows.

Inspired by [Ghostty](https://ghostty.org). Assisted by [YouScan](https://youscan.io).

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

- **GPU/CPU rendering** — wgpu compute shader renders the terminal grid on the GPU; automatically falls
  back to software rendering if no GPU is available
- **Correct colors** — non-sRGB surface ensures palette and ANSI colors render without gamma distortion;
  what you configure is what you see
- **Scrollback survives resize** — logical lines are reflowed on width change; content is never lost
  when you resize the terminal
- **Pane splitting** — binary tree layout, four directions (left/right/up/down), drag-to-resize dividers
  with live reflow, spatial keyboard navigation
- **Tab CWD titles** — tab titles update as you navigate the filesystem; works with any shell via OS API
  (proc_pidinfo on macOS, /proc on Linux), no shell integration required
- **Shell integration** — optional OSC 7 scripts for zsh, bash, fish, PowerShell; auto-injected at
  startup, no manual setup
- **Tab detach** — drag a tab out of the bar to tear it off into a standalone window; the new window
  follows the cursor without release-and-re-grab
- **Always-on-top** — pin a window to float above all other apps
- **Windows Unix aliases** — `ls`, `grep`, `cat`, `rm`, `find` and more work in cmd.exe out of the box;
  embedded in the binary, no external tools needed
- **Built-in security** — paste injection detection, OSC title query blocking, cursor spoofing and mouse
  mode leak prevention

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
