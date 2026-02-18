---
title: Research - Rust Crates Version Verification (serde, toml, dirs)
task_file: User request - verify latest versions and APIs
scratchpad: /home/user/apps/ferrum/.specs/scratchpad/rust-crates-verification.md
created: 2026-02-15
status: complete
---

# Research: Rust Crates Version Verification

## Executive Summary

As of February 2026, **serde** and **dirs** are current with your specifications, but **toml** is outdated. The toml crate has been updated from 0.8 to 1.0.1, requiring a version update in Cargo.toml. The API for basic `from_str()` usage remains largely compatible, though several internal APIs have breaking changes.

## Related Existing Research

None found in `.specs/research/`.

---

## Version Status Summary

| Crate | Your Version | Latest Version | Status | Action Required |
|-------|-------------|----------------|--------|-----------------|
| serde | `1` with `derive` feature | 1.0.228 | ✅ Current | None |
| toml | `0.8` | 1.0.1 | ⚠️ Outdated | Update to `"1.0"` or `"1"` |
| dirs | `6` | 6.0.0 | ✅ Current | None |

---

## Documentation & References

| Resource | Description | Relevance | Link |
|----------|-------------|-----------|------|
| Serde Docs | Official API documentation | Version confirmation and API reference | https://docs.rs/serde/1.0.228/serde/ |
| TOML Docs | Official API documentation | Version confirmation and API reference | https://docs.rs/toml/1.0.1/toml/ |
| TOML Changelog | Breaking changes documentation | Migration guide for 0.8 → 1.0 | https://github.com/toml-rs/toml/blob/main/crates/toml/CHANGELOG.md |
| Dirs Docs | Official API documentation | Version confirmation and platform behavior | https://docs.rs/dirs/6.0.0/dirs/ |
| Dirs config_dir | Function-specific docs | Platform-specific path information | https://docs.rs/dirs/latest/dirs/fn.config_dir.html |

### Key Concepts

- **Serde derive feature**: Optional feature flag that enables `#[derive(Serialize, Deserialize)]` macros
- **TOML 1.1.0 spec**: Latest TOML specification implemented by toml 1.0.1 crate
- **XDG Base Directory**: Linux standard for config/data directory locations

---

## Crate Details

### 1. Serde (1.0.228)

**Status**: ✅ Your specification is correct

**Your Spec**: `serde = { version = "1", features = ["derive"] }`

**Latest Version**: 1.0.228 (February 2026)

**API Details**:
- Core traits: `Serialize`, `Deserialize`, `Serializer`, `Deserializer`
- Derive macros: `Serialize`, `Deserialize` (requires `features = ["derive"]`)
- Latest change (1.0.228): Documentation build improvements only

**Key Function Signatures**:
```rust
pub trait Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

pub trait Deserialize<'de>: Sized {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}
```

**Breaking Changes**: None - your spec is fully compatible

---

### 2. TOML (1.0.1)

**Status**: ⚠️ Your specification is outdated

**Your Spec**: `toml = "0.8"` ← OUTDATED

**Recommended Spec**: `toml = "1.0"` or `toml = "1"`

**Latest Version**: 1.0.1+spec-1.1.0 (implements TOML 1.1.0 specification)

**API Details**:
- Fully compatible with serde 1.0
- Implements `Deserialize` and `Serialize` traits for `Table`, `Value`, `Datetime`

**Key Function Signatures**:
```rust
// Primary deserialization function
pub fn from_str<'de, T>(s: &'de str) -> Result<T, Error>
where
    T: Deserialize<'de>
```

**Breaking Changes from 0.8 to 1.0**:

1. **Deserializer::new / ValueDeserializer::new**: Now return `Result` (previously infallible)
   - Deprecated in favor of `Deserializer::parse` / `ValueDeserializer::parse`

2. **Serializer constructors**: `Serializer::new` and `Serializer::pretty` now take `&mut Buffer` instead of `&mut String`

3. **Value parsing**: `FromStr` impl for `Value` now only parses TOML values, not documents
   - For documents, use `from_str` which now only parses TOML documents
   - `Display` impl for `Value` now only renders values, not documents

4. **Borrowed types**: `toml::de` can no longer deserialize to borrowed types (everything is owned through parsing)

5. **Trait implementations**:
   - `serde::de::Deserializer` now implemented for `toml::Deserializer` (not `&mut toml::Deserializer`)
   - `serde::ser::Serializer` now implemented for `toml::Serializer` (not `&mut toml::Serializer`)

6. **Error types**: `toml::ser::Errors` variants are now private

**Migration Impact**: For basic `toml::from_str()` usage with `#[derive(Deserialize)]`, the migration is straightforward - just update the version number. Advanced users directly using Deserializer/Serializer types will need to adjust their code.

