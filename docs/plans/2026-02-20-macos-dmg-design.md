# Professional macOS DMG Installer

**Date:** 2026-02-20
**Status:** Approved

## Problem

The current `macos-dmg.yml` workflow creates a basic DMG with issues:
- Info.plist has extra indentation from heredoc (inline in YAML)
- No app icon (CFBundleIconFile missing) — generic macOS icon shown
- No Retina support (NSHighResolutionCapable missing)
- No app category (LSApplicationCategoryType missing)
- Plain DMG without custom background or icon positioning
- No `.icns` icon file exists in the repo

The DMG was also not present on v0.1.0 release because `macos-dmg.yml` was added after the release was published.

## Solution

Use `create-dmg` CLI tool to build a professional DMG with custom background, icon positioning, and volume icon. Fix the `.app` bundle to include proper Info.plist and macOS icon.

No Apple Developer Program ($100/year) required. Users will see a Gatekeeper warning on first launch, bypassed via Right-click → Open.

## Architecture: File Organization

```
assets/
  fonts/                    # (existing)
  icon.ico                  # Windows icon (existing)
  AppIcon.icns              # macOS app icon (NEW - convert from ico)
  dmg-background.png        # DMG window background (NEW - 660x400 or 1320x800 Retina)
installer/
  windows/
    ferrum.iss              # Windows Inno Setup (MOVE from installer/ferrum.iss)
  macos/
    Info.plist.template     # macOS app bundle plist template (NEW)
```

Rationale: `assets/` for images/resources, `installer/` for platform-specific configs/templates.

## .app Bundle

### Info.plist.template

Template with `${VERSION}` placeholder, replaced at build time. Key additions vs current:

| Key | Value | Purpose |
|-----|-------|---------|
| CFBundleIconFile | AppIcon | References AppIcon.icns |
| NSHighResolutionCapable | true | Retina display support |
| LSApplicationCategoryType | public.app-category.developer-tools | Launchpad categorization |

### App bundle structure

```
Ferrum.app/
  Contents/
    MacOS/
      Ferrum                # release binary
    Resources/
      AppIcon.icns          # copied from assets/
    Info.plist              # generated from template
```

## DMG Creation

### Tool: create-dmg

Open-source CLI tool (10k+ GitHub stars), installed via `brew install create-dmg` on CI runner.

### Parameters

```bash
create-dmg \
  --volname "Ferrum" \
  --volicon "assets/AppIcon.icns" \
  --background "assets/dmg-background.png" \
  --window-pos 200 120 \
  --window-size 660 400 \
  --icon-size 80 \
  --icon "Ferrum.app" 180 170 \
  --hide-extension "Ferrum.app" \
  --app-drop-link 480 170 \
  --no-internet-enable \
  "ferrum-${TARGET}.dmg" \
  "dist-macos/dmg/"
```

Result: Classic DMG layout with Ferrum.app on left, Applications shortcut on right, custom background.

## DMG Background

Simple minimalist design:
- Size: 660x400 (or 1320x800 for Retina @2x)
- Dark gradient matching terminal aesthetic
- Optional: subtle arrow between app icon and Applications folder
- No text needed (icons are self-explanatory)

## Workflow Changes (macos-dmg.yml)

1. Install `create-dmg` via brew
2. Build release binary (unchanged)
3. Create `.app` bundle using `installer/macos/Info.plist.template` (replaces inline heredoc)
4. Copy `assets/AppIcon.icns` to `.app/Contents/Resources/`
5. Run `create-dmg` with background, positioning, volume icon
6. Upload DMG to GitHub Release (unchanged)

## Gatekeeper UX

Without code signing, users see:
> "Ferrum.app" can't be opened because Apple cannot check it for malicious software.

Workaround: Right-click → Open → Open (or `xattr -d com.apple.quarantine Ferrum.app`)

Add instruction to README for macOS users.

## Assets to Create

1. **`assets/AppIcon.icns`** — Convert from `assets/icon.ico` using `iconutil`
2. **`assets/dmg-background.png`** — Minimalist dark background image
