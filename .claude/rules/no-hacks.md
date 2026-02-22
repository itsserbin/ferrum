# No Hacks — Absolute Prohibitions

Fix the root cause. NEVER suppress compiler warnings or errors with workarounds.

## Forbidden Patterns

### Clippy / compiler suppression
NEVER use any `#[allow(...)]` attribute.
Fix the warning instead.

### Unused variable silencing
NEVER use `_` prefix on variables that carry errors or important results.
NEVER use `let _ = expr` to discard a `Result` or `Option` that may contain an error.
Exception: RAII guards are valid — `let _guard = mutex.lock().unwrap();` keeps the lock alive intentionally.

### Panic shortcuts in production
NEVER use `.unwrap()` or `.expect("TODO")` / `.expect("fix later")` in production code paths.
Use `?`, `if let`, or `match` instead.
Exception: `.unwrap()` is acceptable inside `#[test]` functions.

### Clone to satisfy the borrow checker
NEVER clone a value solely to work around a borrow checker error.
Restructure ownership or introduce a helper instead.

### Type casting to hide mismatches
NEVER use `as` to silence a type mismatch.
Use `From`, `Into`, or `TryFrom` instead.
Exception: numeric widening/narrowing casts with explicit intent are acceptable (`x as f32`, `len as u32`).

### Unsafe without a safety argument
NEVER add an `unsafe` block without a `// SAFETY:` comment explaining why the code is sound.

### Old-school error boxing
NEVER return `Box<dyn std::error::Error>` from functions.
Use `anyhow::Result<T>` — it is the project standard.
