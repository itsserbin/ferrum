use super::*;

// Catppuccin Mocha palette — flat Chrome-style tab bar.
const BAR_BG: u32 = 0x181825;              // Mantle — bar background
const ACTIVE_TAB_BG: u32 = 0x1E1E2E;      // Base — merges with terminal
const INACTIVE_TAB_HOVER: u32 = 0x313244;  // Surface0
const TAB_TEXT_ACTIVE: u32 = 0xCDD6F4;     // Text
const TAB_TEXT_INACTIVE: u32 = 0x6C7086;   // Overlay0
const TAB_BORDER: u32 = 0x313244;          // Surface0
const CLOSE_HOVER_BG_COLOR: u32 = 0xF38BA8; // Red

// Window button colors (non-macOS).
#[cfg(not(target_os = "macos"))]
const WIN_BTN_ICON: u32 = 0x6C7086;        // Overlay0
#[cfg(not(target_os = "macos"))]
const WIN_BTN_HOVER: u32 = 0x313244;       // Surface0
#[cfg(not(target_os = "macos"))]
const WIN_BTN_CLOSE_HOVER: u32 = 0xF38BA8; // Red
#[cfg(not(target_os = "macos"))]
const WIN_BTN_WIDTH: u32 = 46;

// Insertion indicator color (Catppuccin Mocha Mauve).
const INSERTION_COLOR: u32 = 0xCBA6F7;

impl Renderer {
    /// Computes adaptive tab width with overflow compression.
    /// Tabs shrink from max (240px) down to MIN_TAB_WIDTH when many tabs are open.
    pub fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32 {
        let reserved = self.tab_strip_start_x()
            + self.plus_button_reserved_width()
            + self.scaled_px(8)
            + self.window_buttons_reserved_width();
        let available = buf_width.saturating_sub(reserved);
        let min_tab_width = self.scaled_px(MIN_TAB_WIDTH);
        let max_tab_width = self.scaled_px(240);
        (available / tab_count.max(1) as u32).clamp(min_tab_width, max_tab_width)
    }

    pub(crate) fn tab_strip_start_x(&self) -> u32 {
        #[cfg(target_os = "macos")]
        {
            self.scaled_px(70)
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.scaled_px(8)
        }
    }

    fn plus_button_reserved_width(&self) -> u32 {
        self.cell_width + self.scaled_px(20)
    }

    /// Returns total width reserved for window control buttons.
    fn window_buttons_reserved_width(&self) -> u32 {
        #[cfg(not(target_os = "macos"))]
        {
            self.scaled_px(WIN_BTN_WIDTH) * 3
        }
        #[cfg(target_os = "macos")]
        {
            0
        }
    }

    pub(crate) fn tab_origin_x(&self, tab_index: usize, tw: u32) -> u32 {
        self.tab_strip_start_x() + tab_index as u32 * tw
    }

