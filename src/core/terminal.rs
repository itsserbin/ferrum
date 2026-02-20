use crate::core::{Cell, Color, Grid, Row, SecurityConfig, SecurityEventKind};
use std::collections::VecDeque;
use unicode_width::UnicodeWidthChar;
use vte::{Params, Parser, Perform};

mod alt_screen;
mod grid_ops;
mod handlers;
mod reflow;
mod resize;

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
    pub scrollback: VecDeque<Row>,
    max_scrollback: usize,
    pub decckm: bool,               // Application Cursor Key Mode (ESC[?1h/l)
    pub cursor_visible: bool,       // DECTCEM (mode 25)
    pub pending_responses: Vec<u8>, // Bytes queued for PTY replies.
    pub mouse_mode: MouseMode,
    pub sgr_mouse: bool,
    pub security_config: SecurityConfig,
    pending_security_events: Vec<SecurityEventKind>,
    pub cursor_style: CursorStyle,
    pub resize_at: Option<std::time::Instant>,
    scrollback_popped: usize,
    parser: Parser,
    pub cwd: Option<String>,
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
            scrollback_popped: 0,
            parser: Parser::new(),
            cwd: None,
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

    /// Returns and resets the accumulated scrollback-popped counter.
    pub fn drain_scrollback_popped(&mut self) -> usize {
        std::mem::take(&mut self.scrollback_popped)
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

        // Check if to_row is within bounds
        if to_row >= self.grid.rows {
            return;
        }
        let row_has_content =
            // Safe: col < grid.cols and to_row < grid.rows checked above
            (0..self.grid.cols).any(|col| self.grid.get_unchecked(to_row, col).character != ' ');
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
        self.cwd = None;
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
        let mut width = UnicodeWidthChar::width(c).unwrap_or(1);
        if width == 0 {
            // Keep combining marks visible as standalone glyphs instead of dropping them.
            width = 1;
        }

        if self.cursor_col + width > self.grid.cols {
            // Mark current row as soft-wrapped before moving to next row
            self.grid.set_wrapped(self.cursor_row, true);
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
            10..=12 => {
                // LF/VT/FF: move to next row, keep current column.
                // Mark current row as NOT wrapped (hard line break).
                self.grid.set_wrapped(self.cursor_row, false);
                self.cursor_row += 1;
                if self.cursor_row > self.scroll_bottom {
                    self.scroll_up_region(self.scroll_top, self.scroll_bottom);
                    self.cursor_row = self.scroll_bottom;
                }
            }
            13 => {
                self.cursor_col = 0;
            }
            8 => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            9 => {
                const DEFAULT_TAB_WIDTH: usize = 8;
                self.cursor_col = (self.cursor_col + DEFAULT_TAB_WIDTH) & !(DEFAULT_TAB_WIDTH - 1);
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
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() {
            return;
        }
        if params[0] == b"7" {
            if params.len() < 2 || params[1].is_empty() {
                self.cwd = None;
                return;
            }
            let uri = String::from_utf8_lossy(params[1]);
            self.cwd = parse_osc7_uri(&uri);
        }
    }
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

fn parse_osc7_uri(uri: &str) -> Option<String> {
    let after_scheme = uri
        .strip_prefix("file://")
        .or_else(|| uri.strip_prefix("kitty-shell-cwd://"))?;
    let (hostname, path) = if let Some(idx) = after_scheme.find('/') {
        (&after_scheme[..idx], &after_scheme[idx..])
    } else {
        return None;
    };
    if !hostname.is_empty() && hostname != "localhost" {
        let local = gethostname::gethostname();
        if hostname != local.to_string_lossy().as_ref() {
            return None;
        }
    }
    let decoded = percent_decode(path);
    if decoded.is_empty() {
        return None;
    }
    Some(decoded)
}

fn percent_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(h), Some(l)) = (hi, lo) {
                if let (Some(hv), Some(lv)) = (hex_val(h), hex_val(l)) {
                    result.push((hv << 4 | lv) as char);
                    continue;
                }
            }
            result.push('%');
        } else {
            result.push(b as char);
        }
    }
    result
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/core_terminal.rs"]
mod tests;
