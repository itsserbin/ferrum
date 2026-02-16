use crate::core::{Cell, Color, Grid, SecurityConfig, SecurityEventKind};
use std::collections::VecDeque;
use unicode_width::UnicodeWidthChar;
use vte::{Params, Parser, Perform};

mod handlers;

/// Cursor style reported by DECSCUSR.
#[derive(Default, Clone, Copy, PartialEq)]
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
#[derive(Default, Clone, Copy, PartialEq)]
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

    pub fn resize(&mut self, rows: usize, cols: usize) {
        if self.grid.rows == rows && self.grid.cols == cols {
            return;
        }

        let old_rows = self.grid.rows;
        let is_alt = self.alt_grid.is_some();

        // ── Vertical shrink: cursor would be outside new grid ──
        // Shift content up, pushing top rows into scrollback.
        if rows < old_rows && self.cursor_row >= rows {
            let shift = self.cursor_row - rows + 1;
            // Save the top `shift` rows to scrollback (main screen only)
            if !is_alt {
                for r in 0..shift {
                    let row_cells = self.grid.row_cells(r);
                    self.scrollback.push_back(row_cells);
                    if self.scrollback.len() > self.max_scrollback {
                        self.scrollback.pop_front();
                    }
                }
            }
            self.grid.shift_up(shift);
            self.cursor_row -= shift;
        }

        // ── Resize the grid (copies content, pads/truncates) ──
        self.grid = self.grid.resized(rows, cols);

        // ── Vertical grow: pull lines from scrollback to fill new top rows ──
        if rows > old_rows && !is_alt && !self.scrollback.is_empty() {
            let available = rows - old_rows; // new empty rows at bottom
            let pull = available.min(self.scrollback.len());
            // Shift existing content down to make room at top
            self.grid.shift_down(pull);
            // Fill top rows from scrollback
            for i in 0..pull {
                let sb_row = self.scrollback.pop_back().unwrap();
                // Rows are pulled in reverse: last popped goes to row 0
                self.grid.set_row(pull - 1 - i, sb_row);
            }
            self.cursor_row += pull;
        }

        // ── Alt grid: simple resize (no scrollback interaction) ──
        if let Some(ref mut alt) = self.alt_grid {
            *alt = alt.resized(rows, cols);
        }

        // ── Clamp cursor to valid bounds ──
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));

        // ── Reset scroll region to full screen ──
        self.scroll_top = 0;
        self.scroll_bottom = rows - 1;

        self.resize_at = Some(std::time::Instant::now());
    }

    /// Returns whether the terminal is in the alternate screen.
    pub fn is_alt_screen(&self) -> bool {
        self.alt_grid.is_some()
    }

    /// Builds a display grid by combining scrollback with the visible grid.
    /// Called only when `scroll_offset > 0`, so the copy cost is acceptable.
    pub fn build_display(&self, scroll_offset: usize) -> Grid {
        let scroll_offset = scroll_offset.min(self.scrollback.len());
        let mut display = Grid::new(self.grid.rows, self.grid.cols);
        for row in 0..self.grid.rows {
            for col in 0..self.grid.cols {
                let cell = if row < scroll_offset {
                    // Pull row from scrollback.
                    let sb_idx = self.scrollback.len() - scroll_offset + row;
                    if col < self.scrollback[sb_idx].len() {
                        self.scrollback[sb_idx][col].clone()
                    } else {
                        Cell::default() // Width may differ after resize.
                    }
                } else {
                    // Pull row from the live grid.
                    self.grid.get(row - scroll_offset, col).clone()
                };
                display.set(row, col, cell);
            }
        }
        display
    }

    fn scroll_up_region(&mut self, top: usize, bottom: usize) {
        // Persist the top row to scrollback only for the main screen.
        if top == 0 && self.alt_grid.is_none() {
            let row: Vec<Cell> = (0..self.grid.cols)
                .map(|col| self.grid.get(0, col).clone())
                .collect();
            self.scrollback.push_back(row);
            if self.scrollback.len() > self.max_scrollback {
                self.scrollback.pop_front();
            }
        }

        for row in (top + 1)..=bottom {
            for col in 0..self.grid.cols {
                let cell = self.grid.get(row, col).clone();
                self.grid.set(row - 1, col, cell);
            }
        }
        for col in 0..self.grid.cols {
            self.grid.set(bottom, col, Cell::default());
        }
    }

    fn scroll_down_region(&mut self, top: usize, bottom: usize) {
        for row in (top..bottom).rev() {
            for col in 0..self.grid.cols {
                let cell = self.grid.get(row, col).clone();
                self.grid.set(row + 1, col, cell);
            }
        }
        for col in 0..self.grid.cols {
            self.grid.set(top, col, Cell::default());
        }
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

    /// Enters the alternate screen buffer (used by apps like vim/htop).
    fn enter_alt_screen(&mut self) {
        if self.alt_grid.is_none() {
            let alt = Grid::new(self.grid.rows, self.grid.cols);
            self.alt_grid = Some(std::mem::replace(&mut self.grid, alt));
            self.alt_saved_cursor = (self.cursor_row, self.cursor_col);
            self.saved_scroll_top = self.scroll_top;
            self.saved_scroll_bottom = self.scroll_bottom;
            self.cursor_row = 0;
            self.cursor_col = 0;
            self.cursor_style = CursorStyle::BlinkingBlock;
            self.scroll_top = 0;
            self.scroll_bottom = self.grid.rows - 1;
        }
    }

    /// Leaves the alternate screen and restores the main buffer.
    fn leave_alt_screen(&mut self) {
        if let Some(main_grid) = self.alt_grid.take() {
            self.grid = main_grid;
            self.cursor_row = self.alt_saved_cursor.0;
            self.cursor_col = self.alt_saved_cursor.1;
            self.scroll_top = self.saved_scroll_top;
            self.scroll_bottom = self.saved_scroll_bottom;
            self.cursor_style = CursorStyle::default();
        }
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
#[path = "../../tests/unit/core_terminal.rs"]
mod tests;
