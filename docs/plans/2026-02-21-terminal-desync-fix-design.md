# Terminal Desync Fix — Design Document

## Problem

Terminal emulator shows visual desynchronization with TUI programs (nano, vim, claude code):
1. **Background colors reset** — programs set colored backgrounds via SGR, but erase/scroll operations reset them to terminal default
2. **Missing SGR attributes** — dim, italic, strikethrough, underline styles are silently ignored
3. **Scroll resets on output** — user cannot scroll up while PTY output is active; scroll_offset forced to 0 on every PTY data event
4. **Missing private modes** — bracketed paste (2004) and focus reporting (1004) not implemented

## Root Causes

### Erase/Edit/Scroll use `Cell::default()` instead of current BG
Files: `erase.rs`, `edit.rs`, `grid_ops.rs`

All erase (CSI J, CSI K), edit (CSI P, CSI @, CSI X), and scroll (insert/delete line) operations create blank cells with `Cell::DEFAULT` (sentinel colors). Per xterm spec, erased cells should inherit the **current background color**.

### Cell struct lacks attributes
File: `cell.rs`

Only 3 boolean attributes: `bold`, `underline`, `reverse`. Missing: `dim` (SGR 2), `italic` (SGR 3), `strikethrough` (SGR 9), underline styles (4:0-4:3).

### SGR handler ignores codes
File: `sgr.rs`

Codes 2, 3, 9, 21, 23, 25, 29 fall through to `_ => {}`.

### Aggressive scroll reset
File: `pty.rs:17`

`leaf.scroll_offset = 0` on every PTY data event.

## Design

### 1. Cell Model Changes (`cell.rs`)

```rust
#[derive(Clone, Copy, PartialEq, Default)]
pub enum UnderlineStyle {
    #[default]
    None,
    Single,   // SGR 4 or 4:1
    Double,   // SGR 4:2 or SGR 21
    Curly,    // SGR 4:3
}

pub struct Cell {
    pub character: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline_style: UnderlineStyle,  // replaces `underline: bool`
    pub reverse: bool,
    pub strikethrough: bool,
}
```

Remove `underline: bool`, replace with `underline_style: UnderlineStyle`. Check `underline_style != None` where `underline` was previously checked.

### 2. Terminal State Changes (`terminal.rs`)

Add fields:
- `current_dim: bool`
- `current_italic: bool`
- `current_strikethrough: bool`
- `current_underline_style: UnderlineStyle`

Remove: `current_underline: bool`

Update `print()` to use new fields. Update `reset_attributes()` to reset all new fields.

### 3. SGR Handler Changes (`sgr.rs`)

New match arms:

| Code | Action |
|------|--------|
| 2 | `set_dim(true)` |
| 3 | `set_italic(true)` |
| 4 | `set_underline_style(Single)` |
| 9 | `set_strikethrough(true)` |
| 21 | `set_underline_style(Double)` |
| 22 | `set_bold(false)` + `set_dim(false)` |
| 23 | `set_italic(false)` |
| 24 | `set_underline_style(None)` |
| 29 | `set_strikethrough(false)` |

Handle colon-separated sub-params for underline styles (4:0 through 4:3) via vte's sub-parameter support.

### 4. Erase/Edit/Scroll Fix

Add `Terminal::make_blank_cell()`:
```rust
fn make_blank_cell(&self) -> Cell {
    Cell {
        character: ' ',
        fg: self.current_fg,
        bg: self.current_bg,
        ..Cell::DEFAULT  // boolean attributes NOT inherited
    }
}
```

Replace all `Cell::default()` in:
- `erase.rs` — ED (CSI J) and EL (CSI K)
- `edit.rs` — DCH (CSI P), ICH (CSI @), ECH (CSI X)
- `grid_ops.rs` — scroll_up_region, scroll_down_region (new blank rows)

Note: `make_blank_cell` needs to be callable from grid_ops, which operates on Grid, not Terminal. Solution: pass the blank cell as a parameter to grid operations.

### 5. Scroll Lock Fix (`pty.rs`)

Remove `leaf.scroll_offset = 0` from PTY data handler. Scroll to bottom only when:
- User presses a key (keyboard input forwarded to PTY)
- Explicit scroll-to-bottom shortcut

### 6. Renderer Changes

#### GPU (`renderer/gpu/buffers.rs`)
Update cell flag bits:
- bit 0: bold
- bit 1: italic (was reserved, now active)
- bit 2: underline (any style != None)
- bit 3: reverse
- bit 4: dim
- bit 5: strikethrough
- bit 6-7: underline style (00=none, 01=single, 10=double, 11=curly)

Shader changes:
- **dim**: multiply FG color by ~0.6
- **italic**: faux italic (glyph shear transform)
- **strikethrough**: horizontal line at cell vertical center
- **underline styles**: single (existing), double (two lines), curly (sine wave)

#### CPU renderer
Analogous changes for softbuffer backend.

### 7. Private Modes (`private_modes.rs`)

#### Bracketed Paste (DECSET 2004)
Add `bracketed_paste: bool` to Terminal. In keyboard paste handler: wrap pasted text in `\x1b[200~...\x1b[201~` when enabled.

#### Focus Reporting (Mode 1004)
Add `focus_reporting: bool` to Terminal. Send `\x1b[I` / `\x1b[O` on window focus/blur.

## Testing

- SGR tests for all new codes (dim, italic, strikethrough, underline styles)
- Erase tests verifying background color inheritance
- Edit tests verifying background color inheritance
- Scroll tests verifying blank row colors
- Scroll lock tests (scroll_offset preserved during PTY output)
- Bracketed paste tests
- Integration testing with nano, vim, claude code
