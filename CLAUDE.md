# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Ferrum is a GPU-accelerated terminal emulator written in Rust. It's cross-platform (Linux, macOS, Windows) with tab support, detachable windows, and dual CPU/GPU rendering backends.

## Build Commands

### Using Makefile (recommended)
```bash
make build       # Debug build with GPU (default)
make build-cpu   # Debug build with CPU-only
make release     # Release build with GPU
make release-cpu # Release build with CPU-only
make run         # Run with GPU
make run-cpu     # Run with CPU-only
make test        # Run all tests
make help        # Show all targets
```

### Using Cargo directly
```bash
cargo build                         # Build with GPU renderer (default)
cargo build --no-default-features   # Build CPU-only
cargo run                           # Run with GPU renderer
cargo run --no-default-features     # Run CPU-only
cargo test                          # All tests
cargo test core_terminal            # Run specific test module
cargo test -- --nocapture           # Tests with stdout
```

## Architecture

### Data Flow
```
User Input → Event Handler (gui/events/) → PTY Writer / Terminal State
                                                    ↓
Display ← Renderer (CPU/GPU) ← Terminal Grid ← VT Parser ← PTY Output
```

### Module Structure

**`src/core/`** - Terminal emulation engine (GUI-independent)
- `terminal.rs` - Main state machine: cursor, scroll regions, mouse modes, alt-screen
- `terminal/handlers/*.rs` - VT escape sequence handlers (cursor, SGR, scroll, edit, private modes)
- `grid.rs` - 2D cell array with soft/hard line wrap tracking
- `cell.rs` - Cell struct: character + colors + text attributes
- `selection.rs` - Text selection with char/word/line modes
- `color.rs` - Catppuccin Mocha palette, ANSI 16 colors

**`src/pty/`** - PTY session management
- `mod.rs` - Session spawning, resize, reader/writer handles
- Windows: Creates Unix command alias batch scripts (ls, cat, grep, etc.)

**`src/gui/`** - Multi-window tabbed interface
- `state.rs` - App (window manager), FerrumWindow (per-window state), TabState (per-tab terminal)
- `lifecycle.rs` - winit ApplicationHandler, event loop integration
- `events/keyboard/` - Key handling, shortcuts (Ctrl+N/T/W, etc.)
- `events/mouse/` - Click, drag, wheel, tab bar, context menu
- `events/pty.rs` - PTY data arrival handling
- `renderer/backend.rs` - Renderer trait dispatch (GPU-first, CPU fallback)
- `renderer/gpu/` - wgpu-based: compute shader for grid, render shader for UI, composite pass
- `renderer/cpu_render.rs` - softbuffer fallback
- `tabs/` - Tab creation, close, reorder, rename
- `platform/macos.rs` - Native tab bar integration (objc2)

### Key Patterns

**Dual Renderer Backend**: Tries GPU (wgpu), falls back to CPU (softbuffer). Trait-based dispatch in `renderer/backend.rs`.

**Multi-Window**: App-level window manager. Each window has independent renderer, tabs, event state. Tab detach/drag between windows supported.

**Terminal State Machine**: vte crate parses escape sequences → handlers update Terminal state → Grid cells updated → renderer displays.

**Platform Conditionals**: Use `#[cfg(target_os = "...")]` for platform-specific code. macOS has native tab integration; Windows has custom chrome and Unix command aliases.

## Key Constants

In `gui/renderer/mod.rs`:
- `FONT_SIZE: 15.0`
- `TAB_BAR_HEIGHT: 36` (non-macOS)
- `WINDOW_PADDING: 12` (Windows) / `8` (Unix)
- `SCROLLBAR_WIDTH: 6`

In `core/terminal.rs`:
- `MAX_SCROLLBACK: 1000`
- Default TERM: `xterm-256color`

## Testing

Tests live in `tests/unit/` and inline `#[cfg(test)]` modules:
- `core_terminal.rs` - Terminal emulation, escape sequences
- `core_security.rs` - Security event filtering
- `gui_tabs.rs` - Tab state transitions
- `gui_input.rs` - Input preprocessing
- `gui_events_keyboard_rename.rs` - Tab rename handling

## Features

- `gpu` (default) - Enables wgpu, pollster, bytemuck dependencies
- Build without: `--no-default-features`
