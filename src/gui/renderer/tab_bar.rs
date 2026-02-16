use super::*;

impl Renderer {
    /// Computes adaptive tab width with overflow compression.
    /// Tabs shrink from max (240px) down to MIN_TAB_WIDTH when many tabs are open.
    pub fn tab_width(&self, tab_count: usize, buf_width: u32) -> u32 {
        let reserved = self.cell_width * 7; // Reserve space for '+' and window close button.
        let available = buf_width.saturating_sub(reserved);
        (available / tab_count.max(1) as u32).clamp(MIN_TAB_WIDTH, 240)
    }

    /// Returns rectangle for per-tab close button.
    fn close_button_rect(&self, tab_index: usize, tw: u32) -> (u32, u32, u32, u32) {
        let x = tab_index as u32 * tw + tw - self.cell_width - 4;
        let y = (TAB_BAR_HEIGHT - self.cell_height) / 2;
        (x, y, self.cell_width + 4, self.cell_height)
    }

    /// Returns rectangle for new-tab button.
    fn plus_button_rect(&self, tab_count: usize, tw: u32) -> (u32, u32, u32, u32) {
        let x = tab_count as u32 * tw + 4;
        let y = (TAB_BAR_HEIGHT - self.cell_height) / 2;
        (x, y, self.cell_width + 8, self.cell_height)
    }

