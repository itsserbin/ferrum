use super::Terminal;
use crate::config::ThemeChoice;
use crate::core::Color;

fn get_char(term: &Terminal, row: usize, col: usize) -> char {
    term.screen.viewport_get(row, col).first_char()
}

// ── Alternate screen mode ──

#[test]
fn toggles_alternate_screen_mode() {
    let mut term = Terminal::new(4, 4);
    assert!(!term.is_alt_screen());

    term.process(b"\x1b[?1049h");
    assert!(term.is_alt_screen());

    term.process(b"\x1b[?1049l");
    assert!(!term.is_alt_screen());
}

// ── SGR attributes ──

#[test]
fn applies_sgr_and_resets_attributes() {
    let mut term = Terminal::new(2, 4);
    term.process(b"\x1b[31mA\x1b[0mB");

    assert_eq!(term.screen.viewport_get(0, 0).fg, term.ansi_palette[1]);
    assert_eq!(term.screen.viewport_get(0, 1).fg, Color::SENTINEL_FG);
}

// ── Device status reports ──

#[test]
fn reports_cursor_position_response() {
    let mut term = Terminal::new(3, 5);
    term.set_cursor_row(1);
    term.set_cursor_col(3);

    term.process(b"\x1b[6n");

    assert_eq!(term.drain_responses(), b"\x1b[2;4R".to_vec());
}

#[test]
fn reports_cursor_position_response_for_private_query() {
    let mut term = Terminal::new(3, 5);
    term.set_cursor_row(2);
    term.set_cursor_col(4);

    term.process(b"\x1b[?6n");

    assert_eq!(term.drain_responses(), b"\x1b[3;5R".to_vec());
}

// ── Perform trait: print ──

#[test]
fn print_simple_text() {
    let mut term = Terminal::new(4, 80);
    term.process(b"Hello");

    assert_eq!(get_char(&term, 0, 0), 'H');
    assert_eq!(get_char(&term, 0, 1), 'e');
    assert_eq!(get_char(&term, 0, 2), 'l');
    assert_eq!(get_char(&term, 0, 3), 'l');
    assert_eq!(get_char(&term, 0, 4), 'o');
    assert_eq!(term.cursor_col(), 5);
}

#[test]
fn print_wraps_at_edge() {
    let mut term = Terminal::new(4, 80);
    // Print 82 characters: 80 fill row 0, 2 wrap to row 1
    let text: String = (0..82).map(|i| (b'A' + (i % 26) as u8) as char).collect();
    term.process(text.as_bytes());

    // Row 0 should be full (80 chars)
    assert_eq!(get_char(&term, 0, 0), 'A');
    assert_eq!(get_char(&term, 0, 79), 'B'); // index 79: 79%26=1 -> 'B'

    // Row 1 should have 2 chars
    assert_eq!(get_char(&term, 1, 0), 'C'); // index 80: 80%26=2 -> 'C'
    assert_eq!(get_char(&term, 1, 1), 'D'); // index 81: 81%26=3 -> 'D'
    assert_eq!(term.cursor_row(), 1);
    assert_eq!(term.cursor_col(), 2);
}

#[test]
fn print_wide_char() {
    let mut term = Terminal::new(4, 80);
    // CJK character is 2 columns wide
    term.process("漢".as_bytes());

    assert_eq!(get_char(&term, 0, 0), '漢');
    assert_eq!(get_char(&term, 0, 1), ' '); // placeholder
    assert_eq!(term.cursor_col(), 2);
}

#[test]
fn print_combining_mark_is_not_dropped() {
    let mut term = Terminal::new(4, 80);
    term.process("e\u{0301}".as_bytes()); // e + combining acute accent

    assert_eq!(get_char(&term, 0, 0), 'e');
    assert_eq!(get_char(&term, 0, 1), '\u{0301}');
    assert_eq!(term.cursor_col(), 2);
}

// ── Perform trait: execute ──

