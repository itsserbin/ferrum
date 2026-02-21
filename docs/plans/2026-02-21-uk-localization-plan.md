# Ukrainian Localization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a multilingual i18n system to Ferrum with Ukrainian as the first non-English locale, using compile-time verified Rust structs.

**Architecture:** New `src/i18n/` module with a `Translations` struct (~60 `&'static str` fields), per-locale `.rs` files, and a global `RwLock` for hot-reload. `Locale` enum added to `AppConfig`, auto-detected from OS on first run. Language selector in Settings > Terminal tab on all 3 platforms.

**Tech Stack:** Pure Rust (no external i18n crates). `std::sync::RwLock`, `std::sync::OnceLock`, serde for config serialization, `sys-locale` crate for OS language detection.

---

### Task 1: Create the `Translations` struct and `Locale` enum

**Files:**
- Create: `src/i18n/mod.rs`
- Create: `src/i18n/translations.rs`

**Step 1: Create `src/i18n/translations.rs`**

This struct has one `&'static str` field per translatable string. The compiler enforces completeness — any locale file missing a field won't compile.

```rust
/// All user-facing strings in the application.
///
/// Each locale module (`en.rs`, `uk.rs`, …) returns a `&'static Translations`
/// with every field populated. The compiler rejects missing fields.
pub struct Translations {
    // ── Context menu ─────────────────────────────────────────────────
    pub menu_copy: &'static str,
    pub menu_paste: &'static str,
    pub menu_select_all: &'static str,
    pub menu_clear_selection: &'static str,
    pub menu_split_right: &'static str,
    pub menu_split_down: &'static str,
    pub menu_split_left: &'static str,
    pub menu_split_up: &'static str,
    pub menu_close_pane: &'static str,
    pub menu_clear_terminal: &'static str,
    pub menu_reset_terminal: &'static str,
    pub menu_rename: &'static str,
    pub menu_duplicate: &'static str,
    pub menu_close: &'static str,

    // ── Close dialog ─────────────────────────────────────────────────
    pub dialog_close_title: &'static str,
    pub dialog_close_message: &'static str,
    pub dialog_close_confirm: &'static str,
    pub dialog_close_cancel: &'static str,

    // ── Settings window ──────────────────────────────────────────────
    pub settings_title: &'static str,
    pub settings_tab_font: &'static str,
    pub settings_tab_theme: &'static str,
    pub settings_tab_terminal: &'static str,
    pub settings_tab_layout: &'static str,
    pub settings_tab_security: &'static str,
    pub settings_reset: &'static str,

    // ── Font tab ─────────────────────────────────────────────────────
    pub font_size: &'static str,
    pub font_family: &'static str,
    pub font_line_padding: &'static str,

    // ── Theme tab ────────────────────────────────────────────────────
    pub theme_label: &'static str,

    // ── Terminal tab ─────────────────────────────────────────────────
    pub terminal_language: &'static str,
    pub terminal_max_scrollback: &'static str,
    pub terminal_cursor_blink: &'static str,

    // ── Layout tab ───────────────────────────────────────────────────
    pub layout_window_padding: &'static str,
    pub layout_pane_padding: &'static str,
    pub layout_scrollbar_width: &'static str,
    pub layout_tab_bar_height: &'static str,

    // ── Security tab ─────────────────────────────────────────────────
    pub security_mode: &'static str,
    pub security_disabled: &'static str,
    pub security_standard: &'static str,
    pub security_custom: &'static str,
    pub security_paste_protection: &'static str,
    pub security_paste_description: &'static str,
    pub security_block_title: &'static str,
    pub security_block_title_description: &'static str,
    pub security_limit_cursor: &'static str,
    pub security_limit_cursor_description: &'static str,
    pub security_clear_mouse: &'static str,
    pub security_clear_mouse_description: &'static str,

    // ── Security events / popup ──────────────────────────────────────
    pub security_popup_title: &'static str,
    pub security_event_paste: &'static str,
    pub security_event_title_query: &'static str,
    pub security_event_cursor_rewrite: &'static str,
    pub security_event_mouse_leak: &'static str,

    // ── macOS pin button ─────────────────────────────────────────────
    pub pin_window: &'static str,
    pub unpin_window: &'static str,
    pub pin_tooltip: &'static str,
    pub unpin_tooltip: &'static str,
    pub settings_tooltip: &'static str,

    // ── Update notification ──────────────────────────────────────────
    pub update_available: &'static str,
}
```

**Step 2: Create `src/i18n/mod.rs`**

```rust
mod translations;
mod en;
mod uk;
mod detect;

