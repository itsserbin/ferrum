# Release Notes Formatter — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Automate release body formatting so download files are grouped by OS with tables and direct links.

**Architecture:** Single GitHub Actions workflow (`release-notes.yml`) triggers after each release-related workflow completes, checks if all assets are present, then rewrites the release body with OS-grouped markdown tables. Idempotent — safe to run multiple times.

**Tech Stack:** GitHub Actions, bash, `gh` CLI, `jq`

---

## Context

### Existing Workflows (DO NOT MODIFY — auto-generated or manually managed)
- `.github/workflows/release.yml` — cargo-dist: builds archives + checksums, creates GitHub Release
- `.github/workflows/macos-dmg.yml` — builds DMG installers (triggers after Release)
- `.github/workflows/linux-packages.yml` — builds .deb and .rpm (triggers on tag push)
- `.github/workflows/windows-installer.yml` — builds Inno Setup .exe (triggers after Release)

### Workflow Names (exact, for `workflow_run` matching)
- `Release`
- `Publish macOS DMG`
- `Linux Packages`
- `Windows Installer`

### Expected Assets (all must be present before updating body)
These are the asset filename patterns to check:
- `*-aarch64-apple-darwin.dmg` — macOS ARM DMG
- `*-x86_64-apple-darwin.dmg` — macOS Intel DMG
- `*-Setup-x64.exe` — Windows installer
- `*_amd64.deb` — Debian package
- `*.x86_64.rpm` — RPM package
- `*-aarch64-apple-darwin.zip` — macOS ARM portable
- `*-x86_64-apple-darwin.zip` — macOS Intel portable
- `*-x86_64-pc-windows-msvc.zip` — Windows portable
- `*-x86_64-unknown-linux-gnu.zip` — Linux portable

### Target Release Body Format
Match the existing v0.2.0 body (see `gh release view v0.2.0 --json body`). Key sections:
1. Install (Homebrew)
2. Download > macOS (DMG table)
3. Download > Windows (installer table)
4. Download > Linux (deb/rpm table)
5. Portable Archives (all zips + source with sha256 links)

### Repo info for download URLs
Pattern: `https://github.com/$GITHUB_REPOSITORY/releases/download/$TAG/<filename>`

---

## Task 1: Create the workflow file

**Files:**
- Create: `.github/workflows/release-notes.yml`

**Step 1: Write the workflow file**

Create `.github/workflows/release-notes.yml` with the complete workflow. Key elements:

```yaml
name: Release Notes

on:
  workflow_run:
    workflows: ["Release", "Publish macOS DMG", "Windows Installer", "Linux Packages"]
    types: [completed]
  workflow_dispatch:
    inputs:
      tag:
        description: "Release tag (e.g. v0.2.0)"
        required: true

permissions:
  contents: write
```

The single job should:

1. **Determine the tag:**
   - From `workflow_dispatch`: use `inputs.tag`
   - From `workflow_run`: use `github.event.workflow_run.head_branch`
   - Skip if tag doesn't start with `v`

2. **Validate release exists:**
   ```bash
   gh release view "$TAG" --json tagName > /dev/null 2>&1
   ```
   Exit 0 if release doesn't exist yet.

3. **Fetch assets and check completeness:**
   ```bash
   ASSETS=$(gh release view "$TAG" --json assets --jq '.assets[].name')
   ```
   Check all 9 required patterns. Exit 0 if any missing (next trigger will complete).

4. **Build download URLs:**
   Base URL: `https://github.com/${GITHUB_REPOSITORY}/releases/download/${TAG}`
   Find exact filenames from asset list using grep patterns.

5. **Generate markdown body** matching existing v0.2.0 format:
   - `## Install` section with Homebrew command
   - `## Download` with subsections: `### 🍎 macOS`, `### 🪟 Windows`, `### 🐧 Linux`
   - `### 📦 Portable Archives` table with sha256 links
   - Use exact asset names from the fetched list (important for versioned filenames like `ferrum_0.2.0-1_amd64.deb`)

6. **Update release body:**
   ```bash
   gh release edit "$TAG" --notes "$BODY"
   ```

**Implementation notes:**
- Use `jq` to extract asset names from JSON
- Use `grep -m1` to find exact filenames from asset list (handles versioned names like deb/rpm)
- Write body to a temp file, then use `--notes-file` to avoid quoting issues
- The workflow is idempotent — running it multiple times produces the same result
- `workflow_run` only fires on `completed` (not `success`), so check `workflow_run.conclusion == 'success'` OR skip that check since we validate assets anyway

**Step 2: Validate the workflow YAML locally**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release-notes.yml'))" && echo "Valid YAML"
```
Expected: `Valid YAML`

If `pyyaml` not available:
```bash
ruby -ryaml -e "YAML.load_file('.github/workflows/release-notes.yml')" && echo "Valid YAML"
```

**Step 3: Commit**

```bash
git add .github/workflows/release-notes.yml
git commit -m "ci: add release-notes workflow to format release body by OS"
```

---

## Task 2: Test against existing release

**Step 1: Run the workflow manually against v0.2.0**

```bash
gh workflow run release-notes.yml -f tag=v0.2.0
```

**Step 2: Watch the run**

```bash
gh run list --workflow=release-notes.yml --limit 1
gh run watch <run-id>
```

**Step 3: Verify the release body was updated correctly**

```bash
gh release view v0.2.0 --json body --jq '.body'
```

Verify it matches the expected format with all OS sections and correct links.

**Step 4: If issues found — fix, commit, re-run**

Iterate until the body matches expectations. Keep commits atomic.
