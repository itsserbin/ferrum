# Ukrainian Localization Design

## Overview

Add a multilingual i18n system to Ferrum with Ukrainian as the first non-English locale. The system uses compile-time verified Rust structs for translations, auto-detects the OS language, and supports hot-reload without restarting the app.

## Requirements

- Multilingual architecture (EN + UK initially, extensible to any language)
- Auto-detect OS language on first launch, fallback to EN
- Language selector in Settings > Terminal tab
- Hot-reload: changing language applies immediately without restart
- Brand names (Ferrum, font names, theme names) are NOT translated
- All translations managed in one place (`src/i18n/`)

## Architecture

### Module Structure

```
src/i18n/
  mod.rs          — Locale enum, global state (RwLock), t() accessor, set_locale()
  translations.rs — Translations struct with ~60 &'static str fields
  en.rs           — fn english() -> Translations
  uk.rs           — fn ukrainian() -> Translations
  detect.rs       — OS language detection (LANG/LC_ALL on Unix, GetUserDefaultUILanguage on Windows)
```

### Key Types

- `Locale` enum: `En`, `Uk` — serde serializable as `"en"`, `"uk"`
- `Translations` struct: fields for every user-facing string, grouped by area (menu, dialogs, settings, security)
- Global: `static CURRENT: OnceLock<RwLock<&'static Translations>>`
- `t()` returns `&Translations` via RwLock read lock
- `set_locale(locale)` swaps the translation pointer, immediately effective

### Adding a New Language

1. Create `xx.rs` with `fn xxx() -> Translations`
2. Add variant to `Locale` enum
3. Compiler enforces all fields are filled — missing translations won't compile

## Config Integration

- New field `language: Locale` in `AppConfig`
- Default: `Locale::detect()` (reads OS language)
- Serialized as `"en"` / `"uk"` in JSON config

## UI Changes

### Settings Window (Terminal Tab)

- Add "Language" dropdown as the first element (before Max Scrollback)
- Options: "English", "Українська"
- On change: calls `i18n::set_locale()`, saves config, refreshes settings window

### Hot-Reload Behavior

- `set_locale()` updates `RwLock<&'static Translations>`
- All subsequent `t()` calls return new locale's strings
- Settings window re-reads labels on next render cycle
- Context menus, dialogs, security warnings use new locale on next display
- macOS native elements (tab bar, pin button) update via setTitle:/setToolTip: on next sync

## What Gets Translated (~60 strings)

- Context menu: Copy, Paste, Select All, Clear Selection, Split Right/Down/Left/Up, Close Pane, Clear Terminal, Reset Terminal, Rename, Duplicate, Close
- Close dialog: title, warning text, Close/Cancel buttons
- Settings: tab names (Font, Theme, Terminal, Layout, Security), all field labels, security descriptions, "Reset to Defaults"
- Security warnings: title, event descriptions
- macOS pin button: tooltips

## What Does NOT Get Translated

- "Ferrum" (brand)
- Font family names (JetBrains Mono, Fira Code, etc.)
- Theme names (Ferrum Dark, Ferrum Light)

## Testing

- Unit tests in `src/i18n/`: non-empty strings, set_locale/t() correctness, Locale::detect() with various LANG values, serde round-trip
- Compile-time guarantee: missing translation fields cause build failure
- ~5-8 tests total
