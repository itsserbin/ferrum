---
title: Research - VTE Crate 0.13 API
task_file: User request for vte 0.13 API research
scratchpad: /home/user/apps/ferrum/.specs/scratchpad/vte013-res.md
created: 2026-02-15
status: complete
---

# Research: VTE Crate 0.13 API

## Executive Summary

The vte 0.13 crate provides a parser for VTE (Virtual Terminal Emulator) protocol. The Parser feeds bytes one-at-a-time via `advance()` to a struct implementing the `Perform` trait with 8 callback methods. Parameters are accessed via `Params::iter()` which yields `&[u16]` slices. SGR colors are handled in `csi_dispatch` with action 'm', cursor movement with actions 'H', 'A', 'B', 'C', 'D'.

## Related Existing Research

- /home/user/apps/ferrum/.specs/research/research-winit-030-api.md (winit GUI framework)

---

## Documentation & References

| Resource | Description | Relevance | Link |
|----------|-------------|-----------|------|
| vte 0.13.0 docs.rs | Official API documentation | Complete API reference | https://docs.rs/vte/0.13.0/vte/ |
| vte Parser docs | Parser struct documentation | Core usage patterns | https://docs.rs/vte/0.13.0/vte/struct.Parser.html |
| vte Perform trait | Trait method signatures | Required implementations | https://docs.rs/vte/0.13.0/vte/trait.Perform.html |
| vte Params docs | Parameter access methods | CSI parameter parsing | https://docs.rs/vte/0.13.0/vte/struct.Params.html |
| parselog.rs example | Official example code | Usage demonstration | https://github.com/alacritty/vte/blob/master/examples/parselog.rs |
| ANSI escape codes | Terminal protocol reference | Understanding sequences | https://en.wikipedia.org/wiki/ANSI_escape_code |

### Key Concepts