#[test]
fn execute_lf() {
    let mut term = Terminal::new(4, 80);
    term.process(b"A\nB");

    assert_eq!(get_char(&term, 0, 0), 'A');
    // LF moves down one row; col stays after A (col 1).
    assert_eq!(get_char(&term, 1, 1), 'B');
}

#[test]
fn execute_vt_and_ff_behave_like_newline() {
    let mut term = Terminal::new(6, 80);
    term.process(b"A\x0bB\x0cC");

    assert_eq!(get_char(&term, 0, 0), 'A');
    assert_eq!(get_char(&term, 1, 1), 'B');
    assert_eq!(get_char(&term, 2, 2), 'C');
}

#[test]
fn execute_cr() {
    let mut term = Terminal::new(4, 80);
    term.process(b"ABC\rX");

    // CR resets col to 0, X overwrites A
    assert_eq!(get_char(&term, 0, 0), 'X');
    assert_eq!(get_char(&term, 0, 1), 'B');
    assert_eq!(get_char(&term, 0, 2), 'C');
}

#[test]
fn execute_backspace() {
    let mut term = Terminal::new(4, 80);
    term.process(b"AB\x08X");

    // Backspace moves cursor back one; X overwrites B
    assert_eq!(get_char(&term, 0, 0), 'A');
    assert_eq!(get_char(&term, 0, 1), 'X');
}

#[test]
fn execute_tab() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\tX");

    // Tab stops every 8 columns: col 0 -> col 8
    assert_eq!(get_char(&term, 0, 8), 'X');
    assert_eq!(term.cursor_col(), 9);
}

// ── Perform trait: esc_dispatch ──

#[test]
fn esc_save_restore_cursor() {
    let mut term = Terminal::new(10, 80);
    // Move cursor to (3, 5) using CUP
    term.process(b"\x1b[4;6H"); // 1-based: row 4, col 6 -> 0-based: (3, 5)
    assert_eq!(term.cursor_row(), 3);
    assert_eq!(term.cursor_col(), 5);

    // ESC 7 = save cursor
    term.process(b"\x1b7");

    // Move somewhere else
    term.process(b"\x1b[1;1H"); // move to (0, 0)
    assert_eq!(term.cursor_row(), 0);
    assert_eq!(term.cursor_col(), 0);

    // ESC 8 = restore cursor
    term.process(b"\x1b8");
    assert_eq!(term.cursor_row(), 3);
    assert_eq!(term.cursor_col(), 5);
}

#[test]
fn esc_reverse_index_at_top() {
    let mut term = Terminal::new(4, 10);
    // Fill rows with identifiable content
    term.process(b"AAAAAAAAAA"); // row 0
    term.process(b"\n");
    term.set_cursor_col(0);
    term.process(b"BBBBBBBBBB"); // row 1

    // Move cursor to row 0
    term.process(b"\x1b[1;1H"); // CUP to (0, 0)
    assert_eq!(term.cursor_row(), 0);

    // ESC M = Reverse Index at top => scroll_down_region
    term.process(b"\x1bM");

    // Row 0 should now be blank (scroll_down inserts blank at top)
    assert_eq!(get_char(&term, 0, 0), ' ');
    // Old row 0 ('A') should have moved to row 1
    assert_eq!(get_char(&term, 1, 0), 'A');
    // Old row 1 ('B') should have moved to row 2
    assert_eq!(get_char(&term, 2, 0), 'B');
}

#[test]
fn esc_ris_full_reset() {
    let mut term = Terminal::new(4, 80);
    // Set some attributes and move cursor
    term.process(b"\x1b[1;31mHello");
    assert!(term.screen.viewport_get(0, 0).bold);
    assert_eq!(term.cursor_col(), 5);

    // ESC c = RIS (full reset)
    term.process(b"\x1bc");

    assert_eq!(term.cursor_row(), 0);
    assert_eq!(term.cursor_col(), 0);
    assert_eq!(get_char(&term, 0, 0), ' '); // grid cleared
    assert_eq!(term.screen.scrollback_len(), 0);
    assert!(term.cursor_visible);
    assert!(!term.decckm);
}

