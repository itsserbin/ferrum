use crate::core::{Cell, Color, Grid, SecurityConfig, SecurityEventKind};
use std::collections::VecDeque;
use unicode_width::UnicodeWidthChar;
use vte::{Params, Parser, Perform};

mod grid_ops;
mod handlers;

/// Cursor style reported by DECSCUSR.
#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum CursorStyle {
    BlinkingBlock,     // 0, 1
    SteadyBlock,       // 2
    BlinkingUnderline, // 3
    SteadyUnderline,   // 4
    #[default]
    BlinkingBar, // 5
    SteadyBar,         // 6
}

impl CursorStyle {
    pub fn is_blinking(self) -> bool {
        matches!(
            self,
            Self::BlinkingBlock | Self::BlinkingUnderline | Self::BlinkingBar
        )
    }
}

/// Mouse tracking mode (xterm protocol).
#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum MouseMode {
    #[default]
    Off,
    Normal,      // ?1000: press + release
    ButtonEvent, // ?1002: press + release + drag
    AnyEvent,    // ?1003: report all mouse motion
}

pub struct Terminal {
    pub grid: Grid,
    alt_grid: Option<Grid>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    saved_cursor: (usize, usize),     // ESC 7 / ESC 8 (DECSC/DECRC)
    alt_saved_cursor: (usize, usize), // Saved separately for alt-screen enter/leave.
    current_fg: Color,
    current_bg: Color,
    current_bold: bool,
    current_reverse: bool,
    current_underline: bool,
    scroll_top: usize,
    scroll_bottom: usize,
    saved_scroll_top: usize,
    saved_scroll_bottom: usize,
    pub scrollback: VecDeque<Vec<Cell>>,
    max_scrollback: usize,
    pub decckm: bool,               // Application Cursor Key Mode (ESC[?1h/l)
    pub cursor_visible: bool,       // DECTCEM (mode 25)
    pub pending_responses: Vec<u8>, // Bytes queued for PTY replies.
    pub mouse_mode: MouseMode,
    pub sgr_mouse: bool,
    pub security_config: SecurityConfig,
    pending_security_events: Vec<SecurityEventKind>,
    pub cursor_style: CursorStyle,
    resize_at: Option<std::time::Instant>,
    parser: Parser,
}

