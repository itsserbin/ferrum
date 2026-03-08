use crate::core::{
    Color, GraphemeCell, PageCoord, PageList, SecurityConfig,
    SecurityEventKind, TrackedPin, UnderlineStyle,
};
use unicode_width::UnicodeWidthChar;
use vte::{Params, Parser, Perform};

mod alt_screen;
mod grid_ops;
mod handlers;
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
    // ── Primary state (PageList + TrackedPin) ────────────────────────────────
    pub screen: PageList,
    cursor_pin: TrackedPin,
    saved_cursor: PageCoord,
    alt_saved_cursor: PageCoord,
    alt_screen: Option<PageList>,

    // ── Colour and attribute state ───────────────────────────────────────────
    current_fg: Color,
    current_bg: Color,
    pub default_fg: Color,
    pub default_bg: Color,
    pub ansi_palette: [Color; 16],
    current_bold: bool,
    current_dim: bool,
    current_italic: bool,
    current_reverse: bool,
    current_underline_style: UnderlineStyle,
    current_strikethrough: bool,

    // ── Scroll region ────────────────────────────────────────────────────────
    scroll_top: usize,
    scroll_bottom: usize,
    saved_scroll_top: usize,
    saved_scroll_bottom: usize,

    // ── Limits ───────────────────────────────────────────────────────────────
    pub max_scrollback: usize,

    // ── Mode flags ───────────────────────────────────────────────────────────
    pub decckm: bool,               // Application Cursor Key Mode (ESC[?1h/l)
    pub cursor_visible: bool,       // DECTCEM (mode 25)
    pub pending_responses: Vec<u8>, // Bytes queued for PTY replies.
    pub mouse_mode: MouseMode,
    pub sgr_mouse: bool,
    pub bracketed_paste: bool,
    pub focus_reporting: bool,
    pub security_config: SecurityConfig,
    pending_security_events: Vec<SecurityEventKind>,
    pub cursor_style: CursorStyle,
    pub resize_at: Option<std::time::Instant>,

    // ── Selection pins ───────────────────────────────────────────────────────
    pub selection_start_pin: Option<TrackedPin>,
    pub selection_end_pin: Option<TrackedPin>,

    // ── Misc ─────────────────────────────────────────────────────────────────
    parser: Parser,
    pub cwd: Option<String>,
    /// Window/icon title set by OSC 0/1/2.
    pub title: Option<String>,
}

impl Terminal {
    pub fn new(rows: usize, cols: usize) -> Self {
        let palette = crate::config::ThemeChoice::FerrumDark.resolve();
        Self::with_config(rows, cols, 1000, Color::SENTINEL_FG, Color::SENTINEL_BG, palette.ansi)
    }

    pub fn with_config(
        rows: usize,
        cols: usize,
        max_scrollback: usize,
        default_fg: Color,
        default_bg: Color,
        ansi_palette: [Color; 16],
    ) -> Self {
        let screen = PageList::new(rows, cols, max_scrollback);
        let cursor_pin = PageList::pin_at(PageCoord { abs_row: 0, col: 0 });
        Self {
            screen,
            cursor_pin,
            saved_cursor: PageCoord { abs_row: 0, col: 0 },
            alt_saved_cursor: PageCoord { abs_row: 0, col: 0 },
            alt_screen: None,
            current_fg: default_fg,
            current_bg: default_bg,
            default_fg,
            default_bg,
            ansi_palette,
            current_bold: false,
            current_dim: false,
            current_italic: false,
            current_reverse: false,
            current_underline_style: UnderlineStyle::None,
            current_strikethrough: false,
            scroll_top: 0,
            scroll_bottom: rows - 1,
            saved_scroll_top: 0,
            saved_scroll_bottom: rows - 1,
            max_scrollback,
            decckm: false,
            cursor_visible: true,
            pending_responses: Vec::new(),
            mouse_mode: MouseMode::Off,
            sgr_mouse: false,
            bracketed_paste: false,
            focus_reporting: false,
            security_config: SecurityConfig::default(),
            pending_security_events: Vec::new(),
            cursor_style: CursorStyle::default(),
            resize_at: None,
            selection_start_pin: None,
            selection_end_pin: None,
            parser: Parser::new(),
            cwd: None,
            title: None,
        }
    }

