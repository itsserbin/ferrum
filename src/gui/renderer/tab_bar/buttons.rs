#![cfg_attr(target_os = "macos", allow(dead_code))]

use super::super::CpuRenderer;

#[cfg(not(target_os = "macos"))]
use super::super::shared::{tab_math, ui_layout};
#[cfg(not(target_os = "macos"))]
use super::{INACTIVE_TAB_HOVER, PIN_ACTIVE_COLOR, TAB_TEXT_ACTIVE, TAB_TEXT_INACTIVE};

// Window button colors (non-macOS).
#[cfg(not(target_os = "macos"))]
use super::{WIN_BTN_CLOSE_HOVER, WIN_BTN_HOVER, WIN_BTN_ICON};

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
            tab_math::point_in_rect(mouse_pos.0, mouse_pos.1, (pin_x, pin_y, pin_w, pin_h));

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

        let cx = pin_x as f32 + pin_w as f32 / 2.0;
        let cy = pin_y as f32 + pin_h as f32 / 2.0;
        let layout = ui_layout::pin_icon_layout(
            cx,
            cy,
            self.ui_scale() as f32,
            pinned,
            is_hovered,
            PIN_ACTIVE_COLOR,
            TAB_TEXT_ACTIVE,
            TAB_TEXT_INACTIVE,
        );

        self.draw_pin_icon(buffer, buf_width, buf_height, &layout);
    }

    /// Draws a Bootstrap-style vertical pushpin icon from pre-computed layout.
    #[cfg(not(target_os = "macos"))]
    pub(super) fn draw_pin_icon(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        layout: &ui_layout::PinIconLayout,
    ) {
        let color = layout.color;

        // Helper to draw filled rect from (x, y, w, h) in f32.
        let draw_rect = |buf: &mut [u32], r: (f32, f32, f32, f32)| {
            let rx = r.0 as i32;
            let ry = r.1 as i32;
            let rw = r.2 as i32;
            let rh = r.3 as i32;
            for py in ry.max(0)..(ry + rh).min(buf_height as i32) {
                for px in rx.max(0)..(rx + rw).min(buf_width as i32) {
                    let idx = py as usize * buf_width + px as usize;
                    if idx < buf.len() {
                        buf[idx] = color;
                    }
                }
            }
        };

        draw_rect(buffer, layout.head);
        draw_rect(buffer, layout.body);
        draw_rect(buffer, layout.platform);

        // Needle (thin line).
        let (x0, y0, x1, y1) = layout.needle;
        Self::draw_stroked_line(
            buffer,
            buf_width,
            buf_height,
            x0,
            y0,
            x1,
            y1,
            layout.needle_thickness,
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
        let btn_w = self.scaled_px(tab_math::WIN_BTN_WIDTH);

        let buttons = ui_layout::window_buttons_layout(
            buf_width as u32,
            bar_h,
            btn_w,
            mouse_pos,
        );

        for btn in &buttons {
            // Hover background.
            if btn.hovered {
                let hover_bg = if btn.kind == ui_layout::WindowButtonKind::Close {
                    WIN_BTN_CLOSE_HOVER
                } else {
                    WIN_BTN_HOVER
                };
                for py in 0..btn.h as usize {
                    for px in btn.x as usize..(btn.x + btn.w) as usize {
                        if px < buf_width && py < buf_height {
                            let idx = py * buf_width + px;
                            if idx < buffer.len() {
                                buffer[idx] = hover_bg;
                            }
                        }
                    }
                }
            }

            let icon_color =
                if btn.hovered && btn.kind == ui_layout::WindowButtonKind::Close {
                    0xFFFFFF
                } else {
                    WIN_BTN_ICON
                };

            let center_x = btn.x as f32 + btn.w as f32 / 2.0;
            let center_y = btn.h as f32 / 2.0;
            let thickness = (1.25_f32 * self.ui_scale() as f32).clamp(1.15, 2.2);

            match btn.kind {
                ui_layout::WindowButtonKind::Minimize => {
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
                ui_layout::WindowButtonKind::Maximize => {
                    let half = self.scaled_px(5) as f32;
                    let x0 = center_x - half;
                    let y0 = center_y - half;
                    let x1 = center_x + half;
                    let y1 = center_y + half;
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height, x0, y0, x1, y0, thickness, icon_color,
                    );
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height, x0, y1, x1, y1, thickness, icon_color,
                    );
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height, x0, y0, x0, y1, thickness, icon_color,
                    );
                    Self::draw_stroked_line(
                        buffer, buf_width, buf_height, x1, y0, x1, y1, thickness, icon_color,
                    );
                }
                ui_layout::WindowButtonKind::Close => {
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