impl Terminal {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            grid: Grid::new(rows, cols),
            alt_grid: None,
            cursor_row: 0,
            cursor_col: 0,
            saved_cursor: (0, 0),
            alt_saved_cursor: (0, 0),
            current_fg: Color::DEFAULT_FG,
            current_bg: Color::DEFAULT_BG,
            current_bold: false,
            current_reverse: false,
            current_underline: false,
            scroll_top: 0,
            scroll_bottom: rows - 1,
            saved_scroll_top: 0,
            saved_scroll_bottom: rows - 1,
            scrollback: VecDeque::new(),
            max_scrollback: 1000,
            decckm: false,
            cursor_visible: true,
            pending_responses: Vec::new(),
            mouse_mode: MouseMode::Off,
            sgr_mouse: false,
            security_config: SecurityConfig::default(),
            pending_security_events: Vec::new(),
            cursor_style: CursorStyle::default(),
            resize_at: None,
            parser: Parser::new(),
        }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        let mut parser = std::mem::replace(&mut self.parser, Parser::new());
        parser.advance(self, bytes);
        self.parser = parser;
    }

    fn param(&self, params: &Params, default: u16) -> u16 {
        params
            .iter()
            .next()
            .and_then(|p| p.first().copied())
            .unwrap_or(default)
    }

    /// Queues a response that GUI will flush back to PTY.
    fn respond(&mut self, data: &[u8]) {
        self.pending_responses.extend_from_slice(data);
    }

    /// Drains all pending PTY response bytes.
    pub fn drain_responses(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending_responses)
    }

    /// Drains security events detected while parsing terminal output.
    pub fn drain_security_events(&mut self) -> Vec<SecurityEventKind> {
        std::mem::take(&mut self.pending_security_events)
    }

    /// Clears transient mouse-tracking state when the PTY process exits.
    pub fn cleanup_after_process_exit(&mut self) {
        self.clear_mouse_tracking(self.security_config.clear_mouse_on_reset);
    }

    fn reset_attributes(&mut self) {
        handlers::attributes::reset_attributes(self);
    }

    fn set_fg(&mut self, color: Color) {
        self.current_fg = color;
    }

    fn set_bg(&mut self, color: Color) {
        self.current_bg = color;
    }

    fn set_bold(&mut self, value: bool) {
        self.current_bold = value;
    }

    fn set_reverse(&mut self, value: bool) {
        self.current_reverse = value;
    }

    fn set_underline(&mut self, value: bool) {
        self.current_underline = value;
    }

    fn set_decckm(&mut self, enabled: bool) {
        self.decckm = enabled;
    }

    fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    fn set_mouse_mode(&mut self, mode: MouseMode) {
        self.mouse_mode = mode;
    }

    fn set_sgr_mouse(&mut self, enabled: bool) {
        self.sgr_mouse = enabled;
    }

    fn set_cursor_style(&mut self, style: CursorStyle) {
        self.cursor_style = style;
    }

    fn emit_security_event(&mut self, kind: SecurityEventKind) {
        self.pending_security_events.push(kind);
    }

    fn clear_mouse_tracking(&mut self, emit_event: bool) {
        let had_mouse_tracking = self.mouse_mode != MouseMode::Off || self.sgr_mouse;
        self.mouse_mode = MouseMode::Off;
        self.sgr_mouse = false;
        if emit_event && had_mouse_tracking {
            self.emit_security_event(SecurityEventKind::MouseLeak);
        }
    }

    fn maybe_record_cursor_rewrite(&mut self, from_row: usize, to_row: usize) {
        if !self.security_config.limit_cursor_jumps || self.is_alt_screen() || to_row >= from_row {
            return;
        }

        // Suppress false positives: shell redraws prompt after resize.
        if self.resize_at.is_some_and(|t| t.elapsed().as_secs() < 2) {
            return;
        }

        let row_has_content =
            (0..self.grid.cols).any(|col| self.grid.get(to_row, col).character != ' ');
        if row_has_content {
            self.emit_security_event(SecurityEventKind::CursorRewrite);
        }
    }

    fn is_blocked_title_query(&self, action: char, params: &Params) -> bool {
        if action != 't' || !self.security_config.block_title_query {
            return false;
        }
        matches!(self.param(params, 0), 20 | 21)
    }

    fn full_reset(&mut self) {
        let rows = self.grid.rows;
        let cols = self.grid.cols;

        self.alt_grid = None;
        self.grid = Grid::new(rows, cols);
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.saved_cursor = (0, 0);
        self.alt_saved_cursor = (0, 0);
        self.scroll_top = 0;
        self.scroll_bottom = rows.saturating_sub(1);
        self.saved_scroll_top = 0;
        self.saved_scroll_bottom = rows.saturating_sub(1);
        self.scrollback.clear();
        self.decckm = false;
        self.cursor_visible = true;
        self.cursor_style = CursorStyle::default();
        self.pending_responses.clear();
        if self.security_config.clear_mouse_on_reset {
            self.clear_mouse_tracking(true);
        }
        self.reset_attributes();
    }

    fn handle_private_mode(&mut self, params: &Params, intermediates: &[u8], action: char) -> bool {
        handlers::private_modes::handle_private_mode(self, params, intermediates, action)
    }

    fn handle_cursor_style_csi(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        action: char,
    ) -> bool {
        handlers::private_modes::handle_cursor_style_csi(self, params, intermediates, action)
    }

    fn handle_sgr(&mut self, params: &Params) {
        handlers::sgr::handle_sgr(self, params);
    }

    fn handle_cursor_csi(&mut self, action: char, params: &Params) -> bool {
        let from_row = self.cursor_row;
        let handled = handlers::cursor::handle_cursor_csi(self, action, params);
        if handled {
            self.maybe_record_cursor_rewrite(from_row, self.cursor_row);
        }
        handled
    }

    fn handle_inline_edit_csi(&mut self, action: char, params: &Params) -> bool {
        handlers::edit::handle_inline_edit_csi(self, action, params)
    }

    fn handle_scroll_csi(&mut self, action: char, params: &Params) -> bool {
        handlers::scroll::handle_scroll_csi(self, action, params)
    }

    fn handle_erase_csi(&mut self, action: char, params: &Params) -> bool {
        handlers::erase::handle_erase_csi(self, action, params)
    }

    fn handle_device_csi(&mut self, action: char, params: &Params, intermediates: &[u8]) -> bool {
        handlers::device::handle_device_csi(self, action, params, intermediates)
    }
}