// ── Scrolling behavior ──

#[test]
fn lf_at_bottom_scrolls() {
    let mut term = Terminal::new(4, 10);
    // Fill all 4 rows
    for row_char in b"ABCD" {
        let line: Vec<u8> = vec![*row_char; 10];
        term.process(&line);
        if *row_char != b'D' {
            term.process(b"\n");
            term.set_cursor_col(0);
        }
    }
    // Cursor should be at row 3 (bottom)
    assert_eq!(term.cursor_row(), 3);
    assert_eq!(get_char(&term, 0, 0), 'A');

    // LF at bottom triggers scroll
    term.process(b"\n");

    // Row A should go to scrollback
    assert_eq!(term.screen.scrollback_len(), 1);
    assert_eq!(term.screen.scrollback_row(0).cells[0].first_char(), 'A');

    // Content shifts up: B->row0, C->row1, D->row2, blank->row3
    assert_eq!(get_char(&term, 0, 0), 'B');
    assert_eq!(get_char(&term, 1, 0), 'C');
    assert_eq!(get_char(&term, 2, 0), 'D');
    assert_eq!(get_char(&term, 3, 0), ' ');
}

#[test]
fn scrollback_preserved() {
    let mut term = Terminal::new(4, 10);
    // Produce enough LFs to scroll many times
    // Each LF at the bottom will push a row to scrollback
    for i in 0..20u8 {
        let ch = b'A' + (i % 26);
        let line: Vec<u8> = vec![ch; 10];
        term.process(&line);
        term.process(b"\n");
        term.set_cursor_col(0);
    }

    // We scrolled 17 times (20 lines - 4 visible + 1 for the last \n)
    // Scrollback should have grown
    assert!(term.screen.scrollback_len() > 0);
    // Scrollback should not exceed max (1000)
    assert!(term.screen.scrollback_len() <= 1000);
}

// ── OSC 7: working directory reporting ──

#[test]
fn osc7_sets_cwd_from_file_uri() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;file://localhost/Users/test/project\x07");
    assert_eq!(term.cwd.as_deref(), Some("/Users/test/project"));
}

#[test]
fn osc7_sets_cwd_from_file_uri_without_host() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;file:///home/user\x07");
    assert_eq!(term.cwd.as_deref(), Some("/home/user"));
}

#[test]
fn osc7_sets_cwd_from_kitty_scheme() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;kitty-shell-cwd:///tmp/work\x07");
    assert_eq!(term.cwd.as_deref(), Some("/tmp/work"));
}

#[test]
fn osc7_ignores_remote_hostname() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;file://remote-server/var/log\x07");
    assert_eq!(term.cwd, None);
}

#[test]
fn osc7_empty_resets_cwd() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;file:///some/path\x07");
    assert_eq!(term.cwd.as_deref(), Some("/some/path"));
    term.process(b"\x1b]7;\x07");
    assert_eq!(term.cwd, None);
}

#[test]
fn osc7_full_reset_clears_cwd() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;file:///some/path\x07");
    assert_eq!(term.cwd.as_deref(), Some("/some/path"));
    term.process(b"\x1bc");
    assert_eq!(term.cwd, None);
}

#[test]
fn osc7_decodes_percent_encoded_path() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;file:///home/user/my%20project\x07");
    assert_eq!(term.cwd.as_deref(), Some("/home/user/my project"));
}

#[test]
fn osc7_with_st_terminator() {
    let mut term = Terminal::new(4, 80);
    term.process(b"\x1b]7;file:///tmp/test\x1b\\");
    assert_eq!(term.cwd.as_deref(), Some("/tmp/test"));
}

// ── Theme recoloring ──

