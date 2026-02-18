# Technical Analysis - Terminal Crate Ecosystem

## Question 1: Higher-level terminal crate evaluation

| Option | What it provides | Learning Value | Compatibility | Verdict |
|--------|-----------------|----------------|---------------|---------|
| **alacritty_terminal 0.19.0** | Complete terminal: grid, parser, PTY loop, selections | LOW - abstracts everything | Uses vte internally, winit-compatible | Avoid - too high-level |
| **termwiz 0.23.3** | Terminal attributes, surface API, PTY wrappers | MEDIUM - abstracts some details | Cross-platform including Windows ConPTY | Possible but hides internals |
| **vte 0.15.0 (current)** | Just the VT parser | HIGH - you build grid yourself | Pure Rust, no platform deps | Keep - perfect for learning |

### Detailed Analysis

**alacritty_terminal:**
- Provides: Complete terminal emulation (grid, parser, PTY event loop, selections, scrollback)
- Replaces: vte, your grid implementation, scrollback logic
- Learning Value: LOW - it's a complete black box
- Verdict: **Avoid** - defeats purpose of learning project

**termwiz:**
- Provides: Terminal widget abstractions, surface API, PTY management
- Replaces: vte + some grid logic
- Learning Value: MEDIUM - still abstracts important concepts
- Maintained: Active (WezTerm project)
- Verdict: **Possible** but reduces learning significantly

**Current approach (vte 0.15.0):**
- Provides: Only VT escape sequence parsing
- You build: Grid, scrollback, cursor management, selection
- Learning Value: HIGH - you see how everything works
- Verdict: **Recommended** - keep using vte, build the rest yourself

---

## Question 2: Glyph caching/atlasing evaluation

| Option | What it does | Best For | Learning Value | When to Use |
|--------|--------------|----------|----------------|-------------|
| **fontdue 0.9** | Pure rasterization | CPU rendering | HIGH - manual cache | Now (CPU phase) |
| **glyph_brush 0.7.12** | GPU atlas + caching | GPU rendering | MEDIUM | Step 7 (GPU) |
| **cosmic-text 0.14.2** | Full text engine | Complex apps | LOW - black box | Not for learning |
| **Manual cache** | DIY HashMap cache | Learning | HIGHEST | Recommended |

### Detailed Analysis

**fontdue 0.9 (current):**
- Pros: Simplest API, fastest rasterization, focused on one task
- Cons: No caching (you implement), no shaping, no color emoji
- For terminal: Perfect - limited glyph set, don't need shaping
- Verdict: **Keep** - correct choice for learning

**glyph_brush 0.7.12:**
- What: Automatic GPU texture atlas, caching, render API agnostic
- When: Step 7 when switching to wgpu
- Replaces: Manual atlas management
- Latest: 0.7.12 (released 2025-02-21) - actively maintained
- Verdict: **Add later for GPU phase**

**cosmic-text 0.14.2:**
- What: Complete text engine (shaping, layout, bidirectional, emoji, font fallback)
- Uses: rustybuzz (shaping) + swash (rendering)
- For terminal: Overkill - terminals don't need complex text layout
- Verdict: **Avoid** - wrong abstraction level

**Recommended approach:**
Build simple cache yourself:
```rust
struct GlyphCache {
    cache: HashMap<(char, u32), RasterizedGlyph>,
    font: fontdue::Font,
}
```
Learning value: HIGH - you understand caching strategy

---

## Question 3: portable-pty alternatives

| Option | Platforms | API Level | Maturity | Verdict |
|--------|-----------|-----------|----------|---------|
| **portable-pty 0.9.0** | All (Win/Mac/Linux) | High-level traits | Battle-tested (WezTerm) | Keep |
| **nix pty module** | Unix only | Low-level syscalls | Mature | Breaks cross-platform |
| **Raw syscalls** | Platform-specific | Lowest | DIY | Too low-level |

### Analysis

**portable-pty 0.9.0 (current):**
- Cross-platform: Windows ConPTY, Unix PTY
- API: High-level traits (PtySystem, MasterPty, SlavePty)
- Battle-tested: Used by WezTerm
- Latest: 0.9.0 (stable, no breaking changes needed)
- Verdict: **Keep** - perfect abstraction level