    pub(crate) fn tab_insert_index_from_x(
        &self,
        x: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> usize {
        let tw = self.tab_width(tab_count, buf_width);
        let start = self.tab_strip_start_x() as f64;
        let mut idx = tab_count;
        for i in 0..tab_count {
            let center = start + i as f64 * tw as f64 + tw as f64 / 2.0;
            if x < center {
                idx = i;
                break;
            }
        }
        idx
    }

    /// Returns rectangle for per-tab close button.
    fn close_button_rect(&self, tab_index: usize, tw: u32) -> (u32, u32, u32, u32) {
        let btn_w = self.cell_width + self.scaled_px(8);
        let x = self.tab_origin_x(tab_index, tw) + tw - btn_w - self.scaled_px(6);
        let y = (self.tab_bar_height_px().saturating_sub(self.cell_height)) / 2;
        (x, y, btn_w, self.cell_height)
    }

    /// Returns rectangle for new-tab button.
    fn plus_button_rect(&self, tab_count: usize, tw: u32) -> (u32, u32, u32, u32) {
        let x = self.tab_strip_start_x() + tab_count as u32 * tw + self.scaled_px(4);
        let y = (self.tab_bar_height_px().saturating_sub(self.cell_height)) / 2;
        (x, y, self.cell_width + self.scaled_px(8), self.cell_height)
    }

    /// Hit-tests the tab bar and returns the clicked target.
    pub fn hit_test_tab_bar(&self, x: f64, y: f64, tab_count: usize, buf_width: u32) -> TabBarHit {
        if y >= self.tab_bar_height_px() as f64 {
            return TabBarHit::Empty;
        }

        // Window buttons (non-macOS) have highest priority.
        #[cfg(not(target_os = "macos"))]
        if let Some(btn) = self.window_button_at_position(x, y, buf_width) {
            return TabBarHit::WindowButton(btn);
        }

        let tw = self.tab_width(tab_count, buf_width);
        let tab_strip_start = self.tab_strip_start_x();

        // New-tab button has priority over tab body hit-test.
        let (px, py, pw, ph) = self.plus_button_rect(tab_count, tw);
        if x >= px as f64 && x < (px + pw) as f64 && y >= py as f64 && y < (py + ph) as f64 {
            return TabBarHit::NewTab;
        }

        if x < tab_strip_start as f64 {
            return TabBarHit::Empty;
        }

        let rel_x = x as u32 - tab_strip_start;
        let tab_index = rel_x / tw;
        if (tab_index as usize) < tab_count {
            let idx = tab_index as usize;
            let (cx, cy, cw, ch) = self.close_button_rect(idx, tw);
            if x >= cx as f64 && x < (cx + cw) as f64 && y >= cy as f64 && y < (cy + ch) as f64 {
                return TabBarHit::CloseTab(idx);
            }
            return TabBarHit::Tab(idx);
        }

        TabBarHit::Empty
    }

    /// Hit-tests tab hover target (without button checks).
    pub fn hit_test_tab_hover(
        &self,
        x: f64,
        y: f64,
        tab_count: usize,
        buf_width: u32,
    ) -> Option<usize> {
        if y >= self.tab_bar_height_px() as f64 || tab_count == 0 {
            return None;
        }
        let tw = self.tab_width(tab_count, buf_width);
        let tab_strip_start = self.tab_strip_start_x();
        if x < tab_strip_start as f64 {
            return None;
        }
        let rel_x = x as u32 - tab_strip_start;
        let idx = rel_x / tw;
        if (idx as usize) < tab_count {
            Some(idx as usize)
        } else {
            None
        }
    }

    /// Returns tab index when pointer is over a security badge.
    pub fn hit_test_tab_security_badge(
        &self,
        x: f64,
        y: f64,
        tabs: &[TabInfo],
        buf_width: u32,
    ) -> Option<usize> {
        for (idx, tab) in tabs.iter().enumerate() {
            if tab.security_count == 0 {
                continue;
            }
            let Some((sx, sy, sw, sh)) =
                self.security_badge_rect(idx, tabs.len(), buf_width, tab.security_count)
            else {
                continue;
            };
            if x >= sx as f64 && x < (sx + sw) as f64 && y >= sy as f64 && y < (sy + sh) as f64 {
                return Some(idx);
            }
        }
        None
    }

    /// Hit-test window control buttons (non-macOS only).
    #[cfg(not(target_os = "macos"))]
    pub fn window_button_at_position(&self, x: f64, y: f64, buf_width: u32) -> Option<WindowButton> {
        let bar_h = self.tab_bar_height_px();
        if y >= bar_h as f64 {
            return None;
        }
        let btn_w = self.scaled_px(WIN_BTN_WIDTH);
        let close_x = buf_width.saturating_sub(btn_w);
        let min_x = buf_width.saturating_sub(btn_w * 2);
        let minimize_x = buf_width.saturating_sub(btn_w * 3);

        if x >= close_x as f64 && x < buf_width as f64 {
            Some(WindowButton::Close)
        } else if x >= min_x as f64 && x < (min_x + btn_w) as f64 {
            Some(WindowButton::Maximize)
        } else if x >= minimize_x as f64 && x < (minimize_x + btn_w) as f64 {
            Some(WindowButton::Minimize)
        } else {
            None
        }
    }

    fn blend_rgb(dst: u32, src: u32, alpha: u8) -> u32 {
        if alpha == 255 {
            return src;
        }
        if alpha == 0 {
            return dst;
        }

        let a = alpha as u32;
        let inv = 255 - a;

        let dr = (dst >> 16) & 0xFF;
        let dg = (dst >> 8) & 0xFF;
        let db = dst & 0xFF;

        let sr = (src >> 16) & 0xFF;
        let sg = (src >> 8) & 0xFF;
        let sb = src & 0xFF;

        let r = (sr * a + dr * inv + 127) / 255;
        let g = (sg * a + dg * inv + 127) / 255;
        let b = (sb * a + db * inv + 127) / 255;

        (r << 16) | (g << 8) | b
    }

    /// Draws a filled circle at a given center with a given radius.
    fn draw_filled_circle(
        buffer: &mut [u32],
        buf_w: usize,
        cx: i32,
        cy: i32,
        radius: u32,
        color: u32,
    ) {
        if buf_w == 0 || buffer.is_empty() || radius == 0 {
            return;
        }

        let buf_h = buffer.len() / buf_w;
        if buf_h == 0 {
            return;
        }

        let r = radius as f32;
        let min_x = (cx - radius as i32 - 1).max(0);
        let max_x = (cx + radius as i32 + 1).min(buf_w as i32 - 1);
        let min_y = (cy - radius as i32 - 1).max(0);
        let max_y = (cy + radius as i32 + 1).min(buf_h as i32 - 1);

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px as f32 + 0.5 - cx as f32;
                let dy = py as f32 + 0.5 - cy as f32;
                let dist = (dx * dx + dy * dy).sqrt();

                let coverage = (r + 0.5 - dist).clamp(0.0, 1.0);
                if coverage <= 0.0 {
                    continue;
                }

                let idx = py as usize * buf_w + px as usize;
                if idx >= buffer.len() {
                    continue;
                }

                let alpha = (coverage * 255.0).round() as u8;
                buffer[idx] = Self::blend_rgb(buffer[idx], color, alpha);
            }
        }
    }

    fn point_in_rect(x: f64, y: f64, rect: (u32, u32, u32, u32)) -> bool {
        let (rx, ry, rw, rh) = rect;
        x >= rx as f64 && x < (rx + rw) as f64 && y >= ry as f64 && y < (ry + rh) as f64
    }

    fn point_to_segment_distance(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
        let vx = x1 - x0;
        let vy = y1 - y0;
        let len_sq = vx * vx + vy * vy;
        if len_sq <= f32::EPSILON {
            return ((px - x0) * (px - x0) + (py - y0) * (py - y0)).sqrt();
        }

        let t = (((px - x0) * vx + (py - y0) * vy) / len_sq).clamp(0.0, 1.0);
        let proj_x = x0 + t * vx;
        let proj_y = y0 + t * vy;
        ((px - proj_x) * (px - proj_x) + (py - proj_y) * (py - proj_y)).sqrt()
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_stroked_line(
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        thickness: f32,
        color: u32,
    ) {
        if thickness <= 0.0 || buf_width == 0 || buf_height == 0 {
            return;
        }

        let half = thickness * 0.5;
        let min_x = ((x0.min(x1) - half - 1.0).floor() as i32).max(0);
        let max_x = ((x0.max(x1) + half + 1.0).ceil() as i32).min(buf_width as i32 - 1);
        let min_y = ((y0.min(y1) - half - 1.0).floor() as i32).max(0);
        let max_y = ((y0.max(y1) + half + 1.0).ceil() as i32).min(buf_height as i32 - 1);

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let pcx = px as f32 + 0.5;
                let pcy = py as f32 + 0.5;
                let dist = Self::point_to_segment_distance(pcx, pcy, x0, y0, x1, y1);

                let coverage = (half + 0.5 - dist).clamp(0.0, 1.0);
                if coverage <= 0.0 {
                    continue;
                }

                let idx = py as usize * buf_width + px as usize;
                if idx >= buffer.len() {
                    continue;
                }
                let alpha = (coverage * 255.0).round() as u8;
                buffer[idx] = Self::blend_rgb(buffer[idx], color, alpha);
            }
        }
    }

    fn draw_tab_plus_icon(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        rect: (u32, u32, u32, u32),
        color: Color,
    ) {
        let (x, y, w, h) = rect;
        let center_x = x as f32 + w as f32 * 0.5;
        let center_y = y as f32 + h as f32 * 0.5;
        let half = (self.cell_height as f32 / 6.0).clamp(2.5, 3.4);
        let thickness = (1.25_f32 * self.ui_scale() as f32).clamp(1.15, 2.2);
        let pixel = color.to_pixel();

        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x - half,
            center_y,
            center_x + half,
            center_y,
            thickness,
            pixel,
        );
        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x,
            center_y - half,
            center_x,
            center_y + half,
            thickness,
            pixel,
        );
    }

    fn draw_tab_close_icon(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        rect: (u32, u32, u32, u32),
        color: Color,
    ) {
        let (x, y, w, h) = rect;
        let center_x = x as f32 + w as f32 * 0.5;
        let center_y = y as f32 + h as f32 * 0.5;
        let half = (self.cell_height as f32 / 6.0).clamp(2.5, 3.4) * 0.86;
        let thickness = (1.25_f32 * self.ui_scale() as f32).clamp(1.15, 2.2);
        let pixel = color.to_pixel();

        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x - half,
            center_y - half,
            center_x + half,
            center_y + half,
            thickness,
            pixel,
        );
        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x + half,
            center_y - half,
            center_x - half,
            center_y + half,
            thickness,
            pixel,
        );
    }

    /// Returns true if the given tab width is too narrow to display the title.
    fn should_show_number(&self, tw: u32) -> bool {
        tw < self.scaled_px(MIN_TAB_WIDTH_FOR_TITLE)
    }

    fn title_max_chars(&self, tab: &TabInfo, tw: u32, is_hovered: bool) -> usize {
        let tab_padding_h = self.scaled_px(14);
        let show_close = tab.is_active || is_hovered;
        let close_reserved = if show_close {
            self.cell_width + self.scaled_px(8)
        } else {
            0
        };
        let security_reserved = if tab.security_count > 0 {
            let count_chars = tab.security_count.min(99).to_string().len() as u32;
            let count_width = if tab.security_count > 1 {
                count_chars * self.cell_width + self.scaled_px(2)
            } else {
                0
            };
            let badge_min = self.scaled_px(10);
            let badge_max = self.scaled_px(15);
            self.cell_height
                .saturating_sub(self.scaled_px(10))
                .clamp(badge_min, badge_max)
                + count_width
                + self.scaled_px(6)
        } else {
            0
        };
        (tw.saturating_sub(tab_padding_h * 2 + close_reserved + security_reserved)
            / self.cell_width) as usize
    }

    /// Returns full tab title when hover should show a tooltip (compressed or truncated label).
    pub fn tab_hover_tooltip<'a>(
        &self,
        tabs: &'a [TabInfo<'a>],
        hovered_tab: Option<usize>,
        buf_width: u32,
    ) -> Option<&'a str> {
        let idx = hovered_tab?;
        let tab = tabs.get(idx)?;
        if tab.is_renaming || tab.title.is_empty() {
            return None;
        }

        let tw = self.tab_width(tabs.len(), buf_width);
        if self.should_show_number(tw) {
            return Some(tab.title);
        }

        let max_chars = self.title_max_chars(tab, tw, true);
        let title_chars = tab.title.chars().count();
        (title_chars > max_chars).then_some(tab.title)
    }

    /// Draws a small tooltip with full tab title near the pointer.
    pub fn draw_tab_tooltip(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        mouse_pos: (f64, f64),
        title: &str,
    ) {
        if title.is_empty() || buf_width == 0 || buf_height == 0 {
            return;
        }

        let padding_x = self.scaled_px(6);
        let padding_y = self.scaled_px(4);
        let content_chars = title.chars().count() as u32;
        let width = (content_chars * self.cell_width + padding_x * 2 + self.scaled_px(2))
            .min(buf_width.saturating_sub(4) as u32);
        let height = (self.cell_height + padding_y * 2 + self.scaled_px(2)).min(buf_height as u32);
        if width <= self.scaled_px(2) || height <= self.scaled_px(2) {
            return;
        }

        let mut x = mouse_pos.0.round() as i32 + self.scaled_px(10) as i32;
        let mut y = self.tab_bar_height_px() as i32 + self.scaled_px(6) as i32;
        x = x
            .min(buf_width as i32 - width as i32 - self.scaled_px(2) as i32)
            .max(self.scaled_px(2) as i32);
        y = y
            .min(buf_height as i32 - height as i32 - self.scaled_px(2) as i32)
            .max(self.scaled_px(2) as i32);

        let radius = self.scaled_px(6);
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            x,
            y,
            width,
            height,
            radius,
            ACTIVE_TAB_BG,
            245,
        );
        // Subtle border.
        self.draw_rounded_rect(
            buffer,
            buf_width,
            buf_height,
            x,
            y,
            width,
            height,
            radius,
            TAB_BORDER,
            80,
        );

        let text_x = x as u32 + self.scaled_px(1) + padding_x;
        let text_y = y as u32 + self.scaled_px(1) + padding_y;
        let max_chars = ((width - self.scaled_px(2) - padding_x * 2) / self.cell_width) as usize;
        for (ci, ch) in title.chars().take(max_chars).enumerate() {
            let cx = text_x + ci as u32 * self.cell_width;
            self.draw_char_at(
                buffer,
                buf_width,
                buf_height,
                cx,
                text_y,
                ch,
                Color::DEFAULT_FG,
            );
        }
    }

    /// Draws a rounded rect with only the top corners rounded (bottom corners square).
    /// Used for active/hovered tab shapes that merge with the terminal below.
    #[allow(clippy::too_many_arguments)]
    fn draw_top_rounded_rect(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        radius: u32,
        color: u32,
        alpha: u8,
    ) {
        if w == 0 || h == 0 || alpha == 0 || buf_width == 0 || buf_height == 0 {
            return;
        }

        let r = radius.min(w / 2).min(h / 2) as i32;
        let max_x = buf_width as i32 - 1;
        let max_y = buf_height as i32 - 1;

        for py in 0..h as i32 {
            let sy = y + py;
            if sy < 0 || sy > max_y {
                continue;
            }

            for px in 0..w as i32 {
                let sx = x + px;
                if sx < 0 || sx > max_x {
                    continue;
                }

                // Only round the top corners; bottom corners are square.
                let coverage = Self::top_rounded_coverage(px, py, w as i32, h as i32, r);
                if coverage <= 0.0 {
                    continue;
                }

                let idx = sy as usize * buf_width + sx as usize;
                if idx >= buffer.len() {
                    continue;
                }
                let aa_alpha = ((alpha as f32) * coverage).round().clamp(0.0, 255.0) as u8;
                if aa_alpha == 0 {
                    continue;
                }
                buffer[idx] = Self::blend_pixel(buffer[idx], color, aa_alpha);
            }
        }
    }

    /// Coverage function for a rect with only the top two corners rounded.
    fn top_rounded_coverage(px: i32, py: i32, w: i32, h: i32, r: i32) -> f32 {
        if px < 0 || py < 0 || px >= w || py >= h {
            return 0.0;
        }
        if r <= 0 {
            return 1.0;
        }

        let in_tl = px < r && py < r;
        let in_tr = px >= w - r && py < r;
        // Bottom corners are NOT rounded.
        if !(in_tl || in_tr) {
            return 1.0;
        }

        let cx = if in_tl {
            r as f32 - 0.5
        } else {
            (w - r) as f32 - 0.5
        };
        let cy = r as f32 - 0.5;

        let dx = px as f32 + 0.5 - cx;
        let dy = py as f32 + 0.5 - cy;
        let rr = r as f32;
        let dist = (dx * dx + dy * dy).sqrt();
        (rr + 0.5 - dist).clamp(0.0, 1.0)
    }

    /// Draws a 1px border on the top and sides of a top-rounded rect (no bottom border).
    #[allow(clippy::too_many_arguments)]
    fn draw_top_rounded_border(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        radius: u32,
        color: u32,
        alpha: u8,
    ) {
        if w < 2 || h < 2 || alpha == 0 || buf_width == 0 || buf_height == 0 {
            return;
        }

        let r = radius.min(w / 2).min(h / 2) as i32;
        let max_x = buf_width as i32 - 1;
        let max_y = buf_height as i32 - 1;

        for py in 0..h as i32 {
            let sy = y + py;
            if sy < 0 || sy > max_y {
                continue;
            }

            for px in 0..w as i32 {
                let sx = x + px;
                if sx < 0 || sx > max_x {
                    continue;
                }

                // Determine if this pixel is on the border (top or sides, not bottom).
                let on_top = py == 0;
                let on_left = px == 0;
                let on_right = px == w as i32 - 1;

                if !on_top && !on_left && !on_right {
                    continue;
                }

                // Skip bottom row entirely (no bottom border).
                if py >= h as i32 - 1 {
                    continue;
                }

                // Check that the pixel is inside the top-rounded shape.
                let coverage = Self::top_rounded_coverage(px, py, w as i32, h as i32, r);
                if coverage <= 0.0 {
                    continue;
                }

                let idx = sy as usize * buf_width + sx as usize;
                if idx >= buffer.len() {
                    continue;
                }
                let aa_alpha = ((alpha as f32) * coverage).round().clamp(0.0, 255.0) as u8;
                if aa_alpha == 0 {
                    continue;
                }
                buffer[idx] = Self::blend_pixel(buffer[idx], color, aa_alpha);
            }
        }
    }

    /// Draws top tab bar including tabs, controls, and separators.
    pub fn draw_tab_bar(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        tabs: &[TabInfo],
        hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
    ) {
        let tab_bar_height = self.tab_bar_height_px();
        let bar_h = tab_bar_height as usize;

        // Solid bar background fill.
        for py in 0..bar_h.min(buf_height) {
            for px in 0..buf_width {
                let idx = py * buf_width + px;
                if idx < buffer.len() {
                    buffer[idx] = BAR_BG;
                }
            }
        }

        let tw = self.tab_width(tabs.len(), buf_width as u32);
        let text_y = (tab_bar_height.saturating_sub(self.cell_height)) / 2 + self.scaled_px(1);
        let tab_padding_h = self.scaled_px(14);
        let use_numbers = self.should_show_number(tw);
        let tab_radius = self.scaled_px(8);
        let tab_inset_y = self.scaled_px(0); // Tabs start from top of bar.
        let tab_h = tab_bar_height; // Full height so bottom merges with terminal.

        for (i, tab) in tabs.iter().enumerate() {
            let tab_x = self.tab_origin_x(i, tw);
            let is_hovered = hovered_tab == Some(i);

            if tab.is_active {
                // Active tab: solid fill that merges with terminal.
                self.draw_top_rounded_rect(
                    buffer,
                    buf_width,
                    bar_h,
                    tab_x as i32,
                    tab_inset_y as i32,
                    tw,
                    tab_h,
                    tab_radius,
                    ACTIVE_TAB_BG,
                    255,
                );
                // 1px border on top and sides only.
                self.draw_top_rounded_border(
                    buffer,
                    buf_width,
                    bar_h,
                    tab_x as i32,
                    tab_inset_y as i32,
                    tw,
                    tab_h,
                    tab_radius,
                    TAB_BORDER,
                    255,
                );
            } else if is_hovered {
                // Inactive tab hover: subtle highlight.
                self.draw_top_rounded_rect(
                    buffer,
                    buf_width,
                    bar_h,
                    tab_x as i32,
                    tab_inset_y as i32,
                    tw,
                    tab_h,
                    tab_radius,
                    INACTIVE_TAB_HOVER,
                    255,
                );
            }
            // Inactive non-hovered: no background (BAR_BG shows through).

            // Text color based on state.
            let fg = if tab.is_active {
                Color::from_pixel(TAB_TEXT_ACTIVE)
            } else {
                Color::from_pixel(TAB_TEXT_INACTIVE)
            };

            if tab.is_renaming {
                // Inline rename rendering.
                let rename_text = tab.rename_text.unwrap_or("");
                let text_x = tab_x + tab_padding_h;
                let max_chars = (tw.saturating_sub(tab_padding_h * 2) / self.cell_width) as usize;
                let selection_chars = tab.rename_selection.and_then(|(start, end)| {
                    if start >= end {
                        return None;
                    }
                    let start_chars = rename_text
                        .get(..start)
                        .map_or(0, |prefix| prefix.chars().count());
                    let end_chars = rename_text
                        .get(..end)
                        .map_or(start_chars, |prefix| prefix.chars().count());
                    Some((start_chars.min(max_chars), end_chars.min(max_chars)))
                });

                let rename_bg = INACTIVE_TAB_HOVER;
                for py in (text_y as usize)..(text_y + self.cell_height) as usize {
                    for dx in (tab_padding_h as usize - self.scaled_px(2) as usize)
                        ..(tw - tab_padding_h + self.scaled_px(2)) as usize
                    {
                        let px = tab_x as usize + dx;
                        if px < buf_width && py * buf_width + px < buffer.len() {
                            let idx = py * buf_width + px;
                            buffer[idx] = rename_bg;
                        }
                    }
                }

                for (ci, ch) in rename_text.chars().take(max_chars).enumerate() {
                    let cx = text_x + ci as u32 * self.cell_width;
                    let selected =
                        selection_chars.is_some_and(|(start, end)| ci >= start && ci < end);
                    if selected {
                        self.draw_bg(buffer, buf_width, bar_h, cx, text_y, ACTIVE_ACCENT);
                        self.draw_char_at(
                            buffer,
                            buf_width,
                            bar_h,
                            cx,
                            text_y,
                            ch,
                            Color::DEFAULT_BG,
                        );
                    } else {
                        self.draw_char_at(
                            buffer,
                            buf_width,
                            bar_h,
                            cx,
                            text_y,
                            ch,
                            Color::DEFAULT_FG,
                        );
                    }
                }

                let cursor_chars = rename_text
                    .get(..tab.rename_cursor)
                    .map_or(0, |prefix| prefix.chars().count())
                    .min(max_chars);
                let cursor_x = text_x + cursor_chars as u32 * self.cell_width;
                self.draw_char_at(
                    buffer,
                    buf_width,
                    bar_h,
                    cursor_x,
                    text_y,
                    '|',
                    Color::DEFAULT_FG,
                );
            } else if use_numbers {
                // Overflow mode: show tab number (1-based) instead of title.
                let number_str = (i + 1).to_string();
                let show_close = tab.is_active || is_hovered;
                let close_reserved = if show_close {
                    self.cell_width + self.scaled_px(8)
                } else {
                    0
                };
                let text_w = number_str.len() as u32 * self.cell_width;
                let text_x = tab_x + (tw.saturating_sub(text_w + close_reserved)) / 2;

                for (ci, ch) in number_str.chars().enumerate() {
                    let cx = text_x + ci as u32 * self.cell_width;
                    self.draw_char_at(buffer, buf_width, bar_h, cx, text_y, ch, fg);
                }

                if show_close {
                    self.draw_close_button(buffer, buf_width, bar_h, i, tab, tw, mouse_pos);
                }
            } else {
                // Normal mode: show title with close button and security badge.
                let show_close = tab.is_active || is_hovered;
                let close_reserved = if show_close {
                    self.cell_width + self.scaled_px(8)
                } else {
                    0
                };
                let security_reserved = if tab.security_count > 0 {
                    let count_chars = tab.security_count.min(99).to_string().len() as u32;
                    let count_width = if tab.security_count > 1 {
                        count_chars * self.cell_width + self.scaled_px(2)
                    } else {
                        0
                    };
                    let badge_min = self.scaled_px(10);
                    let badge_max = self.scaled_px(15);
                    self.cell_height
                        .saturating_sub(self.scaled_px(10))
                        .clamp(badge_min, badge_max)
                        + count_width
                        + self.scaled_px(6)
                } else {
                    0
                };
                let max_chars = (tw
                    .saturating_sub(tab_padding_h * 2 + close_reserved + security_reserved)
                    / self.cell_width) as usize;
                let title: String = tab.title.chars().take(max_chars).collect();
                let text_x = tab_x + tab_padding_h;

                for (ci, ch) in title.chars().enumerate() {
                    let cx = text_x + ci as u32 * self.cell_width;
                    self.draw_char_at(buffer, buf_width, bar_h, cx, text_y, ch, fg);
                }

                // Security badge.
                if let Some((sx, sy, sw, _sh)) =
                    self.security_badge_rect(i, tabs.len(), buf_width as u32, tab.security_count)
                {
                    self.draw_security_shield_icon(
                        buffer,
                        buf_width,
                        bar_h,
                        sx,
                        sy,
                        sw,
                        SECURITY_ACCENT,
                    );
                    if tab.security_count > 1 {
                        let count_text = tab.security_count.min(99).to_string();
                        let count_x = sx + sw + self.scaled_px(2);
                        for (ci, ch) in count_text.chars().enumerate() {
                            let cx = count_x + ci as u32 * self.cell_width;
                            self.draw_char_at(
                                buffer,
                                buf_width,
                                bar_h,
                                cx,
                                text_y,
                                ch,
                                SECURITY_ACCENT,
                            );
                        }
                    }
                }

                if show_close {
                    self.draw_close_button(buffer, buf_width, bar_h, i, tab, tw, mouse_pos);
                }
            }
        }

        // New-tab (+) button after the last tab.
        let plus_rect = self.plus_button_rect(tabs.len(), tw);
        let plus_hover = Self::point_in_rect(mouse_pos.0, mouse_pos.1, plus_rect);
        if plus_hover {
            let (px, py, pw, ph) = plus_rect;
            self.draw_rounded_rect(
                buffer,
                buf_width,
                bar_h,
                px as i32,
                py as i32,
                pw,
                ph,
                self.scaled_px(5),
                INACTIVE_TAB_HOVER,
                255,
            );
        }
        let plus_fg = if plus_hover {
            Color::from_pixel(TAB_TEXT_ACTIVE)
        } else {
            Color::from_pixel(TAB_TEXT_INACTIVE)
        };
        self.draw_tab_plus_icon(buffer, buf_width, bar_h, plus_rect, plus_fg);

        // Window control buttons (non-macOS).
        #[cfg(not(target_os = "macos"))]
        self.draw_window_buttons(buffer, buf_width, bar_h, mouse_pos);

        // 1px bottom separator between bar and terminal.
        if bar_h > 0 {
            let py = bar_h - 1;
            for px in 0..buf_width {
                let idx = py * buf_width + px;
                if idx < buffer.len() {
                    buffer[idx] = Self::blend_pixel(buffer[idx], TAB_BORDER, 180);
                }
            }
        }
    }

    /// Draws the close button with a circular hover effect.
    #[allow(clippy::too_many_arguments)]
    fn draw_close_button(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        tab_index: usize,
        _tab: &TabInfo,
        tw: u32,
        mouse_pos: (f64, f64),
    ) {
        let (cx, cy, cw, ch) = self.close_button_rect(tab_index, tw);
        let is_close_hovered = mouse_pos.0 >= cx as f64
            && mouse_pos.0 < (cx + cw) as f64
            && mouse_pos.1 >= cy as f64
            && mouse_pos.1 < (cy + ch) as f64
            && mouse_pos.1 < self.tab_bar_height_px() as f64;

        if is_close_hovered {
            let circle_r = (cw.min(ch) / 2).max(self.scaled_px(6));
            let circle_cx = (cx + cw / 2) as i32;
            let circle_cy = (cy + ch / 2) as i32;
            Self::draw_filled_circle(
                buffer,
                buf_width,
                circle_cx,
                circle_cy,
                circle_r,
                CLOSE_HOVER_BG_COLOR,
            );
        }

        let close_fg = Color::from_pixel(TAB_TEXT_INACTIVE);
        self.draw_tab_close_icon(buffer, buf_width, buf_height, (cx, cy, cw, ch), close_fg);
    }

    /// Draws the 3 window control buttons at the right edge (non-macOS).
    #[cfg(not(target_os = "macos"))]
    fn draw_window_buttons(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        mouse_pos: (f64, f64),
    ) {
        let bar_h = self.tab_bar_height_px();
        let btn_w = self.scaled_px(WIN_BTN_WIDTH);
        let bw = buf_width as u32;

        // Button positions from right: Close, Maximize, Minimize.
        let buttons: [(u32, WindowButton); 3] = [
            (bw.saturating_sub(btn_w * 3), WindowButton::Minimize),
            (bw.saturating_sub(btn_w * 2), WindowButton::Maximize),
            (bw.saturating_sub(btn_w), WindowButton::Close),
        ];

        for &(btn_x, ref btn_type) in &buttons {
            let is_hovered = mouse_pos.0 >= btn_x as f64
                && mouse_pos.0 < (btn_x + btn_w) as f64
                && mouse_pos.1 >= 0.0
                && mouse_pos.1 < bar_h as f64;

            // Hover background.
            if is_hovered {
                let hover_bg = if *btn_type == WindowButton::Close {
                    WIN_BTN_CLOSE_HOVER
                } else {
                    WIN_BTN_HOVER
                };
                for py in 0..bar_h as usize {
                    for px in btn_x as usize..(btn_x + btn_w) as usize {
                        if px < buf_width && py < buf_height {
                            let idx = py * buf_width + px;
                            if idx < buffer.len() {
                                buffer[idx] = hover_bg;
                            }
                        }
                    }
                }
            }

            let icon_color = if is_hovered && *btn_type == WindowButton::Close {
                0xFFFFFF
            } else {
                WIN_BTN_ICON
            };

            let center_x = btn_x as f32 + btn_w as f32 / 2.0;
            let center_y = bar_h as f32 / 2.0;
            let thickness = (1.25_f32 * self.ui_scale() as f32).clamp(1.15, 2.2);

            match btn_type {
                WindowButton::Minimize => {
                    // Thin horizontal line.
                    let half_w = self.scaled_px(5) as f32;
                    Self::draw_stroked_line(
                        buffer,
                        buf_width,
                        buf_height,
                        center_x - half_w,
                        center_y,
                        center_x + half_w,
                        center_y,
                        thickness,
                        icon_color,
                    );
                }
                WindowButton::Maximize => {
                    // Small rectangle outline (10x10px scaled).
                    let half = self.scaled_px(5) as f32;
                    let x0 = center_x - half;
                    let y0 = center_y - half;
                    let x1 = center_x + half;
                    let y1 = center_y + half;
                    // Top.
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height,
                        x0, y0, x1, y0, thickness, icon_color,
                    );
                    // Bottom.
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height,
                        x0, y1, x1, y1, thickness, icon_color,
                    );
                    // Left.
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height,
                        x0, y0, x0, y1, thickness, icon_color,
                    );
                    // Right.
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height,
                        x1, y0, x1, y1, thickness, icon_color,
                    );
                }
                WindowButton::Close => {
                    // X shape.
                    let half = self.scaled_px(5) as f32 * 0.7;
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height,
                        center_x - half, center_y - half,
                        center_x + half, center_y + half,
                        thickness, icon_color,
                    );
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height,
                        center_x + half, center_y - half,
                        center_x - half, center_y + half,
                        thickness, icon_color,
                    );
                }
            }
        }
    }

    /// Draws the drag overlay: ghost tab at cursor X + insertion indicator.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_tab_drag_overlay(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        tabs: &[TabInfo],
        source_index: usize,
        current_x: f64,
        insert_index: usize,
    ) {
        let tab_count = tabs.len();
        if source_index >= tab_count {
            return;
        }
        let tw = self.tab_width(tab_count, buf_width as u32);
        let tab_bar_height = self.tab_bar_height_px();
        let bar_h = tab_bar_height as usize;

        // Ghost tab: semi-transparent flat rectangle (60% opacity).
        let ghost_x = (current_x - tw as f64 / 2.0).round() as i32;
        let ghost_color = ACTIVE_TAB_BG;
        let alpha = 153u8; // 60%

        for py in 0..bar_h {
            for dx in 0..tw as usize {
                let px = ghost_x + dx as i32;
                if px < 0 || px as usize >= buf_width || py >= buf_height {
                    continue;
                }
                let idx = py * buf_width + px as usize;
                if idx >= buffer.len() {
                    continue;
                }
                buffer[idx] = Self::blend_rgb(buffer[idx], ghost_color, alpha);
            }
        }

        // Ghost title text.
        let tab = &tabs[source_index];
        let text_y = (tab_bar_height.saturating_sub(self.cell_height)) / 2 + self.scaled_px(1);
        let use_numbers = self.should_show_number(tw);
        let label: String = if use_numbers {
            (source_index + 1).to_string()
        } else {
            let pad = self.scaled_px(14);
            let max = (tw.saturating_sub(pad * 2) / self.cell_width) as usize;
            tab.title.chars().take(max).collect()
        };
        let lw = label.len() as u32 * self.cell_width;
        let tx = ghost_x + ((tw as i32 - lw as i32) / 2).max(4);
        for (ci, ch) in label.chars().enumerate() {
            let cx = tx + ci as i32 * self.cell_width as i32;
            if cx >= 0 && (cx as usize) < buf_width {
                self.draw_char_at(
                    buffer,
                    buf_width,
                    buf_height,
                    cx as u32,
                    text_y,
                    ch,
                    Color::DEFAULT_FG,
                );
            }
        }

        // Insertion indicator: 2px vertical line in Mauve (#CBA6F7).
        let ix = self.tab_origin_x(insert_index, tw);
        let indicator_y_pad = self.scaled_px(4) as usize;
        for py in indicator_y_pad..bar_h.saturating_sub(indicator_y_pad) {
            for dx in 0..self.scaled_px(2) {
                let px = ix + dx;
                if (px as usize) < buf_width && py < buf_height {
                    let idx = py * buf_width + px as usize;
                    if idx < buffer.len() {
                        buffer[idx] = INSERTION_COLOR;
                    }
                }
            }
        }
    }

    /// Returns true when pointer is over the custom window minimize button.
    #[allow(dead_code)]
    pub fn is_window_minimize_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        #[cfg(not(target_os = "macos"))]
        {
            self.window_button_at_position(x, y, buf_width as u32)
                == Some(WindowButton::Minimize)
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (x, y, buf_width);
            false
        }
    }

    /// Returns true when pointer is over the custom window maximize button.
    #[allow(dead_code)]
    pub fn is_window_maximize_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        #[cfg(not(target_os = "macos"))]
        {
            self.window_button_at_position(x, y, buf_width as u32)
                == Some(WindowButton::Maximize)
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (x, y, buf_width);
            false
        }
    }

    /// Returns true when pointer is over the custom window close button.
    #[allow(dead_code)]
    pub fn is_window_close_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        #[cfg(not(target_os = "macos"))]
        {
            self.window_button_at_position(x, y, buf_width as u32)
                == Some(WindowButton::Close)
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (x, y, buf_width);
            false
        }
    }
}
