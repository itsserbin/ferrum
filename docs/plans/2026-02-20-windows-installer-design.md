# Windows Installer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the WiX stub with a proper Inno Setup installer that supports per-user/per-machine installation, shortcuts, PATH integration, and a CI workflow to build it automatically on release.

**Architecture:** Standalone Inno Setup script compiled by a separate GitHub Actions workflow. The workflow triggers on release events, downloads the pre-built `ferrum.exe` from cargo-dist release assets, compiles the installer, and uploads the result back. WiX configuration is removed entirely.

**Tech Stack:** Inno Setup 6, GitHub Actions, PowerShell

---

### Task 1: Remove WiX configuration

**Files:**
- Delete: `wix/main.wxs`
- Delete: `wix/` (entire directory)
- Modify: `Cargo.toml:103-107` (remove `[package.metadata.wix]` section)

**Step 1: Delete the wix directory**

```bash
rm -rf wix/
```

**Step 2: Remove `[package.metadata.wix]` from Cargo.toml**

Remove these lines (103-107):

```toml
[package.metadata.wix]
upgrade-guid = "0DB9556E-D490-4400-991C-DE9B310B579A"
path-guid = "6FA019E8-2899-4DDD-BDB7-B269F5F1F937"
license = false
eula = false
```

**Step 3: Verify Cargo.toml is valid**

Run: `cargo metadata --format-version 1 > /dev/null`
Expected: exits 0, no errors

**Step 4: Commit**

```bash
git add -A && git commit -m "chore: remove WiX installer configuration"
```

---

### Task 2: Create Inno Setup script

**Files:**
- Create: `installer/ferrum.iss`

**Step 1: Create the installer directory**

```bash
mkdir -p installer
```

**Step 2: Write `installer/ferrum.iss`**

```iss
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

#define AppName "Ferrum"
#define AppExeName "ferrum.exe"
#define AppPublisher "itsserbin"
#define AppURL "https://github.com/itsserbin/ferrum"

[Setup]
AppId={{B4E7C8A1-5D3F-4E2A-9C6B-8F1D0E7A3B5C}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}/issues
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputBaseFilename=Ferrum-Setup-x64
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#AppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesEnvironment=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "startmenu"; Description: "Create Start Menu shortcut"; GroupDescription: "Shortcuts:";
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "addtopath"; Description: "Add to PATH"; GroupDescription: "System integration:";

[Files]
Source: "ferrum.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: startmenu
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Code]
procedure EnvAddPath(Path: string; IsSystem: Boolean);
var
  Paths: string;
  RootKey: Integer;
begin
  if IsSystem then
    RootKey := HKLM
  else
    RootKey := HKCU;

  if IsSystem then
    RegQueryStringValue(RootKey, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', Paths)
  else
    RegQueryStringValue(RootKey, 'Environment', 'Path', Paths);

  if Pos(';' + Uppercase(Path) + ';', ';' + Uppercase(Paths) + ';') > 0 then
    exit;

  Paths := Paths + ';' + Path;

  if IsSystem then
    RegWriteExpandStringValue(RootKey, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', Paths)
  else
    RegWriteExpandStringValue(RootKey, 'Environment', 'Path', Paths);
end;

procedure EnvRemovePath(Path: string; IsSystem: Boolean);
var
  Paths: string;
  P: Integer;
  RootKey: Integer;
begin
  if IsSystem then
    RootKey := HKLM
  else
    RootKey := HKCU;

  if IsSystem then begin
    if not RegQueryStringValue(RootKey, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', Paths) then
      exit;
  end else begin
    if not RegQueryStringValue(RootKey, 'Environment', 'Path', Paths) then
      exit;
  end;

  P := Pos(';' + Uppercase(Path), Uppercase(Paths));
  if P = 0 then begin
    P := Pos(Uppercase(Path) + ';', Uppercase(Paths));
    if P = 0 then
      exit;
    Delete(Paths, P, Length(Path) + 1);
  end else
    Delete(Paths, P, Length(Path) + 1);

  if IsSystem then
    RegWriteExpandStringValue(RootKey, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', Paths)
  else
    RegWriteExpandStringValue(RootKey, 'Environment', 'Path', Paths);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and IsTaskSelected('addtopath') then
    EnvAddPath(ExpandConstant('{app}'), IsAdminInstallMode());
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then begin
    EnvRemovePath(ExpandConstant('{app}'), True);
    EnvRemovePath(ExpandConstant('{app}'), False);
  end;
end;
```

**Step 3: Commit**

```bash
git add installer/ferrum.iss && git commit -m "feat: add Inno Setup installer script"
```

---

### Task 3: Create CI workflow

**Files:**
- Create: `.github/workflows/windows-installer.yml`

**Step 1: Write `.github/workflows/windows-installer.yml`**

Pattern follows `macos-dmg.yml`: triggers on `release: [published, prereleased]`, downloads pre-built binary from release assets.

```yaml
name: Windows Installer

on:
  release:
    types: [published, prereleased]

permissions:
  contents: write

jobs:
  build-installer:
    name: Build Inno Setup installer
    runs-on: windows-2022
    env:
      GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
    steps:
      - uses: actions/checkout@v6
        with:
          ref: ${{ github.event.release.tag_name }}
          persist-credentials: false

      - name: Download Windows binary from release
        shell: pwsh
        run: |
          $tag = "${{ github.event.release.tag_name }}"
          $zipName = "Ferrum-x86_64-pc-windows-msvc.zip"
          gh release download $tag --pattern $zipName --dir .
          Expand-Archive -Path $zipName -DestinationPath extracted

      - name: Install Inno Setup
        shell: pwsh
        run: choco install innosetup -y --no-progress

      - name: Build installer
        shell: pwsh
        run: |
          $tag = "${{ github.event.release.tag_name }}"
          $version = $tag.TrimStart('v')
          Copy-Item extracted\ferrum.exe installer\ferrum.exe
          & "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" /DAppVersion=$version installer\ferrum.iss

      - name: Upload installer to release
        shell: pwsh
        run: |
          $tag = "${{ github.event.release.tag_name }}"
          gh release upload $tag installer\Output\Ferrum-Setup-x64.exe --clobber
```

**Step 2: Validate YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/windows-installer.yml'))"`
Expected: exits 0, no errors

**Step 3: Commit**

```bash
git add .github/workflows/windows-installer.yml && git commit -m "ci: add Windows installer workflow (Inno Setup)"
```

---

### Task 4: Update design doc trigger strategy

**Files:**
- Modify: `docs/plans/2026-02-20-windows-installer-design.md:55`

**Step 1: Update the trigger description in the design doc**

Change trigger from `on: push: tags: 'v*'` to `on: release: types: [published, prereleased]` to match actual implementation (follows macos-dmg.yml pattern, no wait loop needed).

**Step 2: Commit all remaining changes**

```bash
git add -A && git commit -m "docs: update design doc to match implementation"
```
