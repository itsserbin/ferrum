# Professional macOS DMG Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a professional macOS DMG installer with custom background, icon positioning, and proper .app bundle — without Apple Developer Program.

**Architecture:** Reorganize installer configs into `installer/{platform}/`, create `Info.plist.template` for macOS, use `create-dmg` CLI in the existing `macos-dmg.yml` workflow. Assets (icons, DMG background) go in `assets/`.

**Tech Stack:** create-dmg (brew), hdiutil, iconutil, sips (macOS built-in), GitHub Actions

**Design doc:** `docs/plans/2026-02-20-macos-dmg-design.md`

---

### Task 1: Create macOS icon (AppIcon.icns)

**Files:**
- Create: `assets/AppIcon.icns`

**Step 1: Extract largest image from ICO and create iconset**

Run locally (macOS required):

```bash
cd /Users/itsserbin/PhpstormProjects/ferrum

# Create iconset directory with required sizes
mkdir -p /tmp/AppIcon.iconset

# Use sips to convert ico to png and create all required sizes
# First, extract the ico to a temporary png (sips can read ico)
sips -s format png assets/icon.ico --out /tmp/icon_source.png

# Create all required icon sizes for macOS
sips -z 16 16     /tmp/icon_source.png --out /tmp/AppIcon.iconset/icon_16x16.png
sips -z 32 32     /tmp/icon_source.png --out /tmp/AppIcon.iconset/icon_16x16@2x.png
sips -z 32 32     /tmp/icon_source.png --out /tmp/AppIcon.iconset/icon_32x32.png
sips -z 64 64     /tmp/icon_source.png --out /tmp/AppIcon.iconset/icon_32x32@2x.png
sips -z 128 128   /tmp/icon_source.png --out /tmp/AppIcon.iconset/icon_128x128.png
sips -z 256 256   /tmp/icon_source.png --out /tmp/AppIcon.iconset/icon_128x128@2x.png
sips -z 256 256   /tmp/icon_source.png --out /tmp/AppIcon.iconset/icon_256x256.png
sips -z 512 512   /tmp/icon_source.png --out /tmp/AppIcon.iconset/icon_256x256@2x.png
sips -z 512 512   /tmp/icon_source.png --out /tmp/AppIcon.iconset/icon_512x512.png
sips -z 1024 1024 /tmp/icon_source.png --out /tmp/AppIcon.iconset/icon_512x512@2x.png
```

Expected: PNG files in `/tmp/AppIcon.iconset/` at all required sizes.

**Step 2: Convert iconset to icns**

```bash
iconutil -c icns /tmp/AppIcon.iconset -o assets/AppIcon.icns
```

Expected: `assets/AppIcon.icns` file created.

**Step 3: Verify the icns file**

```bash
file assets/AppIcon.icns
```

Expected: Output contains "Mac OS X icon" or similar.

**Step 4: Clean up temp files**

```bash
rm -rf /tmp/AppIcon.iconset /tmp/icon_source.png
```

**Step 5: Commit**

```bash
git add assets/AppIcon.icns
git commit -m "feat(macos): add AppIcon.icns for app bundle"
```

---

### Task 2: Create DMG background image

**Files:**
- Create: `assets/dmg-background.png`

**Step 1: Generate a minimalist dark background**

Use `sips` or Python to create a 660x400 dark gradient PNG. If Python is available:

```bash
python3 -c "
from PIL import Image, ImageDraw
w, h = 660, 400
img = Image.new('RGB', (w, h))
draw = ImageDraw.Draw(img)
# Dark gradient from #1e1e2e (Catppuccin base) to #181825 (Catppuccin mantle)
for y in range(h):
    r = int(0x1e + (0x18 - 0x1e) * y / h)
    g = int(0x1e + (0x18 - 0x1e) * y / h)
    b = int(0x2e + (0x25 - 0x2e) * y / h)
    draw.line([(0, y), (w, y)], fill=(r, g, b))
img.save('assets/dmg-background.png')
print('Created 660x400 dark gradient background')
"
```

If Pillow not installed: `pip3 install Pillow` first, or use ImageMagick:

```bash
convert -size 660x400 gradient:'#1e1e2e'-'#181825' assets/dmg-background.png
```

