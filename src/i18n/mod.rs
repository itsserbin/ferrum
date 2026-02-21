mod detect;
mod en;
mod translations;
mod uk;

use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};

pub use translations::Translations;

/// Supported UI locales.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    En,
    Uk,
}

impl Locale {
    /// All variants in display order.
    pub const ALL: &'static [Locale] = &[Locale::En, Locale::Uk];

    /// Human-readable display names, aligned with `ALL`.
    pub const DISPLAY_NAMES: &'static [&'static str] = &["English", "Українська"];

    /// Returns the index of this locale in `ALL` / `DISPLAY_NAMES`.
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&v| v == self).unwrap_or(0)
    }

    /// Returns the locale at the given index, or `En` if out of range.
    pub fn from_index(i: usize) -> Self {
        Self::ALL.get(i).copied().unwrap_or(Locale::En)
    }

    /// Returns the static translation table for this locale.
    pub fn translations(self) -> &'static Translations {
        match self {
            Locale::En => en::translations(),
            Locale::Uk => uk::translations(),
        }
    }

    /// Detects the locale from OS environment variables.
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

fn current_lock() -> &'static RwLock<&'static Translations> {
    CURRENT.get_or_init(|| RwLock::new(Locale::default().translations()))
}

/// Returns the active translation table.
pub fn t() -> &'static Translations {
    *current_lock().read().expect("i18n RwLock poisoned")
}

/// Switches the active locale. Subsequent calls to `t()` return the new locale's strings.
pub fn set_locale(locale: Locale) {
    let translations = locale.translations();
    debug_assert!(translations.all_non_empty());
    let mut guard = current_lock().write().expect("i18n RwLock poisoned");
    *guard = translations;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_and_display_names_same_length() {
        assert_eq!(Locale::ALL.len(), Locale::DISPLAY_NAMES.len());
    }

    #[test]
    fn index_round_trips() {
        for &locale in Locale::ALL {
            assert_eq!(Locale::from_index(locale.index()), locale);
        }
    }

    #[test]
    fn from_index_out_of_range_returns_en() {
        assert_eq!(Locale::from_index(999), Locale::En);
    }

    #[test]
    fn en_translations_non_empty() {
        let tr = Locale::En.translations();
        assert!(!tr.menu_copy.is_empty());
        assert!(!tr.settings_title.is_empty());
        assert!(!tr.update_available.is_empty());
    }

    #[test]
    fn uk_translations_non_empty() {
        let tr = Locale::Uk.translations();
        assert!(!tr.menu_copy.is_empty());
        assert!(!tr.settings_title.is_empty());
        assert!(!tr.update_available.is_empty());
    }

    #[test]
    fn t_returns_translations_without_panic() {
        let tr = t();
        assert!(!tr.menu_copy.is_empty());
    }

    #[test]
    fn set_locale_switches_translations() {
        set_locale(Locale::En);
        let en_copy = t().menu_copy;
        set_locale(Locale::Uk);
        let uk_copy = t().menu_copy;
        assert_ne!(en_copy, uk_copy);
        // Restore to English for other tests.
        set_locale(Locale::En);
    }

    #[test]
    fn translations_fields_all_set() {
        for &locale in Locale::ALL {
            let tr = locale.translations();
            assert!(tr.all_non_empty(), "empty field found for {:?}", locale);
        }
    }
}