pub use translations::Translations;

use std::sync::{OnceLock, RwLock};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Locale {
    #[serde(rename = "en")]
    En,
    #[serde(rename = "uk")]
    Uk,
}

impl Locale {
    pub const ALL: &'static [Locale] = &[Locale::En, Locale::Uk];

    pub const DISPLAY_NAMES: &'static [&'static str] = &["English", "Українська"];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&v| v == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Self {
        Self::ALL.get(i).copied().unwrap_or(Locale::En)
    }

    pub fn translations(self) -> &'static Translations {
        match self {
            Locale::En => en::translations(),
            Locale::Uk => uk::translations(),
        }
    }

    pub fn detect() -> Self {
        detect::detect_locale()
    }
}

impl Default for Locale {
    fn default() -> Self {
        Self::detect()
    }
}

static CURRENT: OnceLock<RwLock<&'static Translations>> = OnceLock::new();

/// Returns the current translations.
pub fn t() -> &'static Translations {
    let lock = CURRENT.get_or_init(|| RwLock::new(Locale::En.translations()));
    *lock.read().unwrap_or_else(|e| e.into_inner())
}

/// Switches the active locale. All subsequent `t()` calls return the new strings.
pub fn set_locale(locale: Locale) {
    let lock = CURRENT.get_or_init(|| RwLock::new(Locale::En.translations()));
    let mut guard = lock.write().unwrap_or_else(|e| e.into_inner());
    *guard = locale.translations();
}
```

**Step 3: Verify it compiles (won't yet — en.rs/uk.rs missing)**

This step is completed implicitly in Task 2.

**Step 4: Commit**

```bash
git add src/i18n/mod.rs src/i18n/translations.rs
git commit -m "feat(i18n): add Translations struct and Locale enum with global accessor"
```

---

### Task 2: Create English and Ukrainian translation files

**Files:**
- Create: `src/i18n/en.rs`
- Create: `src/i18n/uk.rs`

**Step 1: Create `src/i18n/en.rs`**

```rust
use super::Translations;

static EN: Translations = Translations {
    // Context menu
    menu_copy: "Copy",
    menu_paste: "Paste",
    menu_select_all: "Select All",
    menu_clear_selection: "Clear Selection",
    menu_split_right: "Split Right",
    menu_split_down: "Split Down",
    menu_split_left: "Split Left",
    menu_split_up: "Split Up",
    menu_close_pane: "Close Pane",
    menu_clear_terminal: "Clear Terminal",
    menu_reset_terminal: "Reset Terminal",
    menu_rename: "Rename",
    menu_duplicate: "Duplicate",
    menu_close: "Close",

    // Close dialog
    dialog_close_title: "Close Ferrum?",
    dialog_close_message: "Closing this terminal window will stop all running processes in its tabs.",
    dialog_close_confirm: "Close",
    dialog_close_cancel: "Cancel",

    // Settings window
    settings_title: "Ferrum Settings",
    settings_tab_font: "Font",
    settings_tab_theme: "Theme",
    settings_tab_terminal: "Terminal",
    settings_tab_layout: "Layout",
    settings_tab_security: "Security",
    settings_reset: "Reset to Defaults",

    // Font tab
    font_size: "Font Size:",
    font_family: "Font Family:",
    font_line_padding: "Line Padding:",

    // Theme tab
    theme_label: "Theme:",

    // Terminal tab
    terminal_language: "Language:",
    terminal_max_scrollback: "Max Scrollback:",
    terminal_cursor_blink: "Cursor Blink (ms):",

    // Layout tab
    layout_window_padding: "Window Padding:",
    layout_pane_padding: "Pane Padding:",
    layout_scrollbar_width: "Scrollbar Width:",
    layout_tab_bar_height: "Tab Bar Height:",

    // Security tab
    security_mode: "Security Mode:",
    security_disabled: "Disabled",
    security_standard: "Standard",
    security_custom: "Custom",
    security_paste_protection: "Paste Protection",
    security_paste_description: "Warn before pasting text with suspicious control characters",
    security_block_title: "Block Title Query",
    security_block_title_description: "Block programs from reading the terminal window title",
    security_limit_cursor: "Limit Cursor Jumps",
    security_limit_cursor_description: "Restrict how far escape sequences can move the cursor",
    security_clear_mouse: "Clear Mouse on Reset",
    security_clear_mouse_description: "Disable mouse tracking modes when the terminal resets",

    // Security events / popup
    security_popup_title: "Security events",
    security_event_paste: "Paste with newlines detected",
    security_event_title_query: "OSC/CSI title query blocked",
    security_event_cursor_rewrite: "Cursor rewrite detected",
    security_event_mouse_leak: "Mouse reporting leak prevented",

    // macOS pin button
    pin_window: "Pin Window",
    unpin_window: "Unpin Window",
    pin_tooltip: "Pin window on top",
    unpin_tooltip: "Unpin window",
    settings_tooltip: "Settings",

    // Update notification
    update_available: "Update {} available",
};