#[test]
fn recolor_remaps_default_fg_bg() {
    let old_fg = Color::SENTINEL_FG;
    let old_bg = Color::SENTINEL_BG;
    let new_fg = Color { r: 46, g: 52, b: 64 };    // Ferrum Light fg
    let new_bg = Color { r: 245, g: 240, b: 235 };  // Ferrum Light bg
    let ansi = ThemeChoice::FerrumDark.resolve().ansi;

    let mut term = Terminal::new(4, 4);
    assert_eq!(term.screen.viewport_get(0, 0).fg, old_fg);
    assert_eq!(term.screen.viewport_get(0, 0).bg, old_bg);

    term.recolor(old_fg, old_bg, &ansi, new_fg, new_bg, &ansi);

    assert_eq!(term.screen.viewport_get(0, 0).fg, new_fg);
    assert_eq!(term.screen.viewport_get(0, 0).bg, new_bg);
    assert_eq!(term.default_fg, new_fg);
    assert_eq!(term.default_bg, new_bg);
}

#[test]
fn recolor_leaves_custom_sgr_colors_untouched() {
    let old_fg = Color::SENTINEL_FG;
    let old_bg = Color::SENTINEL_BG;
    let new_fg = Color { r: 46, g: 52, b: 64 };
    let new_bg = Color { r: 245, g: 240, b: 235 };
    let ansi = ThemeChoice::FerrumDark.resolve().ansi;

    let mut term = Terminal::new(4, 10);
    term.process(b"\x1b[38;2;255;128;0mHello");

    let custom_fg = Color { r: 255, g: 128, b: 0 };
    assert_eq!(term.screen.viewport_get(0, 0).fg, custom_fg);

    term.recolor(old_fg, old_bg, &ansi, new_fg, new_bg, &ansi);

    assert_eq!(term.screen.viewport_get(0, 0).fg, custom_fg);
    assert_eq!(term.screen.viewport_get(0, 0).bg, new_bg);
}

#[test]
fn recolor_remaps_ansi_palette_colors() {
    let old_fg = Color::SENTINEL_FG;
    let old_bg = Color::SENTINEL_BG;
    let old_ansi = ThemeChoice::FerrumDark.resolve().ansi;
    let new_ansi: [Color; 16] = {
        let mut a = old_ansi;
        a[1] = Color { r: 191, g: 59, b: 59 }; // Ferrum Light red
        a
    };

    let mut term = Terminal::new(4, 10);
    term.process(b"\x1b[31mRed");

    assert_eq!(term.screen.viewport_get(0, 0).fg, old_ansi[1]);

    term.recolor(old_fg, old_bg, &old_ansi, old_fg, old_bg, &new_ansi);

    assert_eq!(term.screen.viewport_get(0, 0).fg, new_ansi[1]);
}

// ── Reflow cursor tracking ──

#[test]
fn reflow_cursor_row_points_to_correct_physical_row_after_narrow_resize() {
    // Terminal 3×8. Write "XXXXXXXX\r\nABCDEFG".
    //   Row 0: X×8  (output, non-cursor)
    //   Row 1: ABCDEFG (cursor at col 7, non-wrapped)
    // Resize to 3×4.  Rewrap:
    //   Rows 0,1 (from X×8):     [X×4 wrapped] [X×4]     → scrollback (2 rows)
    //   Rows 2,3 (from ABCDEFG): [ABCD wrapped] [EFG.]    → viewport rows 0,1
    // Cursor (col 7 in logical line) falls in the EFG row (cols 4-6) — viewport row 1.
    // With the bug: cursor placed on row 0 (ABCD), not row 1 (EFG).
    let mut term = Terminal::new(3, 8);
    term.process(b"XXXXXXXX\r\nABCDEFG");
    assert_eq!(term.cursor_row(), 1);
    assert_eq!(term.cursor_col(), 7);

    term.resize(3, 4);

    // With cursor-bottom anchoring the viewport is positioned so the cursor
    // lands at the last row (new_rows - 1 = 2).  Viewport = [XXXX, ABCD, EFG],
    // cursor on EFG (vrow 2).  Content above stays visible instead of being
    // pushed into scrollback.
    assert_eq!(term.cursor_row(), 2, "cursor should be on EFG row (viewport row 2), not on ABCD row (viewport row 1)");
    // cursor at col 7 in logical "ABCDEFG"; after reflow to 4 cols the cursor
    // falls in the "EFG." row at col 3 (7 - 4 cols consumed by "ABCD" row).
    assert_eq!(term.cursor_col(), 3, "cursor col should reflect actual position in reflowed row");
}