**Alternatives:**
- nix crate: Unix-only, breaks project goals
- tokio-pty-process: Requires async (project uses threads)
- Raw PTY: Too much platform-specific code

**Conclusion:** Stick with portable-pty 0.9.0. No better alternative exists.

---

## Question 4: Color handling

**Terminal emulator color needs:**
1. 16 basic colors: Hardcoded RGB palette
2. 256-color cube (16-231): Formula `16 + 36*r + 6*g + b`
3. 24 grayscale (232-255): Formula `8 + 10*n`
4. True color (24-bit): Direct RGB

**Crates evaluated:**
- ansi_colours: Converts RGB ↔ 256-color palette
- ansi_term: Emits ANSI codes (wrong direction)
- r3bl_ansi_color: Formatting output

**Analysis:**
These crates are for APPLICATIONS (emitting ANSI codes), not for TERMINAL EMULATORS (interpreting them).

**Verdict:** **Not needed**. Implement standard palette formulas yourself. More learning value.

---

## Question 5: Scrollback buffer

**Options:**
1. VecDeque<Line> - **Recommended**
2. Vec<Line> + manual wraparound - Complex
3. fixed-vec-deque - Fixed size

**Analysis:**

VecDeque is perfect:
- O(1) push_back() for new lines
- O(1) pop_front() for old lines
- Growable (configurable max)
- Standard library

```rust
struct Scrollback {
    lines: VecDeque<Vec<Cell>>,
    max_lines: usize,  // e.g., 10000
}
```

**Verdict:** Use VecDeque. Don't overthink it.

---

## Question 6: Input handling

**Need:** KeyEvent → Vec<u8> (escape sequences to send to PTY)

**crossterm provides:** RawInput → KeyEvent (opposite direction!)

**Analysis:**
crossterm is for applications USING terminals, not BUILDING terminals.

You need manual mapping:
```rust
fn key_to_bytes(key: KeyCode) -> Vec<u8> {
    match key {
        KeyCode::ArrowUp => b"\x1b[A".to_vec(),
        KeyCode::Enter => b"\r".to_vec(),
        // etc.
    }
}
```

**Verdict:** **Not applicable**. Build conversion table yourself. Good learning opportunity.

---

## Question 7: Unicode width - CRITICAL

**Problem:**
Without unicode-width:
- CJK characters treated as 1 cell → overlaps, broken grid
- Emoji misaligned
- Cursor position wrong

**Solution:**
```rust
use unicode_width::UnicodeWidthChar;

'A'.width() // 1
'中'.width() // 2 (fullwidth)
'😀'.width() // 2
```

**Verdict:** **Required immediately**. Not optional.

Add: `unicode-width = "0.2"`

**Importance:** CRITICAL for international users.

---

## Question 8: Performance profiling

| Tool | Purpose | Overhead | When |
|------|---------|----------|------|
| Manual timing | Instant::now() | Minimal | Anytime |
| tracing | Structured spans | Low-medium | Step 4+ |
| tracing-tracy | Visual profiler | Medium | Step 7 (GPU) |

**Verdict:** Start with manual timing. Add tracing later if needed (step 7+).

**Not urgent for learning project.**

---

## Question 9: New crates (2025-2026)

**Findings:**
- par-term-emu-core-rust: NEW (2025). VT100-VT520. Unproven.
- term39: Retro terminal multiplexer. Niche.
- No major alternatives to vte emerged.

**Conclusion:** Ecosystem stable. vte remains standard.

---

## Summary Recommendations by Question

1. **Higher-level terminal crate:** Stick with vte 0.15.0 ✅
2. **Glyph caching:** Keep fontdue 0.9, manual cache. Add glyph_brush 0.7.12 for GPU phase. ✅
3. **portable-pty alternatives:** Keep portable-pty 0.9.0 ✅
4. **Color handling:** DIY palette implementation ✅
5. **Scrollback:** Use VecDeque<Vec<Cell>> ✅
6. **Input handling:** DIY key_to_bytes() ✅
7. **Unicode width:** ADD unicode-width 0.2 ⚠️ REQUIRED
8. **Performance profiling:** Manual timing now, tracing later ✅
9. **New crates:** None worth switching to ✅