pub fn translations() -> &'static Translations {
    &EN
}
```

**Step 2: Create `src/i18n/uk.rs`**

```rust
use super::Translations;

static UK: Translations = Translations {
    // Context menu
    menu_copy: "Копіювати",
    menu_paste: "Вставити",
    menu_select_all: "Вибрати все",
    menu_clear_selection: "Зняти виділення",
    menu_split_right: "Розділити праворуч",
    menu_split_down: "Розділити донизу",
    menu_split_left: "Розділити ліворуч",
    menu_split_up: "Розділити догори",
    menu_close_pane: "Закрити панель",
    menu_clear_terminal: "Очистити термінал",
    menu_reset_terminal: "Скинути термінал",
    menu_rename: "Перейменувати",
    menu_duplicate: "Дублювати",
    menu_close: "Закрити",

    // Close dialog
    dialog_close_title: "Закрити Ferrum?",
    dialog_close_message: "Закриття цього вікна терміналу зупинить усі запущені процеси у його вкладках.",
    dialog_close_confirm: "Закрити",
    dialog_close_cancel: "Скасувати",

    // Settings window
    settings_title: "Налаштування Ferrum",
    settings_tab_font: "Шрифт",
    settings_tab_theme: "Тема",
    settings_tab_terminal: "Термінал",
    settings_tab_layout: "Макет",
    settings_tab_security: "Безпека",
    settings_reset: "Скинути до стандартних",

    // Font tab
    font_size: "Розмір шрифту:",
    font_family: "Сімейство шрифтів:",
    font_line_padding: "Міжрядковий відступ:",

    // Theme tab
    theme_label: "Тема:",

    // Terminal tab
    terminal_language: "Мова:",
    terminal_max_scrollback: "Макс. прокрутка:",
    terminal_cursor_blink: "Блимання курсору (мс):",

    // Layout tab
    layout_window_padding: "Відступ вікна:",
    layout_pane_padding: "Відступ панелі:",
    layout_scrollbar_width: "Ширина скролбару:",
    layout_tab_bar_height: "Висота панелі вкладок:",

    // Security tab
    security_mode: "Режим безпеки:",
    security_disabled: "Вимкнено",
    security_standard: "Стандартний",
    security_custom: "Власний",
    security_paste_protection: "Захист вставки",
    security_paste_description: "Попереджати перед вставкою тексту з підозрілими керуючими символами",
    security_block_title: "Блокувати запити заголовку",
    security_block_title_description: "Блокувати програмам читання заголовку вікна терміналу",
    security_limit_cursor: "Обмежити стрибки курсору",
    security_limit_cursor_description: "Обмежити відстань переміщення курсору escape-послідовностями",
    security_clear_mouse: "Скинути мишу при скиданні",
    security_clear_mouse_description: "Вимикати режими відстеження миші при скиданні терміналу",

    // Security events / popup
    security_popup_title: "Події безпеки",
    security_event_paste: "Виявлено вставку з переносами рядків",
    security_event_title_query: "Запит заголовку OSC/CSI заблоковано",
    security_event_cursor_rewrite: "Виявлено перезапис курсору",
    security_event_mouse_leak: "Запобіжено витоку звітування миші",

    // macOS pin button
    pin_window: "Закріпити вікно",
    unpin_window: "Відкріпити вікно",
    pin_tooltip: "Закріпити вікно поверх інших",
    unpin_tooltip: "Відкріпити вікно",
    settings_tooltip: "Налаштування",

    // Update notification
    update_available: "Доступне оновлення {}",
};

pub fn translations() -> &'static Translations {
    &UK
}
```

**Step 3: Commit**

```bash
git add src/i18n/en.rs src/i18n/uk.rs
git commit -m "feat(i18n): add English and Ukrainian translation files"
```

---

### Task 3: Create OS locale detection

**Files:**
- Create: `src/i18n/detect.rs`

**Step 1: Create `src/i18n/detect.rs`**

```rust
use super::Locale;