#[test]
fn reflow_cursor_row_correct_when_logical_line_wraps_three_times() {
    // Terminal 5×12. Write "XXXXXXXXXXXX\r\nABCDEFGHIJK".
    //   Row 0: X×12 (output)
    //   Row 1: ABCDEFGHIJK — cursor at col 11, non-wrapped
    // Resize to 5×4.  Rewrap:
    //   From X×12: [X×4 w] [X×4 w] [X×4]   → scrollback rows 0-2
    //   From ABCDEFGHIJK (11 chars):
    //     [ABCD w] → scrollback row 3
    //     [EFGH w] → viewport row 0
    //     [IJK.]   → viewport row 1  ← cursor (col 11 in logical = 8+3 col range)
    // Cursor must be at viewport row 1, not row 0.
    let mut term = Terminal::new(5, 12);
    term.process(b"XXXXXXXXXXXX\r\nABCDEFGHIJK");
    assert_eq!(term.cursor_row(), 1);
    assert_eq!(term.cursor_col(), 11);

    term.resize(5, 4);

    // With content-anchor reflow the cursor's logical line ("ABCDEFGHIJK") is not
    // reflowed — it is placed as the cursor's physical row (truncated to 4 cols:
    // "ABCD").  The three XXXX rows fill viewport rows 0-2, then the cursor row
    // sits at row 3, with a blank row 4 below it.
    // rewrapped.len() = 3 (three XXXX rows), cursor_viewport_row = min(3, 4) = 3.
    assert_eq!(term.cursor_row(), 3, "cursor should anchor after the 3 XXXX rows (viewport row 3)");
}

#[test]
fn reflow_cursor_stays_at_prompt_not_in_content_area() {
    // Simulate: 8 rows of `ls`-like output (8 chars each) followed by a
    // short shell prompt "$ " (cursor at col 2).  Terminal is 10×8.
    // After narrow resize (10×4): each content row wraps to 2 rows → 16
    // content rows.  With new_rows=10: grid_offset=16+1+1-10=8, scrollback=8,
    // skip=0.  The prompt row is rewrapped row 16; cursor_row in viewport = 8.
    //
    // With the OLD bug cursor_rewrapped_row = 16 (start of prompt logical line,
    // which is a single row), so cursor_row = 16 - 8 - 8 = 0.  The shell
    // would then clear from row 0 to end — erasing all content.
    let mut term = Terminal::new(10, 8);
    // 8 rows of content then prompt "$ ".
    for _ in 0..8 {
        term.process(b"XXXXXXXX\r\n");
    }
    term.process(b"$ ");
    assert_eq!(term.cursor_row(), 8);
    assert_eq!(term.cursor_col(), 2);

    term.resize(10, 4);

    // After narrow reflow: prompt is at some viewport row >= 0.
    // The important invariant: cursor_row must be at the prompt row, which is
    // BELOW the content rows.  Concretely, with the layout above, cursor_row
    // should equal 8 (the 9th viewport row), NOT 0.
    // With cursor-bottom anchoring cursor lands at new_rows - 1 = 9 (last row).
    // The trailing blank row is dropped; cursor is still BELOW all content rows.
    assert_eq!(
        term.cursor_row(), 9,
        "cursor should be at the last viewport row (9), not in the content area"
    );
}

// ── Reflow content preservation ──