Expected: `assets/dmg-background.png` exists, is 660x400 pixels.

**Step 2: Verify**

```bash
sips -g pixelWidth -g pixelHeight assets/dmg-background.png
```

Expected: pixelWidth 660, pixelHeight 400.

**Step 3: Commit**

```bash
git add assets/dmg-background.png
git commit -m "feat(macos): add DMG background image"
```

---

### Task 3: Create Info.plist.template

**Files:**
- Create: `installer/macos/Info.plist.template`

**Step 1: Create the directory and template file**

```bash
mkdir -p installer/macos
```

Write `installer/macos/Info.plist.template`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>Ferrum</string>
  <key>CFBundleDisplayName</key>
  <string>Ferrum</string>
  <key>CFBundleIdentifier</key>
  <string>com.ferrum.terminal</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundleExecutable</key>
  <string>Ferrum</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
</dict>
</plist>
```

**Step 2: Validate plist syntax**

```bash
plutil -lint installer/macos/Info.plist.template
```

Expected: "OK" (note: `${VERSION}` is a literal string so plutil will still parse it as valid XML).

**Step 3: Commit**

```bash
git add installer/macos/Info.plist.template
git commit -m "feat(macos): add Info.plist template with icon and Retina support"
```

---

### Task 4: Move Inno Setup script to installer/windows/

**Files:**
- Move: `installer/ferrum.iss` → `installer/windows/ferrum.iss`
- Modify: `installer/windows/ferrum.iss` (update relative path to icon)

**Step 1: Move the file**

```bash
mkdir -p installer/windows
git mv installer/ferrum.iss installer/windows/ferrum.iss
```

**Step 2: Update icon path in ferrum.iss**

The current line (line 23):
```
SetupIconFile=..\assets\icon.ico
```

Must change to (one more parent directory):
```
SetupIconFile=..\..\assets\icon.ico
```

**Step 3: Verify no other relative paths need updating**

Check `installer/windows/ferrum.iss` for any other `..` paths. The `Source: "ferrum.exe"` on line 42 is a flat reference (CI copies binary to working dir), so no change needed.

**Step 4: Commit**

```bash
git add installer/windows/ferrum.iss
git commit -m "refactor: move Inno Setup script to installer/windows/"
```

---

### Task 5: Update macos-dmg.yml workflow

**Files:**
- Modify: `.github/workflows/macos-dmg.yml`

**Step 1: Rewrite the workflow**

Replace the entire content of `.github/workflows/macos-dmg.yml` with:

```yaml
name: Publish macOS DMG

on:
  release:
    types: [published, prereleased]

permissions:
  contents: write

jobs:
  publish-macos-dmg:
    strategy:
      fail-fast: false
      matrix:
        include:
          - runner: "macos-14"
            target: "aarch64-apple-darwin"
          - runner: "macos-15-intel"
            target: "x86_64-apple-darwin"
    runs-on: ${{ matrix.runner }}
    env:
      GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      RUST_BACKTRACE: "1"
    steps:
      - uses: actions/checkout@v6
        with:
          ref: ${{ github.event.release.tag_name }}
          persist-credentials: false
          submodules: recursive

      - name: Install create-dmg
        run: brew install create-dmg

      - name: Build release binary
        run: |
          rustup target add "${{ matrix.target }}"
          cargo build --release --target "${{ matrix.target }}"

      - name: Package .app bundle
        shell: bash
        run: |
          set -euo pipefail
          TAG="${{ github.event.release.tag_name }}"
          VERSION="${TAG#v}"

          APP_ROOT="dist-macos/Ferrum.app/Contents"
          mkdir -p "$APP_ROOT/MacOS" "$APP_ROOT/Resources"

          cp "target/${{ matrix.target }}/release/ferrum" "$APP_ROOT/MacOS/Ferrum"
          chmod +x "$APP_ROOT/MacOS/Ferrum"

          cp assets/AppIcon.icns "$APP_ROOT/Resources/AppIcon.icns"

          sed "s/\${VERSION}/$VERSION/g" installer/macos/Info.plist.template > "$APP_ROOT/Info.plist"

      - name: Create DMG
        shell: bash
        run: |
          set -euo pipefail
          DMG_NAME="ferrum-${{ matrix.target }}.dmg"

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
            --hdiutil-retries 10 \
            "$DMG_NAME" \
            "dist-macos/Ferrum.app" \
            || true

          # create-dmg returns exit code 2 when it can't set icon positions
          # (common in CI). The DMG is still created successfully.
          if [ ! -f "$DMG_NAME" ]; then
            echo "::error::DMG creation failed"
            exit 1
          fi

          echo "DMG_NAME=$DMG_NAME" >> "$GITHUB_ENV"

      - name: Upload DMG to GitHub Release
        run: |
          gh release upload "${{ github.event.release.tag_name }}" "$DMG_NAME" --clobber
