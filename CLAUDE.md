# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# Rules

- Never use #[allow(clippy::too_many_arguments)]
- Never use #[allow(clippy::too_many_lines)]
- Never use #[allow(dead_code)]

## Project Overview

Ferrum is a GPU-accelerated terminal emulator written in Rust. Cross-platform (Linux, macOS, Windows) with tab support, pane splitting, detachable windows, and dual CPU/GPU rendering backends.

## Build Commands

```bash
cargo build                         # Debug build with GPU renderer (default)
cargo build --no-default-features   # Debug build CPU-only (softbuffer)
cargo run                           # Run with GPU renderer
cargo run --no-default-features     # Run CPU-only
cargo test                          # All tests (353 as of v0.2.0)
cargo test core_terminal            # Run specific test module
cargo test osc7                     # Run tests matching pattern
cargo test -- --nocapture           # Tests with stdout
cargo clippy                        # Lint — must pass with zero warnings
```

**Features**: `gpu` (default) enables wgpu, pollster, bytemuck. Build without: `--no-default-features`.

## Architecture

### Data Flow
```
User Input → Event Handler (gui/events/) → PTY Writer / Terminal State
                                                    ↓
Display ← Renderer (CPU/GPU) ← Terminal Grid ← VT Parser ← PTY Output
```

### Module Structure

**`src/core/`** — Terminal emulation engine (GUI-independent)
- `terminal.rs` — Main state machine: cursor, scroll regions, mouse modes, alt-screen. `Perform` trait impl for vte parser.
- `terminal/handlers/*.rs` — VT escape sequence handlers (cursor, SGR, scroll, edit, erase, private modes, device reports)
- `grid.rs` — 2D cell array with soft/hard line wrap tracking
- `cell.rs` — Cell struct: character + colors + text attributes
- `selection.rs` — Text selection with char/word/line modes
- `color.rs` — Catppuccin Mocha palette, ANSI 16 colors, `dimmed()`/`bold_bright()` methods

**`src/pty/`** — PTY session management
- `mod.rs` — `Session` spawning (portable-pty), shell integration injection (bash, zsh, fish, PowerShell, cmd.exe)
- `cwd.rs` — Cross-platform OS API for querying process CWD by PID (Linux `/proc`, macOS `proc_pidinfo`, Windows stub)
- `shell-integration/` — Scripts that emit OSC 7 for CWD tracking
- `windows-aliases/` — Unix command aliases for cmd.exe (ls, cat, grep, etc.)

**`src/gui/`** — Multi-window tabbed interface
- `state.rs` — `App` (window manager), `FerrumWindow` (per-window), `TabState` (per-tab), `PtyEvent`, `WindowRequest`
- `pane.rs` — Binary tree pane architecture: `PaneNode` (Leaf|Split), `PaneLeaf` (terminal + session + selection), spatial layout and navigation
- `lifecycle/` — winit `ApplicationHandler`: window creation, event dispatch, PTY event draining, CWD polling
- `events/keyboard/` — Key handling: shortcuts, tab/pane navigation, rename editing, selection, clipboard
- `events/mouse/` — Click, drag, wheel, tab bar interactions, tab reorder animation, divider resize
- `events/render_shared.rs` — Shared frame rendering logic for both CPU/GPU paths (pane tree traversal, dividers, dimming)
- `events/menu_actions.rs` — Context menu action handlers
- `renderer/backend.rs` — `RendererBackend` enum dispatch: GPU-first, CPU fallback
- `renderer/traits.rs` — `Renderer` trait: metrics, rendering, hit testing
- `renderer/gpu/` — wgpu-based: compute shader for grid, render shader for UI, glyph atlas, composite pass
- `renderer/shared/` — Shared math: tab layout (`tab_math.rs`), hit testing, scrollbar, path display, overlay layout
- `tabs/` — Tab creation (shell spawn + PTY reader thread), close, reorder, rename
- `platform/macos.rs` — Native tab bar integration via objc2, pin button, toolbar

### Key Patterns

**Dual Renderer**: `Renderer` trait with GPU (wgpu) and CPU (softbuffer) impls. GPU tries first, falls back to CPU. Both use shared layout math from `renderer/shared/`.

**Multi-Window**: `App` manages `HashMap<WindowId, FerrumWindow>`. Each window independent: tabs, renderer, event state. On macOS, each "tab" is a native window in a tab group; on other platforms, tabs are drawn in a custom tab bar.

**Pane Tree**: `PaneNode` is a binary tree — `Leaf(PaneLeaf)` or `Split { first, second, direction, ratio }`. Each `PaneLeaf` owns a `Terminal`, `Session`, and selection state. `PaneLeaf::cwd()` checks OSC 7 first, then falls back to OS API.

**Terminal State Machine**: vte crate → `Perform` trait on `Terminal` → handler functions in `terminal/handlers/` → Grid cell updates → renderer reads grid.

**Platform Conditionals**: `#[cfg(target_os = "...")]` throughout. macOS: native decorations + tab bar (objc2). Windows: custom chrome, rounded corners, Unix aliases. Linux: GTK3 for file dialogs.

**CWD & Tab Titles**: Shells emit OSC 7 → `terminal.cwd` is set. For shells without integration, 1-second OS API polling updates CWD. Tab titles auto-show CWD (with `~/` home prefix) unless user explicitly renames (`is_renamed` flag). `compose_window_title()` drives macOS native tab bar; `build_tab_bar_state()` drives custom tab bar.

## Key Constants

In `gui/renderer/mod.rs`:
- `FONT_SIZE: 14.0`, `TAB_BAR_HEIGHT: 36` (non-macOS), `WINDOW_PADDING: 8`, `SCROLLBAR_WIDTH: 6`

In `gui/pane.rs`:
- `DIVIDER_WIDTH: 1`, `PANE_INNER_PADDING: 4`, `DIVIDER_HIT_ZONE: 6`

In `core/terminal.rs`:
- `MAX_SCROLLBACK: 1000`, Default TERM: `xterm-256color`

## Testing

Tests live in `tests/unit/` and inline `#[cfg(test)]` modules within source files:
- `core_terminal.rs` — Terminal emulation, escape sequences, OSC 7 parsing
- `core_security.rs` — Security event filtering
- `gui_tabs.rs` — Tab state transitions
- `gui_input.rs` — Input preprocessing
- `gui_events_keyboard_rename.rs` — Tab rename handling
- `pty/cwd.rs` — CWD query tests (inline)
- `renderer/shared/path_display.rs` — Path truncation tests (inline)
- `renderer/shared/tab_math.rs`, `scrollbar_math.rs`, `overlay_layout.rs`, `ui_layout.rs` — Layout math tests (inline)
- `pane.rs` — Pane tree split/close/navigate tests (inline)

## Cross-Platform Notes

- Cross-compilation from macOS fails for Linux/Windows due to `ring` crate needing target C compiler. Use native builds or CI.
- Windows: `parse_osc7_uri()` strips leading `/` before drive letter and normalizes slashes.
- macOS: Each tab is a separate native window in a tab group. Window title = tab title (via `sync_window_title`).
- All platforms: `home_dir()` in `path_display.rs` uses `HOME` (Unix) or `USERPROFILE` (Windows).