#[test]
fn reflow_single_narrow_wide_restores_all_rows() {
    // 6 rows × 10 cols; rows 0–4 have distinct chars (A–E), row 5 is blank (cursor).
    // The trailing \r\n advances the cursor to the blank row 5 so all content
    // rows are treated as completed output — not the active input line — and are
    // therefore preserved across reflow cycles.
    let mut term = Terminal::new(6, 10);
    term.process(b"AAAAAAAA\r\nBBBBBBBB\r\nCCCCCCCC\r\nDDDDDDDD\r\nEEEEEEEE\r\n");

    // Narrow resize → rows wrap (8-char content → 2 physical rows each at 4 cols).
    term.resize(6, 4);
    // Wide resize → rows should unwrap back.
    term.resize(6, 10);

    assert_eq!(get_char(&term, 0, 0), 'A', "row A missing after single reflow cycle");
    assert_eq!(get_char(&term, 1, 0), 'B', "row B missing after single reflow cycle");
    assert_eq!(get_char(&term, 2, 0), 'C', "row C missing after single reflow cycle");
    assert_eq!(get_char(&term, 3, 0), 'D', "row D missing after single reflow cycle");
    assert_eq!(get_char(&term, 4, 0), 'E', "row E missing after single reflow cycle");
}

#[test]
fn reflow_intensive_preserves_content_rows() {
    // 6 rows × 10 cols; rows 0–4 have distinct chars (A–E), row 5 is blank (cursor).
    // The trailing \r\n keeps the cursor on the blank row so all content survives
    // repeated narrow→wide reflow cycles.
    let mut term = Terminal::new(6, 10);
    term.process(b"AAAAAAAA\r\nBBBBBBBB\r\nCCCCCCCC\r\nDDDDDDDD\r\nEEEEEEEE\r\n");

    // Five narrow→wide cycles to simulate "intensive reflow".
    for _ in 0..5 {
        term.resize(6, 4);
        term.resize(6, 10);
    }

    assert_eq!(get_char(&term, 0, 0), 'A', "row A lost after intensive reflow");
    assert_eq!(get_char(&term, 1, 0), 'B', "row B lost after intensive reflow");
    assert_eq!(get_char(&term, 2, 0), 'C', "row C lost after intensive reflow");
    assert_eq!(get_char(&term, 3, 0), 'D', "row D lost after intensive reflow");
    assert_eq!(get_char(&term, 4, 0), 'E', "row E lost after intensive reflow");
}

#[test]
fn reflow_intensive_varying_sizes_preserves_content() {
    // 11 rows × 20 cols; rows 0–9 have distinct chars (A–J), row 10 is blank
    // (cursor). Each content line ends with \r\n so the cursor advances to the
    // blank row — all content rows are treated as completed output and survive
    // every resize cycle.
    let mut term = Terminal::new(11, 20);
    term.process(b"AAAAAAAAAAAAAAAA\r\n");
    term.process(b"BBBBBBBBBBBBBBBB\r\n");
    term.process(b"CCCCCCCCCCCCCCCC\r\n");
    term.process(b"DDDDDDDDDDDDDDDD\r\n");
    term.process(b"EEEEEEEEEEEEEEEE\r\n");
    term.process(b"FFFFFFFFFFFFFFFF\r\n");
    term.process(b"GGGGGGGGGGGGGGGG\r\n");
    term.process(b"HHHHHHHHHHHHHHHH\r\n");
    term.process(b"IIIIIIIIIIIIIIII\r\n");
    term.process(b"JJJJJJJJJJJJJJJJ\r\n");

    // Irregular resize sequence.
    term.resize(11, 5);
    term.resize(11, 30);
    term.resize(11, 8);
    term.resize(11, 20);
    term.resize(11, 3);
    term.resize(11, 20);

    assert_eq!(get_char(&term, 0, 0), 'A', "row A lost after varying reflow");
    assert_eq!(get_char(&term, 1, 0), 'B', "row B lost after varying reflow");
    assert_eq!(get_char(&term, 2, 0), 'C', "row C lost after varying reflow");
    assert_eq!(get_char(&term, 3, 0), 'D', "row D lost after varying reflow");
    assert_eq!(get_char(&term, 4, 0), 'E', "row E lost after varying reflow");
    assert_eq!(get_char(&term, 5, 0), 'F', "row F lost after varying reflow");
    assert_eq!(get_char(&term, 6, 0), 'G', "row G lost after varying reflow");
    assert_eq!(get_char(&term, 7, 0), 'H', "row H lost after varying reflow");
    assert_eq!(get_char(&term, 8, 0), 'I', "row I lost after varying reflow");
    assert_eq!(get_char(&term, 9, 0), 'J', "row J lost after varying reflow");
}