- **VTE Protocol**: Virtual Terminal Emulator protocol for parsing terminal escape sequences
- **State Machine**: Parser maintains state while processing bytes sequentially
- **CSI (Control Sequence Introducer)**: ESC [ sequences for cursor control, colors, etc.
- **SGR (Select Graphic Rendition)**: CSI sequences ending in 'm' for text attributes and colors
- **Params**: Fixed-size buffer holding u16 parameter values with subparameter support

---

## Core API

### Imports

```rust
use vte::{Params, Parser, Perform};
```

### Parser Construction

```rust
// Standard parser with default OSC buffer
let mut parser = Parser::new();

// Parser with custom OSC buffer size (const generic)
let mut parser = Parser::<64>::new_with_size();
```

### Parser Methods

**`advance<P: Perform>(&mut self, performer: &mut P, byte: u8)`**
- Processes a single byte through the state machine
- Invokes Perform trait methods on the performer when sequences are recognized
- Must be called once per byte in input stream

**Example usage:**
```rust
let mut parser = Parser::new();
let mut performer = MyTerminal::new();

for byte in input_bytes {
    parser.advance(&mut performer, byte);
}
```

---

## Perform Trait

The `Perform` trait defines 8 callback methods that the Parser invokes. All methods have default no-op implementations but you override them to handle terminal actions.

### Complete Method Signatures

```rust
pub trait Perform {
    fn print(&mut self, c: char);

    fn execute(&mut self, byte: u8);

    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char);

    fn put(&mut self, byte: u8);

    fn unhook(&mut self);

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool);

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char);

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8);
}
```

### Method Details

#### `print(&mut self, c: char)`
- **Purpose**: Output a printable character to the terminal
- **When Called**: For regular ASCII/Unicode characters that should be displayed
- **Parameters**:
  - `c`: The character to print

#### `execute(&mut self, byte: u8)`
- **Purpose**: Execute C0 or C1 control functions
- **When Called**: For control characters like `\n`, `\r`, `\t`, BEL, etc.
- **Parameters**:
  - `byte`: The control byte (0x00-0x1F, 0x7F, or 0x80-0x9F)

#### `csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char)`
- **Purpose**: Handle CSI (Control Sequence Introducer) sequences
- **When Called**: After parsing sequences like `ESC [ <params> <action>`
- **Parameters**:
  - `params`: Parsed numeric parameters from the sequence
  - `intermediates`: Intermediate bytes (rarely used)
  - `ignore`: True if parameter/intermediate limits were exceeded
  - `action`: The final character identifying the sequence (e.g., 'm' for SGR, 'H' for cursor position)
- **Common actions**:
  - `'m'`: SGR (Select Graphic Rendition) - colors and text attributes
  - `'H'` or `'f'`: Cursor position
  - `'A'`: Cursor up
  - `'B'`: Cursor down
  - `'C'`: Cursor forward
  - `'D'`: Cursor back

#### `esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8)`
- **Purpose**: Handle escape sequences (simpler than CSI)
- **When Called**: After parsing sequences like `ESC <byte>`
- **Parameters**:
  - `intermediates`: Intermediate bytes
  - `ignore`: True if intermediate limit was exceeded
  - `byte`: The final byte of the sequence

#### `osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool)`
- **Purpose**: Handle OSC (Operating System Command) sequences
- **When Called**: After parsing sequences like `ESC ] <params> ST` or `ESC ] <params> BEL`
- **Parameters**:
  - `params`: Slice of parameter byte arrays (commonly used for window title)
  - `bell_terminated`: True if sequence ended with BEL (0x07) instead of ST

#### `hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char)`
- **Purpose**: Begin a DCS (Device Control String) sequence
- **When Called**: At the start of a DCS sequence
- **Parameters**: Same as `csi_dispatch`

#### `put(&mut self, byte: u8)`
- **Purpose**: Receive data bytes within a DCS sequence
- **When Called**: For each byte between `hook()` and `unhook()`
- **Parameters**:
  - `byte`: Data byte from the DCS sequence

#### `unhook(&mut self)`
- **Purpose**: End a DCS sequence
- **When Called**: When DCS terminator is encountered

---

## Params API

The `Params` struct is an opaque container for CSI/DCS parameters.

### Methods

**`len(&self) -> usize`**
- Returns the number of parameters

**`is_empty(&self) -> bool`**
- Returns true if no parameters are present

**`iter(&self) -> ParamsIter<'_>`**
- Returns an iterator over all parameters
- Iterator yields `&[u16]` slices
- Each slice contains a parameter value and its subparameters (if any)

### Accessing Parameters

```rust
// Iterate through all parameters
for param in params.iter() {
    // param is &[u16]
    let value = param[0]; // Main parameter value
    // param[1..] are subparameters if present
}

// Get first parameter with default
let first_param = params.iter()
    .next()
    .and_then(|p| p.get(0).copied())
    .unwrap_or(0);

// Get specific parameters (e.g., cursor position)
let mut iter = params.iter();
let row = iter.next().and_then(|p| p.get(0).copied()).unwrap_or(1);
let col = iter.next().and_then(|p| p.get(0).copied()).unwrap_or(1);
```

---

## SGR Color Handling

SGR (Select Graphic Rendition) sequences control text appearance including colors.

### Format
- CSI sequence: `ESC [ <params> m`
- Example: `\x1b[31m` sets red foreground

### Common SGR Parameters

| Parameter | Effect |
|-----------|--------|
| 0 | Reset all attributes |
| 1 | Bold |
| 4 | Underline |
| 30-37 | Foreground colors (black, red, green, yellow, blue, magenta, cyan, white) |
| 40-47 | Background colors (same order) |
| 90-97 | Bright foreground colors |
| 100-107 | Bright background colors |

### Implementation Example

```rust
fn handle_sgr(&mut self, params: &Params) {
    for param in params.iter() {
        let value = param.get(0).copied().unwrap_or(0);

        match value {
            0 => self.reset_attributes(),
            30 => self.set_fg(Color::Black),
            31 => self.set_fg(Color::Red),
            32 => self.set_fg(Color::Green),
            33 => self.set_fg(Color::Yellow),
            34 => self.set_fg(Color::Blue),
            35 => self.set_fg(Color::Magenta),
            36 => self.set_fg(Color::Cyan),
            37 => self.set_fg(Color::White),
            40..=47 => self.set_bg(/* map to color */),
            _ => {} // Unknown parameter
        }
    }
}

// In csi_dispatch:
fn csi_dispatch(&mut self, params: &Params, _: &[u8], _: bool, action: char) {
    match action {
        'm' => self.handle_sgr(params),
        // ... other actions
    }
}
```

---

## Cursor Movement

Cursor movement is handled via CSI sequences in `csi_dispatch`.

### Common Cursor Actions

| Action | Sequence | Description |
|--------|----------|-------------|
| 'H' or 'f' | ESC[{row};{col}H | Set cursor position |
| 'A' | ESC[{n}A | Move cursor up n lines |
| 'B' | ESC[{n}B | Move cursor down n lines |
| 'C' | ESC[{n}C | Move cursor forward n columns |
| 'D' | ESC[{n}D | Move cursor back n columns |

### Implementation Example

```rust
fn csi_dispatch(&mut self, params: &Params, _: &[u8], _: bool, action: char) {
    match action {
        'H' | 'f' => {
            let mut iter = params.iter();
            let row = iter.next().and_then(|p| p.get(0).copied()).unwrap_or(1);
            let col = iter.next().and_then(|p| p.get(0).copied()).unwrap_or(1);
            self.set_cursor_pos(row, col);
        }
        'A' => {
            let n = params.iter().next()
                .and_then(|p| p.get(0).copied())
                .unwrap_or(1);
            self.move_cursor_up(n);
        }
        'B' => {
            let n = params.iter().next()
                .and_then(|p| p.get(0).copied())
                .unwrap_or(1);
            self.move_cursor_down(n);
        }
        'C' => {
            let n = params.iter().next()
                .and_then(|p| p.get(0).copied())
                .unwrap_or(1);
            self.move_cursor_forward(n);
        }
        'D' => {
            let n = params.iter().next()
                .and_then(|p| p.get(0).copied())
                .unwrap_or(1);
            self.move_cursor_back(n);
        }
        _ => {}
    }
}
```

---

## Implementation Guidance

### Installation

Add to `Cargo.toml`:

```toml
[dependencies]
vte = "=0.13.0"
```

### Basic Terminal Implementation

```rust
use std::io::{self, Read};
use vte::{Params, Parser, Perform};

#[derive(Debug, Clone, Copy)]
enum Color {
    Black, Red, Green, Yellow, Blue, Magenta, Cyan, White, Reset
}

struct Terminal {
    fg_color: Color,
    bg_color: Color,
    output: String,
}

impl Terminal {
    fn new() -> Self {
        Self {
            fg_color: Color::White,
            bg_color: Color::Black,
            output: String::new(),
        }
    }
}

impl Perform for Terminal {
    fn print(&mut self, c: char) {
        self.output.push(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.output.push('\n'),
            b'\r' => { /* carriage return */ }
            b'\t' => self.output.push('\t'),
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _: &[u8], _: bool, action: char) {
        match action {
            'm' => {
                // Handle SGR
                for param in params.iter() {
                    let value = param.get(0).copied().unwrap_or(0);
                    match value {
                        0 => { self.fg_color = Color::Reset; }
                        31 => { self.fg_color = Color::Red; }
                        32 => { self.fg_color = Color::Green; }
                        // ... other colors
                        _ => {}
                    }
                }
            }
            'H' | 'f' => {
                // Cursor position
                let mut iter = params.iter();
                let _row = iter.next().and_then(|p| p.get(0).copied()).unwrap_or(1);
                let _col = iter.next().and_then(|p| p.get(0).copied()).unwrap_or(1);
                // Handle cursor positioning
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {}
    fn hook(&mut self, _: &Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _: &[&[u8]], _: bool) {}
}

fn main() {
    let mut parser = Parser::new();
    let mut terminal = Terminal::new();

    // Test with escape sequence
    let input = "\x1b[31mRed Text\x1b[0m Normal";
    for byte in input.bytes() {
        parser.advance(&mut terminal, byte);
    }

    println!("Output: {}", terminal.output);
}
```

---

## Code Examples

### Example 1: Simple Logger

```rust
use vte::{Params, Parser, Perform};

struct Logger;

impl Perform for Logger {
    fn print(&mut self, c: char) {
        println!("[PRINT] '{}'", c);
    }

    fn execute(&mut self, byte: u8) {
        println!("[EXECUTE] 0x{:02x}", byte);
    }

    fn csi_dispatch(&mut self, params: &Params, _: &[u8], _: bool, action: char) {
        print!("[CSI] action='{}' params=[", action);
        for (i, param) in params.iter().enumerate() {
            if i > 0 { print!(", "); }
            print!("{}", param[0]);
        }
        println!("]");
    }

    fn esc_dispatch(&mut self, _: &[u8], _: bool, byte: u8) {
        println!("[ESC] 0x{:02x}", byte);
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell: bool) {
        println!("[OSC] bell={} params={:?}", bell, params);
    }

    fn hook(&mut self, _: &Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
}

fn main() {
    let mut parser = Parser::new();
    let mut logger = Logger;

    let test = "\x1b[31mHello\x1b[0m";
    for byte in test.bytes() {
        parser.advance(&mut logger, byte);
    }
}
```

### Example 2: Reading from stdin

```rust
use std::io::{self, Read};
use vte::{Parser, Perform};

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut parser = Parser::new();
    let mut performer = MyPerformer::new();
    let mut buffer = [0u8; 2048];

    loop {
        match handle.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                // Process each byte
                for byte in &buffer[..n] {
                    parser.advance(&mut performer, *byte);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }
}
```

### Example 3: Testing with Specific Sequences

```rust
fn test_color_sequences() {
    let mut parser = Parser::new();
    let mut terminal = Terminal::new();

    // Test cases
    let tests = [
        ("\x1b[31mRed\x1b[0m", "Red text with reset"),
        ("\x1b[1;32mBold Green\x1b[0m", "Bold green text"),
        ("\x1b[44;33mYellow on Blue\x1b[0m", "Colored foreground and background"),
    ];

    for (input, description) in tests {
        println!("Testing: {}", description);
        for byte in input.bytes() {
            parser.advance(&mut terminal, byte);
        }
        println!("Output: {}\n", terminal.output);
        terminal.output.clear();
    }
}
```

---

## Sources

- [vte 0.13.0 - docs.rs](https://docs.rs/vte/0.13.0/vte/)
- [Parser struct - docs.rs](https://docs.rs/vte/0.13.0/vte/struct.Parser.html)
- [Perform trait - docs.rs](https://docs.rs/vte/0.13.0/vte/trait.Perform.html)
- [Params struct - docs.rs](https://docs.rs/vte/0.13.0/vte/struct.Params.html)
- [ParamsIter - docs.rs](https://docs.rs/vte/0.13.0/vte/struct.ParamsIter.html)
- [parselog.rs example - GitHub](https://github.com/alacritty/vte/blob/master/examples/parselog.rs)
- [alacritty/vte repository - GitHub](https://github.com/alacritty/vte)
- [ANSI escape code - Wikipedia](https://en.wikipedia.org/wiki/ANSI_escape_code)

---

## Verification Summary

| Check | Status | Notes |
|-------|--------|-------|
| Source verification | ✅ | All signatures from official docs.rs/vte/0.13.0 |
| Recency check | ✅ | Version 0.13.0 as requested (not latest but specific version) |
| Alternatives explored | N/A | User requested specific version |
| Actionability | ✅ | Complete working examples with exact imports and signatures |
| Evidence quality | ✅ | Primary sources: official docs, GitHub examples |

**Limitations/Caveats:**
- Code examples not compilation-tested but based on official example structure
- Focused on core functionality (text, colors, cursor) - some CSI sequences not covered
- DCS sequences (hook/put/unhook) minimally covered as rarely used
- Version 0.13.0 is not the latest (0.15.x is current) but matches user requirement
