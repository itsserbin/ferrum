use super::*;

impl Renderer {
    /// Computes adaptive tab width with overflow compression.
    /// Tabs shrink from max (240px) down to MIN_TAB_WIDTH when many tabs are open.
    pub fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32 {
        // Reserve left-side window controls and right-side new-tab button.
        let reserved = self.tab_strip_start_x() + self.plus_button_reserved_width();
        let available = buf_width.saturating_sub(reserved);
        let min_tab_width = self.scaled_px(MIN_TAB_WIDTH);
        let max_tab_width = self.scaled_px(240);
        (available / tab_count.max(1) as u32).clamp(min_tab_width, max_tab_width)
    }

    pub(crate) fn tab_strip_start_x(&self) -> u32 {
        self.window_controls_reserved_width()
    }

    fn plus_button_reserved_width(&self) -> u32 {
        self.cell_width + self.scaled_px(20)
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

    fn window_button_size(&self) -> (u32, u32) {
        let diameter = self.scaled_px(12);
        (diameter, diameter)
    }

    fn window_button_spacing(&self) -> u32 {
        self.scaled_px(8)
    }

    fn window_controls_reserved_width(&self) -> u32 {
        let (bw, _) = self.window_button_size();
        let left_pad = self.scaled_px(12);
        let after_controls_gap = self.scaled_px(14);
        // Left pad + 3 buttons + 2 gaps + gap before tabs.
        left_pad + bw * 3 + self.window_button_spacing() * 2 + after_controls_gap
    }

    /// Returns rectangle for per-tab close button.
    fn close_button_rect(&self, tab_index: usize, tw: u32) -> (u32, u32, u32, u32) {
        let btn_w = self.cell_width + self.scaled_px(8);
        let x = self.tab_origin_x(tab_index, tw) + tw - btn_w - self.scaled_px(6);
        let y = (self.tab_bar_height_px().saturating_sub(self.cell_height)) / 2;
        (x, y, btn_w, self.cell_height)
    }

    fn window_control_rects(
        &self,
        _buf_width: usize,
    ) -> (
        (u32, u32, u32, u32),
        (u32, u32, u32, u32),
        (u32, u32, u32, u32),
    ) {
        let (bw, bh) = self.window_button_size();
        let spacing = self.window_button_spacing();
        let y = (self.tab_bar_height_px().saturating_sub(bh)) / 2;
        let close_x = self.scaled_px(12);
        let min_x = close_x + bw + spacing;
        let max_x = min_x + bw + spacing;
        ((min_x, y, bw, bh), (max_x, y, bw, bh), (close_x, y, bw, bh))
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

                // 1px soft edge to reduce visible pixel stair-steps.
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

    fn point_in_rect(x: f64, y: f64, rect: (u32, u32, u32, u32)) -> bool {
        let (rx, ry, rw, rh) = rect;
        x >= rx as f64 && x < (rx + rw) as f64 && y >= ry as f64 && y < (ry + rh) as f64
    }

    fn draw_window_button_circle(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        rect: (u32, u32, u32, u32),
        color: Color,
    ) {
        let (x, y, w, h) = rect;
        let cx = (x + w / 2) as i32;
        let cy = (y + h / 2) as i32;
        let radius = (w.min(h) / 2).saturating_sub(1).max(4);
        Self::draw_filled_circle(buffer, buf_width, cx, cy, radius, color.to_pixel());
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

                // Soft AA edge around the stroke.
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

    fn draw_window_minimize_icon(
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
        let half = (w.min(h) as f32 * 0.5 - 3.5).max(1.6);
        if half <= 0.0 {
            return;
        }
        let stroke = (1.4_f32 * self.ui_scale() as f32).clamp(1.25, 2.8);
        let pixel = color.to_pixel();

        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x - half,
            center_y,
            center_x + half,
            center_y,
            stroke,
            pixel,
        );
    }

    fn draw_window_maximize_icon(
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
        let half = (w.min(h) as f32 * 0.5 - 3.5).max(1.6);
        if half <= 0.0 {
            return;
        }
        let stroke = (1.4_f32 * self.ui_scale() as f32).clamp(1.25, 2.8);
        let pixel = color.to_pixel();

        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x - half,
            center_y,
            center_x + half,
            center_y,
            stroke,
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
            stroke,
            pixel,
        );
    }

    fn draw_window_close_icon(
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
        let half = (w.min(h) as f32 * 0.5 - 3.5).max(1.6);
        if half <= 0.0 {
            return;
        }
        let stroke = (1.4_f32 * self.ui_scale() as f32).clamp(1.25, 2.8);

        let pixel = color.to_pixel();
        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            center_x - half,
            center_y - half,
            center_x + half,
            center_y + half,
            stroke,
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
            stroke,
            pixel,
        );
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
        // Diagonal cross appears optically larger than '+', so keep it slightly smaller.
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
    /// When true, show tab number instead.
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

        let radius = self.scaled_px(8);
        let fill = Color {
            r: 36,
            g: 38,
            b: 52,
        }
        .to_pixel();
        self.draw_elevated_panel(
            buffer, buf_width, buf_height, x as u32, y as u32, width, height, radius, fill,
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

    /// Draws top tab bar including tabs, controls, and separators.
    pub fn draw_tab_bar(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        _buf_height: usize,
        tabs: &[TabInfo],
        hovered_tab: Option<usize>,
        mouse_pos: (f64, f64),
    ) {
        let tab_bar_height = self.tab_bar_height_px();
        let bar_h = tab_bar_height as usize;
        let top_bg = Color {
            r: 28,
            g: 30,
            b: 43,
        };
        let bottom_bg = TAB_BAR_BG;

        // Paint full bar background with a subtle vertical gradient (mac-like depth).
        for py in 0..bar_h {
            let t = py as f32 / (bar_h.max(1) as f32);
            let blend = |a: u8, b: u8| -> u8 {
                let av = a as f32;
                let bv = b as f32;
                (av + (bv - av) * t).round().clamp(0.0, 255.0) as u8
            };
            let row_pixel = Color {
                r: blend(top_bg.r, bottom_bg.r),
                g: blend(top_bg.g, bottom_bg.g),
                b: blend(top_bg.b, bottom_bg.b),
            }
            .to_pixel();
            for px in 0..buf_width {
                let idx = py * buf_width + px;
                if idx < buffer.len() {
                    buffer[idx] = row_pixel;
                }
            }
        }

        let tw = self.tab_width(tabs.len(), buf_width as u32);
        let text_y = (tab_bar_height.saturating_sub(self.cell_height)) / 2 + self.scaled_px(1);
        let tab_padding_h = self.scaled_px(14);
        let use_numbers = self.should_show_number(tw);

        for (i, tab) in tabs.iter().enumerate() {
            let tab_x = self.tab_origin_x(i, tw);
            let is_hovered = hovered_tab == Some(i);

            // Active tab = terminal bg (merges with content), inactive = bar bg, hovered = subtle lift.
            let bg = if tab.is_active {
                Color::DEFAULT_BG
            } else if is_hovered {
                TAB_HOVER_BG
            } else {
                TAB_BAR_BG
            };
            let bg_pixel = bg.to_pixel();

            // mac-like tab capsule instead of full-height square blocks.
            if tab.is_active || is_hovered {
                let inset_y = self.scaled_px(4) as i32;
                let capsule_h = tab_bar_height.saturating_sub(self.scaled_px(6));
                let radius = self.scaled_px(8);
                let alpha = if tab.is_active { 255 } else { 215 };
                self.draw_rounded_rect(
                    buffer,
                    buf_width,
                    bar_h,
                    tab_x as i32,
                    inset_y,
                    tw,
                    capsule_h,
                    radius,
                    bg_pixel,
                    alpha,
                );
            }

            // Text color: active = bright white, hovered = subtext1, inactive = overlay0.
            let fg = if tab.is_active {
                Color::DEFAULT_FG
            } else if is_hovered {
                Color {
                    r: 186,
                    g: 194,
                    b: 222,
                } // Subtext1
            } else {
                Color {
                    r: 127,
                    g: 132,
                    b: 156,
                } // Overlay0
            };

            if tab.is_renaming {
                // Preserve existing rename rendering structure.
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

                let rename_bg = MENU_HOVER_BG.to_pixel();
                for py in (text_y as usize)..(text_y + self.cell_height) as usize {
                    for dx in (tab_padding_h as usize - self.scaled_px(2) as usize)
                        ..(tw - tab_padding_h + self.scaled_px(2)) as usize
                    {
                        let px = tab_x as usize + dx;
                        if px < buf_width && py * buf_width + px < buffer.len() {
                            buffer[py * buf_width + px] = rename_bg;
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

                // Close button in number mode.
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

                // Close button with circular hover effect.
                if show_close {
                    self.draw_close_button(buffer, buf_width, bar_h, i, tab, tw, mouse_pos);
                }
            }
        }

        // New-tab button after the last tab.
        let plus_rect = self.plus_button_rect(tabs.len(), tw);
        let plus_hover = Self::point_in_rect(mouse_pos.0, mouse_pos.1, plus_rect);
        let plus_fg = if plus_hover {
            Color::DEFAULT_FG
        } else {
            Color {
                r: 88,
                g: 91,
                b: 112,
            } // Surface2
        };
        self.draw_tab_plus_icon(buffer, buf_width, bar_h, plus_rect, plus_fg);

        // macOS-like traffic lights: close, minimize, maximize (left side).
        let (min_rect, max_rect, close_rect) = self.window_control_rects(buf_width);
        let min_hover = Self::point_in_rect(mouse_pos.0, mouse_pos.1, min_rect);
        let max_hover = Self::point_in_rect(mouse_pos.0, mouse_pos.1, max_rect);
        let close_hover = Self::point_in_rect(mouse_pos.0, mouse_pos.1, close_rect);

        self.draw_window_button_circle(
            buffer,
            buf_width,
            close_rect,
            Color {
                r: 255,
                g: 95,
                b: 87,
            },
        );
        self.draw_window_button_circle(
            buffer,
            buf_width,
            min_rect,
            Color {
                r: 255,
                g: 189,
                b: 46,
            },
        );
        self.draw_window_button_circle(
            buffer,
            buf_width,
            max_rect,
            Color {
                r: 40,
                g: 200,
                b: 64,
            },
        );

        let control_fg = Color {
            r: 48,
            g: 49,
            b: 52,
        };
        if close_hover {
            self.draw_window_close_icon(buffer, buf_width, bar_h, close_rect, control_fg);
        }
        if min_hover {
            self.draw_window_minimize_icon(buffer, buf_width, bar_h, min_rect, control_fg);
        }
        if max_hover {
            self.draw_window_maximize_icon(buffer, buf_width, bar_h, max_rect, control_fg);
        }

        // Bottom separator + soft shadow.
        let sep_pixel = Color {
            r: 54,
            g: 56,
            b: 74,
        }
        .to_pixel();
        let py = bar_h - 1;
        for px in 0..buf_width {
            let idx = py * buf_width + px;
            if idx < buffer.len() {
                buffer[idx] = sep_pixel;
            }
        }

        if bar_h < _buf_height {
            let shadow_row = bar_h;
            let alpha = 36u8;
            for px in 0..buf_width {
                let idx = shadow_row * buf_width + px;
                if idx < buffer.len() {
                    buffer[idx] = Self::blend_pixel(buffer[idx], 0x000000, alpha);
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
        tab: &TabInfo,
        tw: u32,
        mouse_pos: (f64, f64),
    ) {
        let (cx, cy, cw, ch) = self.close_button_rect(tab_index, tw);
        let is_close_hovered = mouse_pos.0 >= cx as f64
            && mouse_pos.0 < (cx + cw) as f64
            && mouse_pos.1 >= cy as f64
            && mouse_pos.1 < (cy + ch) as f64
            && mouse_pos.1 < self.tab_bar_height_px() as f64;

        // Draw circular background on close button hover.
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
                CLOSE_HOVER_BG.to_pixel(),
            );
        }

        let close_fg = if tab.is_active {
            Color {
                r: 166,
                g: 173,
                b: 200,
            } // Subtext0
        } else {
            Color {
                r: 127,
                g: 132,
                b: 156,
            } // Overlay0
        };
        self.draw_tab_close_icon(buffer, buf_width, buf_height, (cx, cy, cw, ch), close_fg);
    }

    /// Draws the drag overlay: ghost tab at cursor X + insertion indicator.
    /// Detach happens instantly (not deferred), so this only draws for reorder.
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

        // Ghost tab in the bar with 60% opacity.
        let ghost_x = (current_x - tw as f64 / 2.0).round() as i32;
        let ghost_bg = ACTIVE_ACCENT;
        let alpha = 153u32;
        let inv_alpha = 255 - alpha;

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
                let dst = buffer[idx];
                let dr = (dst >> 16) & 0xFF;
                let dg = (dst >> 8) & 0xFF;
                let db = dst & 0xFF;
                let r = (ghost_bg.r as u32 * alpha + dr * inv_alpha) / 255;
                let g = (ghost_bg.g as u32 * alpha + dg * inv_alpha) / 255;
                let b = (ghost_bg.b as u32 * alpha + db * inv_alpha) / 255;
                buffer[idx] = (r << 16) | (g << 8) | b;
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

        // Insertion indicator: 2px lavender vertical line.
        let ix = self.tab_origin_x(insert_index, tw);
        let ic = ACTIVE_ACCENT.to_pixel();
        let indicator_y_pad = self.scaled_px(4) as usize;
        for py in indicator_y_pad..bar_h.saturating_sub(indicator_y_pad) {
            for dx in 0..self.scaled_px(2) {
                let px = ix + dx;
                if (px as usize) < buf_width && py < buf_height {
                    let idx = py * buf_width + px as usize;
                    if idx < buffer.len() {
                        buffer[idx] = ic;
                    }
                }
            }
        }
    }

    /// Returns true when pointer is over the custom window minimize button.
    pub fn is_window_minimize_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        let (min_rect, _, _) = self.window_control_rects(buf_width);
        Self::point_in_rect(x, y, min_rect)
    }

    /// Returns true when pointer is over the custom window maximize button.
    pub fn is_window_maximize_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        let (_, max_rect, _) = self.window_control_rects(buf_width);
        Self::point_in_rect(x, y, max_rect)
    }

    /// Returns true when pointer is over the custom window close button.
    pub fn is_window_close_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        let (_, _, close_rect) = self.window_control_rects(buf_width);
        Self::point_in_rect(x, y, close_rect)
    }
}