// ── OSC 52 clipboard write ──

#[test]
fn osc52_writes_decoded_text_to_pending_clipboard() {
    let mut term = Terminal::new(24, 80);
    // "hello" in base64 is "aGVsbG8="
    term.process(b"\x1b]52;c;aGVsbG8=\x07");
    assert_eq!(term.pending_clipboard_write.as_deref(), Some("hello"));
}

#[test]
fn osc52_query_is_ignored() {
    let mut term = Terminal::new(24, 80);
    term.process(b"\x1b]52;c;?\x07");
    assert!(term.pending_clipboard_write.is_none());
}

#[test]
fn osc52_invalid_base64_is_ignored() {
    let mut term = Terminal::new(24, 80);
    term.process(b"\x1b]52;c;!!!invalid!!!\x07");
    assert!(term.pending_clipboard_write.is_none());
}

// ── OSC 8 hyperlinks ──

#[test]
fn osc8_sets_hyperlink_on_subsequent_cells() {
    let mut term = Terminal::new(24, 80);
    // Start hyperlink, write "hi", end hyperlink
    term.process(b"\x1b]8;;https://example.com\x07hi\x1b]8;;\x07");
    let row = term.screen.viewport_row(0);
    assert!(row.cells[0].hyperlink_id != 0, "first cell should have hyperlink");
    assert!(row.cells[1].hyperlink_id != 0, "second cell should have hyperlink");
    assert_eq!(row.cells[2].hyperlink_id, 0, "cell after end should have no hyperlink");
    assert_eq!(
        term.hyperlink_url(row.cells[0].hyperlink_id),
        Some("https://example.com")
    );
}

#[test]
fn osc8_empty_uri_ends_hyperlink() {
    let mut term = Terminal::new(24, 80);
    term.process(b"\x1b]8;;https://x.com\x07a\x1b]8;;\x07b");
    let row = term.screen.viewport_row(0);
    assert!(row.cells[0].hyperlink_id != 0);
    assert_eq!(row.cells[1].hyperlink_id, 0);
}

#[test]
fn osc8_same_url_reuses_id() {
    let mut term = Terminal::new(24, 80);
    term.process(b"\x1b]8;;https://a.com\x07x\x1b]8;;\x07");
    term.process(b"\x1b]8;;https://a.com\x07y\x1b]8;;\x07");
    let row = term.screen.viewport_row(0);
    assert_eq!(row.cells[0].hyperlink_id, row.cells[1].hyperlink_id, "same URL should reuse same ID");
}

// ── modifyOtherKeys ──

#[test]
fn modify_other_keys_level_set_by_csi_sequence() {
    let mut term = Terminal::new(24, 80);
    assert_eq!(term.modify_other_keys, 0);
    term.process(b"\x1b[>4;2m");
    assert_eq!(term.modify_other_keys, 2);
    term.process(b"\x1b[>4;1m");
    assert_eq!(term.modify_other_keys, 1);
    term.process(b"\x1b[>4;0m");
    assert_eq!(term.modify_other_keys, 0);
}

#[test]
fn modify_other_keys_reset_by_csi_without_param() {
    let mut term = Terminal::new(24, 80);
    term.process(b"\x1b[>4;2m");
    term.process(b"\x1b[>4m");
    assert_eq!(term.modify_other_keys, 0);
}
