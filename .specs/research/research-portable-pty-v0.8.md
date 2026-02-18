---
title: Research - portable-pty crate v0.8 API
task_file: N/A (Direct user request)
scratchpad: /home/user/apps/ferrum/.specs/scratchpad/a7c4f3e9.md
created: 2026-02-15
status: complete
---

# Research: portable-pty crate v0.8 API

## Executive Summary

The `portable-pty` crate v0.8 provides cross-platform pseudo-terminal (PTY) support in Rust. Key findings: Use `native_pty_system()` to create PTY system, `openpty()` to create PTY pair, `spawn_command()` to launch processes, and `try_clone_reader()`/`take_writer()` for I/O. Critical: Must drop slave after spawning, and reads are blocking. The `Child` trait provides `try_wait()` for non-blocking status checks and `wait()` for blocking waits. All exact types and signatures documented below with complete working example.

## Related Existing Research

None found.

---

## Documentation & References

| Resource | Description | Relevance | Link |
|----------|-------------|-----------|------|
| portable-pty 0.8.0 API docs | Official API reference | Primary source for all type signatures | [docs.rs](https://docs.rs/portable-pty/0.8.0/portable_pty/) |
| PtySystem trait | PTY system abstraction | How to create PTY pairs | [docs.rs](https://docs.rs/portable-pty/0.8.0/portable_pty/trait.PtySystem.html) |
| MasterPty trait | Master PTY control | I/O operations and resizing | [docs.rs](https://docs.rs/portable-pty/0.8.0/portable_pty/trait.MasterPty.html) |
| CommandBuilder struct | Command construction | Spawning processes | [docs.rs](https://docs.rs/portable-pty/0.8.0/portable_pty/cmdbuilder/struct.CommandBuilder.html) |
| PtySize struct | Terminal dimensions | Setting PTY size | [docs.rs](https://docs.rs/portable-pty/0.8.0/portable_pty/struct.PtySize.html) |
| WezTerm Discussion #2392 | Working example | Real-world usage patterns | [GitHub](https://github.com/wezterm/wezterm/discussions/2392) |

### Key Concepts

- **PTY (Pseudo-Terminal)**: Bidirectional communication channel that emulates a terminal
- **Master/Slave**: Master side controls PTY, slave side is where process runs
- **Blocking I/O**: PTY reads block until data available; use threads or async for non-blocking
- **Drop slave after spawn**: Prevents resource leaks and file descriptor issues

---

## Core Types & Signatures

### Structs

**PtySize**
```rust
pub struct PtySize {
    pub rows: u16,          // Number of text rows
    pub cols: u16,          // Number of text columns
    pub pixel_width: u16,   // Cell width in pixels (often ignored)
    pub pixel_height: u16,  // Cell height in pixels (often ignored)
}
```
Implements: `Default`, `Clone`, `Copy`, `Debug`, `Eq`, `PartialEq`

**CommandBuilder**
```rust
pub struct CommandBuilder { /* private fields */ }
```
Key methods:
- `pub fn new<S: AsRef<OsStr>>(program: S) -> Self`
- `pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S)`
- `pub fn cwd<D: AsRef<OsStr>>(&mut self, dir: D)`
- `pub fn env<K, V>(&mut self, key: K, value: V)`
- `pub fn set_controlling_tty(&mut self, controlling_tty: bool)`

**ExitStatus**
```rust
pub struct ExitStatus {
    pub code: Option<u32>,    // Exit code if terminated normally
    pub signal: Option<i32>,  // Signal number if terminated by signal
}
```

### Traits

**PtySystem**
```rust
pub trait PtySystem {
    fn openpty(&self, size: PtySize) -> Result<PtyPair>;
}
```

**MasterPty**
```rust
pub trait MasterPty: Debug + Send {
    fn resize(&self, size: PtySize) -> Result<(), Error>;
    fn get_size(&self) -> Result<PtySize, Error>;
    fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>, Error>;
    fn take_writer(&self) -> Result<Box<dyn Write + Send>, Error>;
    fn process_group_leader(&self) -> Option<pid_t>;
    fn as_raw_fd(&self) -> Option<RawFd>;
    fn get_termios(&self) -> Option<Termios>;  // provided method
}
```

**SlavePty**
```rust
pub trait SlavePty: Debug + Send {
    fn spawn_command(&self, cmd: CommandBuilder)
        -> Result<Box<dyn Child + Send + Sync>>;
}
```

**Child**
```rust
pub trait Child: Debug + ChildKiller {
    fn try_wait(&mut self) -> IoResult<Option<ExitStatus>>;
    fn wait(&mut self) -> IoResult<ExitStatus>;
    fn process_id(&self) -> Option<u32>;
}
```

**ChildKiller**
```rust
pub trait ChildKiller: Debug {
    fn kill(&mut self) -> IoResult<()>;
    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync>;
}
```

### Return Types Reference

| Method | Return Type |
|--------|-------------|
| `native_pty_system()` | `Box<dyn PtySystem>` |
| `openpty(PtySize)` | `anyhow::Result<PtyPair>` |
| `spawn_command(CommandBuilder)` | `anyhow::Result<Box<dyn Child + Send + Sync>>` |
| `try_clone_reader()` | `anyhow::Result<Box<dyn Read + Send>>` |
| `take_writer()` | `anyhow::Result<Box<dyn Write + Send>>` |
| `resize(PtySize)` | `anyhow::Result<()>` |
| `get_size()` | `anyhow::Result<PtySize>` |
| `try_wait()` | `std::io::Result<Option<ExitStatus>>` |
| `wait()` | `std::io::Result<ExitStatus>` |
| `kill()` | `std::io::Result<()>` |

---

## Usage Patterns

### 1. Creating PTY Pair

```rust
use portable_pty::{native_pty_system, PtySize};

let pty_system = native_pty_system();
let pair = pty_system.openpty(PtySize {
    rows: 24,
    cols: 80,
    pixel_width: 0,
    pixel_height: 0,
})?;
```

### 2. Spawning Shell Process

```rust
use portable_pty::CommandBuilder;

let cmd = CommandBuilder::new("/bin/bash");
let mut child = pair.slave.spawn_command(cmd)?;
drop(pair.slave);  // CRITICAL: Drop slave immediately after spawn
```

### 3. Getting Reader/Writer

```rust
use std::io::{Read, Write};

// Get reader for output from shell
let mut reader = pair.master.try_clone_reader()?;

// Get writer for input to shell
let mut writer = pair.master.take_writer()?;
```

### 4. Resizing PTY

```rust
pair.master.resize(PtySize {
    rows: 40,
    cols: 120,
    pixel_width: 0,
    pixel_height: 0,
})?;
```

### 5. Checking if Child is Alive

```rust
// Non-blocking check
match child.try_wait()? {
    None => {
        // Still running
    },
    Some(exit_status) => {
        // Process has exited
        println!("Exit code: {:?}", exit_status.code);
        println!("Signal: {:?}", exit_status.signal);
    }
}

// Blocking wait for completion
let exit_status = child.wait()?;
```

---

## Complete Working Example

```rust
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use anyhow::Result;

fn main() -> Result<()> {
    // 1. Create PTY system
    let pty_system = native_pty_system();

    // 2. Open PTY pair with size
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    // 3. Spawn bash shell
    let cmd = CommandBuilder::new("/bin/bash");
    let mut child = pair.slave.spawn_command(cmd)?;

    // 4. Drop slave - CRITICAL: no longer needed after spawn
    drop(pair.slave);

    // 5. Get reader and writer from master
    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;

    // 6. Write command to shell
    writer.write_all(b"echo hello\n")?;
    writer.flush()?;

    // 7. Read response (blocking operation)
    let mut buffer = [0u8; 1024];
    let n = reader.read(&mut buffer)?;
    let output = String::from_utf8_lossy(&buffer[..n]);
    println!("Output: {}", output);

    // 8. Check if child is still alive (non-blocking)
    match child.try_wait()? {
        None => println!("Child still running"),
        Some(status) => println!("Child exited: {:?}", status),
    }

    // 9. Resize PTY
    pair.master.resize(PtySize {
        rows: 40,
        cols: 100,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    // 10. Clean shutdown
    writer.write_all(b"exit\n")?;
    writer.flush()?;

    let exit_status = child.wait()?;
    println!("Process exited with: {:?}", exit_status);

    Ok(())
}
```

### Cargo.toml

```toml
[dependencies]
portable-pty = "0.8"
anyhow = "1.0"
```

### Required Imports

```rust
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use anyhow::Result;  // or std::io::Result
```

Optional imports:
```rust
use portable_pty::{Child, MasterPty, SlavePty, PtySystem};
```

---

## Potential Issues

| Issue | Impact | Mitigation |
|-------|--------|------------|
| Blocking reads | High - Can hang application | Use separate thread or async runtime for reads |
| Forgetting to drop slave | Medium - Resource leak | Always `drop(pair.slave)` immediately after spawn |
| Buffer size too small | Low - Truncated output | Use larger buffer or loop reading until complete |
| Process not receiving EOF | Medium - Process hangs waiting | Send `\x04` (EOT) character if needed |
| Race conditions on status check | Low - Transient states | Use `try_wait()` in loop if continuous monitoring needed |

---

## Recommendations

### 1. Use Threading for I/O

PTY reads are blocking. For production code, use threads:

```rust
use std::thread;
use std::sync::mpsc;

let (tx, rx) = mpsc::channel();
let reader_thread = thread::spawn(move || {
    let mut buffer = [0u8; 1024];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,  // EOF
            Ok(n) => {
                let output = String::from_utf8_lossy(&buffer[..n]).to_string();
                tx.send(output).ok();
            }
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }
    }
});
```

### 2. Handle Exit Status Properly

Check both code and signal fields:

```rust
match child.wait()? {
    exit_status if exit_status.code == Some(0) => {
        // Clean exit
    },
    exit_status if exit_status.signal.is_some() => {
        // Terminated by signal
        eprintln!("Killed by signal: {:?}", exit_status.signal);
    },
    exit_status => {
        // Non-zero exit code
        eprintln!("Failed with code: {:?}", exit_status.code);
    }
}
```

### 3. Use CommandBuilder for Complex Commands

```rust
let mut cmd = CommandBuilder::new("/bin/bash");
cmd.arg("-c");
cmd.arg("your_script.sh");
cmd.cwd("/path/to/workdir");
cmd.env("MY_VAR", "value");
```

---

## Sources

All information verified against official documentation:

- [portable-pty 0.8.0 API Documentation](https://docs.rs/portable-pty/0.8.0/portable_pty/)
- [PtySystem Trait](https://docs.rs/portable-pty/0.8.0/portable_pty/trait.PtySystem.html)
- [MasterPty Trait](https://docs.rs/portable-pty/0.8.0/portable_pty/trait.MasterPty.html)
- [PtySize Struct](https://docs.rs/portable-pty/0.8.0/portable_pty/struct.PtySize.html)
- [CommandBuilder Struct](https://docs.rs/portable-pty/0.8.0/portable_pty/cmdbuilder/struct.CommandBuilder.html)
- [Child Trait Source](https://docs.rs/portable-pty/0.8.0/src/portable_pty/lib.rs.html)
- [WezTerm Discussion - Spawning Shell Example](https://github.com/wezterm/wezterm/discussions/2392)
- [WezTerm Discussion - Multi-Terminal](https://github.com/wezterm/wezterm/discussions/3739)

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| Source verification | ✅ | All types from official docs.rs v0.8.0 |
| Recency check | ✅ | Version 0.8.0 specifically researched |
| Alternatives explored | ✅ | N/A - specific version requested |
| Actionability | ✅ | Complete working example provided |
| Evidence quality | ✅ | Primary sources (official docs) only |

Limitations/Caveats:
- Example shows synchronous I/O; production code should use threading or async
- Buffer sizes are examples; adjust based on use case
- Reading from PTY is inherently blocking; non-blocking requires architectural changes
- Error handling uses `anyhow::Result`; can substitute other error types as needed