    /// Palette-aware 256-color lookup: indices 0-15 use the current ANSI palette,
    /// 16-231 use the 6x6x6 color cube, 232-255 use the grayscale ramp.
    pub fn color_from_256(&self, n: u16) -> Color {
        match n {
            0..=15 => self.ansi_palette[n as usize],
            _ => Color::from_256(n),
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

    /// Creates a blank `GraphemeCell` that inherits the current fg/bg.
    pub(crate) fn make_blank_grapheme_cell(&self) -> GraphemeCell {
        let mut gc = GraphemeCell::default();
        gc.fg = self.current_fg;
        gc.bg = self.current_bg;
        gc
    }

    /// Clears the screen and scrollback, resetting cursor to (0,0).
    ///
    /// Unlike `full_reset()`, this preserves terminal attributes and mode flags.
    pub fn clear_screen(&mut self) {
        let rows = self.screen.viewport_rows();
        let cols = self.screen.cols();
        let max_sb = self.max_scrollback;
        let new_screen = PageList::new(rows, cols, max_sb);
        self.cursor_pin = PageList::pin_at(PageCoord { abs_row: 0, col: 0 });
        self.screen = new_screen;
        self.reset_scroll_region();
    }

    /// Drains all pending PTY response bytes.
    pub fn drain_responses(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending_responses)
    }

    /// Registers a tracked selection-start pin at the given absolute row/col.
    pub fn set_selection_start(&mut self, abs_row: usize, col: usize) {
        let pin = self.make_selection_pin(abs_row, col);
        self.selection_start_pin = Some(pin);
    }

    /// Registers a tracked selection-end pin at the given absolute row/col.
    pub fn set_selection_end(&mut self, abs_row: usize, col: usize) {
        let pin = self.make_selection_pin(abs_row, col);
        self.selection_end_pin = Some(pin);
    }

    /// Creates a tracked pin at the given absolute row/col.
    fn make_selection_pin(&mut self, abs_row: usize, col: usize) -> TrackedPin {
        let coord = PageCoord { abs_row, col };
        PageList::pin_at(coord)
    }

    /// Clears both selection tracking pins.
    pub fn clear_selection_pins(&mut self) {
        self.selection_start_pin = None;
        self.selection_end_pin = None;
    }

    /// Resets the scroll region to span the entire visible grid.
    pub fn reset_scroll_region(&mut self) {
        self.scroll_top = 0;
        self.scroll_bottom = self.screen.viewport_rows().saturating_sub(1);
    }

    /// Drains security events detected while parsing terminal output.
    pub fn drain_security_events(&mut self) -> Vec<SecurityEventKind> {
        std::mem::take(&mut self.pending_security_events)
    }

    /// Clears transient mouse-tracking state when the PTY process exits.
    pub fn cleanup_after_process_exit(&mut self) {
        self.clear_mouse_tracking(self.security_config.clear_mouse_on_reset);
        self.focus_reporting = false;
        self.bracketed_paste = false;
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

    fn set_underline_style(&mut self, style: UnderlineStyle) {
        self.current_underline_style = style;
    }

    fn set_dim(&mut self, value: bool) {
        self.current_dim = value;
    }

    fn set_italic(&mut self, value: bool) {
        self.current_italic = value;
    }

    fn set_strikethrough(&mut self, value: bool) {
        self.current_strikethrough = value;
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

    fn set_bracketed_paste(&mut self, enabled: bool) {
        self.bracketed_paste = enabled;
    }

    fn set_focus_reporting(&mut self, enabled: bool) {
        self.focus_reporting = enabled;
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

    /// Grace period (in seconds) after a resize during which upward cursor jumps
    /// are not flagged as security events (shells legitimately redraw prompts).
    pub(crate) const RESIZE_CURSOR_JUMP_GRACE_SECS: u64 = 2;

    /// Returns the viewport-relative cursor row (derived from `cursor_pin`).
    pub fn cursor_row(&self) -> usize {
        self.cursor_pin
            .coord()
            .abs_row
            .saturating_sub(self.screen.viewport_start_abs())
    }

    /// Returns the cursor column (derived from `cursor_pin`).
    pub fn cursor_col(&self) -> usize {
        self.cursor_pin.coord().col
    }

    /// Sets the cursor row and keeps `cursor_pin` in sync.
    pub fn set_cursor_row(&mut self, row: usize) {
        let abs = self.screen.viewport_start_abs() + row;
        self.cursor_pin.set_abs_row(abs);
    }

    /// Sets the cursor column and keeps `cursor_pin` in sync.
    pub fn set_cursor_col(&mut self, col: usize) {
        self.cursor_pin.set_col(col);
    }

    /// Sets both cursor row and column, keeping `cursor_pin` in sync.
    pub fn set_cursor(&mut self, row: usize, col: usize) {
        self.set_cursor_row(row);
        self.set_cursor_col(col);
    }

    fn maybe_record_cursor_rewrite(&mut self, from_row: usize, to_row: usize) {
        if !self.security_config.limit_cursor_jumps || self.is_alt_screen() || to_row >= from_row {
            return;
        }

        // Suppress false positives: shell redraws prompt after resize.
        if self
            .resize_at
            .is_some_and(|t| t.elapsed().as_secs() < Self::RESIZE_CURSOR_JUMP_GRACE_SECS)
        {
            return;
        }

        // Check if to_row is within bounds
        if to_row >= self.screen.viewport_rows() {
            return;
        }
        let row_has_content =
            (0..self.screen.cols()).any(|col| self.screen.viewport_get(to_row, col).first_char() != ' ');
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

    /// Recolors all cells when the theme changes.
    ///
    /// Maps old default/ANSI colors to new ones; leaves custom SGR colors untouched.
    pub fn recolor(
        &mut self,
        old_fg: Color,
        old_bg: Color,
        old_ansi: &[Color; 16],
        new_fg: Color,
        new_bg: Color,
        new_ansi: &[Color; 16],
    ) {
        let remap = |color: Color| -> Color {
            if color == old_fg {
                return new_fg;
            }
            if color == old_bg {
                return new_bg;
            }
            for (i, old_c) in old_ansi.iter().enumerate() {
                if color == *old_c {
                    return new_ansi[i];
                }
            }
            color
        };

        self.screen.viewport_recolor(|gc: &mut GraphemeCell| {
            gc.fg = remap(gc.fg);
            gc.bg = remap(gc.bg);
        });
        self.screen.scrollback_recolor(|gc| {
            gc.fg = remap(gc.fg);
            gc.bg = remap(gc.bg);
        });
        if let Some(ref mut alt) = self.alt_screen {
            alt.viewport_recolor(|gc| {
                gc.fg = remap(gc.fg);
                gc.bg = remap(gc.bg);
            });
        }

        self.current_fg = remap(self.current_fg);
        self.current_bg = remap(self.current_bg);
        self.default_fg = new_fg;
        self.default_bg = new_bg;
        self.ansi_palette = *new_ansi;
    }

    pub fn full_reset(&mut self) {
        let rows = self.screen.viewport_rows();
        let cols = self.screen.cols();
        let max_scrollback = self.max_scrollback;

        self.alt_screen = None;
        self.saved_cursor = PageCoord { abs_row: 0, col: 0 };
        self.alt_saved_cursor = PageCoord { abs_row: 0, col: 0 };
        self.reset_scroll_region();
        self.saved_scroll_top = 0;
        self.saved_scroll_bottom = rows.saturating_sub(1);
        self.decckm = false;
        self.cursor_visible = true;
        self.cursor_style = CursorStyle::default();
        self.pending_responses.clear();
        self.bracketed_paste = false;
        if self.security_config.clear_mouse_on_reset {
            self.clear_mouse_tracking(true);
        }
        self.cwd = None;
        self.reset_attributes();
        self.parser = Parser::new();
        let screen = PageList::new(rows, cols, max_scrollback);
        self.cursor_pin = PageList::pin_at(PageCoord { abs_row: 0, col: 0 });
        self.screen = screen;
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
        let from_row = self.cursor_row();
        let handled = handlers::cursor::handle_cursor_csi(self, action, params);
        if handled {
            self.maybe_record_cursor_rewrite(from_row, self.cursor_row());
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

        if self.cursor_col() + width > self.screen.cols() {
            // Mark current row as soft-wrapped before moving to next row.
            let cr = self.cursor_row();
            self.screen.viewport_set_wrapped(cr, true);
            self.set_cursor_col(0);
            let next_row = self.cursor_row() + 1;
            if next_row > self.scroll_bottom {
                self.scroll_up_region(self.scroll_top, self.scroll_bottom);
                self.set_cursor_row(self.scroll_bottom);
            } else {
                self.set_cursor_row(next_row);
            }
        }

        let cr = self.cursor_row();
        let cc = self.cursor_col();
        let mut gc = GraphemeCell::from_char(c);
        gc.fg = self.current_fg;
        gc.bg = self.current_bg;
        gc.bold = self.current_bold;
        gc.dim = self.current_dim;
        gc.italic = self.current_italic;
        gc.reverse = self.current_reverse;
        gc.underline_style = self.current_underline_style;
        gc.strikethrough = self.current_strikethrough;
        self.screen.viewport_set(cr, cc, gc);

        // Reserve the trailing cell for wide glyphs.
        if width == 2 && cc + 1 < self.screen.cols() {
            let spacer_gc = GraphemeCell::spacer();
            self.screen.viewport_set(cr, cc + 1, spacer_gc);
        }

        self.set_cursor_col(cc + width);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            10..=12 => {
                // LF/VT/FF: move to next row, keep current column.
                // Mark current row as NOT wrapped (hard line break).
                let cr = self.cursor_row();
                self.screen.viewport_set_wrapped(cr, false);
                let next_row = cr + 1;
                if next_row > self.scroll_bottom {
                    self.scroll_up_region(self.scroll_top, self.scroll_bottom);
                    self.set_cursor_row(self.scroll_bottom);
                } else {
                    self.set_cursor_row(next_row);
                }
            }
            13 => {
                self.set_cursor_col(0);
            }
            8 => {
                if self.cursor_col() > 0 {
                    self.set_cursor_col(self.cursor_col() - 1);
                }
            }
            9 => {
                const DEFAULT_TAB_WIDTH: usize = 8;
                let new_col =
                    (self.cursor_col() + DEFAULT_TAB_WIDTH) & !(DEFAULT_TAB_WIDTH - 1);
                self.set_cursor_col(new_col.min(self.screen.cols() - 1));
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
        match params[0] {
            // OSC 0: set window title + icon name
            // OSC 1: set icon name (treat as title)
            // OSC 2: set window title
            b"0" | b"1" | b"2" => {
                if params.len() >= 2 {
                    self.title = Some(String::from_utf8_lossy(params[1]).into_owned());
                }
            }
            // OSC 7: CWD notification
            b"7" => {
                if params.len() < 2 || params[1].is_empty() {
                    self.cwd = None;
                    return;
                }
                let uri = String::from_utf8_lossy(params[1]);
                self.cwd = parse_osc7_uri(&uri);
            }
            _ => {}
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
        self.handle_device_csi(action, params, intermediates);
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'7' => {
                self.saved_cursor = PageCoord {
                    abs_row: self.cursor_pin.coord().abs_row,
                    col: self.cursor_col(),
                };
            }
            b'8' => {
                let from_row = self.cursor_row();
                let vstart = self.screen.viewport_start_abs();
                let row = self.saved_cursor.abs_row.saturating_sub(vstart)
                    .min(self.screen.viewport_rows().saturating_sub(1));
                let col = self.saved_cursor.col.min(self.screen.cols().saturating_sub(1));
                self.set_cursor(row, col);
                self.maybe_record_cursor_rewrite(from_row, self.cursor_row());
            }
            b'M' => {
                let from_row = self.cursor_row();
                // Reverse Index: cursor up, scroll down if at top of region
                if self.cursor_row() == self.scroll_top {
                    self.scroll_down_region(self.scroll_top, self.scroll_bottom);
                } else {
                    self.set_cursor_row(self.cursor_row().saturating_sub(1));
                }
                self.maybe_record_cursor_rewrite(from_row, self.cursor_row());
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
    // On Windows, URI path is "/C:\Users\..." — strip leading slash before drive letter
    #[cfg(target_os = "windows")]
    {
        if decoded.len() >= 3 && decoded.as_bytes()[0] == b'/' && decoded.as_bytes()[2] == b':' {
            return Some(decoded[1..].replace('/', "\\"));
        }
    }
    Some(decoded)
}

fn percent_decode(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let mut iter = input.bytes();
    while let Some(b) = iter.next() {
        if b == b'%' {
            if let (Some(h), Some(l)) = (iter.next(), iter.next()) {
                if let (Some(hv), Some(lv)) = (hex_val(h), hex_val(l)) {
                    bytes.push(hv << 4 | lv);
                    continue;
                }
                bytes.extend_from_slice(&[b'%', h, l]);
            } else {
                bytes.push(b'%');
            }
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
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