/// Detects the system locale and returns the closest supported `Locale`.
/// Falls back to `Locale::En` if the system language is not supported.
pub fn detect_locale() -> Locale {
    let lang = system_language().unwrap_or_default().to_lowercase();

    if lang.starts_with("uk") {
        return Locale::Uk;
    }

    Locale::En
}

/// Reads the system language string from OS environment.
fn system_language() -> Option<String> {
    // Check LANGUAGE first (used by GNU gettext, may contain colon-separated list)
    if let Ok(val) = std::env::var("LANGUAGE") {
        if let Some(first) = val.split(':').next() {
            if !first.is_empty() {
                return Some(first.to_string());
            }
        }
    }
    // Then LC_ALL, LC_MESSAGES, LANG (standard Unix locale resolution)
    for var in &["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() && val != "C" && val != "POSIX" {
                return Some(val);
            }
        }
    }
    None
}
```

**Step 2: Commit**

```bash
git add src/i18n/detect.rs
git commit -m "feat(i18n): add OS locale detection"
```

---

### Task 4: Register the i18n module and add `Locale` to `AppConfig`

**Files:**
- Modify: `src/main.rs` — add `mod i18n;`
- Modify: `src/config/model.rs` — add `language: Locale` to `AppConfig`
- Modify: `src/config/mod.rs` — re-export `Locale`

**Step 1: Add `mod i18n;` to `src/main.rs`**

In `src/main.rs`, add `mod i18n;` after `mod core;`:

```rust
mod config;
mod core;
mod gui;
mod i18n;
mod pty;
mod update;
```

**Step 2: Add `language` field to `AppConfig` in `src/config/model.rs`**

Add import at top:
```rust
use crate::i18n::Locale;
```

Add field to `AppConfig`:
```rust
pub(crate) struct AppConfig {
    pub font: FontConfig,
    pub theme: ThemeChoice,
    pub terminal: TerminalConfig,
    pub layout: LayoutConfig,
    pub security: SecuritySettings,
    pub language: Locale,
}
```

**Step 3: Re-export from `src/config/mod.rs`**

Add to the re-export line:
```rust
pub(crate) use model::AppConfig; // already exists — just ensure Locale is usable via crate::i18n::Locale
```

No additional re-export needed — `Locale` lives in `crate::i18n`.

**Step 4: Initialize locale in `App::new()` in `src/gui/mod.rs`**

After `let config = crate::config::load_config();` (around line 234), add:
```rust
crate::i18n::set_locale(config.language);
```

**Step 5: Run `cargo build` to verify compilation**

Run: `cargo build 2>&1 | head -20`
Expected: Successful compilation (or warnings only).

**Step 6: Run tests**

Run: `cargo test`
Expected: All existing tests pass. The `default_config_round_trip` test may need fixing if RON serialization of `Locale` fails.

**Step 7: Commit**

```bash
git add src/main.rs src/config/model.rs src/config/mod.rs src/gui/mod.rs
git commit -m "feat(i18n): register i18n module, add language to AppConfig"
```

---

### Task 5: Write i18n unit tests

**Files:**
- Create: `tests/unit/i18n.rs` (or add `#[cfg(test)]` inline in `src/i18n/mod.rs`)

