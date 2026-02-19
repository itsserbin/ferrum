#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::CpuRenderer;

#[cfg(not(target_os = "macos"))]
use super::{INACTIVE_TAB_HOVER, PIN_ACTIVE_COLOR, TAB_TEXT_ACTIVE, TAB_TEXT_INACTIVE, WIN_BTN_WIDTH};

// Window button colors (non-macOS).
#[cfg(not(target_os = "macos"))]
use super::{WIN_BTN_CLOSE_HOVER, WIN_BTN_HOVER, WIN_BTN_ICON};

#[cfg(not(target_os = "macos"))]
use super::super::WindowButton;

impl CpuRenderer {
    /// Draws the pin button at the left of the tab bar (non-macOS).
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_pin_button(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        mouse_pos: (f64, f64),
        pinned: bool,
    ) {
        let (pin_x, pin_y, pin_w, pin_h) = self.pin_button_rect();
        let is_hovered =
            Self::point_in_rect(mouse_pos.0, mouse_pos.1, (pin_x, pin_y, pin_w, pin_h));

        // Draw hover background.
        if is_hovered {
            self.draw_rounded_rect(
                buffer,
                buf_width,
                buf_height,
                pin_x as i32,
                pin_y as i32,
                pin_w,
                pin_h,
                self.scaled_px(5),
                INACTIVE_TAB_HOVER,
                255,
            );
        }

        // Icon color: active (lavender) when pinned, inactive otherwise.
        let icon_color = if pinned {
            PIN_ACTIVE_COLOR
        } else if is_hovered {
            TAB_TEXT_ACTIVE
        } else {
            TAB_TEXT_INACTIVE
        };

        self.draw_pin_icon(
            buffer,
            buf_width,
            buf_height,
            (pin_x, pin_y, pin_w, pin_h),
            icon_color,
            pinned,
        );
    }

    /// Draws a Bootstrap-style vertical pushpin icon.
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_pin_icon(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        rect: (u32, u32, u32, u32),
        color: u32,
        _pinned: bool,
    ) {
        let (x, y, w, h) = rect;
        let cx = (x as f32 + w as f32 / 2.0) as i32;
        let cy = (y as f32 + h as f32 / 2.0) as i32;
        let s = self.ui_scale() as f32;
        let t = (1.2 * s).clamp(1.0, 2.0);

        // Dimensions (scaled)
        let head_w = (6.0 * s) as i32;
        let head_h = (2.0 * s) as i32;
        let body_w = (3.0 * s) as i32;
        let body_h = (4.0 * s) as i32;
        let platform_w = (7.0 * s) as i32;
        let platform_h = (1.5 * s) as i32;
        let needle_h = (4.0 * s) as i32;

        let top = cy - (6.0 * s) as i32;

        // Helper to draw filled rect
        let draw_rect = |buf: &mut [u32], rx: i32, ry: i32, rw: i32, rh: i32| {
            for py in ry.max(0)..(ry + rh).min(buf_height as i32) {
                for px in rx.max(0)..(rx + rw).min(buf_width as i32) {
                    let idx = py as usize * buf_width + px as usize;
                    if idx < buf.len() {
                        buf[idx] = color;
                    }
                }
            }
        };

        // 1. Top head
        draw_rect(buffer, cx - head_w / 2, top, head_w, head_h);

        // 2. Body
        let body_top = top + head_h;
        draw_rect(buffer, cx - body_w / 2, body_top, body_w, body_h);

        // 3. Platform
        let platform_top = body_top + body_h;
        draw_rect(
            buffer,
            cx - platform_w / 2,
            platform_top,
            platform_w,
            platform_h,
        );

        // 4. Needle
        let needle_top = (platform_top + platform_h) as f32;
        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            cx as f32,
            needle_top,
            cx as f32,
            needle_top + needle_h as f32,
            t,
            color,
        );
    }

    /// Draws the 3 window control buttons at the right edge (non-macOS).
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_window_buttons(
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
                        buffer, buf_width, buf_height, x0, y0, x1, y0, thickness, icon_color,
                    );
                    // Bottom.
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height, x0, y1, x1, y1, thickness, icon_color,
                    );
                    // Left.
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height, x0, y0, x0, y1, thickness, icon_color,
                    );
                    // Right.
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height, x1, y0, x1, y1, thickness, icon_color,
                    );
                }
                WindowButton::Close => {
                    // X shape.
                    let half = self.scaled_px(5) as f32 * 0.7;
                    Self::draw_stroked_line(
                        buffer,
                        buf_width,
                        buf_height,
                        center_x - half,
                        center_y - half,
                        center_x + half,
                        center_y + half,
                        thickness,
                        icon_color,
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
                        icon_color,
                    );
                }
            }
        }
    }
}
