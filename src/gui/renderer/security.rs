use super::*;

impl SecurityPopup {
    fn line_height(&self, cell_height: u32) -> u32 {
        cell_height + 4
    }

    fn width(&self, cell_width: u32) -> u32 {
        let max_line_chars = self
            .lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        let title_chars = self.title.chars().count();
        let content_chars = max_line_chars.max(title_chars) as u32;
        ((content_chars + 3).max(24)) * cell_width
    }

    fn height(&self, cell_height: u32) -> u32 {
        let title_h = self.line_height(cell_height);
        let lines_h = self.line_height(cell_height) * self.lines.len() as u32;
        title_h + lines_h + 8
    }
}

impl Renderer {
    pub(super) fn draw_security_shield_icon(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        x: u32,
        y: u32,
        size: u32,
        color: Color,
    ) {
        let pixel = color.to_pixel();
        let mid = size / 2;
        let top_third = (size / 3).max(1);
        let bottom_start = (size * 2 / 3).max(top_third + 1);

        for dy in 0..size {
            let py = y as usize + dy as usize;
            if py >= buf_height {
                break;
            }

            let half_span = if dy < top_third {
                1 + dy / 2
            } else if dy < bottom_start {
                mid.saturating_sub(1).max(1)
            } else {
                let progress = dy - bottom_start;
                let denom = (size - bottom_start).max(1);
                let shrink = progress * mid.saturating_sub(1) / denom;
                mid.saturating_sub(shrink).max(1)
            };

            let left = mid.saturating_sub(half_span);
            let right = (mid + half_span).min(size.saturating_sub(1));
            for dx in left..=right {
                let px = x as usize + dx as usize;
                if px >= buf_width {
                    continue;
                }
                let idx = py * buf_width + px;
                if idx < buffer.len() {
                    buffer[idx] = pixel;
                }
            }
        }
    }

    pub fn security_badge_rect(
        &self,
        tab_index: usize,
        tab_count: usize,
        buf_width: u32,
        security_count: usize,
    ) -> Option<(u32, u32, u32, u32)> {
        if security_count == 0 || tab_index >= tab_count {
            return None;
        }

        let tw = self.tab_width(tab_count, buf_width);
        let tab_x = self.tab_origin_x(tab_index, tw);
        let badge_min = self.scaled_px(10);
        let badge_max = self.scaled_px(15);
        let badge_size = self
            .cell_height
            .saturating_sub(self.scaled_px(10))
            .clamp(badge_min, badge_max);
        let count_chars = if security_count > 1 {
            security_count.min(99).to_string().len() as u32
        } else {
            0
        };
        let count_width = if count_chars > 0 {
            count_chars * self.cell_width + self.scaled_px(2)
        } else {
            0
        };
        let indicator_width = badge_size + count_width;
        let right_gutter = self.cell_width + self.scaled_px(10); // Keep clear space for the close button area.
        let indicator_right = tab_x + tw.saturating_sub(right_gutter);
        let x = indicator_right.saturating_sub(indicator_width + self.scaled_px(2));
        let y = (self.tab_bar_height_px().saturating_sub(badge_size)) / 2;
        Some((x, y, badge_size, badge_size))
    }

    fn security_popup_rect(
        &self,
        popup: &SecurityPopup,
        buf_width: usize,
        buf_height: usize,
    ) -> (u32, u32, u32, u32) {
        let width = popup.width(self.cell_width).min(buf_width as u32);
        let height = popup.height(self.cell_height).min(buf_height as u32);
        let x = popup.x.min(buf_width as u32 - width);
        let y = popup.y.min(buf_height as u32 - height);
        (x, y, width, height)
    }

    pub fn hit_test_security_popup(
        &self,
        popup: &SecurityPopup,
        x: f64,
        y: f64,
        buf_width: usize,
        buf_height: usize,
    ) -> bool {
        let (px, py, pw, ph) = self.security_popup_rect(popup, buf_width, buf_height);
        x >= px as f64 && x < (px + pw) as f64 && y >= py as f64 && y < (py + ph) as f64
    }

    pub fn draw_security_popup(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        popup: &SecurityPopup,
    ) {
        let (mx, my, mw, mh) = self.security_popup_rect(popup, buf_width, buf_height);
        let mx = mx as usize;
        let my = my as usize;
        let mw = mw as usize;
        let mh = mh as usize;
        let line_h = popup.line_height(self.cell_height) as usize;

        let bg_pixel = MENU_BG.to_pixel();
        let border_pixel = SEPARATOR_COLOR.to_pixel();
        let header_pixel = SECURITY_ACCENT.to_pixel();

        for py in my..((my + mh).min(buf_height)) {
            for px in mx..((mx + mw).min(buf_width)) {
                let idx = py * buf_width + px;
                if idx >= buffer.len() {
                    continue;
                }
                let is_border = py == my || py == my + mh - 1 || px == mx || px == mx + mw - 1;
                buffer[idx] = if is_border { border_pixel } else { bg_pixel };
            }
        }

        let header_y = my as u32 + self.scaled_px(2);
        let header_x = mx as u32 + self.cell_width / 2;
        for (i, ch) in popup.title.chars().enumerate() {
            let x = header_x + i as u32 * self.cell_width;
            self.draw_char_at(
                buffer,
                buf_width,
                buf_height,
                x,
                header_y,
                ch,
                SECURITY_ACCENT,
            );
        }

        let sep_y = my + line_h;
        if sep_y < buf_height {
            for px in (mx + 1)..(mx + mw - 1) {
                let idx = sep_y * buf_width + px;
                if idx < buffer.len() {
                    buffer[idx] = header_pixel;
                }
            }
        }

        for (line_idx, line) in popup.lines.iter().enumerate() {
            let text_y =
                my as u32 + line_h as u32 + self.scaled_px(4) + line_idx as u32 * line_h as u32;
            let text_x = mx as u32 + self.cell_width / 2;
            let mut chars = String::from("• ");
            chars.push_str(line);
            for (i, ch) in chars.chars().enumerate() {
                let x = text_x + i as u32 * self.cell_width;
                self.draw_char_at(
                    buffer,
                    buf_width,
                    buf_height,
                    x,
                    text_y,
                    ch,
                    Color::DEFAULT_FG,
                );
            }
        }
    }
}