**Step 1: Add tests to `src/i18n/mod.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_english_strings_non_empty() {
        let tr = Locale::En.translations();
        assert!(!tr.menu_copy.is_empty());
        assert!(!tr.dialog_close_title.is_empty());
        assert!(!tr.settings_title.is_empty());
        assert!(!tr.security_popup_title.is_empty());
    }

    #[test]
    fn all_ukrainian_strings_non_empty() {
        let tr = Locale::Uk.translations();
        assert!(!tr.menu_copy.is_empty());
        assert!(!tr.dialog_close_title.is_empty());
        assert!(!tr.settings_title.is_empty());
        assert!(!tr.security_popup_title.is_empty());
    }

    #[test]
    fn set_locale_changes_translations() {
        set_locale(Locale::En);
        assert_eq!(t().menu_copy, "Copy");
        set_locale(Locale::Uk);
        assert_eq!(t().menu_copy, "Копіювати");
        set_locale(Locale::En);
        assert_eq!(t().menu_copy, "Copy");
    }

    #[test]
    fn locale_index_roundtrip() {
        for &locale in Locale::ALL {
            assert_eq!(Locale::from_index(locale.index()), locale);
        }
    }

    #[test]
    fn locale_serde_roundtrip() {
        for &locale in Locale::ALL {
            let json = serde_json::to_string(&locale).unwrap();
            let back: Locale = serde_json::from_str(&json).unwrap();
            assert_eq!(back, locale);
        }
    }

    #[test]
    fn locale_display_names_match_all() {
        assert_eq!(Locale::ALL.len(), Locale::DISPLAY_NAMES.len());
    }

    #[test]
    fn detect_respects_lang_env() {
        // Save and restore LANG
        let original = std::env::var("LANG").ok();
        let original_lc = std::env::var("LC_ALL").ok();
        let original_language = std::env::var("LANGUAGE").ok();

        // Clear overriding vars
        std::env::remove_var("LANGUAGE");
        std::env::remove_var("LC_ALL");
        std::env::remove_var("LC_MESSAGES");

        std::env::set_var("LANG", "uk_UA.UTF-8");
        assert_eq!(Locale::detect(), Locale::Uk);

        std::env::set_var("LANG", "en_US.UTF-8");
        assert_eq!(Locale::detect(), Locale::En);

        std::env::set_var("LANG", "fr_FR.UTF-8");
        assert_eq!(Locale::detect(), Locale::En); // fallback

        // Restore
        match original {
            Some(v) => std::env::set_var("LANG", v),
            None => std::env::remove_var("LANG"),
        }
        match original_lc {
            Some(v) => std::env::set_var("LC_ALL", v),
            None => std::env::remove_var("LC_ALL"),
        }
        match original_language {
            Some(v) => std::env::set_var("LANGUAGE", v),
            None => std::env::remove_var("LANGUAGE"),
        }
    }
}
```

**Step 2: Run tests**

Run: `cargo test i18n`
Expected: All tests pass.

**Step 3: Commit**

```bash
git add src/i18n/mod.rs
git commit -m "test(i18n): add unit tests for locale detection and translation switching"
```

---

### Task 6: Replace hardcoded strings in context menus

**Files:**
- Modify: `src/gui/menus.rs`

**Step 1: Replace all hardcoded strings with `crate::i18n::t()` calls**

At the top of the file, the functions `build_terminal_context_menu` and `build_tab_context_menu` use strings like `"Copy"`, `"Paste"`, etc. Replace each with `crate::i18n::t().menu_copy` etc.

Example changes in `build_terminal_context_menu`:
- `MenuItem::new("Copy", ...)` → `MenuItem::new(crate::i18n::t().menu_copy, ...)`
- `MenuItem::new("Paste", ...)` → `MenuItem::new(crate::i18n::t().menu_paste, ...)`
- ...and so on for all 14 menu items

Example changes in `build_tab_context_menu`:
- `MenuItem::new("Rename", ...)` → `MenuItem::new(crate::i18n::t().menu_rename, ...)`
- `MenuItem::new("Duplicate", ...)` → `MenuItem::new(crate::i18n::t().menu_duplicate, ...)`
- `MenuItem::new("Close", ...)` → `MenuItem::new(crate::i18n::t().menu_close, ...)`

**Step 2: Run `cargo build`**

Run: `cargo build 2>&1 | head -20`
Expected: Successful compilation.

**Step 3: Commit**

```bash
git add src/gui/menus.rs
git commit -m "feat(i18n): localize context menu strings"
```

---

### Task 7: Replace hardcoded strings in close dialog

**Files:**
- Modify: `src/gui/platform/close_dialog.rs`

**Step 1: Replace strings in all 3 platform implementations**

In `confirm_window_close_macos`:
- `ns_string!("Close Ferrum?")` → Use `NSString::from_str(crate::i18n::t().dialog_close_title)`
- `ns_string!("Closing this terminal window...")` → `NSString::from_str(crate::i18n::t().dialog_close_message)`
- `ns_string!("Close")` → `NSString::from_str(crate::i18n::t().dialog_close_confirm)`
- `ns_string!("Cancel")` → `NSString::from_str(crate::i18n::t().dialog_close_cancel)`

In `confirm_window_close_windows`:
- `to_wide("Close Ferrum")` → `to_wide(crate::i18n::t().dialog_close_title)`
- The body text → use `crate::i18n::t().dialog_close_message` and `crate::i18n::t().dialog_close_title`

In `confirm_window_close_linux` (zenity / kdialog):
- Replace all `"Close Ferrum"`, `"Closing this terminal window..."`, `"Close"`, `"Cancel"` with `crate::i18n::t()` field references
- Note: zenity args are `&str`, so use `&format!("--title={}", t.dialog_close_title)` etc.

**Step 2: Run `cargo build`**