    /// Hit-tests the tab bar and returns the clicked target.
    pub fn hit_test_tab_bar(&self, x: f64, y: f64, tab_count: usize, buf_width: u32) -> TabBarHit {
        if y >= TAB_BAR_HEIGHT as f64 {
            return TabBarHit::Empty;
        }

        let tw = self.tab_width(tab_count, buf_width);

        // New-tab button has priority over tab body hit-test.
        let (px, py, pw, ph) = self.plus_button_rect(tab_count, tw);
        if x >= px as f64 && x < (px + pw) as f64 && y >= py as f64 && y < (py + ph) as f64 {
            return TabBarHit::NewTab;
        }

        let tab_index = x as u32 / tw;
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
        if y >= TAB_BAR_HEIGHT as f64 || tab_count == 0 {
            return None;
        }
        let tw = self.tab_width(tab_count, buf_width);
        let idx = x as u32 / tw;
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

    /// Draws a rectangle with only the top-left corner rounded.
    fn draw_rect_with_top_left_radius(
        buffer: &mut [u32],
        buf_w: usize,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        radius: u32,
        color: u32,
    ) {
        let r = radius as f32;
        for dy in 0..h {
            for dx in 0..w {
                let px = (x + dx) as usize;
                let py = (y + dy) as usize;
                if px >= buf_w {
                    continue;
                }

                // Only clip the top-left corner.
                if dy < radius && dx < radius {
                    let cx = (radius - dx) as f32 - 0.5;
                    let cy = (radius - dy) as f32 - 0.5;
                    if cx * cx + cy * cy > r * r {
                        continue;
                    }
                }

                if let Some(pixel) = buffer.get_mut(py * buf_w + px) {
                    *pixel = color;
                }
            }
        }
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
        let r = radius as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    let px = cx + dx;
                    let py = cy + dy;
                    if px >= 0 && py >= 0 && (px as usize) < buf_w {
                        if let Some(pixel) = buffer.get_mut(py as usize * buf_w + px as usize) {
                            *pixel = color;
                        }
                    }
                }
            }
        }
    }

    /// Returns true if the given tab width is too narrow to display the title.
    /// When true, show tab number instead.
    fn should_show_number(&self, tw: u32) -> bool {
        tw < MIN_TAB_WIDTH_FOR_TITLE
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
        let bar_bg = TAB_BAR_BG.to_pixel();
        let bar_h = TAB_BAR_HEIGHT as usize;

        // Paint full bar background first.
        for py in 0..bar_h {
            for px in 0..buf_width {
                let idx = py * buf_width + px;
                if idx < buffer.len() {
                    buffer[idx] = bar_bg;
                }
            }
        }

        let tw = self.tab_width(tabs.len(), buf_width as u32);
        let text_y = (TAB_BAR_HEIGHT - self.cell_height) / 2 + 1;
        let tab_padding_h = 14u32;
        let use_numbers = self.should_show_number(tw);

        let active_idx = tabs.iter().position(|t| t.is_active);

        for (i, tab) in tabs.iter().enumerate() {
            let tab_x = i as u32 * tw;
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

            // Flat tab shape: all tabs fill the full bar height.
            // First tab gets a rounded top-left corner; all others are plain rectangles.
            if i == 0 {
                Self::draw_rect_with_top_left_radius(
                    buffer,
                    buf_width,
                    tab_x,
                    0,
                    tw,
                    TAB_BAR_HEIGHT,
                    FIRST_TAB_RADIUS,
                    bg_pixel,
                );
            } else {
                for py in 0..bar_h {
                    for dx in 0..tw as usize {
                        let px = tab_x as usize + dx;
                        if px >= buf_width {
                            continue;
                        }
                        let idx = py * buf_width + px;
                        if idx < buffer.len() {
                            buffer[idx] = bg_pixel;
                        }
                    }
                }
            }

            // Vertical 1px separator between tabs.
            // Hidden near active and hovered tabs to reduce visual noise.
            let is_near_active = active_idx.is_some_and(|ai| i == ai || i + 1 == ai);
            let is_near_hovered = hovered_tab.is_some_and(|hi| i == hi || i + 1 == hi);
            if !tab.is_active && !is_near_active && !is_near_hovered && !is_hovered {
                let sep_x = tab_x + tw - 1;
                if (sep_x as usize) < buf_width {
                    let sep_pixel = SEPARATOR_COLOR.to_pixel();
                    for py in 8..(bar_h - 8) {
                        let idx = py * buf_width + sep_x as usize;
                        if idx < buffer.len() {
                            buffer[idx] = sep_pixel;
                        }
                    }
                }
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
                    for dx in (tab_padding_h as usize - 2)..(tw - tab_padding_h + 2) as usize {
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
                let close_reserved = if show_close { self.cell_width + 4 } else { 0 };
                let text_w = number_str.len() as u32 * self.cell_width;
                let text_x = tab_x + (tw.saturating_sub(text_w + close_reserved)) / 2;

                for (ci, ch) in number_str.chars().enumerate() {
                    let cx = text_x + ci as u32 * self.cell_width;
                    self.draw_char_at(buffer, buf_width, bar_h, cx, text_y, ch, fg);
                }

                // Close button in number mode.
                if show_close {
                    self.draw_close_button(
                        buffer,
                        buf_width,
                        bar_h,
                        i,
                        tab,
                        tw,
                        text_y,
                        mouse_pos,
                    );
                }
            } else {
                // Normal mode: show title with close button and security badge.
                let show_close = tab.is_active || is_hovered;
                let close_reserved = if show_close { self.cell_width + 8 } else { 0 };
                let security_reserved = if tab.security_count > 0 {
                    let count_chars = tab.security_count.min(99).to_string().len() as u32;
                    let count_width = if tab.security_count > 1 {
                        count_chars * self.cell_width + 2
                    } else {
                        0
                    };
                    self.cell_height.saturating_sub(10).clamp(10, 15) + count_width + 6
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
                        let count_x = sx + sw + 2;
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
                    self.draw_close_button(
                        buffer,
                        buf_width,
                        bar_h,
                        i,
                        tab,
                        tw,
                        text_y,
                        mouse_pos,
                    );
                }
            }
        }

        // New-tab button after the last tab.
        let plus_x = tabs.len() as u32 * tw + 12;
        let plus_fg = Color {
            r: 88,
            g: 91,
            b: 112,
        }; // Surface2
        self.draw_char_at(buffer, buf_width, bar_h, plus_x, text_y, '+', plus_fg);

        // Custom window close button.
        let close_win_x = buf_width as u32 - self.cell_width * 2;
        let close_win_fg = Color {
            r: 243,
            g: 139,
            b: 168,
        }; // Red
        self.draw_char_at(
            buffer,
            buf_width,
            bar_h,
            close_win_x,
            text_y,
            '\u{00D7}',
            close_win_fg,
        );

        // Bottom separator — hidden under the active tab (tab merges with terminal area).
        let sep_pixel = SEPARATOR_COLOR.to_pixel();
        let py = bar_h - 1;
        for px in 0..buf_width {
            let in_active = active_idx.is_some_and(|ai| {
                let ax = ai as u32 * tw;
                px >= ax as usize && px < (ax + tw) as usize
            });
            if !in_active {
                let idx = py * buf_width + px;
                if idx < buffer.len() {
                    buffer[idx] = sep_pixel;
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
        text_y: u32,
        mouse_pos: (f64, f64),
    ) {
        let tab_x = tab_index as u32 * tw;
        let close_x = tab_x + tw - self.cell_width - 6;

        // Check if mouse is hovering over the close button area.
        let (cx, cy, cw, ch) = self.close_button_rect(tab_index, tw);
        let is_close_hovered = mouse_pos.0 >= cx as f64
            && mouse_pos.0 < (cx + cw) as f64
            && mouse_pos.1 >= cy as f64
            && mouse_pos.1 < (cy + ch) as f64
            && mouse_pos.1 < TAB_BAR_HEIGHT as f64;

        // Draw circular background on close button hover (~16px diameter).
        if is_close_hovered {
            let circle_r = 8u32;
            let circle_cx = (close_x + self.cell_width / 2) as i32;
            let circle_cy = (text_y + self.cell_height / 2) as i32;
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
        self.draw_char_at(
            buffer,
            buf_width,
            buf_height,
            close_x,
            text_y,
            '\u{00D7}',
            close_fg,
        );
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
        let bar_h = TAB_BAR_HEIGHT as usize;

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
        let text_y = (TAB_BAR_HEIGHT - self.cell_height) / 2 + 1;
        let use_numbers = self.should_show_number(tw);
        let label: String = if use_numbers {
            (source_index + 1).to_string()
        } else {
            let pad = 14u32;
            let max = (tw.saturating_sub(pad * 2) / self.cell_width) as usize;
            tab.title.chars().take(max).collect()
        };
        let lw = label.len() as u32 * self.cell_width;
        let tx = ghost_x + ((tw as i32 - lw as i32) / 2).max(4);
        for (ci, ch) in label.chars().enumerate() {
            let cx = tx + ci as i32 * self.cell_width as i32;
            if cx >= 0 && (cx as usize) < buf_width {
                self.draw_char_at(
                    buffer, buf_width, buf_height, cx as u32, text_y, ch, Color::DEFAULT_FG,
                );
            }
        }

        // Insertion indicator: 2px lavender vertical line.
        let ix = insert_index as u32 * tw;
        let ic = ACTIVE_ACCENT.to_pixel();
        for py in 4..bar_h.saturating_sub(4) {
            for dx in 0..2u32 {
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

    /// Returns true when pointer is over the custom window close button.
    pub fn is_window_close_button(&self, x: f64, y: f64, buf_width: usize) -> bool {
        let close_x = buf_width as u32 - self.cell_width * 3;
        x >= close_x as f64 && y < TAB_BAR_HEIGHT as f64
    }
}
