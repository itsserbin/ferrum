# Native macOS Settings Window — Design Document

## Goal

Replace the custom in-window settings overlay on macOS with a native AppKit settings window. Native NSWindow with NSToolbar categories, NSStepper+NSTextField for numeric values, NSPopUpButton for enums, and a Reset to Defaults button.

## Approach

Platform-conditional: on macOS, Cmd+, opens a native NSWindow. On Linux/Windows, the existing custom overlay remains.

Communication via `mpsc::channel<AppConfig>` — settings window sends updated config snapshots, main event loop applies them live via existing `apply_config_change()`.

Zero new dependencies — objc2, objc2-app-kit, objc2-foundation already in Cargo.toml.

## Architecture

```
Cmd+, on macOS
   │
   ▼
open_settings_window(AppConfig, Sender<AppConfig>)
   │
   ▼
NSWindow + NSToolbar (4 categories)
   │
   ├── Font tab:    Font Size (stepper), Font Family (popup), Line Padding (stepper)
   ├── Theme tab:   Theme (popup)
   ├── Terminal tab: Max Scrollback (stepper), Cursor Blink (stepper)
   └── Layout tab:  Window Padding, Tab Bar Height, Pane Padding, Scrollbar Width (steppers)
   │
   ▼
User changes value → target/action → Rust callback
   │
   ▼
Build new AppConfig → sender.send(config) → main event loop
   │
   ▼
apply_config_change() (existing) → live preview in terminal
   │
   ▼
Window close → save_config() (existing)
```

## UI Layout

NSToolbar with selectable items (macOS Settings style):

```
┌─── Ferrum Settings ─────────────────────────────┐
│  [Font]  [Theme]  [Terminal]  [Layout]          │  ← NSToolbar
├─────────────────────────────────────────────────┤
│                                                 │
│  Font Size           [-]  14.0  [+]            │  ← NSStepper + NSTextField
│                                                 │
│  Font Family         [JetBrains Mono       ▾]  │  ← NSPopUpButton
│                                                 │
│  Line Padding        [-]   2    [+]            │  ← NSStepper + NSTextField
│                                                 │
│                                                 │
│                    [Reset to Defaults]          │  ← NSButton
│                                                 │
└─────────────────────────────────────────────────┘
```

## Controls Per Setting

| Category | Setting | Control | Range | Step |
|----------|---------|---------|-------|------|
| Font | Font Size | NSStepper + NSTextField | 8.0 – 32.0 | 0.5 |
| Font | Font Family | NSPopUpButton | JetBrains Mono, Fira Code | — |
| Font | Line Padding | NSStepper + NSTextField | 0 – 10 | 1 |
| Theme | Theme | NSPopUpButton | Ferrum Dark, Catppuccin Latte | — |
| Terminal | Max Scrollback | NSStepper + NSTextField | 0 – 50000 | 100 |
| Terminal | Cursor Blink (ms) | NSStepper + NSTextField | 100 – 2000 | 50 |
| Layout | Window Padding | NSStepper + NSTextField | 0 – 32 | 1 |
| Layout | Tab Bar Height | NSStepper + NSTextField | 24 – 60 | 1 |
| Layout | Pane Padding | NSStepper + NSTextField | 0 – 16 | 1 |
| Layout | Scrollbar Width | NSStepper + NSTextField | 2 – 16 | 1 |

## Key Design Decisions

1. **NSToolbar with selectable items** (not NSTabView) — matches modern macOS Settings style
2. **NSStepper + NSTextField** for all numerics — precise, editable, consistent
3. **NSPopUpButton** for enums — standard macOS dropdown
4. **Reset to Defaults** button — creates `AppConfig::default()` and sends through channel
5. **Live preview** — every change immediately applies via channel
6. **No Cancel/Apply** — changes apply instantly (like current overlay behavior)
7. **Single instance** — only one settings window at a time (tracked via `settings_window_open: bool`)

## Files

### New
- `src/gui/platform/macos/settings.rs` — native settings window (NSWindow, controls, callbacks)

### Modified
- `src/gui/platform/macos/mod.rs` — re-export settings module
- `src/gui/state.rs` — add `settings_sender`/`settings_receiver` channel, `settings_window_open` flag
- `src/gui/events/keyboard/shortcuts.rs` — Cmd+, opens native window on macOS
- `src/gui/lifecycle/` — poll `settings_receiver` in event loop, apply changes

### Unchanged
- `src/config/model.rs` — AppConfig stays the same
- `src/gui/events/settings_apply.rs` — apply_config_change() reused as-is
- `src/config/persistence.rs` — save_config() reused as-is
- `src/gui/settings/` — custom overlay stays for Linux/Windows

## Non-Goals
- Linux/Windows native settings (future work)
- Animated transitions in the native window (AppKit handles this)
- Custom styling of AppKit controls (use system appearance)