```

Key changes vs current:
1. Added `Install create-dmg` step
2. Split "Package .app and .dmg" into two steps: "Package .app bundle" and "Create DMG"
3. Info.plist now uses `sed` substitution from `installer/macos/Info.plist.template`
4. Copies `assets/AppIcon.icns` to `.app/Contents/Resources/`
5. Uses `create-dmg` instead of raw `hdiutil`
6. Added `--hdiutil-retries 10` for CI robustness
7. Handles `create-dmg` exit code 2 gracefully (icon positioning may fail in CI but DMG is still valid)
8. Source folder for create-dmg is `dist-macos/Ferrum.app` (create-dmg handles the Applications symlink via `--app-drop-link`)

**Step 2: Validate YAML syntax**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/macos-dmg.yml'))" && echo "YAML valid"
```

Expected: "YAML valid"

**Step 3: Commit**

```bash
git add .github/workflows/macos-dmg.yml
git commit -m "feat(macos): professional DMG with create-dmg, icon, and background"
```

---

### Task 6: Update README with macOS DMG install instructions

**Files:**
- Modify: `README.md`

**Step 1: Add macOS DMG section and Gatekeeper note**

After the "### GitHub Releases (all platforms)" section, add DMG files to the list. Also add a macOS Gatekeeper note.

In the expected release files list, add:
```markdown
- `ferrum-aarch64-apple-darwin.dmg` (macOS Apple Silicon)
- `ferrum-x86_64-apple-darwin.dmg` (macOS Intel)
```

After the Linux packages section, add:

```markdown
### macOS DMG

Download the `.dmg` file for your architecture from [GitHub Releases](https://github.com/itsserbin/ferrum/releases/latest), open it, and drag Ferrum to Applications.

> **Note:** Since Ferrum is not signed with an Apple Developer certificate, macOS will show a warning on first launch. To open it: right-click the app → Open → Open. You only need to do this once.
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add macOS DMG install instructions with Gatekeeper note"
```

---

### Task 7: Local verification (manual)

**Step 1: Test .app bundle locally**

```bash
cd /Users/itsserbin/PhpstormProjects/ferrum
cargo build --release

APP_ROOT="dist-macos/Ferrum.app/Contents"
mkdir -p "$APP_ROOT/MacOS" "$APP_ROOT/Resources"
cp target/release/ferrum "$APP_ROOT/MacOS/Ferrum"
chmod +x "$APP_ROOT/MacOS/Ferrum"
cp assets/AppIcon.icns "$APP_ROOT/Resources/AppIcon.icns"
sed "s/\${VERSION}/0.1.2/g" installer/macos/Info.plist.template > "$APP_ROOT/Info.plist"
```

Expected: `dist-macos/Ferrum.app` exists with proper structure.

**Step 2: Verify .app launches**

```bash
open dist-macos/Ferrum.app
```

Expected: Ferrum terminal opens. May show Gatekeeper warning (dismiss with Open).

**Step 3: Test DMG creation locally (requires create-dmg)**

```bash
brew install create-dmg

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
  "ferrum-test.dmg" \
  "dist-macos/Ferrum.app"
```

Expected: `ferrum-test.dmg` created. Open it — should see Ferrum.app on left, Applications on right, dark background.

**Step 4: Clean up test artifacts**

```bash
rm -rf dist-macos ferrum-test.dmg
```

**Step 5: Final commit if any fixes were needed**

Only if adjustments were required during testing.