Run: `cargo build 2>&1 | head -20`
Expected: Successful compilation.

**Step 3: Commit**

```bash
git add src/gui/platform/close_dialog.rs
git commit -m "feat(i18n): localize close confirmation dialog"
```

---

### Task 8: Replace hardcoded strings in security module

**Files:**
- Modify: `src/core/security.rs` — `SecurityEventKind::label()`
- Modify: `src/gui/events/mouse/security_popup.rs` — popup title

**Step 1: Update `SecurityEventKind::label()` in `src/core/security.rs`**

```rust
pub fn label(self) -> &'static str {
    let t = crate::i18n::t();
    match self {
        Self::PasteInjection => t.security_event_paste,
        Self::TitleQuery => t.security_event_title_query,
        Self::CursorRewrite => t.security_event_cursor_rewrite,
        Self::MouseLeak => t.security_event_mouse_leak,
    }
}
```

**Step 2: Update security popup title in `src/gui/events/mouse/security_popup.rs`**

Replace `title: "Security events"` with `title: crate::i18n::t().security_popup_title`.

**Step 3: Run `cargo build`**

Run: `cargo build 2>&1 | head -20`
Expected: Successful compilation.

**Step 4: Commit**

```bash
git add src/core/security.rs src/gui/events/mouse/security_popup.rs
git commit -m "feat(i18n): localize security event labels and popup title"
```

---

### Task 9: Replace hardcoded strings in macOS settings window

**Files:**
- Modify: `src/gui/platform/macos/settings_window.rs`

**Step 1: Replace all hardcoded label strings**

At the top of `open_settings_window`, after getting `mtm`, add:
```rust
let t = crate::i18n::t();
```

Then replace throughout:
- `"Ferrum Settings"` → `t.settings_title`
- `"Font"` → `t.settings_tab_font`
- `"Theme"` → `t.settings_tab_theme`
- `"Terminal"` → `t.settings_tab_terminal`
- `"Layout"` → `t.settings_tab_layout`
- `"Security"` → `t.settings_tab_security`
- `"Font Size:"` → `t.font_size`
- `"Font Family:"` → `t.font_family`
- `"Line Padding:"` → `t.font_line_padding`
- `"Theme:"` → `t.theme_label`
- `"Max Scrollback:"` → `t.terminal_max_scrollback`
- `"Cursor Blink (ms):"` → `t.terminal_cursor_blink`
- `"Window Padding:"` → `t.layout_window_padding`
- `"Pane Padding:"` → `t.layout_pane_padding`
- `"Scrollbar Width:"` → `t.layout_scrollbar_width`
- `"Security Mode:"` → `t.security_mode`
- `"Disabled"` / `"Standard"` / `"Custom"` → `t.security_disabled` / `t.security_standard` / `t.security_custom`
- `"Paste Protection"` → `t.security_paste_protection`
- `"Warn before pasting..."` → `t.security_paste_description`
- `"Block Title Query"` → `t.security_block_title`
- `"Block programs..."` → `t.security_block_title_description`
- `"Limit Cursor Jumps"` → `t.security_limit_cursor`
- `"Restrict how far..."` → `t.security_limit_cursor_description`
- `"Clear Mouse on Reset"` → `t.security_clear_mouse`
- `"Disable mouse tracking..."` → `t.security_clear_mouse_description`
- `ns_string!("Reset to Defaults")` → Use `NSButton::buttonWithTitle_target_action(&NSString::from_str(t.settings_reset), ...)`

Note: `ns_string!()` is a compile-time macro — replace with `&NSString::from_str(...)` for runtime strings.

**Step 2: Run `cargo build`**

Run: `cargo build 2>&1 | head -20`
Expected: Successful compilation.

**Step 3: Commit**

```bash
git add src/gui/platform/macos/settings_window.rs
git commit -m "feat(i18n): localize macOS settings window"
```

---

### Task 10: Replace hardcoded strings in Windows settings window

**Files:**
- Modify: `src/gui/platform/windows/settings_window.rs`

**Step 1: Replace all hardcoded label strings**

Same pattern as macOS. In `create_controls` and `run_win32_window`:
- `to_wide("Ferrum Settings")` → `to_wide(crate::i18n::t().settings_title)`
- Tab names array: `["Font", "Theme", "Terminal", "Layout", "Security"]` → use `t.settings_tab_font`, etc.
- All label strings in `create_spin_row` / `create_combo_row` / `create_checkbox_row` calls
- `to_wide("Reset to Defaults")` → `to_wide(crate::i18n::t().settings_reset)`
- Security mode options: `&["Disabled", "Standard", "Custom"]` → `&[t.security_disabled, t.security_standard, t.security_custom]`

