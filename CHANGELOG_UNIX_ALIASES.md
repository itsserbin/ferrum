# Unix-Style Command Aliases Implementation

## Summary

Added automatic Unix-style command aliases for Windows users. When running Ferrum on Windows with cmd.exe, common Unix commands like `ls`, `pwd`, `cat`, etc. are automatically mapped to their Windows equivalents.

## Implementation

### Approach: doskey Aliases via Startup Script

**Simple and lightweight solution:**
- Creates a temporary batch script (`%TEMP%\ferrum_aliases.cmd`) on Windows
- Script contains doskey commands to map Unix commands to Windows equivalents
- cmd.exe loads this script on startup using `/K` flag
- Zero runtime overhead - mapping happens at the shell level

### Modified Files

- **`src/pty/mod.rs`**: Added `create_unix_aliases_script()` function and modified cmd.exe startup to load aliases

### Code Changes

```rust
#[cfg(windows)]
fn create_unix_aliases_script() -> Option<PathBuf> {
    // Creates %TEMP%\ferrum_aliases.cmd with:
    // doskey ls=dir $*
    // doskey pwd=cd
    // doskey cat=type $*
    // ... etc
}
```

## Supported Commands

17 Unix commands mapped to Windows equivalents:
- `ls`, `ll`, `la` → `dir` variants
- `pwd` → `cd`
- `cat` → `type`
- `rm`, `cp`, `mv` → `del`, `copy`, `move`
- `mkdir`, `rmdir`, `touch`
- `grep` → `findstr`
- `which` → `where`
- `ps`, `kill` → `tasklist`, `taskkill`
- `env` → `set`
- `clear` → `cls`

## Benefits

✅ **Simple**: No complex command parsing or interception
✅ **Fast**: Zero runtime overhead, works at shell level
✅ **Transparent**: User types `ls`, sees `dir` output naturally
✅ **Compatible**: Works with all cmd.exe features (pipes, redirects, etc.)
✅ **Maintainable**: Easy to add/modify aliases - just edit the batch script
✅ **No Dependencies**: Pure cmd.exe feature (doskey)

## Limitations

⚠️ **Windows behavior**: Commands behave like Windows commands (e.g., `ls -la` becomes `dir -la`)
⚠️ **Not true Unix**: Some Unix-specific options won't work
⚠️ **doskey constraints**: Some complex syntax may not translate perfectly
⚠️ **Windows only**: Feature is inactive on Linux/macOS (not needed there)

## Testing

```bash
# Build and run
cargo build --release
target\release\ferrum.exe

# Test commands
ls
pwd
cat Cargo.toml
touch test.txt
rm test.txt
grep "TODO" src/**/*.rs
ps
env
```

## User Documentation

See `UNIX_COMMANDS.md` for complete list of available commands and usage examples.

## Future Improvements

Possible enhancements:
- Add more aliases (e.g., `ln`, `man`, `df`, etc.)
- Support for PowerShell as well (using `Set-Alias`)
- Per-user customization via config file
- Better handling of complex command options

## Why Not Build Custom Commands?

We considered implementing actual Unix commands in Rust (like BusyBox), but the doskey approach is:
1. **Much simpler** - 30 lines of code vs 1000+ lines
2. **More maintainable** - Just a batch script
3. **More compatible** - Uses native Windows commands
4. **Faster** - No command parsing overhead
5. **More transparent** - User sees native Windows output

The doskey approach is the right solution for this use case: providing Unix-familiar command names while leveraging native Windows functionality.

## Changelog

- **2026-02-16**: Initial implementation with 17 Unix command aliases for Windows cmd.exe