impl Perform for Terminal {
    fn print(&mut self, c: char) {
        let width = UnicodeWidthChar::width(c).unwrap_or(1);

        if self.cursor_col + width > self.grid.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
            if self.cursor_row > self.scroll_bottom {
                self.scroll_up_region(self.scroll_top, self.scroll_bottom);
                self.cursor_row = self.scroll_bottom;
            }
        }

        self.grid.set(
            self.cursor_row,
            self.cursor_col,
            Cell {
                character: c,
                fg: self.current_fg,
                bg: self.current_bg,
                bold: self.current_bold,
                reverse: self.current_reverse,
                underline: self.current_underline,
            },
        );

        // Reserve the trailing cell for wide glyphs.
        if width == 2 && self.cursor_col + 1 < self.grid.cols {
            self.grid.set(
                self.cursor_row,
                self.cursor_col + 1,
                Cell {
                    character: ' ',
                    fg: self.current_fg,
                    bg: self.current_bg,
                    bold: self.current_bold,
                    reverse: self.current_reverse,
                    underline: self.current_underline,
                },
            );
        }

        self.cursor_col += width;
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            10 => {
                // \n
                self.cursor_row += 1;
                if self.cursor_row > self.scroll_bottom {
                    self.scroll_up_region(self.scroll_top, self.scroll_bottom);
                    self.cursor_row = self.scroll_bottom;
                }
            }
            13 => {
                // \r
                self.cursor_col = 0;
            }
            8 => {
                // backspace
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            9 => {
                // tab
                self.cursor_col = (self.cursor_col + 8) & !7;
                if self.cursor_col >= self.grid.cols {
                    self.cursor_col = self.grid.cols - 1;
                }
            }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        if self.is_blocked_title_query(action, params) {
            self.emit_security_event(SecurityEventKind::TitleQuery);
            return;
        }
        if self.handle_private_mode(params, intermediates, action) {
            return;
        }
        if self.handle_cursor_style_csi(params, intermediates, action) {
            return;
        }
        if action == 'm' {
            self.handle_sgr(params);
            return;
        }
        if self.handle_cursor_csi(action, params) {
            return;
        }
        if self.handle_inline_edit_csi(action, params) {
            return;
        }
        if self.handle_scroll_csi(action, params) {
            return;
        }
        if self.handle_erase_csi(action, params) {
            return;
        }
        let _ = self.handle_device_csi(action, params, intermediates);
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'7' => self.saved_cursor = (self.cursor_row, self.cursor_col),
            b'8' => {
                let from_row = self.cursor_row;
                self.cursor_row = self.saved_cursor.0;
                self.cursor_col = self.saved_cursor.1;
                self.maybe_record_cursor_rewrite(from_row, self.cursor_row);
            }
            b'M' => {
                let from_row = self.cursor_row;
                // Reverse Index: cursor up, scroll down if at top of region
                if self.cursor_row == self.scroll_top {
                    self.scroll_down_region(self.scroll_top, self.scroll_bottom);
                } else {
                    self.cursor_row = self.cursor_row.saturating_sub(1);
                }
                self.maybe_record_cursor_rewrite(from_row, self.cursor_row);
            }
            b'c' => self.full_reset(), // RIS - full terminal reset
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Terminal;
    use crate::core::Color;

    // ── Migrated from tests/unit/core_terminal.rs ──

    #[test]
    fn toggles_alternate_screen_mode() {
        let mut term = Terminal::new(4, 4);
        assert!(!term.is_alt_screen());

        term.process(b"\x1b[?1049h");
        assert!(term.is_alt_screen());

        term.process(b"\x1b[?1049l");
        assert!(!term.is_alt_screen());
    }

    #[test]
    fn applies_sgr_and_resets_attributes() {
        let mut term = Terminal::new(2, 4);
        term.process(b"\x1b[31mA\x1b[0mB");

        assert_eq!(term.grid.get(0, 0).fg, Color::ANSI[1]);
        assert_eq!(term.grid.get(0, 1).fg, Color::DEFAULT_FG);
    }

    #[test]
    fn reports_cursor_position_response() {
        let mut term = Terminal::new(3, 5);
        term.cursor_row = 1;
        term.cursor_col = 3;

        term.process(b"\x1b[6n");

        assert_eq!(term.drain_responses(), b"\x1b[2;4R".to_vec());
    }

    // ── Perform trait: print ──

    #[test]
    fn print_simple_text() {
        let mut term = Terminal::new(4, 80);
        term.process(b"Hello");

        assert_eq!(term.grid.get(0, 0).character, 'H');
        assert_eq!(term.grid.get(0, 1).character, 'e');
        assert_eq!(term.grid.get(0, 2).character, 'l');
        assert_eq!(term.grid.get(0, 3).character, 'l');
        assert_eq!(term.grid.get(0, 4).character, 'o');
        assert_eq!(term.cursor_col, 5);
    }

    #[test]
    fn print_wraps_at_edge() {
        let mut term = Terminal::new(4, 80);
        // Print 82 characters: 80 fill row 0, 2 wrap to row 1
        let text: String = (0..82).map(|i| (b'A' + (i % 26) as u8) as char).collect();
        term.process(text.as_bytes());

        // Row 0 should be full (80 chars)
        assert_eq!(term.grid.get(0, 0).character, 'A');
        assert_eq!(term.grid.get(0, 79).character, 'B'); // index 79: 79%26=1 -> 'B'

        // Row 1 should have 2 chars
        assert_eq!(term.grid.get(1, 0).character, 'C'); // index 80: 80%26=2 -> 'C'
        assert_eq!(term.grid.get(1, 1).character, 'D'); // index 81: 81%26=3 -> 'D'
        assert_eq!(term.cursor_row, 1);
        assert_eq!(term.cursor_col, 2);
    }

    #[test]
    fn print_wide_char() {
        let mut term = Terminal::new(4, 80);
        // CJK character '漢' is 2 columns wide
        term.process("漢".as_bytes());

        assert_eq!(term.grid.get(0, 0).character, '漢');
        assert_eq!(term.grid.get(0, 1).character, ' '); // placeholder
        assert_eq!(term.cursor_col, 2);
    }

    // ── Perform trait: execute ──

    #[test]
    fn execute_lf() {
        let mut term = Terminal::new(4, 80);
        term.process(b"A\nB");

        assert_eq!(term.grid.get(0, 0).character, 'A');
        // LF moves down one row; col stays after A (col 1)
        assert_eq!(term.grid.get(1, 1).character, 'B');
    }

    #[test]
    fn execute_cr() {
        let mut term = Terminal::new(4, 80);
        term.process(b"ABC\rX");

        // CR resets col to 0, X overwrites A
        assert_eq!(term.grid.get(0, 0).character, 'X');
        assert_eq!(term.grid.get(0, 1).character, 'B');
        assert_eq!(term.grid.get(0, 2).character, 'C');
    }

    #[test]
    fn execute_backspace() {
        let mut term = Terminal::new(4, 80);
        term.process(b"AB\x08X");

        // Backspace moves cursor back one; X overwrites B
        assert_eq!(term.grid.get(0, 0).character, 'A');
        assert_eq!(term.grid.get(0, 1).character, 'X');
    }

    #[test]
    fn execute_tab() {
        let mut term = Terminal::new(4, 80);
        term.process(b"\tX");

        // Tab stops every 8 columns: col 0 -> col 8
        assert_eq!(term.grid.get(0, 8).character, 'X');
        assert_eq!(term.cursor_col, 9);
    }

    // ── Perform trait: esc_dispatch ──

    #[test]
    fn esc_save_restore_cursor() {
        let mut term = Terminal::new(10, 80);
        // Move cursor to (3, 5) using CUP
        term.process(b"\x1b[4;6H"); // 1-based: row 4, col 6 -> 0-based: (3, 5)
        assert_eq!(term.cursor_row, 3);
        assert_eq!(term.cursor_col, 5);

        // ESC 7 = save cursor
        term.process(b"\x1b7");

        // Move somewhere else
        term.process(b"\x1b[1;1H"); // move to (0, 0)
        assert_eq!(term.cursor_row, 0);
        assert_eq!(term.cursor_col, 0);

        // ESC 8 = restore cursor
        term.process(b"\x1b8");
        assert_eq!(term.cursor_row, 3);
        assert_eq!(term.cursor_col, 5);
    }

    #[test]
    fn esc_reverse_index_at_top() {
        let mut term = Terminal::new(4, 10);
        // Fill rows with identifiable content
        term.process(b"AAAAAAAAAA"); // row 0
        term.process(b"\n");
        term.cursor_col = 0;
        term.process(b"BBBBBBBBBB"); // row 1

        // Move cursor to row 0
        term.process(b"\x1b[1;1H"); // CUP to (0, 0)
        assert_eq!(term.cursor_row, 0);

        // ESC M = Reverse Index at top => scroll_down_region
        term.process(b"\x1bM");

        // Row 0 should now be blank (scroll_down inserts blank at top)
        assert_eq!(term.grid.get(0, 0).character, ' ');
        // Old row 0 ('A') should have moved to row 1
        assert_eq!(term.grid.get(1, 0).character, 'A');
        // Old row 1 ('B') should have moved to row 2
        assert_eq!(term.grid.get(2, 0).character, 'B');
    }

    #[test]
    fn esc_ris_full_reset() {
        let mut term = Terminal::new(4, 80);
        // Set some attributes and move cursor
        term.process(b"\x1b[1;31mHello");
        assert!(term.grid.get(0, 0).bold);
        assert_eq!(term.cursor_col, 5);

        // ESC c = RIS (full reset)
        term.process(b"\x1bc");

        assert_eq!(term.cursor_row, 0);
        assert_eq!(term.cursor_col, 0);
        assert_eq!(term.grid.get(0, 0).character, ' '); // grid cleared
        assert!(term.scrollback.is_empty());
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
                term.cursor_col = 0;
            }
        }
        // Cursor should be at row 3 (bottom)
        assert_eq!(term.cursor_row, 3);
        assert_eq!(term.grid.get(0, 0).character, 'A');

        // LF at bottom triggers scroll
        term.process(b"\n");

        // Row A should go to scrollback
        assert_eq!(term.scrollback.len(), 1);
        assert_eq!(term.scrollback[0][0].character, 'A');

        // Content shifts up: B->row0, C->row1, D->row2, blank->row3
        assert_eq!(term.grid.get(0, 0).character, 'B');
        assert_eq!(term.grid.get(1, 0).character, 'C');
        assert_eq!(term.grid.get(2, 0).character, 'D');
        assert_eq!(term.grid.get(3, 0).character, ' ');
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
            term.cursor_col = 0;
        }

        // We scrolled 17 times (20 lines - 4 visible + 1 for the last \n)
        // Scrollback should have grown
        assert!(term.scrollback.len() > 0);
        // Scrollback should not exceed max (1000)
        assert!(term.scrollback.len() <= 1000);
    }
}