**Step 2: Run `cargo build` (if on Windows, or verify with `cargo check`)**

Run: `cargo check 2>&1 | head -20`
Expected: No errors.

**Step 3: Commit**

```bash
git add src/gui/platform/windows/settings_window.rs
git commit -m "feat(i18n): localize Windows settings window"
```

---

### Task 11: Replace hardcoded strings in Linux settings window

**Files:**
- Modify: `src/gui/platform/linux/settings_window.rs`

**Step 1: Replace all hardcoded label strings**

Same pattern. In `build_window` and tab builder functions:
- `"Ferrum Settings"` → `crate::i18n::t().settings_title`
- `Label::new(Some("Font"))` → `Label::new(Some(crate::i18n::t().settings_tab_font))`
- All `labeled_spin`, `labeled_combo`, `labeled_switch` calls: replace label arguments
- `"Reset to Defaults"` → `crate::i18n::t().settings_reset`
- Security mode options: `&["Disabled", "Standard", "Custom"]` → use `t` fields

**Step 2: Run `cargo check`**

Run: `cargo check 2>&1 | head -20`
Expected: No errors.

**Step 3: Commit**

```bash
git add src/gui/platform/linux/settings_window.rs
git commit -m "feat(i18n): localize Linux settings window"
```

---

### Task 12: Replace hardcoded strings in macOS pin.rs and window title

**Files:**
- Modify: `src/gui/platform/macos/pin.rs`
- Modify: `src/gui/mod.rs` — `compose_window_title`

**Step 1: Replace pin button strings in `src/gui/platform/macos/pin.rs`**

In `set_pin_button_state_for_window_ptr`:
- `ns_string!("Unpin Window")` → `&NSString::from_str(crate::i18n::t().unpin_window)`
- `ns_string!("Pin Window")` → `&NSString::from_str(crate::i18n::t().pin_window)`
- `ns_string!("Unpin window")` → `&NSString::from_str(crate::i18n::t().unpin_tooltip)`
- `ns_string!("Pin window on top")` → `&NSString::from_str(crate::i18n::t().pin_tooltip)`

In `setup_toolbar`:
- `ns_string!("Pin Window")` (accessibility) → `&NSString::from_str(crate::i18n::t().pin_window)`
- `ns_string!("Pin window on top")` (tooltip) → `&NSString::from_str(crate::i18n::t().pin_tooltip)`
- `ns_string!("Settings")` (gear button) → `&NSString::from_str(crate::i18n::t().settings_tooltip)`

Note: `NSImage::imageWithSystemSymbolName_accessibilityDescription` takes `Option<&NSString>`, so wrap in `Some(...)`.

**Step 2: Update `compose_window_title` in `src/gui/mod.rs`**

Replace line 167:
```rust
Some(release) => format!("{base} - Update {} available", release.tag_name),
```
With:
```rust
Some(release) => {
    let tmpl = crate::i18n::t().update_available;
    format!("{base} - {}", tmpl.replace("{}", &release.tag_name))
}
```

**Step 3: Run `cargo build`**

Run: `cargo build 2>&1 | head -20`
Expected: Successful compilation.

**Step 4: Commit**

```bash
git add src/gui/platform/macos/pin.rs src/gui/mod.rs
git commit -m "feat(i18n): localize macOS pin button and window title"
```

---

### Task 13: Add Language dropdown to settings UI on all 3 platforms

**Files:**
- Modify: `src/gui/platform/macos/settings_window.rs` — add Language popup to Terminal tab
- Modify: `src/gui/platform/windows/settings_window.rs` — add Language combo to Terminal tab
- Modify: `src/gui/platform/linux/settings_window.rs` — add Language dropdown to Terminal tab
- Modify: `src/config/model.rs` — add `language` to `build_config_from_controls` on all platforms

**Step 1: macOS — add Language popup as first element in Terminal tab**

In `open_settings_window`, in the Terminal tab section (after creating `terminal_view`), add before the Max Scrollback row:

```rust
let language_popup = create_popup_row(
    mtm,
    &terminal_view,
    t.terminal_language,
    crate::i18n::Locale::DISPLAY_NAMES,
    config.language.index(),
    280.0,  // y_offset: first row
);
```

Shift existing rows down:
- Max Scrollback: y_offset from 280.0 → 230.0
- Cursor Blink: y_offset from 230.0 → 180.0