---

### 3. Dirs (6.0.0)

**Status**: ✅ Your specification is correct

**Your Spec**: `dirs = "6"`

**Latest Version**: 6.0.0 (February 2026)

**API Details**:
- Provides platform-specific standard directory locations
- Leverages XDG Base Directory (Linux), Known Folder API (Windows), Standard Directories (macOS)

**Key Function Signatures**:
```rust
pub fn config_dir() -> Option<PathBuf>
```

**Return Value**: `Option<PathBuf>` containing:
- `Some(path)`: Snapshot of system's config directory at invocation time
- `None`: If no valid home directory path could be retrieved from OS

**Platform-Specific Behavior**:

| Platform | Path Returned | Example |
|----------|---------------|---------|
| Linux | `$HOME/.config` | `/home/alice/.config` |
| macOS | `~/Library/Preferences` | `/Users/Alice/Library/Preferences` |
| Windows | `%APPDATA%` | `C:\Users\Alice\AppData\Roaming` |

**Breaking Changes**: None - your spec is fully compatible

---

## Recommendations

1. **Update TOML**: Change `toml = "0.8"` to `toml = "1"` in Cargo.toml
2. **Keep serde**: No changes needed - `serde = { version = "1", features = ["derive"] }` is correct
3. **Keep dirs**: No changes needed - `dirs = "6"` is correct
4. **Test after update**: Run `cargo build` and `cargo test` to verify compatibility

---

## Implementation Guidance

### Installation

Update your `Cargo.toml`:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }  # No change needed ✅
toml = "1"                                         # Updated from "0.8" ⚠️
dirs = "6"                                         # No change needed ✅
```

Then run:
```bash
cargo update
cargo build
```

### Configuration

No configuration changes required. The toml API for basic usage remains compatible:

```rust
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Config {
    field: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_str = fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(())
}
```

### Integration Points

All three crates work together seamlessly:

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct AppConfig {
    name: String,
    port: u16,
}

fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    // Get config directory using dirs
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?;

    let config_path = config_dir.join("myapp").join("config.toml");

    // Read and parse TOML using toml + serde
    let config_str = fs::read_to_string(config_path)?;
    let config: AppConfig = toml::from_str(&config_str)?;

    Ok(config)
}
```

---

## Code Examples

### Complete Working Example

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct DatabaseConfig {
    host: String,
    port: u16,
    database: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    app_name: String,
    database: DatabaseConfig,
}

fn get_config_path() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?;
    Some(config_dir.join("myapp").join("config.toml"))
}

fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let config_path = get_config_path()
        .ok_or("Could not determine config directory")?;

    let config_str = fs::read_to_string(config_path)?;
    let config: AppConfig = toml::from_str(&config_str)?;

    Ok(config)
}

fn save_config(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path()
        .ok_or("Could not determine config directory")?;

    // Create parent directories if they don't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let config_str = toml::to_string_pretty(config)?;
    fs::write(config_path, config_str)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig {
        app_name: "MyApp".to_string(),
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "mydb".to_string(),
        },
    };

    save_config(&config)?;
    let loaded = load_config()?;
    println!("Loaded config: {:?}", loaded);

    Ok(())
}
```

Example TOML file:
```toml
app_name = "MyApp"

[database]
host = "localhost"
port = 5432
database = "mydb"
```

---

## Sources

- [Serde crates.io page](https://crates.io/crates/serde)
- [Serde 1.0.228 Documentation](https://docs.rs/serde/1.0.228/serde/)
- [TOML crates.io page](https://crates.io/crates/toml)
- [TOML 1.0.1 Documentation](https://docs.rs/toml/1.0.1/toml/)
- [TOML Changelog (Breaking Changes)](https://github.com/toml-rs/toml/blob/main/crates/toml/CHANGELOG.md)
- [Dirs crates.io page](https://crates.io/crates/dirs)
- [Dirs 6.0.0 Documentation](https://docs.rs/dirs/6.0.0/dirs/)
- [Dirs config_dir Documentation](https://docs.rs/dirs/latest/dirs/fn.config_dir.html)

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| Source verification | ✅ | All from official docs.rs, crates.io, GitHub repos |
| Recency check | ✅ | All versions current as of February 2026 |
| Alternatives explored | N/A | Version verification task, not comparison |
| Actionability | ✅ | Exact version strings, API signatures, code examples provided |
| Evidence quality | ✅ | All from primary official sources |

**Limitations/Caveats**: This research focuses on the three specific crates requested. Alternative crates exist (e.g., `serde_json`, `directories`, `config`) but were not evaluated as they were not part of the verification request.
