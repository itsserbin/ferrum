use super::Locale;

/// Detects the OS locale by inspecting environment variables in precedence order:
/// `LANGUAGE`, `LC_ALL`, `LC_MESSAGES`, `LANG`.
///
/// Returns `Locale::Uk` if the first matching variable starts with `"uk"`.
/// Falls back to `Locale::En` for any other value or when no variable is set.
pub fn detect_locale() -> Locale {
    let vars = ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"];
    for var in vars {
        let val = match std::env::var(var) {
            Ok(v) if !v.is_empty() => v,
            _ => continue,
        };
        if val.starts_with("uk") {
            return Locale::Uk;
        }
        return Locale::En;
    }
    Locale::En
}