Add `language_popup` to `NativeSettingsState` struct and wire it.

In `build_config_from_controls`, add:
```rust
language: crate::i18n::Locale::from_index(
    state.language_popup.indexOfSelectedItem() as usize,
),
```

Wire `language_popup` to stepper-changed action (same as other popups).

In `reset_controls_to_defaults`, add:
```rust
state.language_popup.selectItemAtIndex(crate::i18n::Locale::default().index() as isize);
```

**Step 2: Windows — add Language combo to Terminal tab**

Add a new combo row in `create_controls` for the Terminal tab, before Scrollback:

```rust
let (language_combo, mut ctrls) = create_combo_row(
    hwnd, hinstance, font, crate::i18n::t().terminal_language, x0, y0,
    crate::i18n::Locale::DISPLAY_NAMES, config.language.index(),
    id::LANGUAGE_COMBO, dpi,
);
terminal_page.append(&mut ctrls);
```

Add `LANGUAGE_COMBO` to the `id` module (e.g., `pub const LANGUAGE_COMBO: i32 = 404;` — pick unused ID, check existing IDs 400-403).

Shift Scrollback to `y0 + sp` and Cursor Blink to `y0 + sp * 2`.

Add `language_combo: HWND` to `Win32State`.

In `build_config`, add:
```rust
language: crate::i18n::Locale::from_index(
    SendMessageW(state.language_combo, CB_GETCURSEL, 0, 0) as usize,
),
```

Wire `LANGUAGE_COMBO` in `on_command` to send config (same as `FONT_FAMILY_COMBO`).

In `reset_controls`, add:
```rust
SendMessageW(state.language_combo, CB_SETCURSEL, crate::i18n::Locale::default().index(), 0);
```

**Step 3: Linux — add Language dropdown to Terminal tab**

In `build_terminal_tab`, before `labeled_spin` for scrollback:

```rust
let language = labeled_combo(
    &vbox,
    crate::i18n::t().terminal_language,
    crate::i18n::Locale::DISPLAY_NAMES,
    config.language.index(),
);
```

Return `language` from the function and add to `Controls` struct.

In `build_config`, add:
```rust
language: crate::i18n::Locale::from_index(c.language.selected() as usize),
```

Wire `language` dropdown `connect_selected_notify` (same as font family).

In `reset_controls`, add:
```rust
c.language.set_selected(crate::i18n::Locale::default().index() as u32);
```

**Step 4: Hook locale change in settings event handler**

Wherever the settings window sends a new `AppConfig` to the main loop (in `src/gui/lifecycle/` or wherever `settings_rx` is polled), after applying the config, add:
```rust
crate::i18n::set_locale(config.language);
```

This is likely in `src/gui/lifecycle/mod.rs` where `self.settings_rx.try_recv()` is called.

**Step 5: Run `cargo build`**

Run: `cargo build 2>&1 | head -20`
Expected: Successful compilation.

**Step 6: Commit**

```bash
git add src/gui/platform/macos/settings_window.rs src/gui/platform/windows/settings_window.rs src/gui/platform/linux/settings_window.rs src/config/model.rs src/gui/lifecycle/mod.rs
git commit -m "feat(i18n): add Language dropdown to settings UI on all platforms"
```

---

### Task 14: Final verification

**Step 1: Run clippy**

Run: `cargo clippy 2>&1`
Expected: Zero warnings (per CLAUDE.md rules).

**Step 2: Run all tests**

Run: `cargo test`
Expected: All tests pass, including new i18n tests.

**Step 3: Fix any issues found**

Fix clippy warnings or test failures.

**Step 4: Final commit if needed**

```bash
git add -A
git commit -m "fix: address clippy warnings and test fixes for i18n"
```

---

## Files Summary

**New files (5):**
- `src/i18n/mod.rs`
- `src/i18n/translations.rs`
- `src/i18n/en.rs`
- `src/i18n/uk.rs`
- `src/i18n/detect.rs`

**Modified files (11):**
- `src/main.rs`
- `src/config/model.rs`
- `src/config/mod.rs`
- `src/gui/mod.rs`
- `src/gui/menus.rs`
- `src/gui/platform/close_dialog.rs`
- `src/gui/platform/macos/settings_window.rs`
- `src/gui/platform/macos/pin.rs`
- `src/gui/platform/windows/settings_window.rs`
- `src/gui/platform/linux/settings_window.rs`
- `src/core/security.rs`
- `src/gui/events/mouse/security_popup.rs`
- `src/gui/lifecycle/mod.rs`
