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

- `Ferrum-aarch64-apple-darwin.zip`
- `Ferrum-x86_64-apple-darwin.zip`
- `Ferrum-x86_64-unknown-linux-gnu.zip`
- `ferrum_<version>_amd64.deb`
- `ferrum-<version>.x86_64.rpm`
- `Ferrum-x86_64-pc-windows-msvc.zip`
- `Ferrum-x86_64-pc-windows-msvc.msi`

macOS archive notes:

- Apple Silicon (M1/M2/M3): use `Ferrum-aarch64-apple-darwin.zip`
- Intel Mac: use `Ferrum-x86_64-apple-darwin.zip`
- Artifacts downloaded from the GitHub Actions UI are always wrapped in an outer `.zip` by GitHub.
- If macOS blocks launch (quarantine), run:

```bash
chmod +x ferrum
xattr -dr com.apple.quarantine ferrum
./ferrum
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
